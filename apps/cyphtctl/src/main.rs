use clap::{Parser, Subcommand};
use gateway_core::{BuildInfo, CreateTokenRequest, LoginRequest, SearchRequest};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;

#[derive(Parser)]
#[command(name = "cyphtctl", about = "CLI for Cypht Gateway")]
struct Cli {
    #[arg(long, env = "CYPHT_GATEWAY_URL", default_value = "http://127.0.0.1:8080")]
    url: String,
    #[arg(long, env = "CYPHT_GATEWAY_TOKEN")]
    token: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Version,
    Status,
    Login {
        #[arg(long)] username: String,
        #[arg(long, env = "CYPHT_PASSWORD")] password: String,
    },
    Me,
    Accounts,
    Mailboxes {
        #[arg(long)] account: String,
    },
    Inbox {
        #[arg(long)] account: Option<String>,
        #[arg(long, default_value_t = 50)] limit: u32,
    },
    Search {
        query: String,
        #[arg(long)] account: Vec<String>,
        #[arg(long, default_value_t = 50)] limit: u32,
    },
    Read { id: String },
    Tokens,
    TokenCreate {
        #[arg(long)] name: String,
        #[arg(long = "scope")] scopes: Vec<String>,
        #[arg(long = "account")] accounts: Vec<String>,
        #[arg(long)] days: Option<u32>,
    },
    TokenRevoke { id: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let base = cli.url.trim_end_matches('/');
    let client = reqwest::Client::new();

    match cli.command {
        Commands::Version => {
            let info = BuildInfo::current();
            println!("cyphtctl {}", info.version);
            println!("git: {}", info.git_short_sha);
            if let Some(tag) = info.git_tag { println!("tag: {tag}"); }
            println!("dirty: {}", info.git_dirty);
        }
        Commands::Status => print_json(client.get(format!("{base}/api/v1/meta/version")).send().await?).await?,
        Commands::Login { username, password } => {
            let response = client.post(format!("{base}/api/v1/auth/login"))
                .header(CONTENT_TYPE, "application/json")
                .json(&LoginRequest { username, password })
                .send().await?;
            print_json(response).await?;
        }
        Commands::Me => print_json(auth(&cli.token, client.get(format!("{base}/api/v1/me")))?.send().await?).await?,
        Commands::Accounts => print_json(auth(&cli.token, client.get(format!("{base}/api/v1/accounts")))?.send().await?).await?,
        Commands::Mailboxes { account } => {
            let url = format!("{base}/api/v1/accounts/{account}/mailboxes");
            print_json(auth(&cli.token, client.get(url))?.send().await?).await?;
        }
        Commands::Inbox { account, limit } => {
            let mut url = format!("{base}/api/v1/messages?limit={limit}");
            if let Some(account) = account { url.push_str("&account_id="); url.push_str(&account); }
            print_json(auth(&cli.token, client.get(url))?.send().await?).await?;
        }
        Commands::Search { query, account, limit } => {
            let request = SearchRequest { account_ids: account, mailbox_id: None, query, limit: Some(limit) };
            print_json(auth(&cli.token, client.post(format!("{base}/api/v1/messages/search")).json(&request))?.send().await?).await?;
        }
        Commands::Read { id } => {
            print_json(auth(&cli.token, client.get(format!("{base}/api/v1/messages/{id}")))?.send().await?).await?;
        }
        Commands::Tokens => print_json(auth(&cli.token, client.get(format!("{base}/api/v1/tokens")))?.send().await?).await?,
        Commands::TokenCreate { name, scopes, accounts, days } => {
            let request = CreateTokenRequest {
                name,
                scopes,
                account_allowlist: accounts,
                expires_in_days: days,
            };
            let response = auth(&cli.token, client.post(format!("{base}/api/v1/tokens")).json(&request))?
                .send().await?;
            print_json(response).await?;
        }
        Commands::TokenRevoke { id } => {
            let response = auth(&cli.token, client.delete(format!("{base}/api/v1/tokens/{id}")))?
                .send().await?;
            if response.status().is_success() {
                println!("revoked {id}");
            } else {
                print_json(response).await?;
            }
        }
    }
    Ok(())
}

fn auth(token: &Option<String>, request: reqwest::RequestBuilder) -> Result<reqwest::RequestBuilder, Box<dyn std::error::Error>> {
    let token = token.as_deref().ok_or("this command requires --token or CYPHT_GATEWAY_TOKEN")?;
    Ok(request.header(AUTHORIZATION, format!("Bearer {token}")))
}

async fn print_json(response: reqwest::Response) -> Result<(), Box<dyn std::error::Error>> {
    let status = response.status();
    let body: Value = response.json().await.unwrap_or_else(|_| serde_json::json!({"error":"invalid JSON response"}));
    println!("{}", serde_json::to_string_pretty(&body)?);
    if !status.is_success() { return Err(format!("request failed with HTTP {status}").into()); }
    Ok(())
}
