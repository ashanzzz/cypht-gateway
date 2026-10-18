use gateway_core::{GatewayError, GatewayResult};
use rusqlite::Connection;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

pub fn migrate(conn: &mut Connection) -> GatewayResult<()> {
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;",
    )
    .map_err(|e| GatewayError::Storage(format!("enable WAL and foreign keys: {e}")))?;

    let current_version: u32 = conn
        .query_row("PRAGMA user_version;", [], |row| row.get(0))
        .map_err(|e| GatewayError::Storage(format!("read schema version: {e}")))?;

    if current_version < 1 {
        apply_v1(conn)?;
    }

    Ok(())
}

fn apply_v1(conn: &mut Connection) -> GatewayResult<()> {
    let tx = conn
        .transaction()
        .map_err(|e| GatewayError::Storage(format!("begin v1 migration tx: {e}")))?;

    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS gateway_sessions (
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
           ON gateway_object_refs(owner, kind);
         PRAGMA user_version = 1;",
    )
    .map_err(|e| GatewayError::Storage(format!("execute v1 migration DDL: {e}")))?;

    tx.commit()
        .map_err(|e| GatewayError::Storage(format!("commit v1 migration tx: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_database_migrates_to_current_schema_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(
            conn.query_row("PRAGMA user_version;", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            0
        );

        migrate(&mut conn).unwrap();

        assert_eq!(
            conn.query_row("PRAGMA user_version;", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            CURRENT_SCHEMA_VERSION
        );
    }

    #[test]
    fn repeated_migration_is_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        migrate(&mut conn).unwrap();
        assert_eq!(
            conn.query_row("PRAGMA user_version;", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            CURRENT_SCHEMA_VERSION
        );
    }
}
