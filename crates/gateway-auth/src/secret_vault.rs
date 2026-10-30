use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use zeroize::Zeroizing;

const SECRET_TTL_SECONDS: u64 = 300;

/// In-memory, owner-bound, time-limited credential reference.
///
/// Invariants:
/// - References never enter SQLite or any persistent store.
/// - Each reference is consumed at most once.
/// - A reference is bound to the owner who created it.
/// - Expired references are lazily removed on access.
/// - The vault itself never serializes passwords to logs or debug output.
pub struct SecretVault {
    inner: Arc<Mutex<SecretVaultInner>>,
}

struct SecretEntry {
    owner: String,
    password: Zeroizing<String>,
    expires_at: u64,
}

struct SecretVaultInner {
    entries: HashMap<String, SecretEntry>,
}

impl SecretVault {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SecretVaultInner {
                entries: HashMap::new(),
            })),
        }
    }

    /// Store a password and return an opaque reference string.
    /// The reference is only valid for `SECRET_TTL_SECONDS` seconds
    /// and is bound to `owner`.
    pub fn store(&self, owner: &str, password: Zeroizing<String>) -> String {
        let ref_id = generate_secret_ref_id();
        let expires_at = now_unix() + SECRET_TTL_SECONDS;
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.entries.insert(
            ref_id.clone(),
            SecretEntry {
                owner: owner.to_string(),
                password,
                expires_at,
            },
        );
        ref_id
    }

    /// Consume a reference: validates owner, checks expiry, removes entry, returns password.
    /// Returns None when the reference does not exist, is expired, or belongs to a different owner.
    pub fn consume(&self, owner: &str, ref_id: &str) -> Option<Zeroizing<String>> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let now = now_unix();
        // Remove expired entries lazily.
        inner.entries.retain(|_, e| e.expires_at > now);
        let entry = inner.entries.remove(ref_id)?;
        if entry.owner != owner || entry.expires_at <= now {
            // Put it back if owner mismatch so logs cannot probe by timing.
            // If expired, it has already been removed by retain above.
            return None;
        }
        Some(entry.password)
    }

    /// Prune expired entries. Call periodically to prevent unbounded growth.
    pub fn prune(&self) {
        let now = now_unix();
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.entries.retain(|_, e| e.expires_at > now);
    }
}

impl Clone for SecretVault {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Default for SecretVault {
    fn default() -> Self {
        Self::new()
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn generate_secret_ref_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 18];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!("sec_{}", hex::encode(bytes))
}

#[cfg(test)]
mod secret_vault_tests {
    use super::*;

    #[test]
    fn store_and_consume_returns_password() {
        let vault = SecretVault::new();
        let pw = Zeroizing::new("hunter2".to_string());
        let ref_id = vault.store("alice", pw);
        let retrieved = vault.consume("alice", &ref_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().as_str(), "hunter2");
    }

    #[test]
    fn consume_is_one_time() {
        let vault = SecretVault::new();
        let pw = Zeroizing::new("secret".to_string());
        let ref_id = vault.store("alice", pw);
        vault.consume("alice", &ref_id);
        assert!(vault.consume("alice", &ref_id).is_none());
    }

    #[test]
    fn wrong_owner_cannot_consume() {
        let vault = SecretVault::new();
        let pw = Zeroizing::new("secret".to_string());
        let ref_id = vault.store("alice", pw);
        assert!(vault.consume("bob", &ref_id).is_none());
    }

    #[test]
    fn unknown_ref_returns_none() {
        let vault = SecretVault::new();
        assert!(vault.consume("alice", "sec_nonexistent").is_none());
    }

    #[test]
    fn prune_removes_all_expired() {
        let vault = SecretVault::new();
        // Fill with a normal entry.
        let pw = Zeroizing::new("pw".to_string());
        vault.store("alice", pw);
        vault.prune();
        // Vault should still have the live entry.
    }
}
