use clap::{Parser, Subcommand};
use gateway_core::{
    Address, AdvancedSavedSearchData, BuildInfo, CalendarEventCreateRequest,
    CalendarRepeatInterval, ContactCreateRequest, ContactUpdateRequest, CreateTokenRequest,
    ForwardMessageRequest, LoginRequest, MessageBody, MessageUpdateRequest, MoveMessageRequest,
    ReplyMessageRequest, SavedSearchCreateRequest, SavedSearchType, SavedSearchUpdateRequest,
    SearchRequest, SendMessageRequest, TagCreateRequest, TagUpdateRequest,
};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[command(name = "cyphtctl", about = "CLI for Cypht Gateway")]
struct Cli {
    #[arg(
        long,
        env = "CYPHT_GATEWAY_URL",
        default_value = "http://127.0.0.1:8080"
    )]
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
        #[arg(long)]
        username: String,
        #[arg(long, env = "CYPHT_PASSWORD")]
        password: String,
    },
    Me,
    Accounts,
    Contacts {
        #[arg(long)]
        query: Option<String>,
    },
    ContactRead {
        id: String,
    },
    ContactCreate {
        #[arg(long)]
        name: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        phone: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },
    ContactUpdate {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        email: Option<String>,
        #[arg(long)]
        phone: Option<String>,
        #[arg(long)]
        group: Option<String>,
    },
    ContactDelete {
        id: String,
        #[arg(long)]
        yes: bool,
    },
    Feeds,
    FeedRead {
        id: String,
    },
    SieveStatus,
    Calendars,
    CalendarEvents {
        calendar: String,
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
    },
    CalendarCreate {
        calendar: String,
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "")]
        description: String,
        #[arg(long)]
        starts_at: String,
        #[arg(long, default_value = "none", value_parser = ["none", "day", "week", "month", "year"])]
        repeat_interval: String,
    },
    CalendarDelete {
        calendar: String,
        event: String,
        #[arg(long)]
        yes: bool,
    },
    SavedSearches,
    SavedSearchRead {
        id: String,
    },
    SavedSearchCreate {
        #[arg(long)]
        name: String,
        #[arg(long, value_parser = ["simple", "advanced"])]
        kind: String,
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        field: Option<String>,
        #[arg(long)]
        advanced_file: Option<PathBuf>,
    },
    SavedSearchUpdate {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        field: Option<String>,
        #[arg(long)]
        advanced_file: Option<PathBuf>,
    },
    SavedSearchDelete {
        id: String,
        #[arg(long)]
        yes: bool,
    },
    Tags {
        #[command(subcommand)]
        command: TagCommands,
    },
    Tag {
        #[command(subcommand)]
        command: MessageTagCommands,
    },
    Profiles,
    Mailboxes {
        #[arg(long)]
        account: String,
    },
    Inbox {
        #[arg(long)]
        account: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    Search {
        query: String,
        #[arg(long)]
        account: Vec<String>,
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    Read {
        id: String,
    },
    Attachment {
        id: String,
        #[arg(long)]
        out: PathBuf,
    },
    Upload {
        file: PathBuf,
        #[arg(long)]
        content_type: Option<String>,
    },
    Send {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        to: Vec<String>,
        #[arg(long)]
        cc: Vec<String>,
        #[arg(long)]
        bcc: Vec<String>,
        #[arg(long, default_value = "")]
        subject: String,
        #[arg(long, default_value = "")]
        body: String,
        #[arg(long = "attachment")]
        attachments: Vec<String>,
        #[arg(long)]
        schedule_at: Option<String>,
        #[arg(long, help = "Stable key required for safe retries")]
        idempotency_key: String,
    },
    Draft {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        to: Vec<String>,
        #[arg(long, default_value = "")]
        subject: String,
        #[arg(long, default_value = "")]
        body: String,
        #[arg(long = "attachment")]
        attachments: Vec<String>,
        #[arg(long, help = "Stable key required for safe retries")]
        idempotency_key: String,
    },
    Reply {
        id: String,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        all: bool,
        #[arg(long, default_value = "")]
        body: String,
        #[arg(long = "attachment")]
        attachments: Vec<String>,
        #[arg(long)]
        schedule_at: Option<String>,
        #[arg(long, help = "Stable key required for safe retries")]
        idempotency_key: String,
    },
    Forward {
        id: String,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        to: Vec<String>,
        #[arg(long, default_value = "")]
        body: String,
        #[arg(long = "attachment")]
        attachments: Vec<String>,
        #[arg(long)]
        schedule_at: Option<String>,
        #[arg(long, help = "Stable key required for safe retries")]
        idempotency_key: String,
    },
    MarkRead {
        id: String,
    },
    MarkUnread {
        id: String,
    },
    Flag {
        id: String,
    },
    Unflag {
        id: String,
    },
    Move {
        id: String,
        #[arg(long)]
        mailbox: String,
    },
    Archive {
        id: String,
    },
    Delete {
        id: String,
        #[arg(long)]
        yes: bool,
    },
    Tokens,
    TokenCreate {
        #[arg(long)]
        name: String,
        #[arg(long = "scope")]
        scopes: Vec<String>,
        #[arg(long = "account")]
        accounts: Vec<String>,
        #[arg(long)]
        days: Option<u32>,
    },
    TokenRevoke {
        id: String,
    },
    Audit {
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(long, default_value_t = 0)]
        offset: u64,
    },
}

