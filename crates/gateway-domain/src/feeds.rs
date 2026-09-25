use crate::GatewayService;
use gateway_auth::{Principal, SCOPE_FEEDS_READ};
use gateway_core::{FeedSubscription, GatewayError, GatewayResult, ObjectKind};
use gateway_cypht::BridgeFeed;

impl GatewayService {
    pub async fn feeds_list(&self, principal: &Principal) -> GatewayResult<Vec<FeedSubscription>> {
        principal.requires(SCOPE_FEEDS_READ)?;
        self.assert_global_resource_allowed(principal)?;
        let session = self.ensure_cypht_session(principal).await?;
        let rows = self.cypht.feeds(&session).await?;
        let feeds = rows
            .into_iter()
            .map(|r| self.map_feed(principal, r))
            .collect::<GatewayResult<Vec<_>>>()?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "feeds.list",
            None,
            true,
            None,
        );
        Ok(feeds)
    }

    pub async fn feed_read(
        &self,
        principal: &Principal,
        id: &str,
    ) -> GatewayResult<FeedSubscription> {
        principal.requires(SCOPE_FEEDS_READ)?;
        self.assert_global_resource_allowed(principal)?;
        let row = self.resolve_feed(principal, id).await?;
        let feed = self.map_feed(principal, row)?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "feeds.read",
            Some(id),
            true,
            None,
        );
        Ok(feed)
    }

    fn feed_public_id(&self, principal: &Principal, row: &BridgeFeed) -> GatewayResult<String> {
        self.ids.encode_private(
            ObjectKind::Feed,
            principal.username.as_str(),
            "cypht",
            [row.id.as_str()],
        )
    }

    fn map_feed(&self, principal: &Principal, row: BridgeFeed) -> GatewayResult<FeedSubscription> {
        Ok(FeedSubscription {
            id: self.feed_public_id(principal, &row)?,
            name: row.name,
            url: row.url,
        })
    }

    async fn resolve_feed(&self, principal: &Principal, id: &str) -> GatewayResult<BridgeFeed> {
        let decoded = self.ids.decode_versioned(ObjectKind::Feed, id)?;
        let valid_id = match decoded.version {
            2 => decoded.parts.len() == 1,
            1 => decoded.parts.len() == 2 && decoded.parts[0] == principal.username,
            _ => false,
        };
        if !valid_id {
            return Err(GatewayError::NotFound("feed".into()));
        }
        let session = self.ensure_cypht_session(principal).await?;
        for row in self.cypht.feeds(&session).await? {
            if self.feed_public_id(principal, &row)? == id {
                return Ok(row);
            }
        }
        Err(GatewayError::NotFound("feed".into()))
    }
}

#[cfg(test)]
#[path = "feeds_tests.rs"]
mod feeds_tests;
