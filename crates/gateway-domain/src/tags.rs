use crate::GatewayService;
use gateway_auth::{Principal, SCOPE_MAIL_READ, SCOPE_TAGS_READ, SCOPE_TAGS_WRITE};
use gateway_core::{
    ActionResult, GatewayError, GatewayResult, ObjectKind, Tag, TagCreateRequest, TagUpdateRequest,
};
use gateway_cypht::{BridgeActionResult, BridgeTag};
use serde_json::{json, Value};

impl GatewayService {
    pub async fn tags(&self, principal: &Principal) -> GatewayResult<Vec<Tag>> {
        principal.requires(SCOPE_TAGS_READ)?;
        self.assert_global_resource_allowed(principal)?;
        let session = self.ensure_cypht_session(principal).await?;
        let rows = self.cypht.tags(&session).await?;
        let tags = rows
            .into_iter()
            .map(|row| self.map_tag(principal, row))
            .collect::<GatewayResult<Vec<_>>>()?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "tags.list",
            None,
            true,
            None,
        );
        Ok(tags)
    }

    pub async fn create_tag(
        &self,
        principal: &Principal,
        request: TagCreateRequest,
    ) -> GatewayResult<Tag> {
        let result = self.create_tag_inner(principal, request).await;
        self.audit_write(
            principal,
            "tags.create",
            result.as_ref().ok().map(|tag| tag.id.as_str()),
            result.is_ok(),
        )?;
        result
    }

    pub async fn update_tag(
        &self,
        principal: &Principal,
        id: &str,
        request: TagUpdateRequest,
    ) -> GatewayResult<Tag> {
        let result = self.update_tag_inner(principal, id, request).await;
        self.audit_write(principal, "tags.update", Some(id), result.is_ok())?;
        result
    }

    pub async fn delete_tag(
        &self,
        principal: &Principal,
        id: &str,
        confirm: bool,
    ) -> GatewayResult<()> {
        let result = self.delete_tag_inner(principal, id, confirm).await;
        self.audit_write(principal, "tags.delete", Some(id), result.is_ok())?;
        result
    }

    pub async fn add_message_tag(
        &self,
        principal: &Principal,
        message_id: &str,
        tag_id: &str,
    ) -> GatewayResult<ActionResult> {
        let result = self
            .message_tag_inner(principal, message_id, tag_id, false, true)
            .await;
        self.audit_write(principal, "mail.tag.add", Some(message_id), result.is_ok())?;
        result
    }

    pub async fn remove_message_tag(
        &self,
        principal: &Principal,
        message_id: &str,
        tag_id: &str,
        confirm: bool,
    ) -> GatewayResult<ActionResult> {
        let result = self
            .message_tag_inner(principal, message_id, tag_id, true, confirm)
            .await;
        self.audit_write(
            principal,
            "mail.tag.remove",
            Some(message_id),
            result.is_ok(),
        )?;
        result
    }

    async fn create_tag_inner(
        &self,
        principal: &Principal,
        request: TagCreateRequest,
    ) -> GatewayResult<Tag> {
        principal.requires(SCOPE_TAGS_WRITE)?;
        self.assert_global_resource_allowed(principal)?;
        validate_tag_name(&request.name)?;
        validate_tag_color(request.color.as_deref())?;
        let session = self.ensure_cypht_session(principal).await?;
        let payload = serde_json::to_value(&request)
            .map_err(|e| GatewayError::Internal(format!("encode tag: {e}")))?;
        let row = self
            .cypht
            .create_tag(&session, &principal.credential, &payload)
            .await?;
        self.map_tag(principal, row)
    }

    async fn update_tag_inner(
        &self,
        principal: &Principal,
        id: &str,
        request: TagUpdateRequest,
    ) -> GatewayResult<Tag> {
        principal.requires(SCOPE_TAGS_WRITE)?;
        self.assert_global_resource_allowed(principal)?;
        if request.name.is_none() && request.color.is_none() {
            return Err(GatewayError::InvalidRequest("tag update is empty".into()));
        }
        if let Some(name) = request.name.as_deref() {
            validate_tag_name(name)?;
        }
        validate_tag_color(request.color.as_deref())?;
        let row = self.resolve_tag(principal, id).await?;
        let session = self.ensure_cypht_session(principal).await?;
        let mut payload = serde_json::to_value(&request)
            .map_err(|e| GatewayError::Internal(format!("encode tag update: {e}")))?;
        payload["tag_id"] = Value::String(row.id);
        self.map_tag(
            principal,
            self.cypht
                .update_tag(&session, &principal.credential, &payload)
                .await?,
        )
    }

    async fn delete_tag_inner(
        &self,
        principal: &Principal,
        id: &str,
        confirm: bool,
    ) -> GatewayResult<()> {
        principal.requires(SCOPE_TAGS_WRITE)?;
        self.assert_global_resource_allowed(principal)?;
        if !confirm {
            return Err(GatewayError::InvalidRequest(
                "confirm=true is required".into(),
            ));
        }
        let row = self.resolve_tag(principal, id).await?;
        let session = self.ensure_cypht_session(principal).await?;
        self.cypht
            .delete_tag(
                &session,
                &principal.credential,
                &json!({"tag_id": row.id, "confirm": true}),
            )
            .await?;
        Ok(())
    }

    async fn message_tag_inner(
        &self,
        principal: &Principal,
        message_id: &str,
        tag_id: &str,
        remove: bool,
        confirm: bool,
    ) -> GatewayResult<ActionResult> {
        principal.requires(SCOPE_TAGS_WRITE)?;
        principal.requires(SCOPE_MAIL_READ)?;
        self.assert_global_resource_allowed(principal)?;
        if remove && !confirm {
            return Err(GatewayError::InvalidRequest(
                "confirm=true is required".into(),
            ));
        }
        let (account_id, folder, uid) = self.decode_message_for_principal(principal, message_id)?;
        let tag = self.resolve_tag(principal, tag_id).await?;
        let session = self.ensure_cypht_session(principal).await?;
        let payload = json!({"tag_id": tag.id, "account_id": account_id, "folder": folder, "uid": uid, "confirm": confirm});
        let raw: BridgeActionResult = if remove {
            self.cypht
                .remove_message_tag(&session, &principal.credential, &payload)
                .await?
        } else {
            self.cypht
                .add_message_tag(&session, &principal.credential, &payload)
                .await?
        };
        Ok(ActionResult {
            status: raw.status,
            mailbox_id: None,
            tag_sync: None,
        })
    }

    async fn resolve_tag(&self, principal: &Principal, id: &str) -> GatewayResult<BridgeTag> {
        let decoded = self.ids.decode_versioned(ObjectKind::Tag, id)?;
        let valid_id = match decoded.version {
            2 => decoded.parts.len() == 1,
            1 => decoded.parts.len() == 2 && decoded.parts[0] == principal.username,
            _ => false,
        };
        if !valid_id {
            return Err(GatewayError::NotFound("tag".into()));
        }
        let session = self.ensure_cypht_session(principal).await?;
        for row in self.cypht.tags(&session).await? {
            if self.tag_public_id(principal, &row.id)? == id {
                return Ok(row);
            }
        }
        Err(GatewayError::NotFound("tag".into()))
    }

    fn tag_public_id(&self, principal: &Principal, internal: &str) -> GatewayResult<String> {
        self.ids.encode_private(
            ObjectKind::Tag,
            principal.username.as_str(),
            "cypht-user-tags",
            [internal],
        )
    }

    fn map_tag(&self, principal: &Principal, row: BridgeTag) -> GatewayResult<Tag> {
        Ok(Tag {
            id: self.tag_public_id(principal, &row.id)?,
            name: row.name,
            color: row.color,
            parent_id: row
                .parent
                .as_deref()
                .map(|id| self.tag_public_id(principal, id))
                .transpose()?,
        })
    }
}

fn validate_tag_name(value: &str) -> GatewayResult<()> {
    if value.trim().is_empty() || value.len() > 200 || value.contains(['<', '>', '\r', '\n']) {
        return Err(GatewayError::InvalidRequest(
            "tag name must contain 1-200 safe characters".into(),
        ));
    }
    Ok(())
}

fn validate_tag_color(value: Option<&str>) -> GatewayResult<()> {
    if value.is_some_and(|color| {
        color.len() != 7
            || !color.starts_with('#')
            || !color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        return Err(GatewayError::InvalidRequest(
            "tag color must be a six-digit hex value".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_values_reject_unsafe_input() {
        assert!(validate_tag_name(" ").is_err());
        assert!(validate_tag_name("<script>").is_err());
        assert!(validate_tag_color(Some("not-a-color")).is_err());
        assert!(validate_tag_name("Work").is_ok());
        assert!(validate_tag_color(Some("#1a73e8")).is_ok());
    }
}
