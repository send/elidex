//! Report generation in JSON and CSV formats.

use crate::analyzer::FeatureCount;
use crate::crawler::SiteResult;
use std::collections::HashMap;
use std::path::Path;

/// Sort a feature count map by count descending.
fn sort_by_count_desc(map: FeatureCount) -> Vec<(String, usize)> {
    let mut items: Vec<_> = map.into_iter().collect();
    items.sort_by_key(|item| std::cmp::Reverse(item.1));
    items
}

/// CSV header for feature aggregation reports.
const FEATURE_CSV_HEADER: &[&str] = &[
    "type",
    "name",
    "total_count",
    "site_count",
    "site_percentage",
];

/// Write full crawl results to the output directory.
///
/// Produces:
/// - `results.json` — full detailed results
/// - `summary.csv` — per-site summary
/// - `html-features.csv` — HTML feature usage rates
/// - `css-features.csv` — CSS feature usage rates
/// - `parser-errors.csv` — parser error classification
pub fn write_results(results: &[SiteResult], output: &Path) -> anyhow::Result<()> {
    // results.json
    let json = serde_json::to_string_pretty(results)?;
    std::fs::write(output.join("results.json"), json)?;

    // summary.csv
    write_summary_csv(results, output)?;

    // Feature CSVs and parser errors
    write_analysis(results, output)?;

    Ok(())
}

/// Write analysis reports from existing results.
pub fn write_analysis(results: &[SiteResult], output: &Path) -> anyhow::Result<()> {
    write_html_features_csv(results, output)?;
    write_css_features_csv(results, output)?;
    write_parser_errors_csv(results, output)?;
    Ok(())
}

