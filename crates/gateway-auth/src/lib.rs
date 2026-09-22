use base64::{engine::general_purpose::STANDARD, Engine as _};
use chacha20poly1305::{aead::{Aead, KeyInit}, XChaCha20Poly1305, XNonce};
use gateway_core::{CreatedToken, GatewayError, GatewayResult};
use gateway_storage::{Store, StoredPat, StoredSession};
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const SCOPE_ACCOUNTS_READ: &str = "accounts.read";
pub const SCOPE_MAIL_READ: &str = "mail.read";
pub const SCOPE_MAIL_SEARCH: &str = "mail.search";
pub const SCOPE_ATTACHMENTS_READ: &str = "attachments.read";
pub const SCOPE_MAIL_SEND: &str = "mail.send";
pub const SCOPE_MAIL_MODIFY: &str = "mail.modify";
pub const SCOPE_MAIL_DELETE: &str = "mail.delete";
pub const SCOPE_TOKENS_MANAGE: &str = "tokens.manage";

pub const DEFAULT_AI_SCOPES: &[&str] = &[
    SCOPE_ACCOUNTS_READ,
    SCOPE_MAIL_READ,
    SCOPE_MAIL_SEARCH,
    SCOPE_ATTACHMENTS_READ,
];

#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Credential {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct CyphtSession {
    pub hm_id: String,
    pub hm_session: String,
}

#[derive(Debug, Clone)]
pub enum AuthKind {
    Session,
    Pat,
}

#[derive(Debug, Clone)]
pub struct Principal {
    pub username: String,
    pub auth_kind: AuthKind,
    pub auth_id: Option<String>,
    pub scopes: Vec<String>,
    pub account_allowlist: Vec<String>,
    pub credential: Credential,
    pub cypht_session: CyphtSession,
}

impl Principal {
    pub fn requires(&self, scope: &str) -> GatewayResult<()> {
        if matches!(&self.auth_kind, AuthKind::Session) || self.scopes.iter().any(|s| s == scope) {
            Ok(())
        } else {
            Err(GatewayError::PermissionDenied(scope.to_string()))
        }
    }

    pub fn account_allowed(&self, account_id: &str) -> bool {
        self.account_allowlist.is_empty() || self.account_allowlist.iter().any(|id| id == account_id)
    }
}

#[derive(Clone)]
pub struct Vault {
    cipher: XChaCha20Poly1305,
    token_hash_key: [u8; 32],
    id_key: [u8; 32],
}

impl Vault {
    pub fn from_base64(value: &str) -> GatewayResult<Self> {
        let bytes = STANDARD.decode(value).map_err(|_| GatewayError::Configuration(
            "GATEWAY_MASTER_KEY must be base64-encoded".into()
        ))?;
        if bytes.len() != 32 {
            return Err(GatewayError::Configuration(
                "GATEWAY_MASTER_KEY must decode to exactly 32 bytes".into(),
            ));
        }
        let mut master_key = [0_u8; 32];
        master_key.copy_from_slice(&bytes);
        let mut encryption_key = derive_key(&master_key, b"cypht-gateway/vault-encryption")?;
        let token_hash_key = derive_key(&master_key, b"cypht-gateway/token-hash")?;
        let id_key = derive_key(&master_key, b"cypht-gateway/object-id")?;
        master_key.zeroize();
        let cipher = XChaCha20Poly1305::new((&encryption_key).into());
        encryption_key.zeroize();
        Ok(Self { cipher, token_hash_key, id_key })
    }

    pub fn seal_json<T: Serialize>(&self, value: &T) -> GatewayResult<Vec<u8>> {
        let plaintext = serde_json::to_vec(value).map_err(|e| GatewayError::Internal(e.to_string()))?;
        let mut nonce = [0_u8; 24];
        OsRng.fill_bytes(&mut nonce);
        let ciphertext = self.cipher.encrypt(XNonce::from_slice(&nonce), plaintext.as_ref())
            .map_err(|_| GatewayError::Crypto)?;
        let mut output = nonce.to_vec();
        output.extend_from_slice(&ciphertext);
        Ok(output)
    }

