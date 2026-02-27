//! Simple robots.txt parser and checker.
//!
//! Implements a subset of the Robots Exclusion Protocol (RFC 9309):
//! - Parses `User-agent`, `Allow`, and `Disallow` directives
//! - Matches against our crawler's user-agent
//! - More specific (longer) path rules take precedence per RFC 9309
//! - Respects `Crawl-delay` if present

use std::collections::HashMap;
use std::time::Duration;

/// A single path rule parsed from robots.txt.
#[derive(Debug, Clone)]
struct PathRule {
    path: String,
    allow: bool,
}

/// Parsed robots.txt rules for a single origin.
#[derive(Debug, Clone, Default)]
pub struct RobotsRules {
    /// Path rules per user-agent (lowercased).
    rules: HashMap<String, Vec<PathRule>>,
    /// Crawl delay per user-agent (lowercased).
    crawl_delay: HashMap<String, Duration>,
}

const WILDCARD: &str = "*";

/// Maximum number of path rules to parse (prevents memory amplification
/// from adversarial robots.txt files).
const MAX_RULES: usize = 10_000;

impl RobotsRules {
    /// Parse a robots.txt body.
    pub fn parse(body: &str) -> Self {
        let mut result = Self::default();
        let mut current_agents: Vec<String> = Vec::new();
        let mut total_rules = 0usize;

        for line in body.lines() {
            let line = line.trim();

            // Strip inline comments.
            let line = if let Some(pos) = line.find('#') {
                line[..pos].trim()
            } else {
                line
            };
            if line.is_empty() {
                continue;
            }

            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();

            match key.as_str() {
                "user-agent" => {
                    let agent = value.to_ascii_lowercase();
                    // If we were collecting rules and hit a new user-agent block,
                    // start fresh.
                    if current_agents.is_empty()
                        || result
                            .rules
                            .contains_key(current_agents.last().unwrap_or(&String::new()))
                    {
                        current_agents.clear();
                    }
                    current_agents.push(agent);
                }
                "disallow" | "allow" => {
                    let allow = key == "allow";
                    if !value.is_empty() && total_rules < MAX_RULES {
                        for agent in &current_agents {
                            result
                                .rules
                                .entry(agent.clone())
                                .or_default()
                                .push(PathRule {
                                    path: value.to_string(),
                                    allow,
                                });
                            total_rules += 1;
                        }
                    }
                }
                "crawl-delay" => {
                    if let Ok(secs) = value.parse::<f64>() {
                        if secs.is_finite() && secs >= 0.0 {
                            let dur = Duration::from_secs_f64(secs);
                            for agent in &current_agents {
                                result.crawl_delay.insert(agent.clone(), dur);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Ensure all declared agents have entries, even if they had no rules.
        // This lets find_matching_agent() detect them and prevent wildcard fallthrough.
        for agent in &current_agents {
            result.rules.entry(agent.clone()).or_default();
        }

        result
    }

    /// Find the best matching key in a set of user-agent names.
    ///
    /// Per RFC 9309 §2.2.1, user-agent matching uses a case-insensitive
    /// substring match. The most specific (longest matching) agent name wins.
    /// Falls back to wildcard only if the key set contains `*`.
    fn best_agent_match<'a>(
        keys: impl Iterator<Item = &'a String>,
        user_agent: &str,
    ) -> Option<&'a str> {
        let ua_lower = user_agent.to_ascii_lowercase();
        let mut best: Option<&str> = None;
        let mut has_wildcard = false;
        for key in keys {
            if key == WILDCARD {
                has_wildcard = true;
                continue;
            }
            if ua_lower.contains(key.as_str()) && best.is_none_or(|prev| key.len() > prev.len()) {
                best = Some(key.as_str());
            }
        }
        best.or(if has_wildcard { Some(WILDCARD) } else { None })
    }

    /// Find the best matching user-agent group for path rules.
    fn find_matching_agent(&self, user_agent: &str) -> Option<&str> {
        Self::best_agent_match(self.rules.keys(), user_agent)
    }

    /// Check if a path is allowed for the given user-agent.
    ///
    /// Per RFC 9309, the most specific (longest matching) rule wins.
    /// If two rules have the same length, `Allow` takes precedence.
    /// If a specific user-agent group exists, only that group is used
    /// (no fallthrough to wildcard).
    pub fn is_allowed(&self, user_agent: &str, path: &str) -> bool {
        let Some(agent_key) = self.find_matching_agent(user_agent) else {
            return true; // No matching group at all.
        };
        let Some(rules) = self.rules.get(agent_key) else {
            return true;
        };

        // Find the longest matching rule.
        let mut best_len = 0;
        let mut best_allow = true; // Default: allowed.

        for rule in rules {
            if path.starts_with(&rule.path) && rule.path.len() >= best_len {
                // Longer path wins; on tie, Allow beats Disallow.
                if rule.path.len() > best_len || rule.allow {
                    best_len = rule.path.len();
                    best_allow = rule.allow;
                }
            }
        }

        if best_len > 0 {
            return best_allow;
        }
        true
    }

    /// Get the crawl delay for the given user-agent.
    ///
    /// Uses the same substring matching as `is_allowed`.
    pub fn crawl_delay(&self, user_agent: &str) -> Option<Duration> {
        let agent = Self::best_agent_match(self.crawl_delay.keys(), user_agent)?;
        self.crawl_delay.get(agent).copied()
    }
}

/// Maximum size for robots.txt body (512 KB).
const MAX_ROBOTS_BYTES: usize = 512 * 1024;

/// Timeout for reading the robots.txt response body.
const ROBOTS_BODY_TIMEOUT: Duration = Duration::from_secs(5);

/// Build the authority string (host + optional port) from a URL.
fn build_authority(url: &reqwest::Url) -> Option<String> {
    let host = url.host_str()?;
    Some(match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    })
}

/// Fetch and parse robots.txt for a given URL's origin.
///
/// The caller's HTTP client should already have a redirect policy that
/// validates each hop (see `crawl_sites`). This function additionally
/// validates the final response URL to guard against open redirects.
pub async fn fetch_robots(client: &reqwest::Client, site_url: &str) -> Option<RobotsRules> {
    let url = reqwest::Url::parse(site_url).ok()?;
    let authority = build_authority(&url)?;
    let robots_url = format!("{}://{authority}/robots.txt", url.scheme());

    match client.get(&robots_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            // Validate final URL after redirects.
            if crate::crawler::validate_url(resp.url()).is_err() {
                return None;
            }
            // Read body with timeout and size limit.
            let body = tokio::time::timeout(ROBOTS_BODY_TIMEOUT, resp.text())
                .await
                .ok()?
                .ok()?;
            if body.len() > MAX_ROBOTS_BYTES {
                return None;
            }
            Some(RobotsRules::parse(&body))
        }
        // If robots.txt is missing or errors, assume everything is allowed.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_robots() {
        let body = "\
User-agent: *
Disallow: /admin
Disallow: /private/

User-agent: elidex-crawler
Disallow: /secret
";
        let rules = RobotsRules::parse(body);

        // Wildcard rules
        assert!(!rules.is_allowed("googlebot", "/admin"));
        assert!(!rules.is_allowed("googlebot", "/private/page"));
        assert!(rules.is_allowed("googlebot", "/public"));

        // Specific agent rules — per RFC 9309, when a specific group matches,
        // only that group is used (no fallthrough to wildcard).
        assert!(!rules.is_allowed("elidex-crawler", "/secret"));
        assert!(rules.is_allowed("elidex-crawler", "/admin")); // wildcard does NOT apply
        assert!(rules.is_allowed("elidex-crawler", "/public"));
    }

    #[test]
    fn parse_crawl_delay() {
        let body = "\
User-agent: *
Crawl-delay: 2
Disallow: /

User-agent: fast-bot
Crawl-delay: 0.5
";
        let rules = RobotsRules::parse(body);

        assert_eq!(rules.crawl_delay("slow-bot"), Some(Duration::from_secs(2)));
        assert_eq!(
            rules.crawl_delay("fast-bot"),
            Some(Duration::from_millis(500))
        );
    }

    #[test]
    fn empty_robots_allows_all() {
        let rules = RobotsRules::parse("");
        assert!(rules.is_allowed("elidex-crawler", "/anything"));
        assert_eq!(rules.crawl_delay("elidex-crawler"), None);
    }

    #[test]
    fn comments_ignored() {
        let body = "\
# This is a comment
User-agent: * # all bots
Disallow: /private # secret stuff
";
        let rules = RobotsRules::parse(body);
        assert!(!rules.is_allowed("bot", "/private"));
        assert!(rules.is_allowed("bot", "/public"));
    }

    #[test]
    fn allow_overrides_disallow_for_specific_path() {
        let body = "\
User-agent: *
Disallow: /private/
Allow: /private/public-page
";
        let rules = RobotsRules::parse(body);
        assert!(!rules.is_allowed("bot", "/private/secret"));
        assert!(rules.is_allowed("bot", "/private/public-page"));
        assert!(rules.is_allowed("bot", "/public"));
    }

    #[test]
    fn invalid_crawl_delay_ignored() {
        // Negative, NaN, and infinity values must not panic and must be ignored.
        let negative = "\
User-agent: *
Crawl-delay: -5
";
        let rules = RobotsRules::parse(negative);
        assert_eq!(rules.crawl_delay("bot"), None);

        let infinity = "\
User-agent: *
Crawl-delay: inf
";
        let rules = RobotsRules::parse(infinity);
        assert_eq!(rules.crawl_delay("bot"), None);

        let nan = "\
User-agent: *
Crawl-delay: NaN
";
        let rules = RobotsRules::parse(nan);
        assert_eq!(rules.crawl_delay("bot"), None);

        // Valid value still works.
        let valid = "\
User-agent: *
Crawl-delay: 2
";
        let rules = RobotsRules::parse(valid);
        assert_eq!(rules.crawl_delay("bot"), Some(Duration::from_secs(2)));
    }

    #[test]
    fn robots_url_includes_port() {
        // Verify that the authority construction logic handles ports correctly.
        let url = reqwest::Url::parse("https://example.com:8443/page").unwrap();
        assert_eq!(build_authority(&url).unwrap(), "example.com:8443");

        // Default port should not be included.
        let url = reqwest::Url::parse("https://example.com/page").unwrap();
        assert_eq!(build_authority(&url).unwrap(), "example.com");
    }

    #[test]
    fn specific_agent_empty_rules_allows_all() {
        // A specific agent group with no Disallow rules should allow everything,
        // and wildcard rules should NOT apply (per RFC 9309).
        let body = "\
User-agent: *
Disallow: /

User-agent: special-bot
";
        let rules = RobotsRules::parse(body);
        // Wildcard blocks everything.
        assert!(!rules.is_allowed("other-bot", "/page"));
        // special-bot has a specific group with no rules → everything allowed.
        assert!(rules.is_allowed("special-bot", "/page"));
        assert!(rules.is_allowed("special-bot", "/"));
    }

    #[test]
    fn disallow_root_blocks_all() {
        let body = "\
User-agent: *
Disallow: /
";
        let rules = RobotsRules::parse(body);
        assert!(!rules.is_allowed("bot", "/"));
        assert!(!rules.is_allowed("bot", "/anything"));
    }
}
