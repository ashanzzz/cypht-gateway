mod calendar;
mod feeds;
mod saved_search_advanced;
mod saved_search_ids;
mod saved_search_validation;
mod saved_search_writes;
mod saved_searches;
mod sieve;
mod tags;

use gateway_auth::{
    AuthKind, AuthService, Credential, CyphtSession, Principal, SCOPE_ACCOUNTS_READ,
    SCOPE_ATTACHMENTS_READ, SCOPE_AUDIT_READ, SCOPE_CONTACTS_READ, SCOPE_CONTACTS_WRITE,
    SCOPE_MAIL_DELETE, SCOPE_MAIL_MODIFY, SCOPE_MAIL_READ, SCOPE_MAIL_SEARCH, SCOPE_MAIL_SEND,
};
use gateway_core::{
    Account, ActionResult, Address, Attachment, AttachmentDownload, AuditPage, Contact,
    ContactCreateRequest, ContactUpdateRequest, CreatedToken, ForwardMessageRequest, GatewayError,
    GatewayResult, LoginResponse, MailWriteResult, Mailbox, Message, MessageBody, MessagePage,
    MessageSummary, MessageUpdateRequest, MoveMessageRequest, ObjectIdCodec, ObjectKind, Profile,
    ReplyMessageRequest, SearchRequest, SendMessageRequest, TokenMetadata, Upload,
};
use gateway_cypht::{
    BridgeAccount, BridgeActionResult, BridgeContact, BridgeMessage, BridgeMessagePage,
    BridgeMessageSummary, BridgeWriteResult, CyphtClient,
};
use gateway_storage::{Store, StoredIdempotency, StoredObjectRef};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct GatewayService {
    auth: AuthService,
    cypht: CyphtClient,
    ids: ObjectIdCodec,
    store: Store,
}

impl GatewayService {
    pub fn new(auth: AuthService, cypht: CyphtClient, ids: ObjectIdCodec, store: Store) -> Self {
        Self {
            auth,
            cypht,
            ids,
            store,
        }
    }

    pub async fn login_sso(
        &self,
        provided_key: &str,
        username: String,
        hm_id: String,
        hm_session: String,
    ) -> GatewayResult<LoginResponse> {
        if provided_key != self.cypht.bridge_key() {
            return Err(GatewayError::Authentication);
        }
        if username.trim().is_empty() || hm_id.is_empty() || hm_session.is_empty() {
            return Err(GatewayError::InvalidRequest(
                "invalid sso parameters".into(),
            ));
        }
        let session = CyphtSession { hm_id, hm_session };
        self.cypht.ping(&session).await?;
        let credential = Credential {
            username: username.trim().to_string(),
            password: String::new(),
        };
        let (token, ttl) = self.auth.issue_session(&credential, &session)?;
        self.audit(&credential.username, None, "auth.sso", None, true, None);
        Ok(LoginResponse {
            access_token: token,
            token_type: "Bearer".into(),
            expires_in: ttl,
        })
    }

    pub async fn login(&self, username: String, password: String) -> GatewayResult<LoginResponse> {
        if username.trim().is_empty() || password.is_empty() {
            return Err(GatewayError::Authentication);
        }
        let credential = Credential {
            username: username.trim().to_string(),
            password,
        };
        let session = self.cypht.login(&credential).await?;
        self.cypht.ping(&session).await?;
        let (token, ttl) = self.auth.issue_session(&credential, &session)?;
        self.audit(&credential.username, None, "auth.login", None, true, None);
        Ok(LoginResponse {
            access_token: token,
            token_type: "Bearer".into(),
            expires_in: ttl,
        })
    }

    pub fn authenticate(&self, bearer: &str) -> GatewayResult<Principal> {
        self.auth.authenticate(bearer)
    }

