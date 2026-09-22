use clap::{Parser, Subcommand};
use gateway_core::BuildInfo;

#[derive(Parser)]
#[command(name = "cyphtctl", about = "CLI for Cypht Gateway")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show the local CLI build version.
    Version,
    /// Query a running gateway health/version endpoint.
    Status {
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        url: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        Commands::Version => {
            let info = BuildInfo::current();
            println!("cyphtctl {}", info.version);
            println!("git: {}", info.git_short_sha);
            if let Some(tag) = info.git_tag {
                println!("tag: {tag}");
            }
            println!("dirty: {}", info.git_dirty);
        }
        Commands::Status { url } => {
            let endpoint = format!("{}/api/v1/meta/version", url.trim_end_matches('/'));
            let response = reqwest::get(endpoint).await?.error_for_status()?;
            let body: serde_json::Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&body)?);
        }
    }
    Ok(())
}
