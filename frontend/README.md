# Management frontend

v0.2 ships a dependency-free, same-origin management UI in `frontend/static/index.html`. It is embedded directly into `gateway-api`, which keeps deployment to one Rust gateway process and removes a separate Node runtime.

The UI currently covers:

- Cypht login/logout
- PAT creation, scopes, expiry and account allow-lists
- PAT listing/revocation
- configured accounts
- unified inbox
- cross-account search
- message reading and attachment metadata

Public browser operations use REST `/api/v1`; the browser never calls the private PHP bridge. If the frontend becomes complex enough to justify React/Vite later, preserve this API boundary and keep generated frontend artifacts reproducible.