    pub fn logout(&self, bearer: &str, principal: &Principal) -> GatewayResult<()> {
        if matches!(&principal.auth_kind, AuthKind::Pat) {
            return Err(GatewayError::InvalidRequest(
                "personal access tokens must be revoked through the token endpoint".into(),
            ));
        }
        self.auth.logout(bearer)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "auth.logout",
            None,
            true,
            None,
        );
        Ok(())
    }

    pub fn create_pat(
        &self,
        principal: &Principal,
        name: &str,
        scopes: Vec<String>,
        account_allowlist: Vec<String>,
        expires_in_days: Option<u32>,
    ) -> GatewayResult<CreatedToken> {
        for account_id in &account_allowlist {
            let decoded = self.ids.decode_versioned(ObjectKind::Account, account_id)?;
            if decoded.version != 2 {
                return Err(GatewayError::InvalidRequest(
                    "new account allow-lists require owner-bound version-2 IDs".into(),
                ));
            }
            let _ = self.decode_account(principal, account_id)?;
        }
        let token =
            self.auth
                .create_pat(principal, name, scopes, account_allowlist, expires_in_days)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "token.create",
            Some(&token.id),
            true,
            None,
        );
        Ok(token)
    }

    pub fn list_pats(&self, principal: &Principal) -> GatewayResult<Vec<TokenMetadata>> {
        self.auth.list_pats(principal)
    }

    pub fn revoke_pat(&self, principal: &Principal, id: &str) -> GatewayResult<()> {
        if !self.auth.revoke_pat(principal, id)? {
            return Err(GatewayError::NotFound("API token".into()));
        }
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "token.revoke",
            Some(id),
            true,
            None,
        );
        Ok(())
    }

    pub fn audit_logs(
        &self,
        principal: &Principal,
        offset: u64,
        limit: u32,
    ) -> GatewayResult<AuditPage> {
        principal.requires(SCOPE_AUDIT_READ)?;
        let limit = limit.clamp(1, 200);
        let entries = self.store.list_audit(&principal.username, offset, limit)?;
        let next_offset = (entries.len() == limit as usize).then_some(offset + u64::from(limit));
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "audit.list",
            None,
            true,
            None,
        );
        Ok(AuditPage {
            entries,
            next_offset,
        })
    }

    pub async fn contacts(
        &self,
        principal: &Principal,
        query: Option<&str>,
    ) -> GatewayResult<Vec<Contact>> {
        principal.requires(SCOPE_CONTACTS_READ)?;
        self.assert_global_resource_allowed(principal)?;
        if query.is_some_and(|value| value.len() > 200) {
            return Err(GatewayError::InvalidRequest(
                "contact query is too long".into(),
            ));
        }
        let session = self.ensure_cypht_session(principal).await?;
        let rows = self.cypht.contacts(&session, query).await?;
        let contacts = rows
            .into_iter()
            .map(|row| self.map_contact(principal, row))
            .collect::<GatewayResult<Vec<_>>>()?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "contacts.list",
            None,
            true,
            None,
        );
        Ok(contacts)
    }

    pub async fn contact(&self, principal: &Principal, id: &str) -> GatewayResult<Contact> {
        principal.requires(SCOPE_CONTACTS_READ)?;
        self.assert_global_resource_allowed(principal)?;
        let row = self.resolve_contact(principal, id).await?;
        let contact = self.map_contact(principal, row)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "contacts.read",
            Some(id),
            true,
            None,
        );
        Ok(contact)
    }

    pub async fn create_contact(
        &self,
        principal: &Principal,
        request: ContactCreateRequest,
    ) -> GatewayResult<Contact> {
        let result = self.create_contact_inner(principal, request).await;
        self.audit_write(
            principal,
            "contacts.create",
            result.as_ref().ok().map(|contact| contact.id.as_str()),
            result.is_ok(),
        )?;
        result
    }

    pub async fn update_contact(
        &self,
        principal: &Principal,
        id: &str,
        request: ContactUpdateRequest,
    ) -> GatewayResult<Contact> {
        let result = self.update_contact_inner(principal, id, request).await;
        self.audit_write(principal, "contacts.update", Some(id), result.is_ok())?;
        result
    }

    pub async fn delete_contact(
        &self,
        principal: &Principal,
        id: &str,
        confirm: bool,
    ) -> GatewayResult<()> {
        let result = self.delete_contact_inner(principal, id, confirm).await;
        self.audit_write(principal, "contacts.delete", Some(id), result.is_ok())?;
        result
    }

    async fn create_contact_inner(
        &self,
        principal: &Principal,
        request: ContactCreateRequest,
    ) -> GatewayResult<Contact> {
        principal.requires(SCOPE_CONTACTS_WRITE)?;
        self.assert_global_resource_allowed(principal)?;
        validate_contact_name(&request.name)?;
        validate_contact_email(&request.email)?;
        validate_contact_optional(request.phone.as_deref())?;
        validate_contact_optional(request.group.as_deref())?;
        let session = self.ensure_cypht_session(principal).await?;
        let payload = serde_json::to_value(&request)
            .map_err(|e| GatewayError::Internal(format!("encode contact: {e}")))?;
        let row = self
            .cypht
            .create_contact(&session, &principal.credential, &payload)
            .await?;
        self.map_contact(principal, row)
    }

    async fn update_contact_inner(
        &self,
        principal: &Principal,
        id: &str,
        request: ContactUpdateRequest,
    ) -> GatewayResult<Contact> {
        principal.requires(SCOPE_CONTACTS_WRITE)?;
        self.assert_global_resource_allowed(principal)?;
        if request.name.is_none()
            && request.email.is_none()
            && request.phone.is_none()
            && request.group.is_none()
        {
            return Err(GatewayError::InvalidRequest(
                "contact update is empty".into(),
            ));
        }
        if let Some(name) = request.name.as_deref() {
            validate_contact_name(name)?;
        }
        if let Some(email) = request.email.as_deref() {
            validate_contact_email(email)?;
        }
        validate_contact_optional(request.phone.as_deref())?;
        validate_contact_optional(request.group.as_deref())?;
        let row = self.resolve_contact(principal, id).await?;
        let session = self.ensure_cypht_session(principal).await?;
        let mut payload = serde_json::to_value(&request)
            .map_err(|e| GatewayError::Internal(format!("encode contact update: {e}")))?;
        payload["contact_id"] = Value::String(row.id);
        let updated = self
            .cypht
            .update_contact(&session, &principal.credential, &payload)
            .await?;
        self.map_contact(principal, updated)
    }

    async fn delete_contact_inner(
        &self,
        principal: &Principal,
        id: &str,
        confirm: bool,
    ) -> GatewayResult<()> {
        principal.requires(SCOPE_CONTACTS_WRITE)?;
        self.assert_global_resource_allowed(principal)?;
        if !confirm {
            return Err(GatewayError::InvalidRequest(
                "confirm=true is required".into(),
            ));
        }
        let row = self.resolve_contact(principal, id).await?;
        let session = self.ensure_cypht_session(principal).await?;
        self.cypht
            .delete_contact(
                &session,
                &principal.credential,
                &serde_json::json!({"contact_id": row.id, "confirm": true}),
            )
            .await?;
        Ok(())
    }

    async fn resolve_contact(
        &self,
        principal: &Principal,
        id: &str,
    ) -> GatewayResult<BridgeContact> {
        let decoded = self.ids.decode_versioned(ObjectKind::Contact, id)?;
        let valid_id = match decoded.version {
            2 => decoded.parts.len() == 1,
            1 => decoded.parts.len() == 2 && decoded.parts[0] == principal.username,
            _ => false,
        };
        if !valid_id {
            return Err(GatewayError::NotFound("contact".into()));
        }
        let session = self.ensure_cypht_session(principal).await?;
        for row in self.cypht.contacts(&session, None).await? {
            if self.contact_public_id(principal, &row)? == id {
                return Ok(row);
            }
        }
        Err(GatewayError::NotFound("contact".into()))
    }

    fn contact_public_id(
        &self,
        principal: &Principal,
        row: &BridgeContact,
    ) -> GatewayResult<String> {
        self.ids.encode_private(
            ObjectKind::Contact,
            principal.username.as_str(),
            &row.source,
            [row.id.as_str()],
        )
    }

    fn map_contact(&self, principal: &Principal, row: BridgeContact) -> GatewayResult<Contact> {
        Ok(Contact {
            id: self.contact_public_id(principal, &row)?,
            source: row.source,
            name: row.name,
            email: row.email,
            phone: nonempty(&row.phone),
            group: nonempty(&row.group),
        })
    }

    fn assert_global_resource_allowed(&self, principal: &Principal) -> GatewayResult<()> {
        if principal.account_allowlist.is_empty() {
            Ok(())
        } else {
            Err(GatewayError::PermissionDenied(
                "account-restricted tokens cannot access global user-config resources".into(),
            ))
        }
    }

    fn audit_write(
        &self,
        principal: &Principal,
        operation: &str,
        resource: Option<&str>,
        success: bool,
    ) -> GatewayResult<()> {
        self.store.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            operation,
            resource,
            success,
            None,
            gateway_auth::now_unix(),
        )
    }
    pub async fn accounts(&self, principal: &Principal) -> GatewayResult<Vec<Account>> {
        principal.requires(SCOPE_ACCOUNTS_READ)?;
        let session = self.ensure_cypht_session(principal).await?;
        let accounts = self.cypht.accounts(&session).await?;
        let mut result = Vec::new();
        for account in accounts {
            let public_id = self.account_public_id(principal, &account.id)?;
            if self.account_allowed(principal, &public_id)? {
                result.push(self.map_account(account, public_id));
            }
        }
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "accounts.list",
            None,
            true,
            None,
        );
        Ok(result)
    }

    pub async fn profiles(&self, principal: &Principal) -> GatewayResult<Vec<Profile>> {
        principal.requires(SCOPE_MAIL_SEND)?;
        let session = self.ensure_cypht_session(principal).await?;
        let rows = self.cypht.profiles(&session).await?;
        let mut result = Vec::new();
        for row in rows {
            let account_id = match row.account_id.as_deref() {
                Some(internal) => {
                    let public = self.account_public_id(principal, internal)?;
                    if !self.account_allowed(principal, &public)? {
                        continue;
                    }
                    Some(public)
                }
                None if !principal.account_allowlist.is_empty() => continue,
                None => None,
            };
            result.push(Profile {
                id: self.ids.encode(ObjectKind::Profile, [row.id.as_str()])?,
                name: row.name,
                address: row.address,
                reply_to: row.reply_to,
                signature: row.signature,
                account_id,
                is_default: row.is_default,
            });
        }
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "profiles.list",
            None,
            true,
            None,
        );
        Ok(result)
    }

    pub async fn mailboxes(
        &self,
        principal: &Principal,
        account_id: &str,
    ) -> GatewayResult<Vec<Mailbox>> {
        principal.requires(SCOPE_ACCOUNTS_READ)?;
        let internal = self.decode_account(principal, account_id)?;
        self.assert_account_allowed(principal, account_id)?;
        let session = self.ensure_cypht_session(principal).await?;
        let rows = self.cypht.mailboxes(&session, &internal).await?;
        rows.into_iter()
            .map(|row| {
                let id = self.encode_cypht_object_id(
                    principal,
                    ObjectKind::Mailbox,
                    [internal.as_str(), row.name.as_str()],
                )?;
                Ok(Mailbox {
                    id,
                    account_id: account_id.to_string(),
                    name: row.name,
                    display_name: row.display_name,
                    role: row.role,
                    total: row.total,
                    unread: row.unread,
                    selectable: row.selectable,
                })
            })
            .collect()
    }

    pub async fn messages(
        &self,
        principal: &Principal,
        account_id: Option<&str>,
        mailbox_id: Option<&str>,
        offset: u32,
        limit: u32,
    ) -> GatewayResult<MessagePage> {
        principal.requires(SCOPE_MAIL_READ)?;
        let limit = limit.clamp(1, 100);
        let session = self.ensure_cypht_session(principal).await?;

        if let Some(account_id) = account_id {
            let internal = self.decode_account(principal, account_id)?;
            self.assert_account_allowed(principal, account_id)?;
            let folder = match mailbox_id {
                Some(mailbox_id) => {
                    self.decode_mailbox_for_account(principal, mailbox_id, &internal)?
                }
                None => "INBOX".into(),
            };
            let page = self
                .cypht
                .messages(&session, &internal, &folder, offset, limit)
                .await?;
            let mapped = self.map_page(principal, page, offset as u64)?;
            self.audit(
                &principal.username,
                principal.auth_id.as_deref(),
                "mail.list",
                Some(account_id),
                true,
                None,
            );
            return Ok(mapped);
        }

        if mailbox_id.is_some() {
            return Err(GatewayError::InvalidRequest(
                "mailbox_id requires account_id".into(),
            ));
        }

        if offset > 1_000 {
            return Err(GatewayError::InvalidRequest(
                "unified inbox offset cannot exceed 1000".into(),
            ));
        }
        let accounts = self.cypht.accounts(&session).await?;
        let mut all = Vec::new();
        let fetch_limit = offset.saturating_add(limit).clamp(1, 1_100);
        for account in accounts {
            let public_id = self.account_public_id(principal, &account.id)?;
            if !self.account_allowed(principal, &public_id)? {
                continue;
            }
            let page = self
                .cypht
                .messages(&session, &account.id, "INBOX", 0, fetch_limit)
                .await?;
            all.extend(page.messages);
        }
        all.sort_by(|a, b| {
            b.timestamp
                .unwrap_or_default()
                .cmp(&a.timestamp.unwrap_or_default())
        });
        let start = (offset as usize).min(all.len());
        let end = start.saturating_add(limit as usize).min(all.len());
        let has_more = end < all.len();
        let messages = all[start..end]
            .iter()
            .cloned()
            .map(|m| self.map_summary(principal, m))
            .collect::<GatewayResult<Vec<_>>>()?;
        let next_offset = has_more.then_some(end as u64);
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "mail.list_unified",
            None,
            true,
            None,
        );
        Ok(MessagePage {
            messages,
            total: None,
            next_offset,
        })
    }

    pub async fn search(
        &self,
        principal: &Principal,
        request: SearchRequest,
    ) -> GatewayResult<MessagePage> {
        principal.requires(SCOPE_MAIL_SEARCH)?;
        let query = request.query.trim();
        if query.is_empty() {
            return Err(GatewayError::InvalidRequest("query cannot be empty".into()));
        }
        if query.chars().count() > 1024 {
            return Err(GatewayError::InvalidRequest(
                "query cannot exceed 1024 characters".into(),
            ));
        }
        if request.account_ids.len() > 100 {
            return Err(GatewayError::InvalidRequest(
                "account_ids cannot contain more than 100 entries".into(),
            ));
        }
        let session = self.ensure_cypht_session(principal).await?;
        let mailbox = request
            .mailbox_id
            .as_deref()
            .map(|mailbox_id| {
                let decoded =
                    self.decode_cypht_object_id(principal, ObjectKind::Mailbox, mailbox_id)?;
                let (account, folder) = match decoded.version {
                    1 if decoded.parts.len() == 2 => {
                        (decoded.parts[0].clone(), decoded.parts[1].clone())
                    }
                    2 if decoded.parts.len() == 3 && decoded.parts[0] == "cypht" => {
                        (decoded.parts[1].clone(), decoded.parts[2].clone())
                    }
                    _ => {
                        return Err(GatewayError::InvalidRequest(
                            "mailbox id payload is incomplete".into(),
                        ))
                    }
                };
                Ok((account, folder))
            })
            .transpose()?;

        let accounts = if request.account_ids.is_empty() {
            if let Some((mailbox_account, _)) = &mailbox {
                let public_id = self.account_public_id(principal, mailbox_account)?;
                self.assert_account_allowed(principal, &public_id)?;
                vec![mailbox_account.clone()]
            } else {
                let mut allowed = Vec::new();
                for account in self.cypht.accounts(&session).await? {
                    let public_id = self.account_public_id(principal, &account.id)?;
                    if self.account_allowed(principal, &public_id)? {
                        allowed.push(account.id);
                    }
                }
                allowed
            }
        } else {
            let mut ids = Vec::new();
            for id in &request.account_ids {
                let internal = self.decode_account(principal, id)?;
                self.assert_account_allowed(principal, id)?;
                ids.push(internal);
            }
            ids
        };

        if let Some((mailbox_account, _)) = &mailbox {
            if accounts.iter().any(|account| account != mailbox_account) {
                return Err(GatewayError::InvalidRequest(
                    "mailbox_id can only be searched with its owning account".into(),
                ));
            }
        }

        if accounts.is_empty() {
            return Ok(MessagePage {
                messages: Vec::new(),
                total: Some(0),
                next_offset: None,
            });
        }
        let folder = mailbox
            .map(|(_, folder)| folder)
            .unwrap_or_else(|| "INBOX".into());
        let page = self
            .cypht
            .search(
                &session,
                &accounts,
                &folder,
                query,
                request.limit.unwrap_or(50).clamp(1, 100),
            )
            .await?;
        let result = self.map_page(principal, page, 0)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "mail.search",
            None,
            true,
            None,
        );
        Ok(result)
    }

    pub async fn message(&self, principal: &Principal, message_id: &str) -> GatewayResult<Message> {
        principal.requires(SCOPE_MAIL_READ)?;
        let (account, folder, uid) = self.decode_message_for_principal(principal, message_id)?;
        let session = self.ensure_cypht_session(principal).await?;
        let raw = self
            .cypht
            .message(&session, &account, &folder, &uid)
            .await?;
        let mapped = self.map_message(principal, raw)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "mail.read",
            Some(message_id),
            true,
            None,
        );
        Ok(mapped)
    }

    pub async fn attachment(
        &self,
        principal: &Principal,
        attachment_id: &str,
    ) -> GatewayResult<AttachmentDownload> {
        principal.requires(SCOPE_ATTACHMENTS_READ)?;
        let (account, folder, uid, part) =
            self.decode_attachment_for_principal(principal, attachment_id)?;
        let session = self.ensure_cypht_session(principal).await?;
        let raw = self
            .cypht
            .attachment(&session, &account, &folder, &uid, &part)
            .await?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "attachment.read",
            Some(attachment_id),
            true,
            None,
        );
        Ok(AttachmentDownload {
            bytes: raw.bytes,
            filename: raw.filename,
            content_type: raw.content_type,
        })
    }

    pub async fn upload(
        &self,
        principal: &Principal,
        filename: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> GatewayResult<Upload> {
        principal.requires(SCOPE_MAIL_SEND)?;
        let filename = filename.trim();
        if filename.is_empty() || filename.chars().count() > 255 {
            return Err(GatewayError::InvalidRequest(
                "attachment filename must contain 1-255 characters".into(),
            ));
        }
        if bytes.is_empty() {
            return Err(GatewayError::InvalidRequest(
                "attachment body cannot be empty".into(),
            ));
        }
        let content_type = if content_type.trim().is_empty() {
            "application/octet-stream"
        } else {
            content_type.trim()
        };
        if content_type.len() > 200 {
            return Err(GatewayError::InvalidRequest(
                "attachment content type is too long".into(),
            ));
        }
        let session = self.ensure_cypht_session(principal).await?;
        let raw = self
            .cypht
            .upload(&session, filename, content_type, bytes)
            .await?;
        let id = self.ids.encode(
            ObjectKind::Upload,
            [principal.username.as_str(), raw.id.as_str()],
        )?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "attachment.upload",
            Some(&id),
            true,
            None,
        );
        Ok(Upload {
            id,
            filename: raw.filename,
            content_type: raw.content_type,
            size: raw.size,
        })
    }

    pub async fn send(
        &self,
        principal: &Principal,
        request: SendMessageRequest,
        idempotency_key: &str,
    ) -> GatewayResult<MailWriteResult> {
        principal.requires(SCOPE_MAIL_SEND)?;
        self.validate_send_request(&request, true)?;
        let operation = "mail.send";
        if let Some(cached) =
            self.idempotency_lookup(principal, operation, idempotency_key, &request)?
        {
            return Ok(cached);
        }
        let session = self.ensure_cypht_session(principal).await?;
        let payload = self.send_payload(principal, &request, None)?;
        if let Some(cached) =
            self.idempotency_claim(principal, operation, idempotency_key, &request)?
        {
            return Ok(cached);
        }
        let raw = self.cypht.send(&session, &payload).await?;
        let result = self.map_write_result(principal, raw)?;
        self.idempotency_store(principal, operation, idempotency_key, &request, &result)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            operation,
            result.message_id.as_deref(),
            true,
            None,
        );
        Ok(result)
    }

    pub async fn draft(
        &self,
        principal: &Principal,
        request: SendMessageRequest,
        idempotency_key: &str,
    ) -> GatewayResult<MailWriteResult> {
        principal.requires(SCOPE_MAIL_SEND)?;
        self.validate_send_request(&request, false)?;
        let operation = "mail.draft";
        if let Some(cached) =
            self.idempotency_lookup(principal, operation, idempotency_key, &request)?
        {
            return Ok(cached);
        }
        let session = self.ensure_cypht_session(principal).await?;
        let payload = self.send_payload(principal, &request, None)?;
        if let Some(cached) =
            self.idempotency_claim(principal, operation, idempotency_key, &request)?
        {
            return Ok(cached);
        }
        let raw = self.cypht.draft(&session, &payload).await?;
        let result = self.map_write_result(principal, raw)?;
        self.idempotency_store(principal, operation, idempotency_key, &request, &result)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            operation,
            result.message_id.as_deref(),
            true,
            None,
        );
        Ok(result)
    }

    pub async fn reply(
        &self,
        principal: &Principal,
        message_id: &str,
        request: ReplyMessageRequest,
        idempotency_key: &str,
    ) -> GatewayResult<MailWriteResult> {
        principal.requires(SCOPE_MAIL_SEND)?;
        principal.requires(SCOPE_MAIL_READ)?;
        let operation = "mail.reply";
        #[derive(Serialize)]
        struct Replay<'a> {
            message_id: &'a str,
            request: &'a ReplyMessageRequest,
        }
        let replay = Replay {
            message_id,
            request: &request,
        };
        if let Some(cached) =
            self.idempotency_lookup(principal, operation, idempotency_key, &replay)?
        {
            return Ok(cached);
        }
        let original = self.message(principal, message_id).await?;
        let profile = self
            .resolve_profile(principal, request.profile_id.as_deref())
            .await?;
        let mut to = reply_target(&original);
        let mut cc = Vec::new();
        if request.reply_all {
            merge_addresses(&mut to, &original.to);
            merge_addresses(&mut cc, &original.cc);
            remove_address(&mut to, &profile.address);
            remove_address(&mut cc, &profile.address);
        }
        if to.is_empty() {
            return Err(GatewayError::InvalidRequest(
                "original message has no reply address".into(),
            ));
        }
        let send = SendMessageRequest {
            profile_id: Some(profile.id),
            to,
            cc,
            bcc: Vec::new(),
            subject: reply_subject(&original.subject),
            body: request.body.clone(),
            attachment_ids: request.attachment_ids.clone(),
            schedule_at: request.schedule_at.clone(),
            delivery_receipt: false,
        };
        self.validate_send_request(&send, true)?;
        let session = self.ensure_cypht_session(principal).await?;
        let payload = self.send_payload(principal, &send, original.message_id_header.as_deref())?;
        if let Some(cached) =
            self.idempotency_claim(principal, operation, idempotency_key, &replay)?
        {
            return Ok(cached);
        }
        let raw = self.cypht.send(&session, &payload).await?;
        let result = self.map_write_result(principal, raw)?;
        self.idempotency_store(principal, operation, idempotency_key, &replay, &result)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            operation,
            Some(message_id),
            true,
            None,
        );
        Ok(result)
    }

    pub async fn forward(
        &self,
        principal: &Principal,
        message_id: &str,
        request: ForwardMessageRequest,
        idempotency_key: &str,
    ) -> GatewayResult<MailWriteResult> {
        principal.requires(SCOPE_MAIL_SEND)?;
        principal.requires(SCOPE_MAIL_READ)?;
        let operation = "mail.forward";
        #[derive(Serialize)]
        struct Replay<'a> {
            message_id: &'a str,
            request: &'a ForwardMessageRequest,
        }
        let replay = Replay {
            message_id,
            request: &request,
        };
        if let Some(cached) =
            self.idempotency_lookup(principal, operation, idempotency_key, &replay)?
        {
            return Ok(cached);
        }
        let original = self.message(principal, message_id).await?;
        let forwarded = forward_body(&request.body, &original);
        let send = SendMessageRequest {
            profile_id: request.profile_id.clone(),
            to: request.to.clone(),
            cc: request.cc.clone(),
            bcc: request.bcc.clone(),
            subject: forward_subject(&original.subject),
            body: forwarded,
            attachment_ids: request.attachment_ids.clone(),
            schedule_at: request.schedule_at.clone(),
            delivery_receipt: false,
        };
        self.validate_send_request(&send, true)?;
        let session = self.ensure_cypht_session(principal).await?;
        let payload = self.send_payload(principal, &send, None)?;
        if let Some(cached) =
            self.idempotency_claim(principal, operation, idempotency_key, &replay)?
        {
            return Ok(cached);
        }
        let raw = self.cypht.send(&session, &payload).await?;
        let result = self.map_write_result(principal, raw)?;
        self.idempotency_store(principal, operation, idempotency_key, &replay, &result)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            operation,
            Some(message_id),
            true,
            None,
        );
        Ok(result)
    }

    pub async fn update_message(
        &self,
        principal: &Principal,
        message_id: &str,
        request: MessageUpdateRequest,
    ) -> GatewayResult<ActionResult> {
        principal.requires(SCOPE_MAIL_MODIFY)?;
        if request.seen.is_none() && request.flagged.is_none() {
            return Err(GatewayError::InvalidRequest(
                "at least one of seen or flagged is required".into(),
            ));
        }
        let (account, folder, uid) = self.decode_message_for_principal(principal, message_id)?;
        let session = self.ensure_cypht_session(principal).await?;
        let mut payload =
            serde_json::json!({"account_id": &account, "folder": &folder, "uid": &uid});
        if let Some(value) = request.seen {
            payload["seen"] = serde_json::json!(value);
        }
        if let Some(value) = request.flagged {
            payload["flagged"] = serde_json::json!(value);
        }
        let raw = self.cypht.update_message(&session, &payload).await?;
        let result = self.map_action_result(principal, &account, raw)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "mail.modify",
            Some(message_id),
            true,
            None,
        );
        Ok(result)
    }

    pub async fn move_message(
        &self,
        principal: &Principal,
        message_id: &str,
        request: MoveMessageRequest,
    ) -> GatewayResult<ActionResult> {
        principal.requires(SCOPE_MAIL_MODIFY)?;
        let (account, folder, uid) = self.decode_message_for_principal(principal, message_id)?;
        let destination =
            self.decode_mailbox_for_account(principal, &request.mailbox_id, &account)?;
        if destination == folder {
            return Err(GatewayError::InvalidRequest(
                "message is already in the destination mailbox".into(),
            ));
        }
        let session = self.ensure_cypht_session(principal).await?;
        let raw = self.cypht.move_message(&session, &principal.credential, &serde_json::json!({"account_id": &account, "folder": &folder, "uid": &uid, "destination": &destination})).await?;
        let result = self.map_action_result(principal, &account, raw)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "mail.move",
            Some(message_id),
            true,
            None,
        );
        Ok(result)
    }

    pub async fn archive_message(
        &self,
        principal: &Principal,
        message_id: &str,
    ) -> GatewayResult<ActionResult> {
        principal.requires(SCOPE_MAIL_MODIFY)?;
        let (account, folder, uid) = self.decode_message_for_principal(principal, message_id)?;
        let session = self.ensure_cypht_session(principal).await?;
        let raw = self
            .cypht
            .archive_message(
                &session,
                &principal.credential,
                &serde_json::json!({"account_id": &account, "folder": &folder, "uid": &uid}),
            )
            .await?;
        let result = self.map_action_result(principal, &account, raw)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "mail.archive",
            Some(message_id),
            true,
            None,
        );
        Ok(result)
    }

    pub async fn delete_message(
        &self,
        principal: &Principal,
        message_id: &str,
    ) -> GatewayResult<ActionResult> {
        principal.requires(SCOPE_MAIL_DELETE)?;
        let (account, folder, uid) = self.decode_message_for_principal(principal, message_id)?;
        let session = self.ensure_cypht_session(principal).await?;
        let raw = self
            .cypht
            .delete_message(
                &session,
                &principal.credential,
                &serde_json::json!({"account_id": &account, "folder": &folder, "uid": &uid}),
            )
            .await?;
        let result = self.map_action_result(principal, &account, raw)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "mail.delete",
            Some(message_id),
            true,
            None,
        );
        Ok(result)
    }

    async fn resolve_profile(
        &self,
        principal: &Principal,
        profile_id: Option<&str>,
    ) -> GatewayResult<Profile> {
        let profiles = self.profiles(principal).await?;
        if let Some(profile_id) = profile_id {
            return profiles
                .into_iter()
                .find(|p| p.id == profile_id)
                .ok_or_else(|| GatewayError::NotFound("sending profile".into()));
        }
        if let Some(profile) = profiles.iter().find(|p| p.is_default) {
            return Ok(profile.clone());
        }
        profiles
            .into_iter()
            .next()
            .ok_or_else(|| GatewayError::NotFound("sending profile".into()))
    }

    fn validate_send_request(
        &self,
        request: &SendMessageRequest,
        recipients_required: bool,
    ) -> GatewayResult<()> {
        if recipients_required
            && request.to.is_empty()
            && request.cc.is_empty()
            && request.bcc.is_empty()
        {
            return Err(GatewayError::InvalidRequest(
                "at least one recipient is required".into(),
            ));
        }
        for address in request
            .to
            .iter()
            .chain(request.cc.iter())
            .chain(request.bcc.iter())
        {
            validate_address(address)?;
        }
        if request.subject.chars().count() > 998 {
            return Err(GatewayError::InvalidRequest(
                "subject cannot exceed 998 characters".into(),
            ));
        }
        if request.body.text.as_deref().map(str::len).unwrap_or(0) > 2 * 1024 * 1024
            || request.body.html.as_deref().map(str::len).unwrap_or(0) > 2 * 1024 * 1024
        {
            return Err(GatewayError::InvalidRequest(
                "message body cannot exceed 2 MiB".into(),
            ));
        }
        if request.attachment_ids.len() > 50 {
            return Err(GatewayError::InvalidRequest(
                "a message cannot contain more than 50 uploaded attachments".into(),
            ));
        }
        if request.schedule_at.as_deref().map(str::len).unwrap_or(0) > 100 {
            return Err(GatewayError::InvalidRequest(
                "schedule_at is too long".into(),
            ));
        }
        Ok(())
    }

    fn send_payload(
        &self,
        principal: &Principal,
        request: &SendMessageRequest,
        in_reply_to: Option<&str>,
    ) -> GatewayResult<Value> {
        let profile_id = request
            .profile_id
            .as_deref()
            .map(|id| self.decode_profile(id))
            .transpose()?;
        let attachment_ids = request
            .attachment_ids
            .iter()
            .map(|id| self.decode_upload(principal, id))
            .collect::<GatewayResult<Vec<_>>>()?;
        Ok(serde_json::json!({
            "profile_id": profile_id,
            "to": &request.to,
            "cc": &request.cc,
            "bcc": &request.bcc,
            "subject": &request.subject,
            "body": &request.body,
            "attachment_ids": attachment_ids,
            "schedule_at": &request.schedule_at,
            "delivery_receipt": request.delivery_receipt,
            "in_reply_to": in_reply_to
        }))
    }

    fn decode_profile(&self, public: &str) -> GatewayResult<String> {
        let parts = self.ids.decode(ObjectKind::Profile, public)?;
        if parts.len() != 1 || parts[0].is_empty() {
            return Err(GatewayError::InvalidRequest(
                "profile id payload is invalid".into(),
            ));
        }
        Ok(parts[0].clone())
    }

    fn decode_upload(&self, principal: &Principal, public: &str) -> GatewayResult<String> {
        let parts = self.ids.decode(ObjectKind::Upload, public)?;
        if parts.len() != 2 || parts[0] != principal.username || parts[1].is_empty() {
            return Err(GatewayError::InvalidRequest(
                "upload does not belong to the current user".into(),
            ));
        }
        Ok(parts[1].clone())
    }

    fn decode_message_for_principal(
        &self,
        principal: &Principal,
        message_id: &str,
    ) -> GatewayResult<(String, String, String)> {
        let decoded = self.decode_cypht_object_id(principal, ObjectKind::Message, message_id)?;
        let (account, folder, uid) = match decoded.version {
            1 if decoded.parts.len() == 3 => (
                decoded.parts[0].clone(),
                decoded.parts[1].clone(),
                decoded.parts[2].clone(),
            ),
            2 if decoded.parts.len() == 4 && decoded.parts[0] == "cypht" => (
                decoded.parts[1].clone(),
                decoded.parts[2].clone(),
                decoded.parts[3].clone(),
            ),
            _ => return Err(GatewayError::NotFound("message".into())),
        };
        let public = self.account_public_id(principal, &account)?;
        self.assert_account_allowed(principal, &public)?;
        Ok((account, folder, uid))
    }

    fn decode_attachment_for_principal(
        &self,
        principal: &Principal,
        attachment_id: &str,
    ) -> GatewayResult<(String, String, String, String)> {
        let decoded =
            self.decode_cypht_object_id(principal, ObjectKind::Attachment, attachment_id)?;
        let (account, folder, uid, part) = match decoded.version {
            1 if decoded.parts.len() == 4 => (
                decoded.parts[0].clone(),
                decoded.parts[1].clone(),
                decoded.parts[2].clone(),
                decoded.parts[3].clone(),
            ),
            2 if decoded.parts.len() == 5 && decoded.parts[0] == "cypht" => (
                decoded.parts[1].clone(),
                decoded.parts[2].clone(),
                decoded.parts[3].clone(),
                decoded.parts[4].clone(),
            ),
            _ => return Err(GatewayError::NotFound("attachment".into())),
        };
        let public = self.account_public_id(principal, &account)?;
        self.assert_account_allowed(principal, &public)?;
        Ok((account, folder, uid, part))
    }

    fn map_write_result(
        &self,
        principal: &Principal,
        raw: BridgeWriteResult,
    ) -> GatewayResult<MailWriteResult> {
        let message_id = match (
            raw.account_id.as_deref(),
            raw.folder.as_deref(),
            raw.uid.as_deref(),
        ) {
            (Some(account), Some(folder), Some(uid)) => Some(self.encode_cypht_object_id(
                principal,
                ObjectKind::Message,
                [account, folder, uid],
            )?),
            _ => None,
        };
        Ok(MailWriteResult {
            status: raw.status,
            message_id_header: raw.message_id_header,
            message_id,
            scheduled: raw.scheduled,
        })
    }

    fn map_action_result(
        &self,
        principal: &Principal,
        account: &str,
        raw: BridgeActionResult,
    ) -> GatewayResult<ActionResult> {
        let mailbox_id = raw
            .folder
            .as_deref()
            .map(|folder| {
                self.encode_cypht_object_id(principal, ObjectKind::Mailbox, [account, folder])
            })
            .transpose()?;
        Ok(ActionResult {
            status: raw.status,
            mailbox_id,
            tag_sync: raw.tag_sync.filter(|status| {
                matches!(
                    status.as_str(),
                    "synced" | "pending" | "none" | "disabled" | "failed"
                )
            }),
        })
    }

    fn idempotency_lookup<T: Serialize>(
        &self,
        principal: &Principal,
        operation: &str,
        key: &str,
        request: &T,
    ) -> GatewayResult<Option<MailWriteResult>> {
        validate_idempotency_key(key)?;
        let now = gateway_auth::now_unix();
        let hash = request_hash(request)?;
        if let Some(stored) =
            self.store
                .get_idempotency(&principal.username, operation, key, now)?
        {
            if stored.request_hash != hash {
                return Err(GatewayError::InvalidRequest(
                    "idempotency key was already used for a different request".into(),
                ));
            }
            if stored.response_json.is_empty() {
                return Err(GatewayError::InvalidRequest("this idempotency key is already in progress or has an unknown external-send result".into()));
            }
            let result = serde_json::from_str(&stored.response_json)
                .map_err(|e| GatewayError::Storage(format!("decode idempotency response: {e}")))?;
            return Ok(Some(result));
        }
        Ok(None)
    }

    fn idempotency_claim<T: Serialize>(
        &self,
        principal: &Principal,
        operation: &str,
        key: &str,
        request: &T,
    ) -> GatewayResult<Option<MailWriteResult>> {
        validate_idempotency_key(key)?;
        let now = gateway_auth::now_unix();
        let hash = request_hash(request)?;
        let value = StoredIdempotency {
            request_hash: hash,
            response_json: String::new(),
            created_at: now,
            expires_at: now + 86_400,
        };
        if self
            .store
            .claim_idempotency(&principal.username, operation, key, &value)?
        {
            return Ok(None);
        }
        self.idempotency_lookup(principal, operation, key, request)
    }

    fn idempotency_store<T: Serialize>(
        &self,
        principal: &Principal,
        operation: &str,
        key: &str,
        request: &T,
        result: &MailWriteResult,
    ) -> GatewayResult<()> {
        let hash = request_hash(request)?;
        let response = serde_json::to_string(result)
            .map_err(|e| GatewayError::Storage(format!("encode idempotency response: {e}")))?;
        if !self.store.complete_idempotency(
            &principal.username,
            operation,
            key,
            &hash,
            &response,
        )? {
            return Err(GatewayError::Storage(
                "idempotency reservation disappeared before completion".into(),
            ));
        }
        Ok(())
    }

    async fn ensure_cypht_session(
        &self,
        principal: &Principal,
    ) -> GatewayResult<gateway_auth::CyphtSession> {
        match self.cypht.ping(&principal.cypht_session).await {
            Ok(()) => return Ok(principal.cypht_session.clone()),
            Err(error @ GatewayError::CapabilityUnavailable(_)) => return Err(error),
            Err(_) => {}
        }
        let fresh = self.cypht.login(&principal.credential).await?;
        self.cypht.ping(&fresh).await?;
        if let (AuthKind::Pat, Some(id)) = (&principal.auth_kind, principal.auth_id.as_deref()) {
            self.auth.refresh_pat_session(id, &fresh)?;
        }
        Ok(fresh)
    }

    fn map_page(
        &self,
        principal: &Principal,
        page: BridgeMessagePage,
        base_offset: u64,
    ) -> GatewayResult<MessagePage> {
        let count = page.messages.len() as u64;
        let messages = page
            .messages
            .into_iter()
            .map(|row| self.map_summary(principal, row))
            .collect::<GatewayResult<Vec<_>>>()?;
        let consumed = base_offset.saturating_add(count);
        let next_offset = page
            .total
            .and_then(|total| (consumed < total).then_some(consumed));
        Ok(MessagePage {
            messages,
            total: page.total,
            next_offset,
        })
    }

    fn map_summary(
        &self,
        principal: &Principal,
        row: BridgeMessageSummary,
    ) -> GatewayResult<MessageSummary> {
        let account_id = self.account_public_id(principal, &row.account_id)?;
        let mailbox_id = self.encode_cypht_object_id(
            principal,
            ObjectKind::Mailbox,
            [row.account_id.as_str(), row.folder.as_str()],
        )?;
        let id = self.encode_cypht_object_id(
            principal,
            ObjectKind::Message,
            [
                row.account_id.as_str(),
                row.folder.as_str(),
                row.uid.as_str(),
            ],
        )?;
        let flags = row.flags.to_ascii_lowercase();
        Ok(MessageSummary {
            id,
            account_id,
            mailbox_id,
            subject: row.subject,
            from: parse_addresses(&row.from),
            to: parse_addresses(&row.to),
            date: row.date,
            timestamp: row.timestamp,
            unread: !flags.contains("\\seen"),
            flagged: flags.contains("\\flagged"),
            has_attachments: row
                .content_type
                .as_deref()
                .map(|v| v.to_ascii_lowercase().contains("multipart/mixed"))
                .unwrap_or(false),
            preview: row.preview,
        })
    }

    fn map_message(&self, principal: &Principal, row: BridgeMessage) -> GatewayResult<Message> {
        let id = self.encode_cypht_object_id(
            principal,
            ObjectKind::Message,
            [
                row.account_id.as_str(),
                row.folder.as_str(),
                row.uid.as_str(),
            ],
        )?;
        let account_id = self.account_public_id(principal, &row.account_id)?;
        let mailbox_id = self.encode_cypht_object_id(
            principal,
            ObjectKind::Mailbox,
            [row.account_id.as_str(), row.folder.as_str()],
        )?;
        let subject = header_string(&row.headers, "subject").unwrap_or_default();
        let from = header_value(&row.headers, "from")
            .map(parse_addresses)
            .unwrap_or_default();
        let to = header_value(&row.headers, "to")
            .map(parse_addresses)
            .unwrap_or_default();
        let cc = header_value(&row.headers, "cc")
            .map(parse_addresses)
            .unwrap_or_default();
        let date = header_string(&row.headers, "date");
        let message_id_header = header_string(&row.headers, "message-id");
        let in_reply_to = header_string(&row.headers, "in-reply-to");
        let attachments = row
            .attachments
            .into_iter()
            .map(|a| {
                Ok(Attachment {
                    id: self.encode_cypht_object_id(
                        principal,
                        ObjectKind::Attachment,
                        [
                            row.account_id.as_str(),
                            row.folder.as_str(),
                            row.uid.as_str(),
                            a.part.as_str(),
                        ],
                    )?,
                    filename: a.filename,
                    content_type: a.content_type,
                    size: a.size,
                    inline: a.inline,
                })
            })
            .collect::<GatewayResult<Vec<_>>>()?;
        Ok(Message {
            id,
            account_id,
            mailbox_id,
            subject,
            from,
            to,
            cc,
            date,
            message_id_header,
            in_reply_to,
            body: MessageBody {
                text: row.body_text,
                html: row.body_html,
            },
            attachments,
            headers: row.headers,
        })
    }

    fn map_account(&self, account: BridgeAccount, public_id: String) -> Account {
        Account {
            id: public_id,
            name: account.name,
            email: account.email,
            protocol: account.protocol,
            server: account.server,
            can_send: account.can_send,
        }
    }

    fn account_public_id(&self, principal: &Principal, internal: &str) -> GatewayResult<String> {
        self.encode_cypht_object_id(principal, ObjectKind::Account, [internal])
    }

    fn encode_cypht_object_id<I, S>(
        &self,
        principal: &Principal,
        kind: ObjectKind,
        internal_parts: I,
    ) -> GatewayResult<String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let source = "cypht";
        let internal_parts: Vec<String> = internal_parts.into_iter().map(Into::into).collect();
        let mut stored_parts = vec![source.to_string()];
        stored_parts.extend(internal_parts.iter().cloned());
        let public_id = self.ids.encode_private(
            kind,
            principal.username.as_str(),
            source,
            internal_parts.clone(),
        )?;
        if let Some(existing) =
            self.store
                .get_object_ref(&principal.username, kind.as_str(), &public_id)?
        {
            let existing_parts = self.auth.open_object_id_parts(&existing.parts_ciphertext)?;
            if existing_parts != stored_parts {
                return Err(GatewayError::Crypto);
            }
            return Ok(public_id);
        }
        let parts_ciphertext = self.auth.seal_object_id_parts(&stored_parts)?;
        self.store.put_object_ref(&StoredObjectRef {
            public_id: public_id.clone(),
            owner: principal.username.clone(),
            kind: kind.as_str().into(),
            parts_ciphertext,
            created_at: gateway_auth::now_unix(),
        })?;
        Ok(public_id)
    }

    fn decode_cypht_object_id(
        &self,
        principal: &Principal,
        kind: ObjectKind,
        public_id: &str,
    ) -> GatewayResult<gateway_core::DecodedObjectId> {
        let decoded = self.ids.decode_versioned(kind, public_id)?;
        if decoded.version == 1 {
            return Ok(decoded);
        }
        if decoded.version != 2 || decoded.parts.len() != 1 {
            return Err(GatewayError::NotFound(kind.as_str().into()));
        }
        let reference = self
            .store
            .get_object_ref(&principal.username, kind.as_str(), public_id)?
            .ok_or_else(|| GatewayError::NotFound(kind.as_str().into()))?;
        let parts = self
            .auth
            .open_object_id_parts(&reference.parts_ciphertext)?;
        let Some((source, internal_parts)) = parts.split_first() else {
            return Err(GatewayError::NotFound(kind.as_str().into()));
        };
        if self.ids.encode_private(
            kind,
            principal.username.as_str(),
            source,
            internal_parts.to_vec(),
        )? != public_id
        {
            return Err(GatewayError::NotFound(kind.as_str().into()));
        }
        Ok(gateway_core::DecodedObjectId { version: 2, parts })
    }

    fn decode_account(&self, principal: &Principal, public: &str) -> GatewayResult<String> {
        let decoded = self.decode_cypht_object_id(principal, ObjectKind::Account, public)?;
        match decoded.version {
            1 if decoded.parts.len() == 1 => Ok(decoded.parts[0].clone()),
            2 if decoded.parts.len() == 2 && decoded.parts[0] == "cypht" => {
                Ok(decoded.parts[1].clone())
            }
            _ => Err(GatewayError::NotFound("account".into())),
        }
    }

    fn decode_mailbox_for_account(
        &self,
        principal: &Principal,
        mailbox_id: &str,
        account_internal: &str,
    ) -> GatewayResult<String> {
        let decoded = self.decode_cypht_object_id(principal, ObjectKind::Mailbox, mailbox_id)?;
        let (account, folder) = match decoded.version {
            1 if decoded.parts.len() == 2 => (&decoded.parts[0], &decoded.parts[1]),
            2 if decoded.parts.len() == 3 && decoded.parts[0] == "cypht" => {
                (&decoded.parts[1], &decoded.parts[2])
            }
            _ => return Err(GatewayError::NotFound("mailbox".into())),
        };
        if account != account_internal {
            return Err(GatewayError::NotFound("mailbox".into()));
        }
        Ok(folder.clone())
    }

    fn account_allowed(&self, principal: &Principal, public_id: &str) -> GatewayResult<bool> {
        if principal.account_allowed(public_id) {
            return Ok(true);
        }
        let internal = self.decode_account(principal, public_id)?;
        for allowed_id in &principal.account_allowlist {
            let Ok(decoded) =
                self.decode_cypht_object_id(principal, ObjectKind::Account, allowed_id)
            else {
                continue;
            };
            let allowed_internal = match decoded.version {
                1 if decoded.parts.len() == 1 => Some(decoded.parts[0].as_str()),
                2 if decoded.parts.len() == 2 && decoded.parts[0] == "cypht" => {
                    Some(decoded.parts[1].as_str())
                }
                _ => None,
            };
            if allowed_internal == Some(internal.as_str()) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn assert_account_allowed(&self, principal: &Principal, public_id: &str) -> GatewayResult<()> {
        if self.account_allowed(principal, public_id)? {
            Ok(())
        } else {
            Err(GatewayError::PermissionDenied(
                "account is not allowed by this token".into(),
            ))
        }
    }

    fn audit(
        &self,
        username: &str,
        auth_id: Option<&str>,
        operation: &str,
        resource: Option<&str>,
        success: bool,
        detail: Option<&str>,
    ) {
        let _ = self.store.audit(
            username,
            auth_id,
            operation,
            resource,
            success,
            detail,
            gateway_auth::now_unix(),
        );
    }
}

fn parse_addresses(value: &Value) -> Vec<Address> {
    match value {
        Value::Array(values) => values.iter().filter_map(parse_address).collect(),
        Value::Object(_) | Value::String(_) => parse_address(value).into_iter().collect(),
        _ => Vec::new(),
    }
}

fn parse_address(value: &Value) -> Option<Address> {
    match value {
        Value::String(raw) => {
            let raw = raw.trim();
            if let Some((name, rest)) = raw.rsplit_once('<') {
                let email = rest.trim_end_matches('>').trim();
                if !email.is_empty() {
                    return Some(Address {
                        name: nonempty(name.trim().trim_matches('"')),
                        email: email.into(),
                    });
                }
            }
            (!raw.is_empty()).then(|| Address {
                name: None,
                email: raw.into(),
            })
        }
        Value::Object(map) => {
            let email = map
                .get("email")
                .and_then(Value::as_str)
                .or_else(|| map.get("address").and_then(Value::as_str))?;
            Some(Address {
                name: map.get("name").and_then(Value::as_str).and_then(nonempty),
                email: email.to_string(),
            })
        }
        _ => None,
    }
}

fn nonempty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

fn header_value<'a>(headers: &'a Value, key: &str) -> Option<&'a Value> {
    headers
        .as_object()?
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v)
}