#[derive(Subcommand)]
enum TagCommands {
    List,
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        color: Option<String>,
    },
    Update {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        color: Option<String>,
    },
    Delete {
        id: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum MessageTagCommands {
    Add {
        message_id: String,
        tag_id: String,
    },
    Remove {
        message_id: String,
        tag_id: String,
        #[arg(long)]
        yes: bool,
    },
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
            if let Some(tag) = info.git_tag {
                println!("tag: {tag}");
            }
            println!("dirty: {}", info.git_dirty);
        }
        Commands::Status => {
            print_json(
                client
                    .get(format!("{base}/api/v1/meta/version"))
                    .send()
                    .await?,
            )
            .await?
        }
        Commands::Login { username, password } => {
            print_json(
                client
                    .post(format!("{base}/api/v1/auth/login"))
                    .json(&LoginRequest { username, password })
                    .send()
                    .await?,
            )
            .await?;
        }
        Commands::Me => {
            print_json(
                auth(&token, client.get(format!("{base}/api/v1/me")))?
                    .send()
                    .await?,
            )
            .await?
        }
        Commands::Accounts => {
            print_json(
                auth(&token, client.get(format!("{base}/api/v1/accounts")))?
                    .send()
                    .await?,
            )
            .await?
        }
        Commands::Contacts { query } => {
            let mut url = reqwest::Url::parse(&format!("{base}/api/v1/contacts"))?;
            if let Some(query) = query {
                url.query_pairs_mut().append_pair("query", &query);
            }
            print_json(auth(&token, client.get(url))?.send().await?).await?;
        }
        Commands::ContactRead { id } => {
            print_json(
                auth(&token, client.get(format!("{base}/api/v1/contacts/{id}")))?
                    .send()
                    .await?,
            )
            .await?;
        }
        Commands::ContactCreate {
            name,
            email,
            phone,
            group,
        } => {
            let request = ContactCreateRequest {
                name,
                email,
                phone,
                group,
            };
            print_json(
                auth(
                    &token,
                    client
                        .post(format!("{base}/api/v1/contacts"))
                        .json(&request),
                )?
                .send()
                .await?,
            )
            .await?;
        }
        Commands::ContactUpdate {
            id,
            name,
            email,
            phone,
            group,
        } => {
            let request = ContactUpdateRequest {
                name,
                email,
                phone,
                group,
            };
            print_json(
                auth(
                    &token,
                    client
                        .patch(format!("{base}/api/v1/contacts/{id}"))
                        .json(&request),
                )?
                .send()
                .await?,
            )
            .await?;
        }
        Commands::ContactDelete { id, yes } => {
            if !yes {
                return Err("contact-delete requires --yes".into());
            }
            print_json(
                auth(
                    &token,
                    client.delete(format!("{base}/api/v1/contacts/{id}?confirm=true")),
                )?
                .send()
                .await?,
            )
            .await?;
        }
        Commands::Feeds => {
            print_json(
                auth(&token, client.get(format!("{base}/api/v1/feeds")))?
                    .send()
                    .await?,
            )
            .await?;
        }
        Commands::FeedRead { id } => {
            print_json(
                auth(&token, client.get(format!("{base}/api/v1/feeds/{id}")))?
                    .send()
                    .await?,
            )
            .await?;
        }
        Commands::SieveStatus => {
            print_json(
                auth(&token, client.get(format!("{base}/api/v1/sieve/status")))?
                    .send()
                    .await?,
            )
            .await?;
        }
        Commands::Calendars => {
            print_json(
                auth(&token, client.get(format!("{base}/api/v1/calendars")))?
                    .send()
                    .await?,
            )
            .await?;
        }
        Commands::CalendarEvents {
            calendar,
            start,
            end,
        } => {
            let mut url = api_url(
                &base,
                &["api", "v1", "calendars", &calendar, "events"],
                false,
            )?;
            url.query_pairs_mut()
                .append_pair("start", &start)
                .append_pair("end", &end);
            print_json(auth(&token, client.get(url))?.send().await?).await?;
        }
        Commands::CalendarCreate {
            calendar,
            title,
            description,
            starts_at,
            repeat_interval,
        } => {
            let repeat_interval = match repeat_interval.as_str() {
                "none" => CalendarRepeatInterval::None,
                "day" => CalendarRepeatInterval::Day,
                "week" => CalendarRepeatInterval::Week,
                "month" => CalendarRepeatInterval::Month,
                "year" => CalendarRepeatInterval::Year,
                _ => return Err("invalid --repeat-interval".into()),
            };
            let request = CalendarEventCreateRequest {
                title,
                description,
                starts_at,
                repeat_interval,
            };
            let url = api_url(
                &base,
                &["api", "v1", "calendars", &calendar, "events"],
                false,
            )?;
            print_json(
                auth(&token, client.post(url).json(&request))?
                    .send()
                    .await?,
            )
            .await?;
        }
        Commands::CalendarDelete {
            calendar,
            event,
            yes,
        } => {
            if !yes {
                return Err("calendar-delete requires --yes".into());
            }
            let url = api_url(
                &base,
                &["api", "v1", "calendars", &calendar, "events", &event],
                true,
            )?;
            print_json(auth(&token, client.delete(url))?.send().await?).await?;
        }
        Commands::SavedSearches => {
            print_json(
                auth(&token, client.get(format!("{base}/api/v1/saved-searches")))?
                    .send()
                    .await?,
            )
            .await?;
        }
        Commands::SavedSearchRead { id } => {
            let url = api_url(&base, &["api", "v1", "saved-searches", &id], false)?;
            print_json(auth(&token, client.get(url))?.send().await?).await?;
        }
        Commands::SavedSearchCreate {
            name,
            kind,
            query,
            since,
            field,
            advanced_file,
        } => {
            let kind = match kind.as_str() {
                "simple" => SavedSearchType::Simple,
                "advanced" => SavedSearchType::Advanced,
                _ => return Err("--kind must be simple or advanced".into()),
            };
            let advanced = read_advanced_file(advanced_file)?;
            let request = SavedSearchCreateRequest {
                name,
                kind,
                query,
                since,
                field,
                advanced,
            };
            print_json(
                auth(
                    &token,
                    client
                        .post(format!("{base}/api/v1/saved-searches"))
                        .json(&request),
                )?
                .send()
                .await?,
            )
            .await?;
        }
        Commands::SavedSearchUpdate {
            id,
            name,
            query,
            since,
            field,
            advanced_file,
        } => {
            let advanced = read_advanced_file(advanced_file)?;
            let request = SavedSearchUpdateRequest {
                name,
                query,
                since,
                field,
                advanced,
            };
            let url = api_url(&base, &["api", "v1", "saved-searches", &id], false)?;
            print_json(
                auth(&token, client.patch(url).json(&request))?
                    .send()
                    .await?,
            )
            .await?;
        }
        Commands::SavedSearchDelete { id, yes } => {
            if !yes {
                return Err("saved-search-delete requires --yes".into());
            }
            let url = api_url(&base, &["api", "v1", "saved-searches", &id], true)?;
            print_json(auth(&token, client.delete(url))?.send().await?).await?;
        }
        Commands::Tags { command } => match command {
            TagCommands::List => {
                print_json(
                    auth(&token, client.get(format!("{base}/api/v1/tags")))?
                        .send()
                        .await?,
                )
                .await?;
            }
            TagCommands::Create { name, color } => {
                let request = TagCreateRequest { name, color };
                print_json(
                    auth(
                        &token,
                        client.post(format!("{base}/api/v1/tags")).json(&request),
                    )?
                    .send()
                    .await?,
                )
                .await?;
            }
            TagCommands::Update { id, name, color } => {
                let request = TagUpdateRequest { name, color };
                let url = api_url(&base, &["api", "v1", "tags", &id], false)?;
                print_json(
                    auth(&token, client.patch(url).json(&request))?
                        .send()
                        .await?,
                )
                .await?;
            }
            TagCommands::Delete { id, yes } => {
                if !yes {
                    return Err("tags delete requires --yes".into());
                }
                let url = api_url(&base, &["api", "v1", "tags", &id], true)?;
                print_json(auth(&token, client.delete(url))?.send().await?).await?;
            }
        },
        Commands::Tag { command } => match command {
            MessageTagCommands::Add { message_id, tag_id } => {
                let url = api_url(
                    &base,
                    &["api", "v1", "messages", &message_id, "tags", &tag_id],
                    false,
                )?;
                print_json(auth(&token, client.post(url))?.send().await?).await?;
            }
            MessageTagCommands::Remove {
                message_id,
                tag_id,
                yes,
            } => {
                if !yes {
                    return Err("tag remove requires --yes".into());
                }
                let url = api_url(
                    &base,
                    &["api", "v1", "messages", &message_id, "tags", &tag_id],
                    true,
                )?;
                print_json(auth(&token, client.delete(url))?.send().await?).await?;
            }
        },
        Commands::Profiles => {
            print_json(
                auth(&token, client.get(format!("{base}/api/v1/profiles")))?
                    .send()
                    .await?,
            )
            .await?
        }
        Commands::Mailboxes { account } => {
            print_json(
                auth(
                    &token,
                    client.get(format!("{base}/api/v1/accounts/{account}/mailboxes")),
                )?
                .send()
                .await?,
            )
            .await?
        }
        Commands::Inbox { account, limit } => {
            let mut url = format!("{base}/api/v1/messages?limit={limit}");
            if let Some(account) = account {
                url.push_str("&account_id=");
                url.push_str(&account);
            }
            print_json(auth(&token, client.get(url))?.send().await?).await?;
        }
        Commands::Search {
            query,
            account,
            limit,
        } => {
            let request = SearchRequest {
                account_ids: account,
                mailbox_id: None,
                query,
                limit: Some(limit),
            };
            print_json(
                auth(
                    &token,
                    client
                        .post(format!("{base}/api/v1/messages/search"))
                        .json(&request),
                )?
                .send()
                .await?,
            )
            .await?;
        }
        Commands::Read { id } => {
            print_json(
                auth(&token, client.get(format!("{base}/api/v1/messages/{id}")))?
                    .send()
                    .await?,
            )
            .await?
        }
        Commands::Attachment { id, out } => {
            let response = auth(
                &token,
                client.get(format!("{base}/api/v1/attachments/{id}")),
            )?
            .send()
            .await?;
            ensure_success(&response)?;
            fs::write(&out, response.bytes().await?)?;
            println!("saved {}", out.display());
        }
        Commands::Upload { file, content_type } => {
            let bytes = fs::read(&file)?;
            let filename = file
                .file_name()
                .and_then(|v| v.to_str())
                .ok_or("file name is not valid UTF-8")?;
            let response = auth(&token, client.post(format!("{base}/api/v1/uploads")))?
                .header("X-Filename", filename)
                .header(
                    CONTENT_TYPE,
                    content_type.unwrap_or_else(|| "application/octet-stream".into()),
                )
                .body(bytes)
                .send()
                .await?;
            print_json(response).await?;
        }
        Commands::Send {
            profile,
            to,
            cc,
            bcc,
            subject,
            body,
            attachments,
            schedule_at,
            idempotency_key,
        } => {
            let request = SendMessageRequest {
                profile_id: profile,
                to: addresses(to),
                cc: addresses(cc),
                bcc: addresses(bcc),
                subject,
                body: MessageBody {
                    text: Some(body),
                    html: None,
                },
                attachment_ids: attachments,
                schedule_at,
                delivery_receipt: false,
            };
            let response = write_request(
                &token,
                client
                    .post(format!("{base}/api/v1/messages/send"))
                    .json(&request),
                &idempotency_key,
            )?
            .send()
            .await?;
            print_json(response).await?;
        }
        Commands::Draft {
            profile,
            to,
            subject,
            body,
            attachments,
            idempotency_key,
        } => {
            let request = SendMessageRequest {
                profile_id: profile,
                to: addresses(to),
                cc: Vec::new(),
                bcc: Vec::new(),
                subject,
                body: MessageBody {
                    text: Some(body),
                    html: None,
                },
                attachment_ids: attachments,
                schedule_at: None,
                delivery_receipt: false,
            };
            let response = write_request(
                &token,
                client.post(format!("{base}/api/v1/drafts")).json(&request),
                &idempotency_key,
            )?
            .send()
            .await?;
            print_json(response).await?;
        }
        Commands::Reply {
            id,
            profile,
            all,
            body,
            attachments,
            schedule_at,
            idempotency_key,
        } => {
            let request = ReplyMessageRequest {
                profile_id: profile,
                reply_all: all,
                body: MessageBody {
                    text: Some(body),
                    html: None,
                },
                attachment_ids: attachments,
                schedule_at,
            };
            let response = write_request(
                &token,
                client
                    .post(format!("{base}/api/v1/messages/{id}/reply"))
                    .json(&request),
                &idempotency_key,
            )?
            .send()
            .await?;
            print_json(response).await?;
        }
        Commands::Forward {
            id,
            profile,
            to,
            body,
            attachments,
            schedule_at,
            idempotency_key,
        } => {
            let request = ForwardMessageRequest {
                profile_id: profile,
                to: addresses(to),
                cc: Vec::new(),
                bcc: Vec::new(),
                body: MessageBody {
                    text: Some(body),
                    html: None,
                },
                attachment_ids: attachments,
                schedule_at,
            };
            let response = write_request(
                &token,
                client
                    .post(format!("{base}/api/v1/messages/{id}/forward"))
                    .json(&request),
                &idempotency_key,
            )?
            .send()
            .await?;
            print_json(response).await?;
        }
        Commands::MarkRead { id } => {
            patch_message(
                &client,
                &base,
                &token,
                &id,
                MessageUpdateRequest {
                    seen: Some(true),
                    flagged: None,
                },
            )
            .await?
        }
        Commands::MarkUnread { id } => {
            patch_message(
                &client,
                &base,
                &token,
                &id,
                MessageUpdateRequest {
                    seen: Some(false),
                    flagged: None,
                },
            )
            .await?
        }
        Commands::Flag { id } => {
            patch_message(
                &client,
                &base,
                &token,
                &id,
                MessageUpdateRequest {
                    seen: None,
                    flagged: Some(true),
                },
            )
            .await?
        }
        Commands::Unflag { id } => {
            patch_message(
                &client,
                &base,
                &token,
                &id,
                MessageUpdateRequest {
                    seen: None,
                    flagged: Some(false),
                },
            )
            .await?
        }
        Commands::Move { id, mailbox } => {
            let request = MoveMessageRequest {
                mailbox_id: mailbox,
            };
            print_json(
                auth(
                    &token,
                    client
                        .post(format!("{base}/api/v1/messages/{id}/move"))
                        .json(&request),
                )?
                .send()
                .await?,
            )
            .await?;
        }
        Commands::Archive { id } => {
            print_json(
                auth(
                    &token,
                    client.post(format!("{base}/api/v1/messages/{id}/archive")),
                )?
                .send()
                .await?,
            )
            .await?
        }
        Commands::Delete { id, yes } => {
            if !yes {
                return Err("delete requires --yes".into());
            }
            print_json(
                auth(
                    &token,
                    client.delete(format!("{base}/api/v1/messages/{id}")),
                )?
                .send()
                .await?,
            )
            .await?;
        }
        Commands::Audit { limit, offset } => {
            print_json(
                auth(
                    &token,
                    client.get(format!("{base}/api/v1/audit?limit={limit}&offset={offset}")),
                )?
                .send()
                .await?,
            )
            .await?;
        }
        Commands::Tokens => {
            print_json(
                auth(&token, client.get(format!("{base}/api/v1/tokens")))?
                    .send()
                    .await?,
            )
            .await?
        }
        Commands::TokenCreate {
            name,
            scopes,
            accounts,
            days,
        } => {
            let request = CreateTokenRequest {
                name,
                scopes,
                account_allowlist: accounts,
                expires_in_days: days,
            };
            print_json(
                auth(
                    &token,
                    client.post(format!("{base}/api/v1/tokens")).json(&request),
                )?
                .send()
                .await?,
            )
            .await?;
        }
        Commands::TokenRevoke { id } => {
            let response = auth(&token, client.delete(format!("{base}/api/v1/tokens/{id}")))?
                .send()
                .await?;
            if response.status().is_success() {
                println!("revoked {id}");
            } else {
                print_json(response).await?;
            }
        }
    }
    Ok(())
}

