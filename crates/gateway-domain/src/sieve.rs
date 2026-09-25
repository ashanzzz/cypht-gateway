use crate::GatewayService;
use gateway_auth::{Principal, SCOPE_SIEVE_READ};
use gateway_core::{GatewayResult, SieveStatus};

impl GatewayService {
    pub async fn sieve_status(&self, principal: &Principal) -> GatewayResult<Vec<SieveStatus>> {
        principal.requires(SCOPE_SIEVE_READ)?;
        let session = self.ensure_cypht_session(principal).await?;
        let rows = self.cypht.sieve_status(&session).await?;
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let account_id = self.account_public_id(principal, &row.account_id)?;
            if !self.account_allowed(principal, &account_id)? {
                continue;
            }
            result.push(SieveStatus {
                account_id,
                name: row.name,
                protocol: row.protocol,
                enabled: row.enabled,
                configured: row.configured,
                status: row.status,
                remote_probe: row.remote_probe,
            });
        }
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "sieve.status",
            None,
            true,
            None,
        );
        Ok(result)
    }
}