    pub fn open_json<T: for<'de> Deserialize<'de>>(&self, value: &[u8]) -> GatewayResult<T> {
        if value.len() <= 24 {
            return Err(GatewayError::Crypto);
        }
        let (nonce, ciphertext) = value.split_at(24);
        let plaintext = self.cipher.decrypt(XNonce::from_slice(nonce), ciphertext)
            .map_err(|_| GatewayError::Crypto)?;
        serde_json::from_slice(&plaintext).map_err(|_| GatewayError::Crypto)
    }

    pub fn token_hash(&self, token: &str) -> GatewayResult<String> {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&self.token_hash_key).map_err(|_| GatewayError::Crypto)?;
        mac.update(token.as_bytes());
        Ok(hex::encode(mac.finalize().into_bytes()))
    }

    pub fn id_key(&self) -> [u8; 32] {
        self.id_key
    }
}

#[derive(Clone)]
pub struct AuthService {
    store: Store,
    vault: Vault,
    session_ttl_seconds: u64,
}

impl AuthService {
    pub fn new(store: Store, vault: Vault, session_ttl_seconds: u64) -> Self {
        Self { store, vault, session_ttl_seconds }
    }

    pub fn vault(&self) -> &Vault { &self.vault }

    pub fn issue_session(&self, credential: &Credential, cypht_session: &CyphtSession) -> GatewayResult<(String, u64)> {
        let token = format!("cypht_at_{}", random_hex(32));
        let hash = self.vault.token_hash(&token)?;
        let now = now_unix();
        let stored = StoredSession {
            username: credential.username.clone(),
            credential_ciphertext: self.vault.seal_json(credential)?,
            cypht_session_ciphertext: self.vault.seal_json(cypht_session)?,
            created_at: now,
            expires_at: now + self.session_ttl_seconds as i64,
        };
        self.store.put_session(&hash, &stored)?;
        Ok((token, self.session_ttl_seconds))
    }

    pub fn authenticate(&self, bearer: &str) -> GatewayResult<Principal> {
        let hash = self.vault.token_hash(bearer)?;
        let now = now_unix();
        if let Some(session) = self.store.get_session(&hash, now)? {
            return Ok(Principal {
                username: session.username,
                auth_kind: AuthKind::Session,
                auth_id: None,
                scopes: vec!["*".into()],
                account_allowlist: Vec::new(),
                credential: self.vault.open_json(&session.credential_ciphertext)?,
                cypht_session: self.vault.open_json(&session.cypht_session_ciphertext)?,
            });
        }
        if let Some(pat) = self.store.get_pat_by_hash(&hash, now)? {
            self.store.touch_pat(&pat.id, now)?;
            let credential = self.vault.open_json(&pat.credential_ciphertext)?;
            let cypht_session = self.vault.open_json(
                pat.cypht_session_ciphertext
                    .as_deref()
                    .ok_or(GatewayError::Authentication)?,
            )?;
            return Ok(Principal {
                username: pat.username,
                auth_kind: AuthKind::Pat,
                auth_id: Some(pat.id),
                scopes: pat.scopes,
                account_allowlist: pat.account_allowlist,
                credential,
                cypht_session,
            });
        }
        Err(GatewayError::Authentication)
    }

    pub fn logout(&self, bearer: &str) -> GatewayResult<()> {
        let hash = self.vault.token_hash(bearer)?;
        self.store.delete_session(&hash)
    }

