//! Service Worker Static Router with URLPattern matching.
//!
//! Implements the WHATWG URLPattern spec (urlpattern.spec.whatwg.org) for
//! route condition matching, and the SW Static Router API (SW §8.4) for
//! routing decisions before FetchEvent dispatch.

use std::collections::HashMap;

/// The source to use for a matched route (WHATWG SW §8.4 `RouterSourceEnum`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterSource {
    /// Dispatch to the SW thread as a FetchEvent (default behavior).
    FetchEvent,
    /// Skip the SW entirely and go directly to network.
    Network,
    /// Read directly from a named cache.
    Cache(String),
    /// Race network fetch and SW fetch handler; first response wins.
    RaceNetworkAndFetchHandler,
}

/// A compiled URLPattern (urlpattern.spec.whatwg.org).
///
/// Matches URL components (pathname, hostname, etc.) against patterns with
/// `:name` named groups, `*` wildcards, and literal segments.
#[derive(Debug, Clone)]
pub struct UrlPattern {
    /// Compiled regex for pathname matching.
    pathname_re: regex::Regex,
    /// Compiled regex for hostname matching (None = match any).
    hostname_re: Option<regex::Regex>,
    /// Named group names in order (for exec() result).
    group_names: Vec<String>,
}

impl UrlPattern {
    /// Create a URLPattern from a pathname pattern string.
    ///
    /// Pattern syntax (subset of urlpattern.spec.whatwg.org):
    /// - `:name` — named group (matches a single path segment: `[^/]+`)
    /// - `*` — wildcard (matches any characters: `.*`)
    /// - Literal characters are matched exactly.
    ///
    /// Examples:
    /// - `/api/:version/users/:id` — matches `/api/v1/users/42`
    /// - `/static/*` — matches `/static/css/style.css`
    /// - `/page` — exact match for `/page`
    pub fn pathname(pattern: &str) -> Result<Self, String> {
        let (re_str, groups) = compile_pattern(pattern);
        let pathname_re = regex::Regex::new(&format!("^{re_str}$"))
            .map_err(|e| format!("invalid pathname pattern '{pattern}': {e}"))?;
        Ok(Self {
            pathname_re,
            hostname_re: None,
            group_names: groups,
        })
    }

    /// Create a URLPattern matching both hostname and pathname.
    pub fn hostname_and_pathname(hostname: &str, pathname: &str) -> Result<Self, String> {
        let (path_re, groups) = compile_pattern(pathname);
        let (host_re, _host_groups) = compile_pattern(hostname);
        let pathname_re = regex::Regex::new(&format!("^{path_re}$"))
            .map_err(|e| format!("invalid pathname pattern '{pathname}': {e}"))?;
        let hostname_re = regex::Regex::new(&format!("^{host_re}$"))
            .map_err(|e| format!("invalid hostname pattern '{hostname}': {e}"))?;
        // Only pathname groups are captured by exec(); hostname matching is boolean.
        Ok(Self {
            pathname_re,
            hostname_re: Some(hostname_re),
            group_names: groups,
        })
    }

    /// Test if a URL matches this pattern (urlpattern.spec.whatwg.org `test()`).
    ///
    /// Accepts a pre-parsed `url::Url` to avoid redundant parsing on hot paths.
    pub fn test(&self, url: &url::Url) -> bool {
        if let Some(ref host_re) = self.hostname_re {
            let host = url.host_str().unwrap_or("");
            if !host_re.is_match(host) {
                return false;
            }
        }
        self.pathname_re.is_match(url.path())
    }

    /// Convenience: test against a URL string. Parses the URL first.
    pub fn test_str(&self, url: &str) -> bool {
        let Ok(parsed) = url::Url::parse(url) else {
            return false;
        };
        self.test(&parsed)
    }

    /// Convenience: execute against a URL string.
    pub fn exec_str(&self, url: &str) -> Option<HashMap<String, String>> {
        let parsed = url::Url::parse(url).ok()?;
        self.exec(&parsed)
    }

    /// Execute pattern matching and return captured groups.
    ///
    /// Returns `None` if the URL doesn't match. Returns `Some(groups)` with
    /// named group values if it matches.
    pub fn exec(&self, url: &url::Url) -> Option<HashMap<String, String>> {
        if let Some(ref host_re) = self.hostname_re {
            let host = url.host_str().unwrap_or("");
            if !host_re.is_match(host) {
                return None;
            }
        }
        let caps = self.pathname_re.captures(url.path())?;
        let mut groups = HashMap::new();
        for (i, name) in self.group_names.iter().enumerate() {
            if let Some(m) = caps.get(i + 1) {
                groups.insert(name.clone(), m.as_str().to_owned());
            }
        }
        Some(groups)
    }
}

/// Router condition using URLPattern (WHATWG SW §8.4).
#[derive(Debug, Clone)]
pub enum RouterCondition {
    /// Match using a URLPattern on the pathname component.
    UrlPattern(UrlPattern),
}

impl RouterCondition {
    /// Check if a URL matches this condition.
    pub fn matches(&self, url: &url::Url) -> bool {
        match self {
            RouterCondition::UrlPattern(pattern) => pattern.test(url),
        }
    }
}

/// A single routing rule: condition → source.
#[derive(Debug, Clone)]
pub struct RouterRule {
    pub condition: RouterCondition,
    pub source: RouterSource,
}

