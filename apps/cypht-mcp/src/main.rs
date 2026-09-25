use clap::{Parser, ValueEnum};
use gateway_mcp::CyphtMcpServer;
use rmcp::{
    transport::{
        stdio,
        streamable_http_server::{
            session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
        },
    },
    ServiceExt,
};
use std::net::SocketAddr;
use tracing::info;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Transport {
    Stdio,
    Http,
}

#[derive(Parser, Debug)]
#[command(name = "cypht-mcp", about = "MCP server for Cypht Gateway")]
struct Args {
    #[arg(
        long,
        env = "CYPHT_GATEWAY_URL",
        default_value = "http://127.0.0.1:8080"
    )]
    url: String,
    #[arg(long, env = "CYPHT_GATEWAY_TOKEN")]
    token: Option<String>,
    #[arg(long, env = "CYPHT_MCP_ALLOW_WRITE", default_value_t = false)]
    allow_write: bool,
    #[arg(long, value_enum, default_value_t = Transport::Stdio)]
    transport: Transport,
    #[arg(long, env = "CYPHT_MCP_BIND", default_value = "127.0.0.1:8790")]
    bind: SocketAddr,
    #[arg(long = "allowed-host")]
    allowed_hosts: Vec<String>,
    #[arg(long = "allowed-origin")]
    allowed_origins: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cypht_mcp=info".into()),
        )
        .init();
    let args = Args::parse();
    match args.transport {
        Transport::Stdio => {
            let server =
                CyphtMcpServer::new(args.url.clone(), args.token.clone(), args.allow_write);
            let service = server.serve(stdio()).await?;
            service.waiting().await?;
        }
        Transport::Http => {
            // Remote HTTP callers must authenticate with their own Bearer PAT.
            // Never fall back to a process-wide token in multi-client mode.
            let server = CyphtMcpServer::new(args.url.clone(), None, args.allow_write);
            let mut config = StreamableHttpServerConfig::default();
            if !args.allowed_hosts.is_empty() {
                config.allowed_hosts = args.allowed_hosts;
            }
            if !args.allowed_origins.is_empty() {
                config.allowed_origins = args.allowed_origins;
            }
            config.max_request_body_bytes = 16 * 1024 * 1024;
            let service: StreamableHttpService<CyphtMcpServer, LocalSessionManager> =
                StreamableHttpService::new(
                    move || Ok(server.clone()),
                    LocalSessionManager::default().into(),
                    config,
                );
            let app = axum::Router::new().nest_service("/mcp", service);
            let listener = tokio::net::TcpListener::bind(args.bind).await?;
            info!(addr=%args.bind, allow_write=args.allow_write, "cypht MCP Streamable HTTP server started at /mcp");
            axum::serve(listener, app).await?;
        }
    }
    Ok(())
}
