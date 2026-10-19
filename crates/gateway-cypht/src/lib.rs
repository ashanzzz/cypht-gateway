use gateway_auth::{Credential, CyphtSession};
use gateway_core::{GatewayError, GatewayResult};
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_DISPOSITION, CONTENT_TYPE, COOKIE},
    Client, StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use url::Url;

const MAX_ATTACHMENT_BYTES: usize = 25 * 1024 * 1024;
const CYPHT_BASELINE_VERSION: &str = "2.13.0";

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
pub struct BridgeProfile {
    pub id: String,
    pub name: String,
    pub address: String,
    pub reply_to: String,
    pub signature: String,
    pub account_id: Option<String>,
    #[serde(rename = "default")]
    pub is_default: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeContact {
    pub id: String,
    pub source: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub group: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeTag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub parent: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeSavedSearch {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub query: Option<String>,
    pub since: Option<String>,
    pub field: Option<String>,
    pub advanced: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeSieveStatus {
    pub account_id: String,
    pub name: String,
    pub protocol: String,
    pub enabled: bool,
    pub configured: bool,
    pub status: String,
    pub remote_probe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeCalendarEvent {
    pub id: String,
    pub title: String,
    pub description: String,
    pub starts_at: i64,
    pub occurrence_at: i64,
    pub repeat_interval: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeFeed {
    pub id: String,
    pub name: String,
    pub url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeUpload {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeWriteResult {
    pub status: String,
    pub message_id_header: Option<String>,
    pub account_id: Option<String>,
    pub folder: Option<String>,
    pub uid: Option<String>,
    pub scheduled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeActionResult {
    pub status: String,
    pub folder: Option<String>,
    pub tag_sync: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BridgeDownload {
    pub bytes: Vec<u8>,
    pub filename: Option<String>,
    pub content_type: Option<String>,
}

pub const SUPPORTED_BRIDGE_API_VERSION: u32 = 1;

pub const CAP_MAIL_READ: &str = "mail.read.v1";
pub const CAP_MAIL_WRITE: &str = "mail.write.v1";
pub const CAP_ACCOUNTS_READ: &str = "accounts.read.v1";
pub const CAP_ACCOUNTS_WRITE: &str = "accounts.write.v1";
pub const CAP_ACCOUNTS_TEST: &str = "accounts.test.v1";
pub const CAP_CONTACTS_READ: &str = "contacts.read.v1";
pub const CAP_CONTACTS_WRITE: &str = "contacts.write.v1";
pub const CAP_TAGS_READ: &str = "tags.read.v1";
pub const CAP_TAGS_WRITE: &str = "tags.write.v1";
pub const CAP_CALENDAR_READ: &str = "calendar.read.v1";
pub const CAP_CALENDAR_WRITE: &str = "calendar.write.v1";
pub const CAP_SAVED_SEARCHES_READ: &str = "saved_searches.read.v1";
pub const CAP_SAVED_SEARCHES_WRITE: &str = "saved_searches.write.v1";
pub const CAP_SIEVE_READ: &str = "sieve.read.v1";
pub const CAP_FEEDS_READ: &str = "feeds.read.v1";
pub const CAP_USER_CONFIG_DURABLE: &str = "user_config.durable.v1";

#[derive(Debug, Clone, Deserialize)]
pub struct BridgePing {
    pub status: String,
    pub bridge_version: String,
    pub bridge_api_version: u32,
    pub cypht_version: String,
    #[serde(default)]
    pub durable_user_config: bool,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

impl BridgePing {
    pub fn supports(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }
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
    cached_ping: Arc<RwLock<Option<BridgePing>>>,
}

impl CyphtClient {
    pub fn bridge_key(&self) -> &str {
        &self.config.bridge_key
    }

    pub fn new(config: CyphtConfig) -> GatewayResult<Self> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("cypht-gateway/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| GatewayError::Configuration(format!("build HTTP client: {e}")))?;
        Ok(Self {
            http,
            config,
            cached_ping: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn login(&self, credential: &Credential) -> GatewayResult<CyphtSession> {
        let url = self.page_url("process_api_login")?;
        let response = self
            .http
            .post(url)
            .form(&[
                ("username", credential.username.as_str()),
                ("password", credential.password.as_str()),
                ("api_login_key", self.config.api_login_key.as_str()),
            ])
            .send()
            .await
            .map_err(upstream)?;
        if !response.status().is_success() {
            return Err(GatewayError::Authentication);
        }
        let payload: LoginPayload = response
            .json()
            .await
            .map_err(|_| GatewayError::Authentication)?;
        if payload.hm_id.is_empty() || payload.hm_session.is_empty() {
            return Err(GatewayError::Authentication);
        }
        Ok(CyphtSession {
            hm_id: payload.hm_id,
            hm_session: payload.hm_session,
        })
    }

    pub async fn ping(&self, session: &CyphtSession) -> GatewayResult<()> {
        self.bridge_ping(session).await.map(|_| ())
    }

    pub async fn supports_durable_user_config(
        &self,
        session: &CyphtSession,
    ) -> GatewayResult<bool> {
        Ok(self.bridge_ping(session).await?.durable_user_config)
    }

    pub async fn bridge_ping(&self, session: &CyphtSession) -> GatewayResult<BridgePing> {
        let ping: BridgePing = self.bridge_get(session, "ajax_gateway_ping", &[]).await?;
        validate_ping_versions(&ping)?;
        if let Ok(mut lock) = self.cached_ping.write() {
            *lock = Some(ping.clone());
        }
        Ok(ping)
    }

    async fn cached_or_fetch_ping(&self, session: &CyphtSession) -> GatewayResult<BridgePing> {
        let cached = self
            .cached_ping
            .read()
            .map_err(|_| GatewayError::Internal("lock poisoned".into()))?
            .clone();
        match cached {
            Some(ping) => Ok(ping),
            None => self.bridge_ping(session).await,
        }
    }

    pub async fn require_capabilities(
        &self,
        session: &CyphtSession,
        capabilities: &[&'static str],
    ) -> GatewayResult<()> {
        let ping = self.cached_or_fetch_ping(session).await?;
        for cap in capabilities {
            if !ping.supports(cap) {
                return Err(GatewayError::CapabilityUnavailable((*cap).to_string()));
            }
        }
        Ok(())
    }

    pub async fn require_capability(
        &self,
        session: &CyphtSession,
        capability: &str,
    ) -> GatewayResult<()> {
        let ping = self.cached_or_fetch_ping(session).await?;
        if !ping.supports(capability) {
            return Err(GatewayError::CapabilityUnavailable(capability.to_string()));
        }
        Ok(())
    }

    async fn get_with_capability<T: DeserializeOwned>(
        &self,
        session: &CyphtSession,
        capability: &'static str,
        page: &str,
        params: &[(&str, &str)],
    ) -> GatewayResult<T> {
        self.require_capability(session, capability).await?;
        self.bridge_get(session, page, params).await
    }

    async fn post_user_config_with_capability<T: DeserializeOwned>(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        capability: &'static str,
        page: &str,
        payload: &serde_json::Value,
    ) -> GatewayResult<T> {
        self.require_capabilities(session, &[capability, CAP_USER_CONFIG_DURABLE])
            .await?;
        self.bridge_post_with_config_key(session, Some(credential), page, payload)
            .await
    }

    async fn write_with_capability<T: DeserializeOwned>(
        &self,
        session: &CyphtSession,
        capability: &'static str,
        page: &str,
        payload: &serde_json::Value,
    ) -> GatewayResult<T> {
        self.require_capability(session, capability).await?;
        self.bridge_post(session, page, payload).await
    }

    pub async fn accounts(&self, session: &CyphtSession) -> GatewayResult<Vec<BridgeAccount>> {
        self.get_with_capability(session, CAP_ACCOUNTS_READ, "ajax_gateway_accounts", &[])
            .await
    }

    pub async fn profiles(&self, session: &CyphtSession) -> GatewayResult<Vec<BridgeProfile>> {
        self.get_with_capability(session, CAP_ACCOUNTS_READ, "ajax_gateway_profiles", &[])
            .await
    }

    pub async fn contacts(
        &self,
        session: &CyphtSession,
        query: Option<&str>,
    ) -> GatewayResult<Vec<BridgeContact>> {
        let params = query
            .map(|value| vec![("query", value)])
            .unwrap_or_default();
        self.get_with_capability(session, CAP_CONTACTS_READ, "ajax_gateway_contacts", &params)
            .await
    }

    pub async fn contact(&self, session: &CyphtSession, id: &str) -> GatewayResult<BridgeContact> {
        self.get_with_capability(
            session,
            CAP_CONTACTS_READ,
            "ajax_gateway_contact",
            &[("contact_id", id)],
        )
        .await
    }

    pub async fn create_contact(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeContact> {
        self.post_user_config_with_capability(
            session,
            credential,
            CAP_CONTACTS_WRITE,
            "ajax_gateway_contact_create",
            payload,
        )
        .await
    }

    pub async fn update_contact(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeContact> {
        self.post_user_config_with_capability(
            session,
            credential,
            CAP_CONTACTS_WRITE,
            "ajax_gateway_contact_update",
            payload,
        )
        .await
    }

    pub async fn delete_contact(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.post_user_config_with_capability(
            session,
            credential,
            CAP_CONTACTS_WRITE,
            "ajax_gateway_contact_delete",
            payload,
        )
        .await
    }
    pub async fn tags(&self, session: &CyphtSession) -> GatewayResult<Vec<BridgeTag>> {
        self.get_with_capability(session, CAP_TAGS_READ, "ajax_gateway_tags", &[])
            .await
    }

    pub async fn create_tag(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeTag> {
        self.post_user_config_with_capability(
            session,
            credential,
            CAP_TAGS_WRITE,
            "ajax_gateway_tag_create",
            payload,
        )
        .await
    }

    pub async fn update_tag(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeTag> {
        self.post_user_config_with_capability(
            session,
            credential,
            CAP_TAGS_WRITE,
            "ajax_gateway_tag_update",
            payload,
        )
        .await
    }

    pub async fn delete_tag(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.post_user_config_with_capability(
            session,
            credential,
            CAP_TAGS_WRITE,
            "ajax_gateway_tag_delete",
            payload,
        )
        .await
    }

    pub async fn sieve_status(
        &self,
        session: &CyphtSession,
    ) -> GatewayResult<Vec<BridgeSieveStatus>> {
        self.get_with_capability(session, CAP_SIEVE_READ, "ajax_gateway_sieve_status", &[])
            .await
    }

    pub async fn calendar_events(
        &self,
        session: &CyphtSession,
        start_at: i64,
        end_at: i64,
    ) -> GatewayResult<Vec<BridgeCalendarEvent>> {
        let start_s = start_at.to_string();
        let end_s = end_at.to_string();
        self.get_with_capability(
            session,
            CAP_CALENDAR_READ,
            "ajax_gateway_calendar_events",
            &[("start_at", &start_s), ("end_at", &end_s)],
        )
        .await
    }

    pub async fn create_calendar_event(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeCalendarEvent> {
        self.post_user_config_with_capability(
            session,
            credential,
            CAP_CALENDAR_WRITE,
            "ajax_gateway_calendar_create",
            payload,
        )
        .await
    }

    pub async fn delete_calendar_event(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.post_user_config_with_capability(
            session,
            credential,
            CAP_CALENDAR_WRITE,
            "ajax_gateway_calendar_delete",
            payload,
        )
        .await
    }
    pub async fn feeds(&self, session: &CyphtSession) -> GatewayResult<Vec<BridgeFeed>> {
        self.get_with_capability(session, CAP_FEEDS_READ, "ajax_gateway_feeds", &[])
            .await
    }

    pub async fn feed(&self, session: &CyphtSession, id: &str) -> GatewayResult<BridgeFeed> {
        self.get_with_capability(
            session,
            CAP_FEEDS_READ,
            "ajax_gateway_feed",
            &[("feed_id", id)],
        )
        .await
    }
    pub async fn saved_searches(
        &self,
        session: &CyphtSession,
    ) -> GatewayResult<Vec<BridgeSavedSearch>> {
        self.get_with_capability(
            session,
            CAP_SAVED_SEARCHES_READ,
            "ajax_gateway_saved_searches",
            &[],
        )
        .await
    }

    pub async fn create_saved_search(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeSavedSearch> {
        self.post_user_config_with_capability(
            session,
            credential,
            CAP_SAVED_SEARCHES_WRITE,
            "ajax_gateway_saved_search_create",
            payload,
        )
        .await
    }

    pub async fn update_saved_search(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeSavedSearch> {
        self.post_user_config_with_capability(
            session,
            credential,
            CAP_SAVED_SEARCHES_WRITE,
            "ajax_gateway_saved_search_update",
            payload,
        )
        .await
    }

    pub async fn delete_saved_search(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.post_user_config_with_capability(
            session,
            credential,
            CAP_SAVED_SEARCHES_WRITE,
            "ajax_gateway_saved_search_delete",
            payload,
        )
        .await
    }
    pub async fn add_message_tag(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.post_user_config_with_capability(
            session,
            credential,
            CAP_TAGS_WRITE,
            "ajax_gateway_message_tag_add",
            payload,
        )
        .await
    }

    pub async fn remove_message_tag(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.post_user_config_with_capability(
            session,
            credential,
            CAP_TAGS_WRITE,
            "ajax_gateway_message_tag_remove",
            payload,
        )
        .await
    }
    pub async fn mailboxes(
        &self,
        session: &CyphtSession,
        account_id: &str,
    ) -> GatewayResult<Vec<BridgeMailbox>> {
        self.get_with_capability(
            session,
            CAP_MAIL_READ,
            "ajax_gateway_mailboxes",
            &[("account_id", account_id)],
        )
        .await
    }

    pub async fn messages(
        &self,
        session: &CyphtSession,
        account_id: &str,
        folder: &str,
        offset: u32,
        limit: u32,
    ) -> GatewayResult<BridgeMessagePage> {
        let offset_s = offset.to_string();
        let limit_s = limit.to_string();
        self.get_with_capability(
            session,
            CAP_MAIL_READ,
            "ajax_gateway_messages",
            &[
                ("account_id", account_id),
                ("folder", folder),
                ("offset", &offset_s),
                ("limit", &limit_s),
            ],
        )
        .await
    }

    pub async fn search(
        &self,
        session: &CyphtSession,
        account_ids: &[String],
        folder: &str,
        query: &str,
        limit: u32,
    ) -> GatewayResult<BridgeMessagePage> {
        let account_ids = account_ids.join(",");
        let limit_s = limit.to_string();
        self.get_with_capability(
            session,
            CAP_MAIL_READ,
            "ajax_gateway_search",
            &[
                ("account_ids", &account_ids),
                ("folder", folder),
                ("query", query),
                ("limit", &limit_s),
            ],
        )
        .await
    }

    pub async fn message(
        &self,
        session: &CyphtSession,
        account_id: &str,
        folder: &str,
        uid: &str,
    ) -> GatewayResult<BridgeMessage> {
        self.get_with_capability(
            session,
            CAP_MAIL_READ,
            "ajax_gateway_message",
            &[("account_id", account_id), ("folder", folder), ("uid", uid)],
        )
        .await
    }

    pub async fn attachment(
        &self,
        session: &CyphtSession,
        account_id: &str,
        folder: &str,
        uid: &str,
        part: &str,
    ) -> GatewayResult<BridgeDownload> {
        self.require_capability(session, CAP_MAIL_READ).await?;
        let mut url = self.page_url("ajax_gateway_attachment")?;
        url.query_pairs_mut()
            .append_pair("account_id", account_id)
            .append_pair("folder", folder)
            .append_pair("uid", uid)
            .append_pair("part", part);
        let mut response = self
            .http
            .get(url)
            .headers(self.bridge_headers(session)?)
            .send()
            .await
            .map_err(upstream)?;
        self.ensure_bridge_status(&response)?;
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let filename = response
            .headers()
            .get(CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .and_then(filename_from_disposition);
        if response
            .content_length()
            .is_some_and(|length| length > MAX_ATTACHMENT_BYTES as u64)
        {
            return Err(attachment_too_large());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(upstream)? {
            if chunk.len() > MAX_ATTACHMENT_BYTES.saturating_sub(bytes.len()) {
                return Err(attachment_too_large());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(BridgeDownload {
            bytes,
            filename,
            content_type,
        })
    }

    pub async fn upload(
        &self,
        session: &CyphtSession,
        filename: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> GatewayResult<BridgeUpload> {
        self.require_capability(session, CAP_MAIL_WRITE).await?;
        let url = self.page_url("ajax_gateway_upload")?;
        let mut headers = self.bridge_headers(session)?;
        headers.insert(
            "x-cypht-gateway-filename",
            HeaderValue::from_str(filename)
                .map_err(|_| GatewayError::InvalidRequest("invalid attachment filename".into()))?,
        );
        headers.insert(
            "x-cypht-gateway-content-type",
            HeaderValue::from_str(content_type).map_err(|_| {
                GatewayError::InvalidRequest("invalid attachment content type".into())
            })?,
        );
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
        let response = self
            .http
            .post(url)
            .headers(headers)
            .body(bytes.to_vec())
            .send()
            .await
            .map_err(upstream)?;
        self.decode_envelope(response).await
    }

    pub async fn send(
        &self,
        session: &CyphtSession,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeWriteResult> {
        self.write_with_capability(session, CAP_MAIL_WRITE, "ajax_gateway_send", payload)
            .await
    }

    pub async fn draft(
        &self,
        session: &CyphtSession,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeWriteResult> {
        self.write_with_capability(session, CAP_MAIL_WRITE, "ajax_gateway_draft", payload)
            .await
    }

    pub async fn update_message(
        &self,
        session: &CyphtSession,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.write_with_capability(
            session,
            CAP_MAIL_WRITE,
            "ajax_gateway_message_update",
            payload,
        )
        .await
    }

    pub async fn move_message(
        &self,
        session: &CyphtSession,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.write_with_capability(
            session,
            CAP_MAIL_WRITE,
            "ajax_gateway_message_move",
            payload,
        )
        .await
    }

    pub async fn archive_message(
        &self,
        session: &CyphtSession,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.write_with_capability(
            session,
            CAP_MAIL_WRITE,
            "ajax_gateway_message_archive",
            payload,
        )
        .await
    }

    pub async fn delete_message(
        &self,
        session: &CyphtSession,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.write_with_capability(
            session,
            CAP_MAIL_WRITE,
            "ajax_gateway_message_delete",
            payload,
        )
        .await
    }

    async fn bridge_get<T: DeserializeOwned>(
        &self,
        session: &CyphtSession,
        page: &str,
        params: &[(&str, &str)],
    ) -> GatewayResult<T> {
        let mut url = self.page_url(page)?;
        {
            let mut pairs = url.query_pairs_mut();
            for (name, value) in params {
                pairs.append_pair(name, value);
            }
        }
        let response = self
            .http
            .get(url)
            .headers(self.bridge_headers(session)?)
            .send()
            .await
            .map_err(upstream)?;
        self.decode_envelope(response).await
    }

    async fn bridge_post<T: DeserializeOwned>(
        &self,
        session: &CyphtSession,
        page: &str,
        payload: &serde_json::Value,
    ) -> GatewayResult<T> {
        self.bridge_post_with_config_key(session, None, page, payload)
            .await
    }

    async fn bridge_post_with_config_key<T: DeserializeOwned>(
        &self,
        session: &CyphtSession,
        credential: Option<&Credential>,
        page: &str,
        payload: &serde_json::Value,
    ) -> GatewayResult<T> {
        let url = self.page_url(page)?;
        let payload = serde_json::to_string(payload)
            .map_err(|e| GatewayError::Internal(format!("encode bridge payload: {e}")))?;
        let mut headers = self.bridge_headers(session)?;
        if let Some(credential) = credential {
            let ping = self.cached_or_fetch_ping(session).await?;
            validate_ping_versions(&ping)?;
            if ping.durable_user_config {
                headers.insert(
                    "x-cypht-gateway-config-key",
                    HeaderValue::from_str(&credential.password)
                        .map_err(|_| GatewayError::Authentication)?,
                );
            }
        }
        let response = self
            .http
            .post(url)
            .headers(headers)
            .form(&[("payload", payload)])
            .send()
            .await
            .map_err(upstream)?;
        self.decode_envelope(response).await
    }

    async fn decode_envelope<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> GatewayResult<T> {
        self.ensure_bridge_status(&response)?;
        let envelope: BridgeEnvelope<T> = response
            .json()
            .await
            .map_err(|e| GatewayError::Upstream(format!("invalid bridge JSON: {e}")))?;
        if !envelope.ok {
            return Err(GatewayError::Upstream(
                envelope
                    .error
                    .unwrap_or_else(|| "bridge operation failed".into()),
            ));
        }
        envelope
            .data
            .ok_or_else(|| GatewayError::Upstream("bridge response missing data".into()))
    }

    fn ensure_bridge_status(&self, response: &reqwest::Response) -> GatewayResult<()> {
        if let Some(error) = bridge_status_error(response.status()) {
            return Err(error);
        }
        Ok(())
    }

    fn bridge_headers(&self, session: &CyphtSession) -> GatewayResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        let cookie = format!("hm_id={}; hm_session={}", session.hm_id, session.hm_session);
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&cookie).map_err(|_| GatewayError::Authentication)?,
        );
        headers.insert(
            "x-cypht-gateway-key",
            HeaderValue::from_str(&self.config.bridge_key)
                .map_err(|_| GatewayError::Configuration("invalid bridge key header".into()))?,
        );
        Ok(headers)
    }

    fn page_url(&self, page: &str) -> GatewayResult<Url> {
        let mut url = self.config.base_url.clone();
        url.set_query(None);
        url.query_pairs_mut().append_pair("page", page);
        Ok(url)
    }
}

fn filename_from_disposition(value: &str) -> Option<String> {
    for part in value.split(';').map(str::trim) {
        if let Some(filename) = part.strip_prefix("filename=") {
            let filename = filename.trim_matches('"').trim();
            if !filename.is_empty() {
                return Some(filename.to_string());
            }
        }
    }
    None
}

fn upstream(error: reqwest::Error) -> GatewayError {
    GatewayError::Upstream(error.to_string())
}

fn validate_ping_versions(ping: &BridgePing) -> GatewayResult<()> {
    if ping.status != "ok" {
        return Err(GatewayError::Upstream("bridge health check failed".into()));
    }
    if ping.bridge_version != env!("CARGO_PKG_VERSION") {
        return Err(GatewayError::Upstream(format!(
            "bridge version mismatch: gateway={}, bridge={}",
            env!("CARGO_PKG_VERSION"),
            ping.bridge_version,
        )));
    }
    if ping.bridge_api_version != SUPPORTED_BRIDGE_API_VERSION {
        return Err(GatewayError::Upstream(format!(
            "unsupported bridge API version: gateway supports {SUPPORTED_BRIDGE_API_VERSION}, bridge reported {}",
            ping.bridge_api_version,
        )));
    }
    tracing::info!(
        bridge_api_version = ping.bridge_api_version,
        upstream_cypht_version = %ping.cypht_version,
        bridge_version = %ping.bridge_version,
        baseline = %CYPHT_BASELINE_VERSION,
        capabilities_count = ping.capabilities.len(),
        capabilities = ?ping.capabilities,
        "Connected to upstream Cypht"
    );
    Ok(())
}
fn attachment_too_large() -> GatewayError {
    GatewayError::PayloadTooLarge("attachment exceeds the 25 MiB download limit".into())
}

fn bridge_status_error(status: StatusCode) -> Option<GatewayError> {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Some(GatewayError::Authentication),
        StatusCode::CONFLICT => Some(GatewayError::Conflict(
            "saved search name already exists or changed".into(),
        )),
        StatusCode::PAYLOAD_TOO_LARGE => Some(attachment_too_large()),
        StatusCode::NOT_IMPLEMENTED => Some(GatewayError::CapabilityUnavailable(
            "Cypht optional module is not available".into(),
        )),
        _ if !status.is_success() => Some(GatewayError::Upstream(format!(
            "bridge returned HTTP {status}"
        ))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_capability_negotiation() {
        let ping = BridgePing {
            status: "ok".into(),
            bridge_version: env!("CARGO_PKG_VERSION").into(),
            bridge_api_version: SUPPORTED_BRIDGE_API_VERSION,
            cypht_version: CYPHT_BASELINE_VERSION.into(),
            durable_user_config: true,
            capabilities: vec![
                "mail.read.v1".into(),
                "user_config.durable.v1".into(),
                "contacts.read.v1".into(),
                "contacts.write.v1".into(),
            ],
        };
        assert!(ping.supports("mail.read.v1"));
        assert!(ping.supports("user_config.durable.v1"));
        assert!(!ping.supports("accounts.write.v1"));
    }

    #[test]
    fn ping_accepts_reviewed_cypht_and_other_upstream_versions() {
        let mut ping = BridgePing {
            status: "ok".into(),
            bridge_version: env!("CARGO_PKG_VERSION").into(),
            bridge_api_version: SUPPORTED_BRIDGE_API_VERSION,
            cypht_version: CYPHT_BASELINE_VERSION.into(),
            durable_user_config: false,
            capabilities: Vec::new(),
        };
        assert!(validate_ping_versions(&ping).is_ok());

        let legacy: BridgePing = serde_json::from_str(
            r#"{"status":"ok","bridge_version":"0.4.0","bridge_api_version":1,"cypht_version":"2.12.0"}"#,
        )
        .expect("legacy ping with version should deserialize");
        assert_eq!(legacy.bridge_api_version, 1);
        assert!(!legacy.durable_user_config);

        // Rejects mismatched bridge API version
        ping.bridge_api_version = 99;
        assert!(validate_ping_versions(&ping).is_err());
        ping.bridge_api_version = SUPPORTED_BRIDGE_API_VERSION;

        // Follow upstream dynamically without hard-rejecting newer releases
        ping.cypht_version = "2.12.2".into();
        assert!(validate_ping_versions(&ping).is_ok());
        ping.cypht_version = "2.13.0".into();
        assert!(validate_ping_versions(&ping).is_ok());
    }
    #[test]
    fn optional_module_unavailable_is_not_an_upstream_failure() {
        assert!(matches!(
            bridge_status_error(StatusCode::NOT_IMPLEMENTED),
            Some(GatewayError::CapabilityUnavailable(_))
        ));
        assert!(matches!(
            bridge_status_error(StatusCode::PAYLOAD_TOO_LARGE),
            Some(GatewayError::PayloadTooLarge(_))
        ));
        assert!(matches!(
            bridge_status_error(StatusCode::CONFLICT),
            Some(GatewayError::Conflict(_))
        ));
    }
}
