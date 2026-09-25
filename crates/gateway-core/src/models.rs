use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeResponse {
    pub username: String,
    pub auth_kind: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub account_allowlist: Vec<String>,
    pub expires_in_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedToken {
    pub id: String,
    pub name: String,
    pub token: String,
    pub prefix: String,
    pub scopes: Vec<String>,
    pub account_allowlist: Vec<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<String>,
    pub account_allowlist: Vec<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub protocol: String,
    pub server: Option<String>,
    pub can_send: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub address: String,
    pub reply_to: String,
    pub signature: String,
    pub account_id: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mailbox {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub display_name: String,
    pub role: Option<String>,
    pub total: Option<u64>,
    pub unread: Option<u64>,
    pub selectable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Address {
    pub name: Option<String>,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSummary {
    pub id: String,
    pub account_id: String,
    pub mailbox_id: String,
    pub subject: String,
    pub from: Vec<Address>,
    pub to: Vec<Address>,
    pub date: Option<String>,
    pub timestamp: Option<i64>,
    pub unread: bool,
    pub flagged: bool,
    pub has_attachments: bool,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub size: Option<u64>,
    pub inline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageBody {
    pub text: Option<String>,
    pub html: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub account_id: String,
    pub mailbox_id: String,
    pub subject: String,
    pub from: Vec<Address>,
    pub to: Vec<Address>,
    pub cc: Vec<Address>,
    pub date: Option<String>,
    pub message_id_header: Option<String>,
    pub in_reply_to: Option<String>,
    pub body: MessageBody,
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub headers: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePage {
    pub messages: Vec<MessageSummary>,
    pub total: Option<u64>,
    pub next_offset: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    #[serde(default)]
    pub account_ids: Vec<String>,
    pub mailbox_id: Option<String>,
    pub query: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub profile_id: Option<String>,
    #[serde(default)]
    pub to: Vec<Address>,
    #[serde(default)]
    pub cc: Vec<Address>,
    #[serde(default)]
    pub bcc: Vec<Address>,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub body: MessageBody,
    #[serde(default)]
    pub attachment_ids: Vec<String>,
    pub schedule_at: Option<String>,
    #[serde(default)]
    pub delivery_receipt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyMessageRequest {
    pub profile_id: Option<String>,
    #[serde(default)]
    pub reply_all: bool,
    #[serde(default)]
    pub body: MessageBody,
    #[serde(default)]
    pub attachment_ids: Vec<String>,
    pub schedule_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardMessageRequest {
    pub profile_id: Option<String>,
    #[serde(default)]
    pub to: Vec<Address>,
    #[serde(default)]
    pub cc: Vec<Address>,
    #[serde(default)]
    pub bcc: Vec<Address>,
    #[serde(default)]
    pub body: MessageBody,
    #[serde(default)]
    pub attachment_ids: Vec<String>,
    pub schedule_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageUpdateRequest {
    pub seen: Option<bool>,
    pub flagged: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveMessageRequest {
    pub mailbox_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub source: String,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactCreateRequest {
    pub name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TagCreateRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TagUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavedSearchType {
    Simple,
    Advanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedSearchTerm {
    pub term: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedSearchTarget {
    pub target: String,
    pub orig: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedSearchSource {
    pub account_id: String,
    pub mailbox_id: Option<String>,
    #[serde(default)]
    pub all_folders: bool,
    #[serde(default)]
    pub subfolders: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedSearchTimeRange {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedSearchOther {
    pub limit: u32,
    #[serde(default)]
    pub flags: Vec<String>,
    pub charset: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedSavedSearchData {
    pub terms: Vec<AdvancedSearchTerm>,
    pub targets: Vec<AdvancedSearchTarget>,
    pub sources: Vec<AdvancedSearchSource>,
    pub times: Vec<AdvancedSearchTimeRange>,
    pub other: AdvancedSearchOther,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSearch {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: SavedSearchType,
    pub query: Option<String>,
    pub since: Option<String>,
    pub field: Option<String>,
    pub advanced: Option<AdvancedSavedSearchData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SavedSearchCreateRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: SavedSearchType,
    pub query: Option<String>,
    pub since: Option<String>,
    pub field: Option<String>,
    pub advanced: Option<AdvancedSavedSearchData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SavedSearchUpdateRequest {
    pub name: Option<String>,
    pub query: Option<String>,
    pub since: Option<String>,
    pub field: Option<String>,
    pub advanced: Option<AdvancedSavedSearchData>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CalendarRepeatInterval {
    #[default]
    None,
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SieveStatus {
    pub account_id: String,
    pub name: String,
    pub protocol: String,
    pub enabled: bool,
    pub configured: bool,
    pub status: String,
    pub remote_probe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub provider: String,
    pub timezone: Option<String>,
    pub capabilities: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub description: String,
    pub starts_at: String,
    pub occurrence_at: String,
    pub repeat_interval: CalendarRepeatInterval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarEventCreateRequest {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub starts_at: String,
    #[serde(default)]
    pub repeat_interval: CalendarRepeatInterval,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSubscription {
    pub id: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Upload {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct AttachmentDownload {
    pub bytes: Vec<u8>,
    pub filename: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailWriteResult {
    pub status: String,
    pub message_id_header: Option<String>,
    pub message_id: Option<String>,
    pub scheduled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub status: String,
    pub mailbox_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_sync: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    pub created_at: i64,
    pub username: String,
    pub auth_id: Option<String>,
    pub operation: String,
    pub resource: Option<String>,
    pub success: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPage {
    pub entries: Vec<AuditEntry>,
    pub next_offset: Option<u64>,
}
