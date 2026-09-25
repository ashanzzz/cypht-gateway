# Sieve status

The Gateway exposes only redacted Sieve capability status for Cypht 2.12.0.

- `GET /api/v1/sieve/status`
- Scope: `sieve.read`
- Account-restricted PATs receive only the status of accounts allowed by the token.
- The default AI scope preset does not include `sieve.read`.

Each result contains an opaque `account_id`, a safe display name, protocol, enabled state, configured state, a bounded status value, and `remote_probe=false`. It never returns Sieve hosts, passwords, credentials, script names, script text, capabilities, raw remote errors, or filter data.

Sieve script and filter writes remain deferred. The Bridge does not connect to ManageSieve for this status endpoint. A configured Sieve host is reported as configuration state only. Live provider probing is not part of the first release.