    pub fn create_pat(&self, principal: &Principal, name: &str, scopes: Vec<String>, account_allowlist: Vec<String>, expires_in_days: Option<u32>) -> GatewayResult<CreatedToken> {
        if matches!(&principal.auth_kind, AuthKind::Pat) {
            principal.requires(SCOPE_TOKENS_MANAGE)?;
            for scope in &scopes {
                if !principal.scopes.iter().any(|owned| owned == scope) {
                    return Err(GatewayError::PermissionDenied(format!(
                        "token cannot delegate scope it does not own: {scope}"
                    )));
                }
            }
            if !principal.account_allowlist.is_empty() {
                if account_allowlist.is_empty() {
                    return Err(GatewayError::PermissionDenied(
                        "token cannot broaden a restricted account allowlist".into(),
                    ));
                }
                for account_id in &account_allowlist {
                    if !principal.account_allowlist.iter().any(|owned| owned == account_id) {
                        return Err(GatewayError::PermissionDenied(
                            "token cannot delegate an account it cannot access".into(),
                        ));
                    }
                }
            }
        }
        validate_scopes(&scopes)?;
        if account_allowlist.len() > 100 {
            return Err(GatewayError::InvalidRequest(
                "account_allowlist cannot contain more than 100 entries".into(),
            ));
        }
        if let Some(days) = expires_in_days {
            if !(1..=3650).contains(&days) {
                return Err(GatewayError::InvalidRequest(
                    "expires_in_days must be between 1 and 3650".into(),
                ));
            }
        }
        if name.trim().is_empty() || name.len() > 100 {
            return Err(GatewayError::InvalidRequest("token name must contain 1-100 characters".into()));
        }
        let public_id = random_hex(8);
        let secret = random_hex(32);
        let token = format!("cypht_pat_{public_id}_{secret}");
        let prefix = format!("cypht_pat_{public_id}_****");
        let hash = self.vault.token_hash(&token)?;
        let now = now_unix();
        let expires_at = expires_in_days.map(|days| now + i64::from(days) * 86_400);
        let pat = StoredPat {
            id: format!("tok_{public_id}"),
            username: principal.username.clone(),
            name: name.trim().to_string(),
            prefix: prefix.clone(),
            scopes: scopes.clone(),
            account_allowlist: account_allowlist.clone(),
            credential_ciphertext: self.vault.seal_json(&principal.credential)?,
            cypht_session_ciphertext: Some(self.vault.seal_json(&principal.cypht_session)?),
            created_at: now,
            expires_at,
            last_used_at: None,
            revoked_at: None,
        };
        self.store.put_pat(&hash, &pat)?;
        Ok(CreatedToken {
            id: pat.id,
            name: pat.name,
            token,
            prefix,
            scopes,
            account_allowlist,
            created_at: now,
            expires_at,
        })
    }

    pub fn list_pats(&self, principal: &Principal) -> GatewayResult<Vec<gateway_core::TokenMetadata>> {
        if matches!(&principal.auth_kind, AuthKind::Pat) {
            principal.requires(SCOPE_TOKENS_MANAGE)?;
        }
        self.store.list_pats(&principal.username)
    }

    pub fn revoke_pat(&self, principal: &Principal, id: &str) -> GatewayResult<bool> {
        if matches!(&principal.auth_kind, AuthKind::Pat) {
            principal.requires(SCOPE_TOKENS_MANAGE)?;
        }
        self.store.revoke_pat(&principal.username, id, now_unix())
    }

    pub fn refresh_pat_session(&self, pat_id: &str, session: &CyphtSession) -> GatewayResult<()> {
        let ciphertext = self.vault.seal_json(session)?;
        self.store.update_pat_cypht_session(pat_id, &ciphertext)
    }
}

pub fn now_unix() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

fn derive_key(master_key: &[u8; 32], label: &[u8]) -> GatewayResult<[u8; 32]> {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(master_key).map_err(|_| GatewayError::Crypto)?;
    mac.update(label);
    let bytes = mac.finalize().into_bytes();
    let mut output = [0_u8; 32];
    output.copy_from_slice(&bytes);
    Ok(output)
}

fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0_u8; bytes];
    OsRng.fill_bytes(&mut buf);
    hex::encode(buf)
}