fn header_string(headers: &Value, key: &str) -> Option<String> {
    let value = header_value(headers, key)?;
    match value {
        Value::String(v) => Some(v.clone()),
        Value::Array(values) => values.first().and_then(Value::as_str).map(str::to_string),
        _ => Some(value.to_string()),
    }
}

fn validate_contact_name(value: &str) -> GatewayResult<()> {
    if value.trim().is_empty() || value.len() > 500 || value.contains(['\r', '\n']) {
        return Err(GatewayError::InvalidRequest(
            "contact name must contain 1-500 safe characters".into(),
        ));
    }
    Ok(())
}

fn validate_contact_email(value: &str) -> GatewayResult<()> {
    if value.len() > 320 || !value.contains('@') || value.contains(['\r', '\n']) {
        return Err(GatewayError::InvalidRequest(
            "contact email is invalid".into(),
        ));
    }
    Ok(())
}

fn validate_contact_optional(value: Option<&str>) -> GatewayResult<()> {
    if value.is_some_and(|value| value.len() > 500 || value.contains(['\r', '\n'])) {
        return Err(GatewayError::InvalidRequest(
            "contact field is too long or contains a newline".into(),
        ));
    }
    Ok(())
}
fn validate_address(address: &Address) -> GatewayResult<()> {
    let email = address.email.trim();
    if email.is_empty()
        || email.len() > 320
        || !email.contains('@')
        || email.contains('\r')
        || email.contains('\n')
    {
        return Err(GatewayError::InvalidRequest(format!(
            "invalid recipient address: {}",
            address.email
        )));
    }
    if address
        .name
        .as_deref()
        .map(|v| v.contains('\r') || v.contains('\n'))
        .unwrap_or(false)
    {
        return Err(GatewayError::InvalidRequest(
            "recipient display name contains invalid characters".into(),
        ));
    }
    Ok(())
}

