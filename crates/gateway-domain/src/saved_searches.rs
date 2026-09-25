use crate::saved_search_advanced::from_bridge;
use crate::saved_search_ids::{
    ensure_saved_search_id, retire_missing_saved_search_ids, saved_search_id_name,
};
use crate::GatewayService;
use gateway_auth::{CyphtSession, Principal, SCOPE_SEARCHES_READ};
use gateway_core::{GatewayError, GatewayResult, SavedSearch, SavedSearchType};
use gateway_cypht::BridgeSavedSearch;
use std::collections::HashSet;

impl GatewayService {
    pub async fn saved_searches(&self, principal: &Principal) -> GatewayResult<Vec<SavedSearch>> {
        principal.requires(SCOPE_SEARCHES_READ)?;
        self.assert_global_resource_allowed(principal)?;
        let session = self.ensure_cypht_session(principal).await?;
        let rows = self.current_saved_searches(principal, &session).await?;
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            result.push(self.map_saved_search(principal, &session, row).await?);
        }
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "saved_searches.list",
            None,
            true,
            None,
        );
        Ok(result)
    }

    pub async fn saved_search(
        &self,
        principal: &Principal,
        id: &str,
    ) -> GatewayResult<SavedSearch> {
        principal.requires(SCOPE_SEARCHES_READ)?;
        self.assert_global_resource_allowed(principal)?;
        let session = self.ensure_cypht_session(principal).await?;
        let rows = self.current_saved_searches(principal, &session).await?;
        let name = self.saved_search_name(principal, id)?;
        let row = rows
            .into_iter()
            .find(|row| row.name == name)
            .ok_or_else(|| GatewayError::NotFound("saved search".into()))?;
        let saved_search = self.map_saved_search(principal, &session, row).await?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "saved_searches.read",
            Some(id),
            true,
            None,
        );
        Ok(saved_search)
    }
    pub(super) fn saved_search_name(
        &self,
        principal: &Principal,
        id: &str,
    ) -> GatewayResult<String> {
        saved_search_id_name(&self.ids, &self.auth, &self.store, principal, id)?
            .ok_or_else(|| GatewayError::NotFound("saved search".into()))
    }

    pub(super) async fn current_saved_searches(
        &self,
        principal: &Principal,
        session: &CyphtSession,
    ) -> GatewayResult<Vec<BridgeSavedSearch>> {
        let rows = self.cypht.saved_searches(session).await?;
        let live_names = rows
            .iter()
            .map(|row| row.name.clone())
            .collect::<HashSet<_>>();
        retire_missing_saved_search_ids(
            &self.ids,
            &self.auth,
            &self.store,
            principal,
            &live_names,
        )?;
        Ok(rows)
    }
    pub(super) async fn map_saved_search(
        &self,
        principal: &Principal,
        session: &CyphtSession,
        row: BridgeSavedSearch,
    ) -> GatewayResult<SavedSearch> {
        let id = ensure_saved_search_id(&self.ids, &self.auth, &self.store, principal, &row.name)?;
        self.map_saved_search_with_id(principal, session, row, id)
            .await
    }

    pub(super) async fn map_saved_search_with_id(
        &self,
        principal: &Principal,
        session: &CyphtSession,
        row: BridgeSavedSearch,
        id: String,
    ) -> GatewayResult<SavedSearch> {
        match row.kind.as_str() {
            "simple" => Ok(SavedSearch {
                id,
                name: row.name,
                kind: SavedSearchType::Simple,
                query: row.query,
                since: row.since,
                field: row.field,
                advanced: None,
            }),
            "advanced" => {
                let raw = row.advanced.ok_or_else(|| {
                    GatewayError::CapabilityUnavailable("advanced saved search format".into())
                })?;
                Ok(SavedSearch {
                    id,
                    name: row.name,
                    kind: SavedSearchType::Advanced,
                    query: None,
                    since: None,
                    field: None,
                    advanced: Some(from_bridge(self, principal, session, raw).await?),
                })
            }
            _ => Err(GatewayError::CapabilityUnavailable(
                "saved search format is unsupported".into(),
            )),
        }
    }
}