fn validate_scopes(scopes: &[String]) -> GatewayResult<()> {
    const ALLOWED: &[&str] = &[
        SCOPE_ACCOUNTS_READ,
        SCOPE_MAIL_READ,
        SCOPE_MAIL_SEARCH,
        SCOPE_ATTACHMENTS_READ,
        SCOPE_MAIL_SEND,
        SCOPE_MAIL_MODIFY,
        SCOPE_MAIL_DELETE,
        SCOPE_TOKENS_MANAGE,
    ];
    for (index, scope) in scopes.iter().enumerate() {
        if !ALLOWED.contains(&scope.as_str()) {
            return Err(GatewayError::InvalidRequest(format!("unknown scope: {scope}")));
        }
        if scopes[..index].iter().any(|existing| existing == scope) {
            return Err(GatewayError::InvalidRequest(format!("duplicate scope: {scope}")));
        }
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    fn test_vault() -> Vault {
        Vault::from_base64(&STANDARD.encode([9_u8; 32])).unwrap()
    }

    fn session_principal() -> Principal {
        Principal {
            username: "alice".into(),
            auth_kind: AuthKind::Session,
            auth_id: None,
            scopes: vec!["*".into()],
            account_allowlist: Vec::new(),
            credential: Credential { username: "alice".into(), password: "secret".into() },
            cypht_session: CyphtSession { hm_id: "id".into(), hm_session: "session".into() },
        }
    }

    #[test]
    fn vault_round_trips_credentials() {
        let vault = test_vault();
        let credential = Credential { username: "alice".into(), password: "secret".into() };
        let sealed = vault.seal_json(&credential).unwrap();
        let opened: Credential = vault.open_json(&sealed).unwrap();
        assert_eq!(opened.username, "alice");
        assert_eq!(opened.password, "secret");
    }

    #[test]
    fn read_only_pat_cannot_manage_tokens() {
        let store = Store::open_memory().unwrap();
        let auth = AuthService::new(store, test_vault(), 3600);
        let owner = session_principal();
        let created = auth.create_pat(
            &owner,
            "reader",
            vec![SCOPE_MAIL_READ.into()],
            Vec::new(),
            None,
        ).unwrap();
        let pat = auth.authenticate(&created.token).unwrap();
        assert!(matches!(auth.list_pats(&pat), Err(GatewayError::PermissionDenied(_))));
        assert!(matches!(auth.revoke_pat(&pat, &created.id), Err(GatewayError::PermissionDenied(_))));
    }

    #[test]
    fn pat_cannot_delegate_more_privilege_than_it_owns() {
        let store = Store::open_memory().unwrap();
        let auth = AuthService::new(store, test_vault(), 3600);
        let owner = session_principal();
        let created = auth.create_pat(
            &owner,
            "limited-manager",
            vec![SCOPE_TOKENS_MANAGE.into(), SCOPE_MAIL_READ.into()],
            vec!["account-a".into()],
            None,
        ).unwrap();
        let pat = auth.authenticate(&created.token).unwrap();
        assert!(matches!(
            auth.create_pat(&pat, "escalated", vec![SCOPE_MAIL_SEND.into()], vec!["account-a".into()], None),
            Err(GatewayError::PermissionDenied(_))
        ));
        assert!(matches!(
            auth.create_pat(&pat, "broader", vec![SCOPE_MAIL_READ.into()], Vec::new(), None),
            Err(GatewayError::PermissionDenied(_))
        ));
    }

    #[test]
    fn pat_with_manage_scope_can_list_tokens() {
        let store = Store::open_memory().unwrap();
        let auth = AuthService::new(store, test_vault(), 3600);
        let owner = session_principal();
        let created = auth.create_pat(
            &owner,
            "manager",
            vec![SCOPE_TOKENS_MANAGE.into()],
            Vec::new(),
            None,
        ).unwrap();
        let pat = auth.authenticate(&created.token).unwrap();
        assert!(!auth.list_pats(&pat).unwrap().is_empty());
    }
}