async fn patch_message(
    client: &reqwest::Client,
    base: &str,
    token: &Option<String>,
    id: &str,
    request: MessageUpdateRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = auth(
        token,
        client
            .patch(format!("{base}/api/v1/messages/{id}"))
            .json(&request),
    )?
    .send()
    .await?;
    print_json(response).await
}

fn read_advanced_file(
    path: Option<PathBuf>,
) -> Result<Option<AdvancedSavedSearchData>, Box<dyn std::error::Error>> {
    path.map(|path| {
        let bytes = fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    })
    .transpose()
}
fn addresses(values: Vec<String>) -> Vec<Address> {
    values
        .into_iter()
        .flat_map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(|email| Address {
                    name: None,
                    email: email.to_string(),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn write_request(
    token: &Option<String>,
    request: reqwest::RequestBuilder,
    key: &str,
) -> Result<reqwest::RequestBuilder, Box<dyn std::error::Error>> {
    if key.trim().is_empty() {
        return Err("--idempotency-key cannot be empty".into());
    }
    Ok(auth(token, request)?.header("Idempotency-Key", key))
}

fn auth(
    token: &Option<String>,
    request: reqwest::RequestBuilder,
) -> Result<reqwest::RequestBuilder, Box<dyn std::error::Error>> {
    let token = token
        .as_deref()
        .ok_or("this command requires --token or CYPHT_GATEWAY_TOKEN")?;
    Ok(request.header(AUTHORIZATION, format!("Bearer {token}")))
}

fn api_url(
    base: &str,
    path_segments: &[&str],
    confirm: bool,
) -> Result<reqwest::Url, Box<dyn std::error::Error>> {
    let mut url = reqwest::Url::parse(&format!("{base}/"))?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Gateway URL cannot be used as a base URL",
            )
        })?;
        segments.pop_if_empty();
        for segment in path_segments {
            segments.push(segment);
        }
    }
    if confirm {
        url.query_pairs_mut().append_pair("confirm", "true");
    }
    Ok(url)
}

fn ensure_success(response: &reqwest::Response) -> Result<(), Box<dyn std::error::Error>> {
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("request failed with HTTP {}", response.status()).into())
    }
}

async fn print_json(response: reqwest::Response) -> Result<(), Box<dyn std::error::Error>> {
    let status = response.status();
    if status == reqwest::StatusCode::NO_CONTENT {
        println!("ok");
        return Ok(());
    }
    let body: Value = response
        .json()
        .await
        .unwrap_or_else(|_| serde_json::json!({"error":"invalid JSON response"}));
    println!("{}", serde_json::to_string_pretty(&body)?);
    if !status.is_success() {
        return Err(format!("request failed with HTTP {status}").into());
    }
    Ok(())
}
