use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

mod analyzer;
mod crawler;
mod report;
mod robots;
mod sites;

#[derive(Parser)]
#[command(
    name = "elidex-crawler",
    about = "Web compatibility survey tool for elidex",
    long_about = "Crawls websites and detects legacy HTML, CSS, and JavaScript features.\n\n\
        Use the `crawl` subcommand to fetch sites and detect features,\n\
        then `analyze` to generate summary reports from the results.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Crawl websites and detect legacy features
    ///
    /// Reads site URLs from one or more CSV files, fetches each page,
    /// and detects legacy HTML tags, CSS vendor prefixes, and JS patterns.
    /// Results are written to the output directory as JSON and CSV files.
    Crawl {
        /// Path(s) to site list CSV files (url,category,language)
        #[arg(long = "sites", required = true, value_name = "FILE")]
        site_files: Vec<PathBuf>,

        /// Output directory for results
        #[arg(long, default_value = "output", value_name = "DIR")]
        output: PathBuf,

        /// Maximum concurrent requests (1-1000)
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u16).range(1..=1000))]
        concurrency: u16,

        /// Per-site timeout in seconds (1-300)
        #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=300))]
        timeout: u64,
    },
    /// Analyze crawl results and generate summary reports
    ///
    /// Reads a results.json file from a previous crawl and produces
    /// per-feature CSV summaries in the output directory.
    Analyze {
        /// Path to results.json from a previous crawl
        #[arg(long, value_name = "FILE")]
        input: PathBuf,

        /// Output directory for analysis reports
        #[arg(long, default_value = "output", value_name = "DIR")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Crawl {
            site_files,
            output,
            concurrency,
            timeout,
        } => {
            let sites = sites::load_sites(&site_files)?;
            tracing::info!("Loaded {} sites", sites.len());

            let config = crawler::CrawlConfig {
                concurrency: usize::from(concurrency),
                timeout_secs: timeout,
                retries: 1,
                user_agent: "elidex-crawler/0.1".to_string(),
            };

            let crawl_output = crawler::crawl_sites(&sites, &config).await?;
            let results = crawl_output.results;
            tracing::info!(
                "Crawled {} sites ({} successful, {} panicked)",
                results.len(),
                results.iter().filter(|r| r.error.is_none()).count(),
                crawl_output.panicked,
            );

            std::fs::create_dir_all(&output)?;
            report::write_results(&results, &output)?;
            tracing::info!("Results written to {}", output.display());
        }
        Command::Analyze { input, output } => {
            let data = std::fs::read_to_string(&input)?;
            let results: Vec<crawler::SiteResult> = serde_json::from_str(&data)?;
            tracing::info!("Loaded {} site results", results.len());

            std::fs::create_dir_all(&output)?;
            report::write_analysis(&results, &output)?;
            tracing::info!("Analysis written to {}", output.display());
        }
    }

    Ok(())
}
