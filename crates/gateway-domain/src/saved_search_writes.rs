use crate::saved_search_advanced::to_bridge;
use crate::saved_search_ids::{complete_saved_search_rename, retire_saved_search_id};
use crate::saved_search_validation::validate_advanced;
use crate::GatewayService;
use gateway_auth::{Principal, SCOPE_SEARCHES_WRITE};
use gateway_core::{
    GatewayError, GatewayResult, SavedSearch, SavedSearchCreateRequest, SavedSearchType,
    SavedSearchUpdateRequest,
};
use serde_json::{json, Map, Value};

impl GatewayService {
    pub async fn create_saved_search(
        &self,
        principal: &Principal,
        request: SavedSearchCreateRequest,
    ) -> GatewayResult<SavedSearch> {
        let result = self.create_saved_search_inner(principal, request).await;
        self.audit_write(
            principal,
            "saved_searches.create",
            result.as_ref().ok().map(|row| row.id.as_str()),
            result.is_ok(),
        )?;
        result
    }

    pub async fn update_saved_search(
        &self,
        principal: &Principal,
        id: &str,
        request: SavedSearchUpdateRequest,
    ) -> GatewayResult<SavedSearch> {
        let result = self.update_saved_search_inner(principal, id, request).await;
        self.audit_write(principal, "saved_searches.update", Some(id), result.is_ok())?;
        result
    }

    pub async fn delete_saved_search(
        &self,
        principal: &Principal,
        id: &str,
        confirm: bool,
    ) -> GatewayResult<()> {
        let result = self.delete_saved_search_inner(principal, id, confirm).await;
        self.audit_write(principal, "saved_searches.delete", Some(id), result.is_ok())?;
        result
    }

    async fn create_saved_search_inner(
        &self,
        principal: &Principal,
        request: SavedSearchCreateRequest,
    ) -> GatewayResult<SavedSearch> {
        principal.requires(SCOPE_SEARCHES_WRITE)?;
        self.assert_global_resource_allowed(principal)?;
        validate_saved_search_name(&request.name)?;
        let session = self.ensure_cypht_session(principal).await?;
        let payload = match request.kind {
            SavedSearchType::Simple => {
                if request
                    .query
                    .as_deref()
                    .is_none_or(|query| query.trim().is_empty())
                {
                    return Err(GatewayError::InvalidRequest(
                        "simple saved search query is required".into(),
                    ));
                }
                if request.advanced.is_some() {
                    return Err(GatewayError::InvalidRequest(
                        "simple saved search cannot include advanced data".into(),
                    ));
                }
                validate_simple(
                    request.query.as_deref(),
                    request.since.as_deref(),
                    request.field.as_deref(),
                )?;
                let mut payload = Map::new();
                payload.insert("name".into(), Value::String(request.name));
                payload.insert("type".into(), Value::String("simple".into()));
                insert_optional_string(&mut payload, "query", request.query);
                insert_optional_string(&mut payload, "since", request.since);
                insert_optional_string(&mut payload, "field", request.field);
                Value::Object(payload)
            }
            SavedSearchType::Advanced => {
                if request.query.is_some() || request.since.is_some() || request.field.is_some() {
                    return Err(GatewayError::InvalidRequest(
                        "advanced saved search cannot include simple fields".into(),
                    ));
                }
                let advanced = request.advanced.ok_or_else(|| {
                    GatewayError::InvalidRequest("advanced saved search data is required".into())
                })?;
                validate_advanced(&advanced)?;
                let bridge_data = to_bridge(self, principal, &session, &advanced).await?;
                json!({"name": request.name, "type": "advanced", "advanced": bridge_data})
            }
        };
        let row = self
            .cypht
            .create_saved_search(&session, &principal.credential, &payload)
            .await?;
        self.map_saved_search(principal, &session, row).await
    }