fn validate_idempotency_key(key: &str) -> GatewayResult<()> {
    if key.is_empty() || key.len() > 200 || !key.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(GatewayError::InvalidRequest(
            "Idempotency-Key must contain 1-200 visible ASCII characters".into(),
        ));
    }
    Ok(())
}

fn request_hash<T: Serialize>(value: &T) -> GatewayResult<String> {
    let encoded = serde_json::to_vec(value)
        .map_err(|e| GatewayError::Internal(format!("encode request hash: {e}")))?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}

fn reply_target(message: &Message) -> Vec<Address> {
    if let Some(value) = header_value(&message.headers, "reply-to") {
        let parsed = parse_addresses(value);
        if !parsed.is_empty() {
            return parsed;
        }
    }
    message.from.clone()
}

fn merge_addresses(target: &mut Vec<Address>, source: &[Address]) {
    for address in source {
        if !target
            .iter()
            .any(|existing| existing.email.eq_ignore_ascii_case(&address.email))
        {
            target.push(address.clone());
        }
    }
}

fn remove_address(addresses: &mut Vec<Address>, email: &str) {
    addresses.retain(|a| !a.email.eq_ignore_ascii_case(email));
}

fn reply_subject(subject: &str) -> String {
    if subject.trim_start().to_ascii_lowercase().starts_with("re:") {
        subject.to_string()
    } else {
        format!("Re: {subject}")
    }
}

