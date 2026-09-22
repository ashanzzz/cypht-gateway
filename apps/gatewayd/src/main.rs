use gateway_api::{router, ApiState};
use gateway_auth::{AuthService, Vault};
use gateway_core::{BuildInfo, ObjectIdCodec};
use gateway_cypht::{CyphtClient, CyphtConfig};
use gateway_domain::GatewayService;
use gateway_storage::Store;
use std::{env, net::SocketAddr, path::PathBuf};
use tracing::info;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gatewayd=info,tower_http=info".into()),
        )
        .init();

    let config = RuntimeConfig::from_env()?;
    if let Some(parent) = config.db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let store = Store::open(&config.db_path)?;
    let vault = Vault::from_base64(&config.master_key)?;
    let ids = ObjectIdCodec::new(vault.id_key())?;
    let auth = AuthService::new(store.clone(), vault, config.session_ttl_seconds);
    let cypht = CyphtClient::new(CyphtConfig {
        base_url: config.cypht_base_url,
        api_login_key: config.cypht_api_login_key,
        bridge_key: config.cypht_bridge_key,
    })?;
    let service = GatewayService::new(auth, cypht, ids, store);
    let build = BuildInfo::current();
    let app = router(ApiState { service, build: build.clone() });

    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    info!(
        addr = %config.bind,
        version = build.version,
        git = build.git_short_sha,
        cypht = %config.cypht_display_url,
        "cypht-gateway started"
    );
    axum::serve(listener, app).await?;
    Ok(())
}

struct RuntimeConfig {
    bind: SocketAddr,
    db_path: PathBuf,
    master_key: String,
    session_ttl_seconds: u64,
    cypht_base_url: Url,
    cypht_display_url: String,
    cypht_api_login_key: String,
    cypht_bridge_key: String,
}

impl RuntimeConfig {
    fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let bind = env::var("GATEWAY_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into()).parse()?;
        let db_path = PathBuf::from(env::var("GATEWAY_DB_PATH").unwrap_or_else(|_| "/var/lib/cypht-gateway/gateway.db".into()));
        let master_key = required("GATEWAY_MASTER_KEY")?;
        let session_ttl_seconds = env::var("GATEWAY_SESSION_TTL_SECONDS")
            .unwrap_or_else(|_| "3600".into())
            .parse::<u64>()?;
        let cypht_display_url = required("CYPHT_BASE_URL")?;
        let mut cypht_base_url = Url::parse(&cypht_display_url)?;
        if !cypht_base_url.path().ends_with('/') {
            let path = format!("{}/", cypht_base_url.path().trim_end_matches('/'));
            cypht_base_url.set_path(&path);
        }
        Ok(Self {
            bind,
            db_path,
            master_key,
            session_ttl_seconds,
            cypht_base_url,
            cypht_display_url,
            cypht_api_login_key: required("CYPHT_API_LOGIN_KEY")?,
            cypht_bridge_key: required("CYPHT_BRIDGE_KEY")?,
        })
    }
}

fn required(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(format!("required environment variable {name} is missing").into()),
    }
}
