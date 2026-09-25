use gateway_core::{AuditEntry, GatewayError, GatewayResult, TokenMetadata};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSession {
    pub username: String,
    pub credential_ciphertext: Vec<u8>,
    pub cypht_session_ciphertext: Vec<u8>,
    pub created_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPat {
    pub id: String,
    pub username: String,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<String>,
    pub account_allowlist: Vec<String>,
    pub credential_ciphertext: Vec<u8>,
    pub cypht_session_ciphertext: Option<Vec<u8>>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredIdempotency {
    pub request_hash: String,
    pub response_json: String,
    pub created_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredObjectRef {
    pub public_id: String,
    pub owner: String,
    pub kind: String,
    pub parts_ciphertext: Vec<u8>,
    pub created_at: i64,
}

#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> GatewayResult<Self> {
        let conn = Connection::open(path).map_err(storage_err)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_memory() -> GatewayResult<Self> {
        let conn = Connection::open_in_memory().map_err(storage_err)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> GatewayResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| GatewayError::Storage("database lock poisoned".into()))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS gateway_sessions (
               token_hash TEXT PRIMARY KEY,
               username TEXT NOT NULL,
               credential_ciphertext BLOB NOT NULL,
               cypht_session_ciphertext BLOB NOT NULL,
               created_at INTEGER NOT NULL,
               expires_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS gateway_pats (
               id TEXT PRIMARY KEY,
               token_hash TEXT NOT NULL UNIQUE,
               username TEXT NOT NULL,
               name TEXT NOT NULL,
               prefix TEXT NOT NULL,
               scopes_json TEXT NOT NULL,
               account_allowlist_json TEXT NOT NULL,
               credential_ciphertext BLOB NOT NULL,
               cypht_session_ciphertext BLOB,
               created_at INTEGER NOT NULL,
               expires_at INTEGER,
               last_used_at INTEGER,
               revoked_at INTEGER
             );
             CREATE INDEX IF NOT EXISTS idx_gateway_pats_user ON gateway_pats(username);
             CREATE TABLE IF NOT EXISTS gateway_audit (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               created_at INTEGER NOT NULL,
               username TEXT NOT NULL,
               auth_id TEXT,
               operation TEXT NOT NULL,
               resource TEXT,
               success INTEGER NOT NULL,
               detail TEXT
             );
             CREATE TABLE IF NOT EXISTS gateway_idempotency (
               username TEXT NOT NULL,
               operation TEXT NOT NULL,
               idempotency_key TEXT NOT NULL,
               request_hash TEXT NOT NULL,
               response_json TEXT NOT NULL,
               created_at INTEGER NOT NULL,
               expires_at INTEGER NOT NULL,
               PRIMARY KEY (username, operation, idempotency_key)
             );
             CREATE INDEX IF NOT EXISTS idx_gateway_idempotency_expiry ON gateway_idempotency(expires_at);
             CREATE TABLE IF NOT EXISTS gateway_object_refs (
               public_id TEXT PRIMARY KEY,
               owner TEXT NOT NULL,
               kind TEXT NOT NULL,
               parts_ciphertext BLOB NOT NULL,
               created_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_gateway_object_refs_owner_kind
               ON gateway_object_refs(owner, kind);"
        ).map_err(storage_err)?;
        Ok(())
    }

    pub fn put_session(&self, token_hash: &str, session: &StoredSession) -> GatewayResult<()> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute(
            "INSERT OR REPLACE INTO gateway_sessions
             (token_hash, username, credential_ciphertext, cypht_session_ciphertext, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![token_hash, session.username, session.credential_ciphertext, session.cypht_session_ciphertext, session.created_at, session.expires_at],
        ).map_err(storage_err)?;
        Ok(())
    }

    pub fn get_session(&self, token_hash: &str, now: i64) -> GatewayResult<Option<StoredSession>> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let row = conn.query_row(
            "SELECT username, credential_ciphertext, cypht_session_ciphertext, created_at, expires_at
             FROM gateway_sessions WHERE token_hash=?1 AND expires_at>?2",
            params![token_hash, now],
            |row| Ok(StoredSession {
                username: row.get(0)?,
                credential_ciphertext: row.get(1)?,
                cypht_session_ciphertext: row.get(2)?,
                created_at: row.get(3)?,
                expires_at: row.get(4)?,
            }),
        ).optional().map_err(storage_err)?;
        Ok(row)
    }

    pub fn delete_session(&self, token_hash: &str) -> GatewayResult<()> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute(
            "DELETE FROM gateway_sessions WHERE token_hash=?1",
            params![token_hash],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn put_pat(&self, token_hash: &str, pat: &StoredPat) -> GatewayResult<()> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let scopes =
            serde_json::to_string(&pat.scopes).map_err(|e| GatewayError::Storage(e.to_string()))?;
        let allowlist = serde_json::to_string(&pat.account_allowlist)
            .map_err(|e| GatewayError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO gateway_pats
             (id, token_hash, username, name, prefix, scopes_json, account_allowlist_json,
              credential_ciphertext, cypht_session_ciphertext, created_at, expires_at, last_used_at, revoked_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![pat.id, token_hash, pat.username, pat.name, pat.prefix, scopes, allowlist,
                    pat.credential_ciphertext, pat.cypht_session_ciphertext, pat.created_at,
                    pat.expires_at, pat.last_used_at, pat.revoked_at],
        ).map_err(storage_err)?;
        Ok(())
    }

    pub fn get_pat_by_hash(&self, token_hash: &str, now: i64) -> GatewayResult<Option<StoredPat>> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let row = conn
            .query_row(
                "SELECT id, username, name, prefix, scopes_json, account_allowlist_json,
                    credential_ciphertext, cypht_session_ciphertext, created_at, expires_at,
                    last_used_at, revoked_at
             FROM gateway_pats
             WHERE token_hash=?1 AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at>?2)",
                params![token_hash, now],
                decode_pat_row,
            )
            .optional()
            .map_err(storage_err)?;
        Ok(row)
    }

    pub fn list_pats(&self, username: &str) -> GatewayResult<Vec<TokenMetadata>> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let mut stmt = conn.prepare(
            "SELECT id, name, prefix, scopes_json, account_allowlist_json, created_at, expires_at, last_used_at, revoked_at
             FROM gateway_pats WHERE username=?1 ORDER BY created_at DESC"
        ).map_err(storage_err)?;
        let rows = stmt
            .query_map(params![username], |row| {
                let scopes_json: String = row.get(3)?;
                let allowlist_json: String = row.get(4)?;
                Ok(TokenMetadata {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    prefix: row.get(2)?,
                    scopes: serde_json::from_str(&scopes_json).unwrap_or_default(),
                    account_allowlist: serde_json::from_str(&allowlist_json).unwrap_or_default(),
                    created_at: row.get(5)?,
                    expires_at: row.get(6)?,
                    last_used_at: row.get(7)?,
                    revoked_at: row.get(8)?,
                })
            })
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn revoke_pat(&self, username: &str, id: &str, now: i64) -> GatewayResult<bool> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let changed = conn.execute(
            "UPDATE gateway_pats SET revoked_at=?1 WHERE id=?2 AND username=?3 AND revoked_at IS NULL",
            params![now, id, username],
        ).map_err(storage_err)?;
        Ok(changed > 0)
    }

    pub fn touch_pat(&self, id: &str, now: i64) -> GatewayResult<()> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute(
            "UPDATE gateway_pats SET last_used_at=?1 WHERE id=?2",
            params![now, id],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn update_pat_cypht_session(&self, id: &str, ciphertext: &[u8]) -> GatewayResult<()> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute(
            "UPDATE gateway_pats SET cypht_session_ciphertext=?1 WHERE id=?2",
            params![ciphertext, id],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn get_idempotency(
        &self,
        username: &str,
        operation: &str,
        key: &str,
        now: i64,
    ) -> GatewayResult<Option<StoredIdempotency>> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let row = conn
            .query_row(
                "SELECT request_hash, response_json, created_at, expires_at
             FROM gateway_idempotency
             WHERE username=?1 AND operation=?2 AND idempotency_key=?3 AND expires_at>?4",
                params![username, operation, key, now],
                |row| {
                    Ok(StoredIdempotency {
                        request_hash: row.get(0)?,
                        response_json: row.get(1)?,
                        created_at: row.get(2)?,
                        expires_at: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(storage_err)?;
        Ok(row)
    }

    pub fn claim_idempotency(
        &self,
        username: &str,
        operation: &str,
        key: &str,
        value: &StoredIdempotency,
    ) -> GatewayResult<bool> {
        let conn = self.conn.lock().map_err(lock_err)?;
        // Expired reservations must not block a fresh atomic claim. The store
        // mutex keeps this delete+insert sequence serialized inside this
        // process, while SQLite's primary key prevents duplicate claims.
        conn.execute(
            "DELETE FROM gateway_idempotency
             WHERE username=?1 AND operation=?2 AND idempotency_key=?3 AND expires_at<=?4",
            params![username, operation, key, value.created_at],
        )
        .map_err(storage_err)?;
        let changed = conn.execute(
            "INSERT OR IGNORE INTO gateway_idempotency
             (username, operation, idempotency_key, request_hash, response_json, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![username, operation, key, value.request_hash, value.response_json, value.created_at, value.expires_at],
        ).map_err(storage_err)?;
        Ok(changed == 1)
    }

    pub fn complete_idempotency(
        &self,
        username: &str,
        operation: &str,
        key: &str,
        request_hash: &str,
        response_json: &str,
    ) -> GatewayResult<bool> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let changed = conn
            .execute(
                "UPDATE gateway_idempotency SET response_json=?1
             WHERE username=?2 AND operation=?3 AND idempotency_key=?4 AND request_hash=?5",
                params![response_json, username, operation, key, request_hash],
            )
            .map_err(storage_err)?;
        Ok(changed == 1)
    }

    pub fn put_object_refs(&self, values: &[StoredObjectRef]) -> GatewayResult<()> {
        let mut conn = self.conn.lock().map_err(lock_err)?;
        let transaction = conn.transaction().map_err(storage_err)?;
        for value in values {
            let changed = transaction
                .execute(
                    "INSERT INTO gateway_object_refs (public_id, owner, kind, parts_ciphertext, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(public_id) DO UPDATE SET parts_ciphertext=excluded.parts_ciphertext
                     WHERE gateway_object_refs.owner=excluded.owner AND gateway_object_refs.kind=excluded.kind",
                    params![value.public_id, value.owner, value.kind, value.parts_ciphertext, value.created_at],
                )
                .map_err(storage_err)?;
            if changed == 0 {
                return Err(GatewayError::Storage(
                    "opaque object reference collision".into(),
                ));
            }
        }
        transaction.commit().map_err(storage_err)
    }
    pub fn put_object_ref(&self, value: &StoredObjectRef) -> GatewayResult<()> {
        self.put_object_refs(std::slice::from_ref(value))
    }
    pub fn list_object_refs(&self, owner: &str, kind: &str) -> GatewayResult<Vec<StoredObjectRef>> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let mut statement = conn
            .prepare(
                "SELECT public_id, owner, kind, parts_ciphertext, created_at
                 FROM gateway_object_refs WHERE owner=?1 AND kind=?2",
            )
            .map_err(storage_err)?;
        let rows = statement
            .query_map(params![owner, kind], |row| {
                Ok(StoredObjectRef {
                    public_id: row.get(0)?,
                    owner: row.get(1)?,
                    kind: row.get(2)?,
                    parts_ciphertext: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }
    pub fn get_object_ref(
        &self,
        owner: &str,
        kind: &str,
        public_id: &str,
    ) -> GatewayResult<Option<StoredObjectRef>> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let row = conn
            .query_row(
                "SELECT public_id, owner, kind, parts_ciphertext, created_at
                 FROM gateway_object_refs WHERE public_id=?1 AND owner=?2 AND kind=?3",
                params![public_id, owner, kind],
                |row| {
                    Ok(StoredObjectRef {
                        public_id: row.get(0)?,
                        owner: row.get(1)?,
                        kind: row.get(2)?,
                        parts_ciphertext: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(storage_err)?;
        Ok(row)
    }

    pub fn put_idempotency(
        &self,
        username: &str,
        operation: &str,
        key: &str,
        value: &StoredIdempotency,
    ) -> GatewayResult<()> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute(
            "INSERT OR REPLACE INTO gateway_idempotency
             (username, operation, idempotency_key, request_hash, response_json, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![username, operation, key, value.request_hash, value.response_json, value.created_at, value.expires_at],
        ).map_err(storage_err)?;
        Ok(())
    }

    pub fn prune_idempotency(&self, now: i64) -> GatewayResult<usize> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute(
            "DELETE FROM gateway_idempotency WHERE expires_at<=?1",
            params![now],
        )
        .map_err(storage_err)
    }

    pub fn list_audit(
        &self,
        username: &str,
        offset: u64,
        limit: u32,
    ) -> GatewayResult<Vec<AuditEntry>> {
        let limit = limit.clamp(1, 200);
        let offset = i64::try_from(offset)
            .map_err(|_| GatewayError::InvalidRequest("audit offset is too large".into()))?;
        let conn = self.conn.lock().map_err(lock_err)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, created_at, username, auth_id, operation, resource, success, detail
             FROM gateway_audit WHERE username=?1 ORDER BY id DESC LIMIT ?2 OFFSET ?3",
            )
            .map_err(storage_err)?;
        let rows = stmt
            .query_map(params![username, i64::from(limit), offset], |row| {
                Ok(AuditEntry {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    username: row.get(2)?,
                    auth_id: row.get(3)?,
                    operation: row.get(4)?,
                    resource: row.get(5)?,
                    success: row.get::<_, i64>(6)? != 0,
                    detail: row.get(7)?,
                })
            })
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn audit(
        &self,
        username: &str,
        auth_id: Option<&str>,
        operation: &str,
        resource: Option<&str>,
        success: bool,
        detail: Option<&str>,
        now: i64,
    ) -> GatewayResult<()> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute(
            "INSERT INTO gateway_audit (created_at, username, auth_id, operation, resource, success, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![now, username, auth_id, operation, resource, if success {1} else {0}, detail],
        ).map_err(storage_err)?;
        Ok(())
    }
}

fn decode_pat_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredPat> {
    let scopes_json: String = row.get(4)?;
    let allowlist_json: String = row.get(5)?;
    Ok(StoredPat {
        id: row.get(0)?,
        username: row.get(1)?,
        name: row.get(2)?,
        prefix: row.get(3)?,
        scopes: serde_json::from_str(&scopes_json).unwrap_or_default(),
        account_allowlist: serde_json::from_str(&allowlist_json).unwrap_or_default(),
        credential_ciphertext: row.get(6)?,
        cypht_session_ciphertext: row.get(7)?,
        created_at: row.get(8)?,
        expires_at: row.get(9)?,
        last_used_at: row.get(10)?,
        revoked_at: row.get(11)?,
    })
}

fn storage_err(error: rusqlite::Error) -> GatewayError {
    GatewayError::Storage(error.to_string())
}

fn lock_err<T>(_: std::sync::PoisonError<T>) -> GatewayError {
    GatewayError::Storage("database lock poisoned".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_refs_are_bound_to_owner_and_kind() {
        let store = Store::open_memory().unwrap();
        let reference = StoredObjectRef {
            public_id: "v2.signed-id".into(),
            owner: "alice".into(),
            kind: "message".into(),
            parts_ciphertext: vec![1, 2, 3],
            created_at: 10,
        };
        store.put_object_ref(&reference).unwrap();

        assert_eq!(
            store
                .get_object_ref("alice", "message", "v2.signed-id")
                .unwrap()
                .unwrap()
                .parts_ciphertext,
            vec![1, 2, 3]
        );
        assert!(store
            .get_object_ref("bob", "message", "v2.signed-id")
            .unwrap()
            .is_none());
        assert!(store
            .get_object_ref("alice", "attachment", "v2.signed-id")
            .unwrap()
            .is_none());
    }

    fn sample_pat() -> StoredPat {
        StoredPat {
            id: "tok_test".into(),
            username: "alice".into(),
            name: "reader".into(),
            prefix: "cypht_pat_test_****".into(),
            scopes: vec!["mail.read".into()],
            account_allowlist: vec!["account-1".into()],
            credential_ciphertext: vec![1, 2, 3],
            cypht_session_ciphertext: Some(vec![4, 5, 6]),
            created_at: 100,
            expires_at: Some(1_000),
            last_used_at: None,
            revoked_at: None,
        }
    }

    #[test]
    fn pat_lifecycle_honors_expiry_and_revocation() {
        let store = Store::open_memory().unwrap();
        let pat = sample_pat();
        store.put_pat("hash", &pat).unwrap();
        assert!(store.get_pat_by_hash("hash", 500).unwrap().is_some());
        assert!(store.get_pat_by_hash("hash", 1_000).unwrap().is_none());
        assert!(store.revoke_pat("alice", "tok_test", 600).unwrap());
        assert!(store.get_pat_by_hash("hash", 700).unwrap().is_none());
    }

    #[test]
    fn sessions_expire() {
        let store = Store::open_memory().unwrap();
        let session = StoredSession {
            username: "alice".into(),
            credential_ciphertext: vec![1],
            cypht_session_ciphertext: vec![2],
            created_at: 100,
            expires_at: 200,
        };
        store.put_session("hash", &session).unwrap();
        assert!(store.get_session("hash", 199).unwrap().is_some());
        assert!(store.get_session("hash", 200).unwrap().is_none());
    }

    #[test]
    fn audit_is_user_scoped_and_paginated() {
        let store = Store::open_memory().unwrap();
        store
            .audit(
                "alice",
                Some("tok_a"),
                "mail.read",
                Some("msg1"),
                true,
                None,
                100,
            )
            .unwrap();
        store
            .audit(
                "bob",
                Some("tok_b"),
                "mail.read",
                Some("msg2"),
                true,
                None,
                101,
            )
            .unwrap();
        store
            .audit("alice", None, "mail.search", None, true, None, 102)
            .unwrap();
        let first = store.list_audit("alice", 0, 1).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].operation, "mail.search");
        let second = store.list_audit("alice", 1, 10).unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].resource.as_deref(), Some("msg1"));
    }

    #[test]
    fn idempotency_round_trip_and_expiry() {
        let store = Store::open_memory().unwrap();
        let value = StoredIdempotency {
            request_hash: "abc".into(),
            response_json: "{\"status\":\"sent\"}".into(),
            created_at: 100,
            expires_at: 200,
        };
        assert!(store
            .claim_idempotency("alice", "mail.send", "key-1", &value)
            .unwrap());
        assert!(!store
            .claim_idempotency("alice", "mail.send", "key-1", &value)
            .unwrap());
        assert_eq!(
            store
                .get_idempotency("alice", "mail.send", "key-1", 150)
                .unwrap()
                .unwrap()
                .request_hash,
            "abc"
        );
        assert!(store
            .complete_idempotency(
                "alice",
                "mail.send",
                "key-1",
                "abc",
                "{\"status\":\"sent\"}"
            )
            .unwrap());
        assert!(store
            .get_idempotency("alice", "mail.send", "key-1", 200)
            .unwrap()
            .is_none());
        let replacement = StoredIdempotency {
            request_hash: "def".into(),
            response_json: String::new(),
            created_at: 201,
            expires_at: 300,
        };
        assert!(store
            .claim_idempotency("alice", "mail.send", "key-1", &replacement)
            .unwrap());
        assert_eq!(
            store
                .get_idempotency("alice", "mail.send", "key-1", 250)
                .unwrap()
                .unwrap()
                .request_hash,
            "def"
        );
    }
}