    async fn update_saved_search_inner(
        &self,
        principal: &Principal,
        id: &str,
        request: SavedSearchUpdateRequest,
    ) -> GatewayResult<SavedSearch> {
        principal.requires(SCOPE_SEARCHES_WRITE)?;
        self.assert_global_resource_allowed(principal)?;
        if request.name.is_none()
            && request.query.is_none()
            && request.since.is_none()
            && request.field.is_none()
            && request.advanced.is_none()
        {
            return Err(GatewayError::InvalidRequest(
                "saved search update is empty".into(),
            ));
        }
        if let Some(name) = request.name.as_deref() {
            validate_saved_search_name(name)?;
        }
        let session = self.ensure_cypht_session(principal).await?;
        let rows = self.current_saved_searches(principal, &session).await?;
        let old_name = self.saved_search_name(principal, id)?;
        let current = rows
            .iter()
            .find(|row| row.name == old_name)
            .cloned()
            .ok_or_else(|| GatewayError::NotFound("saved search".into()))?;
        let new_name = request.name.as_deref().unwrap_or(&old_name);
        if new_name != old_name && rows.iter().any(|row| row.name == new_name) {
            return Err(GatewayError::Conflict(
                "saved search name already exists".into(),
            ));
        }
        let mut payload = Map::new();
        payload.insert("search_name".into(), Value::String(old_name.clone()));
        if new_name != old_name {
            payload.insert("name".into(), Value::String(new_name.into()));
        }
        match current.kind.as_str() {
            "simple" => {
                if request.advanced.is_some() {
                    return Err(GatewayError::InvalidRequest(
                        "simple saved search cannot be changed to advanced".into(),
                    ));
                }
                if request.query.is_some() || request.since.is_some() || request.field.is_some() {
                    validate_simple(
                        request.query.as_deref().or(current.query.as_deref()),
                        request.since.as_deref().or(current.since.as_deref()),
                        request.field.as_deref().or(current.field.as_deref()),
                    )?;
                    insert_optional_string(&mut payload, "query", request.query);
                    insert_optional_string(&mut payload, "since", request.since);
                    insert_optional_string(&mut payload, "field", request.field);
                }
            }
            "advanced" => {
                if request.query.is_some() || request.since.is_some() || request.field.is_some() {
                    return Err(GatewayError::InvalidRequest(
                        "advanced saved search cannot include simple fields".into(),
                    ));
                }
                if let Some(advanced) = request.advanced {
                    validate_advanced(&advanced)?;
                    payload.insert(
                        "advanced".into(),
                        to_bridge(self, principal, &session, &advanced).await?,
                    );
                }
            }
            _ => {
                return Err(GatewayError::CapabilityUnavailable(
                    "saved search format".into(),
                ))
            }
        }
        let renamed = new_name != old_name;
        if renamed {
            if !self.cypht.supports_durable_user_config(&session).await? {
                return Err(GatewayError::CapabilityUnavailable(
                    "durable user-config writes".into(),
                ));
            }
            retire_saved_search_id(&self.ids, &self.auth, &self.store, principal, id)?;
        }
        let row = self
            .cypht
            .update_saved_search(&session, &principal.credential, &Value::Object(payload))
            .await?;
        if renamed {
            complete_saved_search_rename(
                &self.ids,
                &self.auth,
                &self.store,
                principal,
                id,
                new_name,
            )?;
        }
        self.map_saved_search_with_id(principal, &session, row, id.to_string())
            .await
    }

    async fn delete_saved_search_inner(
        &self,
        principal: &Principal,
        id: &str,
        confirm: bool,
    ) -> GatewayResult<()> {
        principal.requires(SCOPE_SEARCHES_WRITE)?;
        self.assert_global_resource_allowed(principal)?;
        if !confirm {
            return Err(GatewayError::InvalidRequest(
                "confirm=true is required to delete a saved search".into(),
            ));
        }
        let session = self.ensure_cypht_session(principal).await?;
        let rows = self.current_saved_searches(principal, &session).await?;
        let name = self.saved_search_name(principal, id)?;
        if !rows.iter().any(|row| row.name == name) {
            return Err(GatewayError::NotFound("saved search".into()));
        }
        if !self.cypht.supports_durable_user_config(&session).await? {
            return Err(GatewayError::CapabilityUnavailable(
                "durable user-config writes".into(),
            ));
        }
        retire_saved_search_id(&self.ids, &self.auth, &self.store, principal, id)?;
        self.cypht
            .delete_saved_search(
                &session,
                &principal.credential,
                &json!({"search_name": name, "confirm": true}),
            )
            .await?;
        Ok(())
    }
}
fn validate_saved_search_name(name: &str) -> GatewayResult<()> {
    if name.is_empty()
        || name.trim() != name
        || name.len() > 100
        || name.chars().any(char::is_control)
    {
        Err(GatewayError::InvalidRequest(
            "saved search name must contain 1-100 visible characters".into(),
        ))
    } else {
        Ok(())
    }
}

fn validate_simple(
    query: Option<&str>,
    since: Option<&str>,
    field: Option<&str>,
) -> GatewayResult<()> {
    if query.is_some_and(|value| value.trim().is_empty() || value.len() > 1000)
        || since.is_some_and(|value| {
            !matches!(
                value,
                "today"
                    | "-1 week"
                    | "-2 weeks"
                    | "-4 weeks"
                    | "-6 weeks"
                    | "-6 months"
                    | "-1 year"
                    | "-5 years"
            )
        })
        || field.is_some_and(|value| {
            !matches!(value, "TEXT" | "BODY" | "FROM" | "SUBJECT" | "TO" | "CC")
        })
    {
        return Err(GatewayError::InvalidRequest(
            "simple saved search data is invalid".into(),
        ));
    }
    Ok(())
}

fn insert_optional_string(map: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        map.insert(key.into(), Value::String(value));
    }
}

#[cfg(test)]
#[path = "saved_search_writes_tests.rs"]
mod tests;
