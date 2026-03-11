//! Site list loading from CSV files.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A site entry from the input CSV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub url: String,
    pub category: String,
    pub language: String,
}

/// Maximum number of sites to load across all CSV files.
const MAX_SITES: usize = 100_000;

/// Load sites from one or more CSV files.
///
/// CSV format: `url,category,language`
///
/// Returns an error if the total number of sites exceeds [`MAX_SITES`].
pub fn load_sites(paths: &[impl AsRef<Path>]) -> anyhow::Result<Vec<Site>> {
    let mut sites = Vec::new();
    for path in paths {
        let file_path = path.as_ref();
        let mut rdr = csv::Reader::from_path(file_path)
            .map_err(|e| anyhow::anyhow!("{}: {e}", file_path.display()))?;
        for (i, result) in rdr.deserialize().enumerate() {
            let site: Site =
                result.map_err(|e| anyhow::anyhow!("{}:{}: {e}", file_path.display(), i + 2))?;
            sites.push(site);
            if sites.len() > MAX_SITES {
                anyhow::bail!(
                    "too many sites: exceeded limit of {MAX_SITES} (at {}:{})",
                    file_path.display(),
                    i + 2
                );
            }
        }
    }
    Ok(sites)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_sites_from_csv() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sites.csv");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "url,category,language").unwrap();
        writeln!(f, "https://example.com,news,en").unwrap();
        writeln!(f, "https://example.co.jp,news,ja").unwrap();

        let sites = load_sites(&[&path]).unwrap();
        assert_eq!(sites.len(), 2);
        assert_eq!(sites[0].url, "https://example.com");
        assert_eq!(sites[0].category, "news");
        assert_eq!(sites[0].language, "en");
        assert_eq!(sites[1].language, "ja");
    }
}
