use gateway_auth::{AuthKind, AuthService, Credential, Principal, SCOPE_ACCOUNTS_READ, SCOPE_MAIL_READ, SCOPE_MAIL_SEARCH};
use gateway_core::{
    Account, Address, Attachment, CreatedToken, GatewayError, GatewayResult, LoginResponse,
    Mailbox, Message, MessageBody, MessagePage, MessageSummary, ObjectIdCodec, ObjectKind,
    SearchRequest, TokenMetadata,
};
use gateway_cypht::{BridgeAccount, BridgeMessage, BridgeMessagePage, BridgeMessageSummary, CyphtClient};
use gateway_storage::Store;
use serde_json::Value;

#[derive(Clone)]
pub struct GatewayService {
    auth: AuthService,
    cypht: CyphtClient,
    ids: ObjectIdCodec,
    store: Store,
}

impl GatewayService {
    pub fn new(auth: AuthService, cypht: CyphtClient, ids: ObjectIdCodec, store: Store) -> Self {
        Self { auth, cypht, ids, store }
    }

    pub async fn login(&self, username: String, password: String) -> GatewayResult<LoginResponse> {
        if username.trim().is_empty() || password.is_empty() {
            return Err(GatewayError::Authentication);
        }
        let credential = Credential { username: username.trim().to_string(), password };
        let session = self.cypht.login(&credential).await?;
        self.cypht.ping(&session).await?;
        let (token, ttl) = self.auth.issue_session(&credential, &session)?;
        self.audit(&credential.username, None, "auth.login", None, true, None);
        Ok(LoginResponse { access_token: token, token_type: "Bearer".into(), expires_in: ttl })
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
        self.audit(&principal.username, principal.auth_id.as_deref(), "auth.logout", None, true, None);
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
            let _ = self.decode_account(account_id)?;
        }
        let token = self.auth.create_pat(principal, name, scopes, account_allowlist, expires_in_days)?;
        self.audit(&principal.username, principal.auth_id.as_deref(), "token.create", Some(&token.id), true, None);
        Ok(token)
    }

    pub fn list_pats(&self, principal: &Principal) -> GatewayResult<Vec<TokenMetadata>> {
        self.auth.list_pats(principal)
    }

    pub fn revoke_pat(&self, principal: &Principal, id: &str) -> GatewayResult<()> {
        if !self.auth.revoke_pat(principal, id)? {
            return Err(GatewayError::NotFound("API token".into()));
        }
        self.audit(&principal.username, principal.auth_id.as_deref(), "token.revoke", Some(id), true, None);
        Ok(())
    }

    pub async fn accounts(&self, principal: &Principal) -> GatewayResult<Vec<Account>> {
        principal.requires(SCOPE_ACCOUNTS_READ)?;
        let session = self.ensure_cypht_session(principal).await?;
        let accounts = self.cypht.accounts(&session).await?;
        let mut result = Vec::new();
        for account in accounts {
            let public_id = self.account_public_id(&account.id)?;
            if principal.account_allowed(&public_id) {
                result.push(self.map_account(account, public_id));
            }
        }
        self.audit(&principal.username, principal.auth_id.as_deref(), "accounts.list", None, true, None);
        Ok(result)
    }

    pub async fn mailboxes(&self, principal: &Principal, account_id: &str) -> GatewayResult<Vec<Mailbox>> {
        principal.requires(SCOPE_ACCOUNTS_READ)?;
        self.assert_account_allowed(principal, account_id)?;
        let internal = self.decode_account(account_id)?;
        let session = self.ensure_cypht_session(principal).await?;
        let rows = self.cypht.mailboxes(&session, &internal).await?;
        rows.into_iter().map(|row| {
            let id = self.ids.encode(ObjectKind::Mailbox, [&internal, row.name.as_str()])?;
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
        }).collect()
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
            self.assert_account_allowed(principal, account_id)?;
            let internal = self.decode_account(account_id)?;
            let folder = match mailbox_id {
                Some(mailbox_id) => self.decode_mailbox_for_account(mailbox_id, &internal)?,
                None => "INBOX".into(),
            };
            let page = self.cypht.messages(&session, &internal, &folder, offset, limit).await?;
            let mapped = self.map_page(page, offset as u64)?;
            self.audit(&principal.username, principal.auth_id.as_deref(), "mail.list", Some(account_id), true, None);
            return Ok(mapped);
        }

        if mailbox_id.is_some() {
            return Err(GatewayError::InvalidRequest("mailbox_id requires account_id".into()));
        }

        if offset > 1_000 {
            return Err(GatewayError::InvalidRequest("unified inbox offset cannot exceed 1000".into()));
        }
        let accounts = self.cypht.accounts(&session).await?;
        let mut all = Vec::new();
        let fetch_limit = offset.saturating_add(limit).clamp(1, 1_100);
        for account in accounts {
            let public_id = self.account_public_id(&account.id)?;
            if !principal.account_allowed(&public_id) {
                continue;
            }
            let page = self.cypht.messages(&session, &account.id, "INBOX", 0, fetch_limit).await?;
            all.extend(page.messages);
        }
        all.sort_by(|a, b| b.timestamp.unwrap_or_default().cmp(&a.timestamp.unwrap_or_default()));
        let start = (offset as usize).min(all.len());
        let end = start.saturating_add(limit as usize).min(all.len());
        let has_more = end < all.len();
        let messages = all[start..end].iter().cloned().map(|m| self.map_summary(m)).collect::<GatewayResult<Vec<_>>>()?;
        let next_offset = has_more.then_some(end as u64);
        self.audit(&principal.username, principal.auth_id.as_deref(), "mail.list_unified", None, true, None);
        Ok(MessagePage { messages, total: None, next_offset })
    }

    pub async fn search(&self, principal: &Principal, request: SearchRequest) -> GatewayResult<MessagePage> {
        principal.requires(SCOPE_MAIL_SEARCH)?;
        let query = request.query.trim();
        if query.is_empty() {
            return Err(GatewayError::InvalidRequest("query cannot be empty".into()));
        }
        if query.chars().count() > 1024 {
            return Err(GatewayError::InvalidRequest("query cannot exceed 1024 characters".into()));
        }
        if request.account_ids.len() > 100 {
            return Err(GatewayError::InvalidRequest("account_ids cannot contain more than 100 entries".into()));
        }
        let session = self.ensure_cypht_session(principal).await?;
        let mailbox = request.mailbox_id.as_deref().map(|mailbox_id| {
            let parts = self.ids.decode(ObjectKind::Mailbox, mailbox_id)?;
            if parts.len() != 2 {
                return Err(GatewayError::InvalidRequest("mailbox id payload is incomplete".into()));
            }
            Ok((parts[0].clone(), parts[1].clone()))
        }).transpose()?;

        let accounts = if request.account_ids.is_empty() {
            if let Some((mailbox_account, _)) = &mailbox {
                let public_id = self.account_public_id(mailbox_account)?;
                self.assert_account_allowed(principal, &public_id)?;
                vec![mailbox_account.clone()]
            } else {
                self.cypht.accounts(&session).await?
                    .into_iter()
                    .filter_map(|a| self.account_public_id(&a.id).ok().filter(|id| principal.account_allowed(id)).map(|_| a.id))
                    .collect::<Vec<_>>()
            }
        } else {
            let mut ids = Vec::new();
            for id in &request.account_ids {
                self.assert_account_allowed(principal, id)?;
                ids.push(self.decode_account(id)?);
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
            return Ok(MessagePage { messages: Vec::new(), total: Some(0), next_offset: None });
        }
        let folder = mailbox.map(|(_, folder)| folder).unwrap_or_else(|| "INBOX".into());
        let page = self.cypht.search(
            &session,
            &accounts,
            &folder,
            query,
            request.limit.unwrap_or(50).clamp(1, 100),
        ).await?;
        let result = self.map_page(page, 0)?;
        self.audit(&principal.username, principal.auth_id.as_deref(), "mail.search", None, true, None);
        Ok(result)
    }

    pub async fn message(&self, principal: &Principal, message_id: &str) -> GatewayResult<Message> {
        principal.requires(SCOPE_MAIL_READ)?;
        let parts = self.ids.decode(ObjectKind::Message, message_id)?;
        if parts.len() != 3 {
            return Err(GatewayError::InvalidRequest("message id payload is incomplete".into()));
        }
        let account_public = self.account_public_id(&parts[0])?;
        self.assert_account_allowed(principal, &account_public)?;
        let session = self.ensure_cypht_session(principal).await?;
        let raw = self.cypht.message(&session, &parts[0], &parts[1], &parts[2]).await?;
        let mapped = self.map_message(raw)?;
        self.audit(&principal.username, principal.auth_id.as_deref(), "mail.read", Some(message_id), true, None);
        Ok(mapped)
    }

    async fn ensure_cypht_session(&self, principal: &Principal) -> GatewayResult<gateway_auth::CyphtSession> {
        if self.cypht.ping(&principal.cypht_session).await.is_ok() {
            return Ok(principal.cypht_session.clone());
        }
        let fresh = self.cypht.login(&principal.credential).await?;
        self.cypht.ping(&fresh).await?;
        if let (AuthKind::Pat, Some(id)) = (&principal.auth_kind, principal.auth_id.as_deref()) {
            self.auth.refresh_pat_session(id, &fresh)?;
        }
        Ok(fresh)
    }

    fn map_page(&self, page: BridgeMessagePage, base_offset: u64) -> GatewayResult<MessagePage> {
        let count = page.messages.len() as u64;
        let messages = page.messages.into_iter().map(|row| self.map_summary(row)).collect::<GatewayResult<Vec<_>>>()?;
        let consumed = base_offset.saturating_add(count);
        let next_offset = page.total.and_then(|total| (consumed < total).then_some(consumed));
        Ok(MessagePage { messages, total: page.total, next_offset })
    }

    fn map_summary(&self, row: BridgeMessageSummary) -> GatewayResult<MessageSummary> {
        let account_id = self.account_public_id(&row.account_id)?;
        let mailbox_id = self.ids.encode(ObjectKind::Mailbox, [&row.account_id, row.folder.as_str()])?;
        let id = self.ids.encode(ObjectKind::Message, [&row.account_id, row.folder.as_str(), row.uid.as_str()])?;
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
            has_attachments: row.content_type.as_deref().map(|v| v.to_ascii_lowercase().contains("multipart/mixed")).unwrap_or(false),
            preview: row.preview,
        })
    }

    fn map_message(&self, row: BridgeMessage) -> GatewayResult<Message> {
        let id = self.ids.encode(ObjectKind::Message, [&row.account_id, row.folder.as_str(), row.uid.as_str()])?;
        let account_id = self.account_public_id(&row.account_id)?;
        let mailbox_id = self.ids.encode(ObjectKind::Mailbox, [&row.account_id, row.folder.as_str()])?;
        let subject = header_string(&row.headers, "subject").unwrap_or_default();
        let from = header_value(&row.headers, "from").map(parse_addresses).unwrap_or_default();
        let to = header_value(&row.headers, "to").map(parse_addresses).unwrap_or_default();
        let cc = header_value(&row.headers, "cc").map(parse_addresses).unwrap_or_default();
        let date = header_string(&row.headers, "date");
        let message_id_header = header_string(&row.headers, "message-id");
        let in_reply_to = header_string(&row.headers, "in-reply-to");
        let attachments = row.attachments.into_iter().map(|a| {
            Ok(Attachment {
                id: self.ids.encode(ObjectKind::Attachment, [&row.account_id, row.folder.as_str(), row.uid.as_str(), a.part.as_str()])?,
                filename: a.filename,
                content_type: a.content_type,
                size: a.size,
                inline: a.inline,
            })
        }).collect::<GatewayResult<Vec<_>>>()?;
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
            body: MessageBody { text: row.body_text, html: row.body_html },
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

    fn account_public_id(&self, internal: &str) -> GatewayResult<String> {
        self.ids.encode(ObjectKind::Account, [internal])
    }

    fn decode_account(&self, public: &str) -> GatewayResult<String> {
        self.ids.decode(ObjectKind::Account, public)?.into_iter().next()
            .ok_or_else(|| GatewayError::InvalidRequest("account id payload is empty".into()))
    }

    fn decode_mailbox_for_account(&self, mailbox_id: &str, account_internal: &str) -> GatewayResult<String> {
        let parts = self.ids.decode(ObjectKind::Mailbox, mailbox_id)?;
        if parts.len() != 2 || parts[0] != account_internal {
            return Err(GatewayError::InvalidRequest("mailbox does not belong to account".into()));
        }
        Ok(parts[1].clone())
    }

    fn assert_account_allowed(&self, principal: &Principal, public_id: &str) -> GatewayResult<()> {
        if principal.account_allowed(public_id) { Ok(()) }
        else { Err(GatewayError::PermissionDenied("account is not allowed by this token".into())) }
    }

    fn audit(&self, username: &str, auth_id: Option<&str>, operation: &str, resource: Option<&str>, success: bool, detail: Option<&str>) {
        let _ = self.store.audit(username, auth_id, operation, resource, success, detail, gateway_auth::now_unix());
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
                    return Some(Address { name: nonempty(name.trim().trim_matches('"')), email: email.into() });
                }
            }
            (!raw.is_empty()).then(|| Address { name: None, email: raw.into() })
        }
        Value::Object(map) => {
            let email = map.get("email").and_then(Value::as_str)
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
    headers.as_object()?.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
}

fn header_string(headers: &Value, key: &str) -> Option<String> {
    let value = header_value(headers, key)?;
    match value {
        Value::String(v) => Some(v.clone()),
        Value::Array(values) => values.first().and_then(Value::as_str).map(str::to_string),
        _ => Some(value.to_string()),
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
}
