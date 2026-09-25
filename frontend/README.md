# Management frontend

v0.4 ships a dependency-free, same-origin management UI in `frontend/static/index.html`. It is embedded directly into `gateway-api`, which keeps deployment to one Rust gateway process and removes a separate Node runtime.

The UI covers:

- Cypht login/logout
- PAT creation, scopes, expiry and account allow-lists
- PAT listing/revocation
- configured accounts and sending profiles
- unified inbox and cross-account search
- message reading
- attachment upload/download
- compose, draft, scheduled send, reply/reply-all and forward
- read/unread, flag/unflag, move, archive and delete
- user-scoped audit log view
- MCP stdio/Streamable HTTP connection guidance

Public browser operations use REST `/api/v1`; the browser never calls the private PHP bridge. If the frontend later moves to React/Vite, preserve this API boundary and reproducible release build.
