use gateway_auth::{Credential, CyphtSession};
use gateway_core::{GatewayError, GatewayResult};
use reqwest::{header::{HeaderMap, HeaderValue, COOKIE}, Client, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;
use url::Url;

#[derive(Debug, Clone)]
pub struct CyphtConfig {
    pub base_url: Url,
    pub api_login_key: String,
    pub bridge_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeAccount {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub protocol: String,
    pub server: Option<String>,
    pub can_send: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMailbox {
    pub name: String,
    pub display_name: String,
    pub role: Option<String>,
    pub total: Option<u64>,
    pub unread: Option<u64>,
    pub selectable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMessageSummary {
    pub uid: String,
    pub account_id: String,
    pub folder: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub from: serde_json::Value,
    #[serde(default)]
    pub to: serde_json::Value,
    pub date: Option<String>,
    pub timestamp: Option<i64>,
    #[serde(default)]
    pub flags: String,
    pub content_type: Option<String>,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMessage {
    pub uid: String,
    pub account_id: String,
    pub folder: String,
    #[serde(default)]
    pub headers: serde_json::Value,
    #[serde(default)]
    pub body_text: Option<String>,
    #[serde(default)]
    pub body_html: Option<String>,
    #[serde(default)]
    pub structure: serde_json::Value,
    #[serde(default)]
    pub attachments: Vec<BridgeAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeAttachment {
    pub part: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub size: Option<u64>,
    pub inline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMessagePage {
    pub total: Option<u64>,
    pub messages: Vec<BridgeMessageSummary>,
}

#[derive(Debug, Deserialize)]
struct BridgePing {
    status: String,
    bridge_version: String,
}

#[derive(Debug, Deserialize)]
struct LoginPayload {
    hm_id: String,
    hm_session: String,
}

#[derive(Debug, Deserialize)]
struct BridgeEnvelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Clone)]
pub struct CyphtClient {
    http: Client,
    config: CyphtConfig,
}

impl CyphtClient {
    pub fn new(config: CyphtConfig) -> GatewayResult<Self> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("cypht-gateway/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| GatewayError::Configuration(format!("build HTTP client: {e}")))?;
        Ok(Self { http, config })
    }

    pub async fn login(&self, credential: &Credential) -> GatewayResult<CyphtSession> {
        let url = self.page_url("process_api_login")?;
        let response = self.http.post(url)
            .form(&[
                ("username", credential.username.as_str()),
                ("password", credential.password.as_str()),
                ("api_login_key", self.config.api_login_key.as_str()),
            ])
            .send().await.map_err(upstream)?;
        if !response.status().is_success() {
            return Err(GatewayError::Authentication);
        }
        let payload: LoginPayload = response.json().await.map_err(|_| GatewayError::Authentication)?;
        if payload.hm_id.is_empty() || payload.hm_session.is_empty() {
            return Err(GatewayError::Authentication);
        }
        Ok(CyphtSession { hm_id: payload.hm_id, hm_session: payload.hm_session })
    }

    pub async fn ping(&self, session: &CyphtSession) -> GatewayResult<()> {
        let ping: BridgePing = self.bridge_get(session, "ajax_gateway_ping", &[]).await?;
        if ping.status != "ok" {
            return Err(GatewayError::Upstream("bridge health check failed".into()));
        }
        if ping.bridge_version != env!("CARGO_PKG_VERSION") {
            return Err(GatewayError::Upstream(format!(
                "bridge version {} does not match gateway version {}",
                ping.bridge_version,
                env!("CARGO_PKG_VERSION"),
            )));
        }
        Ok(())
    }

    pub async fn accounts(&self, session: &CyphtSession) -> GatewayResult<Vec<BridgeAccount>> {
        self.bridge_get(session, "ajax_gateway_accounts", &[]).await
    }

    pub async fn mailboxes(&self, session: &CyphtSession, account_id: &str) -> GatewayResult<Vec<BridgeMailbox>> {
        self.bridge_get(session, "ajax_gateway_mailboxes", &[("account_id", account_id)]).await
    }

    pub async fn messages(&self, session: &CyphtSession, account_id: &str, folder: &str, offset: u32, limit: u32) -> GatewayResult<BridgeMessagePage> {
        let offset_s = offset.to_string();
        let limit_s = limit.to_string();
        self.bridge_get(session, "ajax_gateway_messages", &[
            ("account_id", account_id),
            ("folder", folder),
            ("offset", &offset_s),
            ("limit", &limit_s),
        ]).await
    }

    pub async fn search(&self, session: &CyphtSession, account_ids: &[String], folder: &str, query: &str, limit: u32) -> GatewayResult<BridgeMessagePage> {
        let account_ids = account_ids.join(",");
        let limit_s = limit.to_string();
        self.bridge_get(session, "ajax_gateway_search", &[
            ("account_ids", &account_ids),
            ("folder", folder),
            ("query", query),
            ("limit", &limit_s),
        ]).await
    }

    pub async fn message(&self, session: &CyphtSession, account_id: &str, folder: &str, uid: &str) -> GatewayResult<BridgeMessage> {
        self.bridge_get(session, "ajax_gateway_message", &[
            ("account_id", account_id),
            ("folder", folder),
            ("uid", uid),
        ]).await
    }

    async fn bridge_get<T: DeserializeOwned>(&self, session: &CyphtSession, page: &str, params: &[(&str, &str)]) -> GatewayResult<T> {
        let mut url = self.page_url(page)?;
        {
            let mut pairs = url.query_pairs_mut();
            for (name, value) in params {
                pairs.append_pair(name, value);
            }
        }
        let response = self.http.get(url)
            .headers(self.bridge_headers(session)?)
            .send().await.map_err(upstream)?;
        if response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::FORBIDDEN {
            return Err(GatewayError::Authentication);
        }
        if !response.status().is_success() {
            return Err(GatewayError::Upstream(format!("bridge returned HTTP {}", response.status())));
        }
        let envelope: BridgeEnvelope<T> = response.json().await.map_err(|e| GatewayError::Upstream(format!("invalid bridge JSON: {e}")))?;
        if !envelope.ok {
            return Err(GatewayError::Upstream(envelope.error.unwrap_or_else(|| "bridge operation failed".into())));
        }
        envelope.data.ok_or_else(|| GatewayError::Upstream("bridge response missing data".into()))
    }

    fn bridge_headers(&self, session: &CyphtSession) -> GatewayResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        let cookie = format!("hm_id={}; hm_session={}", session.hm_id, session.hm_session);
        headers.insert(COOKIE, HeaderValue::from_str(&cookie).map_err(|_| GatewayError::Authentication)?);
        headers.insert("x-cypht-gateway-key", HeaderValue::from_str(&self.config.bridge_key)
            .map_err(|_| GatewayError::Configuration("invalid bridge key header".into()))?);
        Ok(headers)
    }

    fn page_url(&self, page: &str) -> GatewayResult<Url> {
        let mut url = self.config.base_url.clone();
        url.set_query(None);
        url.query_pairs_mut().append_pair("page", page);
        Ok(url)
    }
}

fn upstream(error: reqwest::Error) -> GatewayError {
    GatewayError::Upstream(error.to_string())
}