fn forward_subject(subject: &str) -> String {
    let lower = subject.trim_start().to_ascii_lowercase();
    if lower.starts_with("fwd:") || lower.starts_with("fw:") {
        subject.to_string()
    } else {
        format!("Fwd: {subject}")
    }
}

fn forward_body(prefix: &MessageBody, original: &Message) -> MessageBody {
    let original_text = original.body.text.as_deref().unwrap_or("");
    let from = original
        .from
        .first()
        .map(|a| a.email.as_str())
        .unwrap_or("");
    let header = format!(
        "\n\n---------- Forwarded message ----------\nFrom: {from}\nDate: {}\nSubject: {}\n\n",
        original.date.as_deref().unwrap_or(""),
        original.subject
    );
    let mut text = prefix.text.clone().unwrap_or_default();
    text.push_str(&header);
    text.push_str(original_text);
    MessageBody {
        text: Some(text),
        html: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_and_plain_addresses() {
        let named = Value::String("Alice Example <alice@example.com>".into());
        let addresses = parse_addresses(&named);
        assert_eq!(addresses.len(), 1);
        assert_eq!(addresses[0].name.as_deref(), Some("Alice Example"));
        assert_eq!(addresses[0].email, "alice@example.com");

        let plain = Value::String("bob@example.com".into());
        let addresses = parse_addresses(&plain);
        assert_eq!(addresses[0].name, None);
        assert_eq!(addresses[0].email, "bob@example.com");
    }

    #[test]
    fn finds_headers_case_insensitively() {
        let headers = serde_json::json!({"Subject":"Hello", "FROM":"alice@example.com"});
        assert_eq!(header_string(&headers, "subject").as_deref(), Some("Hello"));
        assert!(header_value(&headers, "from").is_some());
    }

    #[test]
    fn subject_prefixes_are_stable() {
        assert_eq!(reply_subject("Hello"), "Re: Hello");
        assert_eq!(reply_subject("Re: Hello"), "Re: Hello");
        assert_eq!(forward_subject("Hello"), "Fwd: Hello");
        assert_eq!(forward_subject("FW: Hello"), "FW: Hello");
    }

    fn service_with_store(store: Store) -> GatewayService {
        let vault =
            gateway_auth::Vault::from_base64("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
                .unwrap();
        let auth = AuthService::new(store.clone(), vault, 3600);
        let cypht = CyphtClient::new(gateway_cypht::CyphtConfig {
            base_url: "http://127.0.0.1/".parse().unwrap(),
            api_login_key: "unused".into(),
            bridge_key: "unused".into(),
        })
        .unwrap();
        GatewayService::new(auth, cypht, ObjectIdCodec::new([7_u8; 32]).unwrap(), store)
    }

    fn contact_service() -> GatewayService {
        service_with_store(Store::open_memory().unwrap())
    }

    fn contact_principal(username: &str) -> Principal {
        Principal {
            username: username.into(),
            auth_kind: AuthKind::Pat,
            auth_id: Some("token-1".into()),
            scopes: vec![SCOPE_CONTACTS_READ.into()],
            account_allowlist: Vec::new(),
            credential: Credential {
                username: username.into(),
                password: "unused".into(),
            },
            cypht_session: gateway_auth::CyphtSession {
                hm_id: "unused".into(),
                hm_session: "unused".into(),
            },
        }
    }

    #[test]
    fn contacts_hide_internal_ids_and_respect_owner_and_allowlist() {
        let service = contact_service();
        let row = BridgeContact {
            id: "private-contact-123".into(),
            source: "local".into(),
            name: "Example".into(),
            email: "example@example.com".into(),
            phone: String::new(),
            group: String::new(),
        };
        let alice = contact_principal("alice");
        let id = service.contact_public_id(&alice, &row).unwrap();
        assert_eq!(service.contact_public_id(&alice, &row).unwrap(), id);
        assert_ne!(
            service
                .contact_public_id(&contact_principal("bob"), &row)
                .unwrap(),
            id
        );
        let parts = service.ids.decode(ObjectKind::Contact, &id).unwrap();
        assert_eq!(parts.len(), 1);
        assert!(!id.contains("alice"));
        assert!(!id.contains(&row.id) && !parts.contains(&row.id));

        let mut restricted = alice;
        restricted.account_allowlist = vec!["mail-account-only".into()];
        assert!(matches!(
            service.assert_global_resource_allowed(&restricted),
            Err(GatewayError::PermissionDenied(_))
        ));
    }

    #[test]
    fn new_mail_ids_hide_internal_parts_and_bind_owner_and_kind() {
        let service = contact_service();
        let alice = contact_principal("alice");
        let id = service
            .encode_cypht_object_id(
                &alice,
                ObjectKind::Message,
                ["imap-account-17", "INBOX", "uid-938"],
            )
            .unwrap();

        assert!(id.starts_with("v2."));
        assert!(!id.contains("imap-account-17"));
        assert!(!id.contains("INBOX"));
        assert!(!id.contains("uid-938"));
        let public_payload = service
            .ids
            .decode_versioned(ObjectKind::Message, &id)
            .unwrap();
        assert_eq!(public_payload.version, 2);
        assert_eq!(public_payload.parts.len(), 1);
        assert!(!public_payload.parts.contains(&"alice".to_string()));
        assert!(!id.contains("alice"));

        let resolved = service
            .decode_cypht_object_id(&alice, ObjectKind::Message, &id)
            .unwrap();
        assert_eq!(
            resolved.parts,
            vec![
                "cypht".to_string(),
                "imap-account-17".to_string(),
                "INBOX".to_string(),
                "uid-938".to_string(),
            ]
        );
        let stored = service
            .store
            .get_object_ref("alice", ObjectKind::Message.as_str(), &id)
            .unwrap()
            .unwrap();
        assert!(!stored
            .parts_ciphertext
            .windows(b"imap-account-17".len())
            .any(|window| window == b"imap-account-17"));
        assert_eq!(
            service
                .auth
                .open_object_id_parts(&stored.parts_ciphertext)
                .unwrap(),
            resolved.parts
        );
        assert!(matches!(
            service.decode_cypht_object_id(&contact_principal("bob"), ObjectKind::Message, &id),
            Err(GatewayError::NotFound(_))
        ));
        assert!(service
            .ids
            .decode_versioned(ObjectKind::Attachment, &id)
            .is_err());

        let account_id = service
            .account_public_id(&alice, "imap-account-17")
            .unwrap();
        let mailbox_id = service
            .encode_cypht_object_id(&alice, ObjectKind::Mailbox, ["imap-account-17", "INBOX"])
            .unwrap();
        let attachment_id = service
            .encode_cypht_object_id(
                &alice,
                ObjectKind::Attachment,
                ["imap-account-17", "INBOX", "uid-938", "2.1"],
            )
            .unwrap();
        for (kind, id) in [
            (ObjectKind::Account, account_id),
            (ObjectKind::Mailbox, mailbox_id),
            (ObjectKind::Attachment, attachment_id),
        ] {
            assert!(id.starts_with("v2."));
            assert!(!id.contains("imap-account-17"));
            assert_eq!(
                service
                    .decode_cypht_object_id(&alice, kind, &id)
                    .unwrap()
                    .version,
                2
            );
        }
    }

    #[test]
    fn private_ids_resolve_after_reopening_the_gateway_database() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cypht-gateway-object-refs-{}-{unique}.sqlite",
            std::process::id()
        ));
        let alice = contact_principal("alice");
        let id;
        {
            let service = service_with_store(Store::open(&path).unwrap());
            id = service
                .encode_cypht_object_id(
                    &alice,
                    ObjectKind::Message,
                    ["imap-account-17", "INBOX", "uid-938"],
                )
                .unwrap();
        }
        {
            let reopened = service_with_store(Store::open(&path).unwrap());
            let decoded = reopened
                .decode_cypht_object_id(&alice, ObjectKind::Message, &id)
                .unwrap();
            assert_eq!(decoded.version, 2);
            assert_eq!(decoded.parts.last().map(String::as_str), Some("uid-938"));
        }
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
    }

    #[test]
    fn legacy_message_ids_and_account_allowlists_remain_compatible() {
        let service = contact_service();
        let mut alice = contact_principal("alice");
        let legacy_account = service
            .ids
            .encode(ObjectKind::Account, ["imap-account-17"])
            .unwrap();
        let legacy_account_for_request = legacy_account.clone();
        let legacy_message = service
            .ids
            .encode(ObjectKind::Message, ["imap-account-17", "INBOX", "uid-938"])
            .unwrap();
        let current_account = service
            .account_public_id(&alice, "imap-account-17")
            .unwrap();
        alice.account_allowlist = vec![legacy_account];

        assert!(service.account_allowed(&alice, &current_account).unwrap());
        let mut current_allowlist = contact_principal("alice");
        current_allowlist.account_allowlist = vec![current_account.clone()];
        assert!(service
            .account_allowed(&current_allowlist, &legacy_account_for_request)
            .unwrap());
        assert_eq!(
            service
                .decode_message_for_principal(&alice, &legacy_message)
                .unwrap(),
            ("imap-account-17".into(), "INBOX".into(), "uid-938".into())
        );

        let other_account = service
            .account_public_id(&alice, "imap-account-18")
            .unwrap();
        assert!(!service.account_allowed(&alice, &other_account).unwrap());
    }

    #[test]
    fn new_pat_allowlists_reject_ownerless_legacy_account_ids() {
        let service = contact_service();
        let principal = contact_principal("alice");
        let legacy_account = service
            .ids
            .encode(ObjectKind::Account, ["imap-account-17"])
            .unwrap();

        assert!(matches!(
            service.create_pat(
                &principal,
                "legacy-reader",
                vec![SCOPE_CONTACTS_READ.into()],
                vec![legacy_account],
                None,
            ),
            Err(GatewayError::InvalidRequest(_))
        ));
    }

    #[test]
    fn action_result_reports_only_known_tag_sync_states() {
        let service = contact_service();
        let principal = contact_principal("alice");
        let result = service
            .map_action_result(
                &principal,
                "account",
                BridgeActionResult {
                    status: "moved".into(),
                    folder: Some("Archive".into()),
                    tag_sync: Some("pending".into()),
                },
            )
            .unwrap();
        assert_eq!(result.tag_sync.as_deref(), Some("pending"));
        assert!(result.mailbox_id.is_some());

        let result = service
            .map_action_result(
                &principal,
                "account",
                BridgeActionResult {
                    status: "moved".into(),
                    folder: None,
                    tag_sync: Some("upstream private text".into()),
                },
            )
            .unwrap();
        assert!(result.tag_sync.is_none());
    }
    #[test]
    fn contact_validation_rejects_unsafe_and_empty_values() {
        assert!(validate_contact_name(" ").is_err());
        assert!(validate_contact_name("line\nsecond").is_err());
        assert!(validate_contact_email("not-an-email").is_err());
        assert!(validate_contact_optional(Some("line\rsecond")).is_err());
    }
    #[test]
    fn idempotency_keys_reject_whitespace_and_empty_values() {
        assert!(validate_idempotency_key("").is_err());
        assert!(validate_idempotency_key("bad key").is_err());
        assert!(validate_idempotency_key("agent-123").is_ok());
    }
}
