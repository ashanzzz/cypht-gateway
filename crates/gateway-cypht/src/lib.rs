use gateway_auth::{Credential, CyphtSession};
use gateway_core::{GatewayError, GatewayResult};
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_DISPOSITION, CONTENT_TYPE, COOKIE},
    Client, StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;
use url::Url;

const MAX_ATTACHMENT_BYTES: usize = 25 * 1024 * 1024;
const CYPHT_BASELINE_VERSION: &str = "2.12.0";

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

#[derive(Debug, Deserialize)]
struct BridgePing {
    status: String,
    bridge_version: String,
    cypht_version: String,
    #[serde(default)]
    durable_user_config: bool,
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

    async fn bridge_ping(&self, session: &CyphtSession) -> GatewayResult<BridgePing> {
        let ping: BridgePing = self.bridge_get(session, "ajax_gateway_ping", &[]).await?;
        validate_ping_versions(&ping)?;
        Ok(ping)
    }

    pub async fn accounts(&self, session: &CyphtSession) -> GatewayResult<Vec<BridgeAccount>> {
        self.bridge_get(session, "ajax_gateway_accounts", &[]).await
    }

    pub async fn profiles(&self, session: &CyphtSession) -> GatewayResult<Vec<BridgeProfile>> {
        self.bridge_get(session, "ajax_gateway_profiles", &[]).await
    }

    pub async fn contacts(
        &self,
        session: &CyphtSession,
        query: Option<&str>,
    ) -> GatewayResult<Vec<BridgeContact>> {
        let params = query
            .map(|value| vec![("query", value)])
            .unwrap_or_default();
        self.bridge_get(session, "ajax_gateway_contacts", &params)
            .await
    }

    pub async fn contact(&self, session: &CyphtSession, id: &str) -> GatewayResult<BridgeContact> {
        self.bridge_get(session, "ajax_gateway_contact", &[("contact_id", id)])
            .await
    }

    pub async fn create_contact(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeContact> {
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            true,
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
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            true,
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
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            true,
            "ajax_gateway_contact_delete",
            payload,
        )
        .await
    }
    pub async fn tags(&self, session: &CyphtSession) -> GatewayResult<Vec<BridgeTag>> {
        self.bridge_get(session, "ajax_gateway_tags", &[]).await
    }

    pub async fn create_tag(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeTag> {
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            true,
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
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            true,
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
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            true,
            "ajax_gateway_tag_delete",
            payload,
        )
        .await
    }

    pub async fn sieve_status(
        &self,
        session: &CyphtSession,
    ) -> GatewayResult<Vec<BridgeSieveStatus>> {
        self.bridge_get(session, "ajax_gateway_sieve_status", &[])
            .await
    }

    pub async fn calendar_events(
        &self,
        session: &CyphtSession,
        start_at: i64,
        end_at: i64,
    ) -> GatewayResult<Vec<BridgeCalendarEvent>> {
        self.bridge_get(
            session,
            "ajax_gateway_calendar_events",
            &[
                ("start_at", &start_at.to_string()),
                ("end_at", &end_at.to_string()),
            ],
        )
        .await
    }

    pub async fn create_calendar_event(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeCalendarEvent> {
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            true,
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
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            true,
            "ajax_gateway_calendar_delete",
            payload,
        )
        .await
    }
    pub async fn feeds(&self, session: &CyphtSession) -> GatewayResult<Vec<BridgeFeed>> {
        self.bridge_get(session, "ajax_gateway_feeds", &[]).await
    }

    pub async fn feed(&self, session: &CyphtSession, id: &str) -> GatewayResult<BridgeFeed> {
        self.bridge_get(session, "ajax_gateway_feed", &[("feed_id", id)])
            .await
    }
    pub async fn saved_searches(
        &self,
        session: &CyphtSession,
    ) -> GatewayResult<Vec<BridgeSavedSearch>> {
        self.bridge_get(session, "ajax_gateway_saved_searches", &[])
            .await
    }

    pub async fn create_saved_search(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeSavedSearch> {
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            true,
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
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            true,
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
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            true,
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
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            true,
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
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            true,
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
        self.bridge_get(
            session,
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
        self.bridge_get(
            session,
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
        self.bridge_get(
            session,
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
        self.bridge_get(
            session,
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
        self.bridge_post(session, "ajax_gateway_send", payload)
            .await
    }

    pub async fn draft(
        &self,
        session: &CyphtSession,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeWriteResult> {
        self.bridge_post(session, "ajax_gateway_draft", payload)
            .await
    }

    pub async fn update_message(
        &self,
        session: &CyphtSession,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.bridge_post(session, "ajax_gateway_message_update", payload)
            .await
    }

    pub async fn move_message(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            false,
            "ajax_gateway_message_move",
            payload,
        )
        .await
    }

    pub async fn archive_message(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            false,
            "ajax_gateway_message_archive",
            payload,
        )
        .await
    }

    pub async fn delete_message(
        &self,
        session: &CyphtSession,
        credential: &Credential,
        payload: &serde_json::Value,
    ) -> GatewayResult<BridgeActionResult> {
        self.bridge_post_with_config_key(
            session,
            Some(credential),
            false,
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
        self.bridge_post_with_config_key(session, None, false, page, payload)
            .await
    }

    async fn bridge_post_with_config_key<T: DeserializeOwned>(
        &self,
        session: &CyphtSession,
        credential: Option<&Credential>,
        require_storage: bool,
        page: &str,
        payload: &serde_json::Value,
    ) -> GatewayResult<T> {
        let url = self.page_url(page)?;
        let payload = serde_json::to_string(payload)
            .map_err(|e| GatewayError::Internal(format!("encode bridge payload: {e}")))?;
        let mut headers = self.bridge_headers(session)?;
        if let Some(credential) = credential {
            let ping: BridgePing = self.bridge_get(session, "ajax_gateway_ping", &[]).await?;
            validate_ping_versions(&ping)?;
            if require_storage && !ping.durable_user_config {
                return Err(GatewayError::CapabilityUnavailable(
                    "durable user-config writes".into(),
                ));
            }
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
        return Err(GatewayError::Upstream(
            "bridge and Gateway versions do not match".into(),
        ));
    }
    if ping.cypht_version != CYPHT_BASELINE_VERSION {
        return Err(GatewayError::CapabilityUnavailable(format!(
            "Cypht {CYPHT_BASELINE_VERSION} is required for this Bridge",
        )));
    }
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
    fn ping_accepts_reviewed_cypht_and_rejects_other_versions() {
        let mut ping = BridgePing {
            status: "ok".into(),
            bridge_version: env!("CARGO_PKG_VERSION").into(),
            cypht_version: CYPHT_BASELINE_VERSION.into(),
            durable_user_config: false,
        };
        assert!(validate_ping_versions(&ping).is_ok());
        let legacy: BridgePing = serde_json::from_str(
            r#"{"status":"ok","bridge_version":"0.4.0","cypht_version":"2.12.0"}"#,
        )
        .expect("legacy ping without storage flag should deserialize");
        assert!(!legacy.durable_user_config);
        ping.cypht_version = "2.12.2".into();
        assert!(matches!(
            validate_ping_versions(&ping),
            Err(GatewayError::CapabilityUnavailable(_))
        ));
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
