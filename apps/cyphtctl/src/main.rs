use clap::{Parser, Subcommand};
use gateway_core::{
    Address, BuildInfo, CreateTokenRequest, ForwardMessageRequest, LoginRequest, MessageBody,
    MessageUpdateRequest, MoveMessageRequest, ReplyMessageRequest, SearchRequest, SendMessageRequest,
};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;
use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

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
    Login { #[arg(long)] username: String, #[arg(long, env = "CYPHT_PASSWORD")] password: String },
    Me,
    Accounts,
    Profiles,
    Mailboxes { #[arg(long)] account: String },
    Inbox { #[arg(long)] account: Option<String>, #[arg(long, default_value_t = 50)] limit: u32 },
    Search { query: String, #[arg(long)] account: Vec<String>, #[arg(long, default_value_t = 50)] limit: u32 },
    Read { id: String },
    Attachment { id: String, #[arg(long)] out: PathBuf },
    Upload { file: PathBuf, #[arg(long)] content_type: Option<String> },
    Send {
        #[arg(long)] profile: Option<String>, #[arg(long)] to: Vec<String>, #[arg(long)] cc: Vec<String>,
        #[arg(long)] bcc: Vec<String>, #[arg(long, default_value = "")] subject: String,
        #[arg(long, default_value = "")] body: String, #[arg(long = "attachment")] attachments: Vec<String>,
        #[arg(long)] schedule_at: Option<String>, #[arg(long)] idempotency_key: Option<String>,
    },
    Draft {
        #[arg(long)] profile: Option<String>, #[arg(long)] to: Vec<String>, #[arg(long, default_value = "")] subject: String,
        #[arg(long, default_value = "")] body: String, #[arg(long = "attachment")] attachments: Vec<String>,
        #[arg(long)] idempotency_key: Option<String>,
    },
    Reply {
        id: String, #[arg(long)] profile: Option<String>, #[arg(long)] all: bool,
        #[arg(long, default_value = "")] body: String, #[arg(long = "attachment")] attachments: Vec<String>,
        #[arg(long)] schedule_at: Option<String>, #[arg(long)] idempotency_key: Option<String>,
    },
    Forward {
        id: String, #[arg(long)] profile: Option<String>, #[arg(long)] to: Vec<String>,
        #[arg(long, default_value = "")] body: String, #[arg(long = "attachment")] attachments: Vec<String>,
        #[arg(long)] schedule_at: Option<String>, #[arg(long)] idempotency_key: Option<String>,
    },
    MarkRead { id: String },
    MarkUnread { id: String },
    Flag { id: String },
    Unflag { id: String },
    Move { id: String, #[arg(long)] mailbox: String },
    Archive { id: String },
    Delete { id: String, #[arg(long)] yes: bool },
    Tokens,
    TokenCreate { #[arg(long)] name: String, #[arg(long = "scope")] scopes: Vec<String>, #[arg(long = "account")] accounts: Vec<String>, #[arg(long)] days: Option<u32> },
    TokenRevoke { id: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let base = cli.url.trim_end_matches('/').to_string();
    let client = reqwest::Client::new();
    let token = cli.token.clone();

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
            print_json(client.post(format!("{base}/api/v1/auth/login")).json(&LoginRequest { username, password }).send().await?).await?;
        }
        Commands::Me => print_json(auth(&token, client.get(format!("{base}/api/v1/me")))?.send().await?).await?,
        Commands::Accounts => print_json(auth(&token, client.get(format!("{base}/api/v1/accounts")))?.send().await?).await?,
        Commands::Profiles => print_json(auth(&token, client.get(format!("{base}/api/v1/profiles")))?.send().await?).await?,
        Commands::Mailboxes { account } => print_json(auth(&token, client.get(format!("{base}/api/v1/accounts/{account}/mailboxes")))?.send().await?).await?,
        Commands::Inbox { account, limit } => {
            let mut url = format!("{base}/api/v1/messages?limit={limit}");
            if let Some(account) = account { url.push_str("&account_id="); url.push_str(&account); }
            print_json(auth(&token, client.get(url))?.send().await?).await?;
        }
        Commands::Search { query, account, limit } => {
            let request = SearchRequest { account_ids: account, mailbox_id: None, query, limit: Some(limit) };
            print_json(auth(&token, client.post(format!("{base}/api/v1/messages/search")).json(&request))?.send().await?).await?;
        }
        Commands::Read { id } => print_json(auth(&token, client.get(format!("{base}/api/v1/messages/{id}")))?.send().await?).await?,
        Commands::Attachment { id, out } => {
            let response = auth(&token, client.get(format!("{base}/api/v1/attachments/{id}")))?.send().await?;
            ensure_success(&response)?;
            fs::write(&out, response.bytes().await?)?;
            println!("saved {}", out.display());
        }
        Commands::Upload { file, content_type } => {
            let bytes = fs::read(&file)?;
            let filename = file.file_name().and_then(|v| v.to_str()).ok_or("file name is not valid UTF-8")?;
            let response = auth(&token, client.post(format!("{base}/api/v1/uploads")))?
                .header("X-Filename", filename)
                .header(CONTENT_TYPE, content_type.unwrap_or_else(|| "application/octet-stream".into()))
                .body(bytes).send().await?;
            print_json(response).await?;
        }
        Commands::Send { profile, to, cc, bcc, subject, body, attachments, schedule_at, idempotency_key } => {
            let request = SendMessageRequest {
                profile_id: profile, to: addresses(to), cc: addresses(cc), bcc: addresses(bcc), subject,
                body: MessageBody { text: Some(body), html: None }, attachment_ids: attachments,
                schedule_at, delivery_receipt: false,
            };
            let response = write_request(&token, client.post(format!("{base}/api/v1/messages/send")).json(&request), idempotency_key)?.send().await?;
            print_json(response).await?;
        }
        Commands::Draft { profile, to, subject, body, attachments, idempotency_key } => {
            let request = SendMessageRequest {
                profile_id: profile, to: addresses(to), cc: Vec::new(), bcc: Vec::new(), subject,
                body: MessageBody { text: Some(body), html: None }, attachment_ids: attachments,
                schedule_at: None, delivery_receipt: false,
            };
            let response = write_request(&token, client.post(format!("{base}/api/v1/drafts")).json(&request), idempotency_key)?.send().await?;
            print_json(response).await?;
        }
        Commands::Reply { id, profile, all, body, attachments, schedule_at, idempotency_key } => {
            let request = ReplyMessageRequest { profile_id: profile, reply_all: all, body: MessageBody { text: Some(body), html: None }, attachment_ids: attachments, schedule_at };
            let response = write_request(&token, client.post(format!("{base}/api/v1/messages/{id}/reply")).json(&request), idempotency_key)?.send().await?;
            print_json(response).await?;
        }
        Commands::Forward { id, profile, to, body, attachments, schedule_at, idempotency_key } => {
            let request = ForwardMessageRequest { profile_id: profile, to: addresses(to), cc: Vec::new(), bcc: Vec::new(), body: MessageBody { text: Some(body), html: None }, attachment_ids: attachments, schedule_at };
            let response = write_request(&token, client.post(format!("{base}/api/v1/messages/{id}/forward")).json(&request), idempotency_key)?.send().await?;
            print_json(response).await?;
        }
        Commands::MarkRead { id } => patch_message(&client, &base, &token, &id, MessageUpdateRequest { seen: Some(true), flagged: None }).await?,
        Commands::MarkUnread { id } => patch_message(&client, &base, &token, &id, MessageUpdateRequest { seen: Some(false), flagged: None }).await?,
        Commands::Flag { id } => patch_message(&client, &base, &token, &id, MessageUpdateRequest { seen: None, flagged: Some(true) }).await?,
        Commands::Unflag { id } => patch_message(&client, &base, &token, &id, MessageUpdateRequest { seen: None, flagged: Some(false) }).await?,
        Commands::Move { id, mailbox } => {
            let request = MoveMessageRequest { mailbox_id: mailbox };
            print_json(auth(&token, client.post(format!("{base}/api/v1/messages/{id}/move")).json(&request))?.send().await?).await?;
        }
        Commands::Archive { id } => print_json(auth(&token, client.post(format!("{base}/api/v1/messages/{id}/archive")))?.send().await?).await?,
        Commands::Delete { id, yes } => {
            if !yes { return Err("delete requires --yes".into()); }
            print_json(auth(&token, client.delete(format!("{base}/api/v1/messages/{id}")))?.send().await?).await?;
        }
        Commands::Tokens => print_json(auth(&token, client.get(format!("{base}/api/v1/tokens")))?.send().await?).await?,
        Commands::TokenCreate { name, scopes, accounts, days } => {
            let request = CreateTokenRequest { name, scopes, account_allowlist: accounts, expires_in_days: days };
            print_json(auth(&token, client.post(format!("{base}/api/v1/tokens")).json(&request))?.send().await?).await?;
        }
        Commands::TokenRevoke { id } => {
            let response = auth(&token, client.delete(format!("{base}/api/v1/tokens/{id}")))?.send().await?;
            if response.status().is_success() { println!("revoked {id}"); } else { print_json(response).await?; }
        }
    }
    Ok(())
}

async fn patch_message(client: &reqwest::Client, base: &str, token: &Option<String>, id: &str, request: MessageUpdateRequest) -> Result<(), Box<dyn std::error::Error>> {
    let response = auth(token, client.patch(format!("{base}/api/v1/messages/{id}")).json(&request))?.send().await?;
    print_json(response).await
}

fn addresses(values: Vec<String>) -> Vec<Address> {
    values.into_iter().flat_map(|v| v.split(',').map(str::trim).filter(|v| !v.is_empty()).map(|email| Address { name: None, email: email.to_string() }).collect::<Vec<_>>()).collect()
}

fn write_request(token: &Option<String>, request: reqwest::RequestBuilder, key: Option<String>) -> Result<reqwest::RequestBuilder, Box<dyn std::error::Error>> {
    Ok(auth(token, request)?.header("Idempotency-Key", key.unwrap_or_else(default_idempotency_key)))
}

fn default_idempotency_key() -> String {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    format!("cyphtctl-{}-{millis}", std::process::id())
}

fn auth(token: &Option<String>, request: reqwest::RequestBuilder) -> Result<reqwest::RequestBuilder, Box<dyn std::error::Error>> {
    let token = token.as_deref().ok_or("this command requires --token or CYPHT_GATEWAY_TOKEN")?;
    Ok(request.header(AUTHORIZATION, format!("Bearer {token}")))
}

fn ensure_success(response: &reqwest::Response) -> Result<(), Box<dyn std::error::Error>> {
    if response.status().is_success() { Ok(()) } else { Err(format!("request failed with HTTP {}", response.status()).into()) }
}

async fn print_json(response: reqwest::Response) -> Result<(), Box<dyn std::error::Error>> {
    let status = response.status();
    let body: Value = response.json().await.unwrap_or_else(|_| serde_json::json!({"error":"invalid JSON response"}));
    println!("{}", serde_json::to_string_pretty(&body)?);
    if !status.is_success() { return Err(format!("request failed with HTTP {status}").into()); }
    Ok(())
}