/// Evaluate a list of router rules against a request URL.
///
/// Returns the source of the first matching rule, or `None` if no rules match
/// (which means the default FetchEvent path should be used).
pub fn evaluate_routes<'a>(rules: &'a [RouterRule], url: &url::Url) -> Option<&'a RouterSource> {
    rules
        .iter()
        .find(|rule| rule.condition.matches(url))
        .map(|rule| &rule.source)
}

// --- Pattern compilation ---

/// Compile a URLPattern component string to a regex + named groups.
///
/// Implements the core subset of the URLPattern pattern syntax:
/// - `:name` → named capturing group `([^/]+)`
/// - `*` → wildcard `(.*)`
/// - Literal characters are escaped for regex safety.
fn compile_pattern(pattern: &str) -> (String, Vec<String>) {
    let mut regex = String::new();
    let mut groups = Vec::new();
    let mut chars = pattern.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            ':' => {
                // Named group: read the name (alphanumeric + underscore).
                let mut name = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc.is_alphanumeric() || nc == '_' {
                        name.push(nc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if name.is_empty() {
                    // Bare `:` — treat as literal.
                    regex.push_str(&regex::escape(":"));
                } else {
                    groups.push(name);
                    regex.push_str("([^/]+)");
                }
            }
            '*' => {
                groups.push("0".to_string());
                regex.push_str("(.*)");
            }
            _ => {
                regex.push_str(&regex::escape(&c.to_string()));
            }
        }
    }

    (regex, groups)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> url::Url {
        url::Url::parse(s).unwrap()
    }

    #[test]
    fn pathname_exact_match() {
        let pattern = UrlPattern::pathname("/page").unwrap();
        assert!(pattern.test_str("https://example.com/page"));
        assert!(!pattern.test_str("https://example.com/page/extra"));
        assert!(!pattern.test_str("https://example.com/other"));
    }

    #[test]
    fn pathname_named_group() {
        let pattern = UrlPattern::pathname("/api/:version/users/:id").unwrap();
        assert!(pattern.test_str("https://example.com/api/v1/users/42"));
        assert!(pattern.test_str("https://example.com/api/v2/users/abc"));
        assert!(!pattern.test_str("https://example.com/api/v1/posts/42"));
        assert!(!pattern.test_str("https://example.com/api/v1/users/"));
    }

    #[test]
    fn pathname_wildcard() {
        let pattern = UrlPattern::pathname("/static/*").unwrap();
        assert!(pattern.test_str("https://example.com/static/css/style.css"));
        assert!(pattern.test_str("https://example.com/static/"));
        assert!(!pattern.test_str("https://example.com/api/data"));
    }

    #[test]
    fn pathname_exec_captures() {
        let pattern = UrlPattern::pathname("/users/:id/posts/:post_id").unwrap();
        let groups = pattern
            .exec_str("https://example.com/users/42/posts/100")
            .unwrap();
        assert_eq!(groups.get("id").unwrap(), "42");
        assert_eq!(groups.get("post_id").unwrap(), "100");
    }

    #[test]
    fn exec_no_match_returns_none() {
        let pattern = UrlPattern::pathname("/users/:id").unwrap();
        assert!(pattern.exec_str("https://example.com/posts/1").is_none());
    }

    #[test]
    fn hostname_and_pathname() {
        let pattern = UrlPattern::hostname_and_pathname("cdn.example.com", "/assets/*").unwrap();
        assert!(pattern.test_str("https://cdn.example.com/assets/img/logo.png"));
        assert!(!pattern.test_str("https://api.example.com/assets/data.json"));
        assert!(!pattern.test_str("https://cdn.example.com/other/file.txt"));
    }

    #[test]
    fn evaluate_routes_with_url_pattern() {
        let rules = vec![
            RouterRule {
                condition: RouterCondition::UrlPattern(UrlPattern::pathname("/static/*").unwrap()),
                source: RouterSource::Network,
            },
            RouterRule {
                condition: RouterCondition::UrlPattern(UrlPattern::pathname("/api/*").unwrap()),
                source: RouterSource::Cache("api-v1".into()),
            },
            RouterRule {
                condition: RouterCondition::UrlPattern(UrlPattern::pathname("/*").unwrap()),
                source: RouterSource::FetchEvent,
            },
        ];

        assert_eq!(
            evaluate_routes(&rules, &url("https://example.com/static/lib.js")),
            Some(&RouterSource::Network)
        );
        assert_eq!(
            evaluate_routes(&rules, &url("https://example.com/api/data")),
            Some(&RouterSource::Cache("api-v1".into()))
        );
        assert_eq!(
            evaluate_routes(&rules, &url("https://example.com/page")),
            Some(&RouterSource::FetchEvent)
        );
    }

    #[test]
    fn literal_colon_and_special_chars() {
        let pattern = UrlPattern::pathname("/path/with.dots/and-dashes").unwrap();
        assert!(pattern.test_str("https://example.com/path/with.dots/and-dashes"));
        assert!(!pattern.test_str("https://example.com/path/with_dots/and-dashes"));
    }

    #[test]
    fn empty_pattern_matches_root() {
        let pattern = UrlPattern::pathname("/").unwrap();
        assert!(pattern.test_str("https://example.com/"));
        assert!(!pattern.test_str("https://example.com/page"));
    }
}