fn write_summary_csv(results: &[SiteResult], output: &Path) -> anyhow::Result<()> {
    let mut wtr = csv::Writer::from_path(output.join("summary.csv"))?;
    wtr.write_record([
        "url",
        "category",
        "language",
        "status_code",
        "error",
        "deprecated_tags_count",
        "deprecated_attrs_count",
        "vendor_prefix_count",
        "parser_error_count",
        "uses_document_write",
    ])?;

    for r in results {
        let dep_tags: usize = r.html_features.deprecated_tags.values().sum();
        let dep_attrs: usize = r.html_features.deprecated_attrs.values().sum();
        let vendor: usize = r.css_features.vendor_prefixes.values().sum();

        wtr.write_record([
            &r.url,
            &r.category,
            &r.language,
            &r.status_code.map_or("-".to_string(), |c| c.to_string()),
            r.error.as_deref().unwrap_or(""),
            &dep_tags.to_string(),
            &dep_attrs.to_string(),
            &vendor.to_string(),
            &r.parser_errors.len().to_string(),
            &r.html_features.uses_document_write.to_string(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

/// Aggregated feature counts with per-site usage tracking.
struct FeatureAggregator {
    counts: FeatureCount,
    site_counts: FeatureCount,
}

impl FeatureAggregator {
    fn new() -> Self {
        Self {
            counts: HashMap::new(),
            site_counts: HashMap::new(),
        }
    }

    /// Add one site's feature counts into the aggregator.
    fn add_site(&mut self, features: &FeatureCount) {
        for (name, count) in features {
            *self.counts.entry(name.clone()).or_default() += count;
            *self.site_counts.entry(name.clone()).or_default() += 1;
        }
    }

    /// Write aggregated rows to a CSV writer, sorted by total count descending.
    #[allow(clippy::cast_precision_loss)]
    fn write_csv(
        self,
        wtr: &mut csv::Writer<std::fs::File>,
        type_name: &str,
        total_sites: usize,
    ) -> anyhow::Result<()> {
        let items = sort_by_count_desc(self.counts);
        for (name, count) in &items {
            let sites = self.site_counts.get(name).copied().unwrap_or(0);
            let pct = if total_sites > 0 {
                (sites as f64 / total_sites as f64) * 100.0
            } else {
                0.0
            };
            wtr.write_record([
                type_name,
                name,
                &count.to_string(),
                &sites.to_string(),
                &format!("{pct:.1}"),
            ])?;
        }
        Ok(())
    }
}

fn write_html_features_csv(results: &[SiteResult], output: &Path) -> anyhow::Result<()> {
    let mut tags = FeatureAggregator::new();
    let mut attrs = FeatureAggregator::new();

    for r in results {
        tags.add_site(&r.html_features.deprecated_tags);
        attrs.add_site(&r.html_features.deprecated_attrs);
    }

    let mut wtr = csv::Writer::from_path(output.join("html-features.csv"))?;
    wtr.write_record(FEATURE_CSV_HEADER)?;

    tags.write_csv(&mut wtr, "deprecated_tag", results.len())?;
    attrs.write_csv(&mut wtr, "deprecated_attr", results.len())?;

    wtr.flush()?;
    Ok(())
}

fn write_css_features_csv(results: &[SiteResult], output: &Path) -> anyhow::Result<()> {
    let mut prefixes = FeatureAggregator::new();
    let mut nonstandard = FeatureAggregator::new();
    let mut aliases = FeatureAggregator::new();

    for r in results {
        prefixes.add_site(&r.css_features.vendor_prefixes);
        nonstandard.add_site(&r.css_features.non_standard_properties);
        aliases.add_site(&r.css_features.aliased_properties);
    }

    let mut wtr = csv::Writer::from_path(output.join("css-features.csv"))?;
    wtr.write_record(FEATURE_CSV_HEADER)?;

    prefixes.write_csv(&mut wtr, "vendor_prefix", results.len())?;
    nonstandard.write_csv(&mut wtr, "non_standard", results.len())?;
    aliases.write_csv(&mut wtr, "aliased", results.len())?;

    wtr.flush()?;
    Ok(())
}

fn write_parser_errors_csv(results: &[SiteResult], output: &Path) -> anyhow::Result<()> {
    let mut error_counts: FeatureCount = HashMap::new();

    for r in results {
        for err in &r.parser_errors {
            *error_counts.entry(err.clone()).or_default() += 1;
        }
    }

    let mut wtr = csv::Writer::from_path(output.join("parser-errors.csv"))?;
    wtr.write_record(["error", "count"])?;

    let errors = sort_by_count_desc(error_counts);
    for (err, count) in &errors {
        wtr.write_record([err.as_str(), &count.to_string()])?;
    }

    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer;
    use tempfile::TempDir;

    fn sample_results() -> Vec<SiteResult> {
        let mut html_features = analyzer::html::HtmlFeatures::default();
        html_features
            .deprecated_tags
            .insert("center".to_string(), 2);
        html_features
            .deprecated_attrs
            .insert("bgcolor".to_string(), 1);
        html_features.uses_document_write = true;

        let mut css_features = analyzer::css::CssFeatures::default();
        css_features
            .vendor_prefixes
            .insert("-webkit-".to_string(), 5);

        vec![SiteResult {
            url: "https://example.com".to_string(),
            category: "news".to_string(),
            language: "en".to_string(),
            status_code: Some(200),
            error: None,
            html_features,
            css_features,
            js_features: analyzer::js::JsFeatures::default(),
            parser_errors: vec!["Unexpected token".to_string()],
        }]
    }

    #[test]
    fn write_results_creates_files() {
        let dir = TempDir::new().unwrap();
        let results = sample_results();
        write_results(&results, dir.path()).unwrap();

        assert!(dir.path().join("results.json").exists());
        assert!(dir.path().join("summary.csv").exists());
        assert!(dir.path().join("html-features.csv").exists());
        assert!(dir.path().join("css-features.csv").exists());
        assert!(dir.path().join("parser-errors.csv").exists());
    }

    #[test]
    fn summary_csv_content() {
        let dir = TempDir::new().unwrap();
        let results = sample_results();
        write_results(&results, dir.path()).unwrap();

        let content = std::fs::read_to_string(dir.path().join("summary.csv")).unwrap();
        assert!(content.contains("https://example.com"));
        assert!(content.contains("200"));
    }
}
