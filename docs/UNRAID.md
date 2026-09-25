# Unraid Docker deployment (Cypht 2.12.0)

This guide separates the current Unraid Cypht service from the local Gateway test. No Gateway container is deployed on Unraid. The local Gateway and MCP processes are running on the development computer.

## Verified current state

A read-only SSH inspection on September 23, 2026 confirmed that container `Cypht` runs `cypht/cypht:2.12.0` on an x86_64 host. It is healthy and currently publishes host port `8088` to container port `80`. No Gateway container is running. The private Gateway Bridge directory is absent. A filtered environment-name inspection found no `API_LOGIN_KEY` or `GATEWAY_BRIDGE_KEY` variables; their values were not read or printed.

Cypht has these persistent mounts:

| Host path | Container path |
|---|---|
| `/mnt/cache/appdata/cypht/attachments` | `/var/lib/hm3/attachments` |
| `/mnt/cache/appdata/cypht/app_data` | `/var/lib/hm3/app_data` |
| `/mnt/cache/appdata/cypht/users` | `/var/lib/hm3/users` |

Cypht is attached to Docker's default `bridge` network. A read-only inspection showed the MySQL backend uses the Unraid host. The container does not set `CYPHT_MODULES`. The Unraid Docker template has no explicit module-list parameter. Cypht 2.12.0 source defaults to `core,contacts,local_contacts,feeds,imap,smtp,account,idle_timer,calendar,themes,nux,developer,history,saved_searches,advanced_search,highlights,profiles,inline_message,imap_folders,keyboard_shortcuts,tags,brute_force` when this variable is absent. This is a source-based expectation, not a runtime dump. Append `api_login,gateway` to that list in the replacement image template. Host port `8080` is already published by another container. A read-only socket check on September 23, 2026 found no listeners on ports `8089` or `18080`. Recheck those ports before cutover.

Cypht remains at `http://192.168.8.11:8088/`. Deploy Gateway on a separate, verified-free host port. The Compose edge proxy serves the Gateway UI at `/`, REST at `/api/v1/`, and MCP at `/mcp`. It does not replace or proxy the Cypht native UI.

## Local development from the current computer

Copy `deploy/local-dev.env.example` to the ignored `deploy/local-dev.env` file. Do not commit this file.

Run local health and UI checks without Bridge credentials:

```powershell
.\scripts\run-local.ps1 -HealthOnly
```

Health-only mode creates temporary process keys, stores its test database under the system temporary directory, and cannot log in to Cypht. For Bridge login tests, fill the three separate secrets with values matching Cypht and run `scripts\run-local.ps1` without `-HealthOnly`.

The local Gateway UI and REST API run at `http://127.0.0.1:18080/`. Start the separate Rust MCP process to serve `http://127.0.0.1:8790/mcp`:

```powershell
cypht-mcp --transport http --bind 127.0.0.1:8790 --url http://127.0.0.1:18080
```

This Gateway uses temporary health-only keys and cannot log in to Cypht. A local Docker profile can use `http://192.168.8.11:8088/` as its upstream. The optional local edge profile uses `http://127.0.0.1:8088/` if Docker is installed and that port is free.

This local test does not move the Unraid port. The current Cypht container lacks the Gateway Bridge. Authenticated Gateway login and mail operations remain unverified until a persistent Cypht image provides the Bridge.

## Build and install the private Bridge

Build `docker/Dockerfile.cypht` from this repository. It uses the pinned `cypht/cypht:2.12.0` image and copies the Bridge into the image. Deploy that image through the Unraid template. Do not copy files into a running container because container recreation removes them. The image also installs a small runtime config file that maps Cypht's `API_LOGIN_KEY` environment variable to the 2.12.0 `api_login_key` setting; the secret is not baked into the image.

Preserve all three verified data mounts. Configure separate `API_LOGIN_KEY` and `GATEWAY_BRIDGE_KEY` values in Cypht. Set their matching `CYPHT_API_LOGIN_KEY` and `CYPHT_BRIDGE_KEY` values in Gateway. Never commit or log these values. The custom image uses `USER_CONFIG_TYPE=custom:Gateway_User_Config_File` for file-backed user settings. If the Unraid template overrides it with `DB`, Contacts and Tags writes stay disabled. Do not change an existing user-settings backend without a migration and backup.

Inspect Cypht's generated module configuration before changing `CYPHT_MODULES`. Keep all current modules enabled. Add `api_login` and `gateway` only after the custom image is ready. Local Contacts also needs `contacts` and `local_contacts`.

## Deploy Gateway on a separate host port

Keep Cypht on its current host mapping `8088 -> 80`. Do not move or replace its native UI. Publish the Gateway edge on a separate host port, such as `18081`, after confirming that port is free on Unraid. The example environment file uses `http://192.168.8.11:18081/` for the Gateway.

Before deployment, back up the Unraid template and verify all persistent mounts. Create the `cypht-gateway` Docker network by starting the Compose stack. Attach the Cypht container to this network with the lowercase alias `cypht`. Keep the Compose services `gateway`, `mcp`, and `edge` on this network so the Gateway can resolve its upstream.

Set `CYPHT_BASE_URL=http://cypht:80/` in `deploy/unraid.env`. Do not point Gateway to `http://192.168.8.11:8088/` after both containers share the private network. Direct Gateway REST binds to host loopback at `18080`. The edge proxy binds to `192.168.8.11:18081`. MCP binds to host loopback at `8790`.

Copy `deploy/unraid.env.example` to the ignored `deploy/unraid.env`. Set unique values for `GATEWAY_MASTER_KEY`, `CYPHT_API_LOGIN_KEY`, and `CYPHT_BRIDGE_KEY`. The API login and Bridge keys must match their separate Cypht container values. Keep the file out of Git and restrict access to it.

Confirm host ports `18080`, `18081`, and `8790` are free before starting. Run Compose from the repository root:

```bash
docker compose --profile edge --env-file deploy/unraid.env -f deploy/docker-compose.yml up -d --build
```

The edge proxy serves the Gateway UI at `/`, REST at `/api/v1/`, and MCP at `/mcp`. It blocks public `ajax_gateway_*` Bridge requests. MCP accepts the configured Gateway host and preserves each caller's bearer token. Cypht remains separately available at `http://192.168.8.11:8088/`.

This deployment has not been applied. Do not recreate or reconfigure the Cypht container until the user-data mounts and Docker network settings are backed up and verified.
## Verify without changing mail

1. Confirm the rebuilt Cypht image reports version `2.12.0`.
2. Confirm Cypht loads `api_login` and `gateway` from the persistent image.
3. Confirm Bridge ping and both version handshakes succeed over the private network.
4. Confirm `GET /healthz` and `GET /api/v1/meta/version` work through the proxy.
5. Confirm `/mcp` preserves the caller's bearer token and stays read-only by default.
6. Read accounts, mailboxes, bounded message pages, and search results. Do not report message bodies or credentials.
7. Test Contacts writes only with synthetic records after persistence and new-session readback pass.
8. Do not create a sending Profile, send, move, or delete real mail without separate authorization.

A fresh browser session may show different Cypht accounts from an older session. Confirm saved settings at the current Cypht URL before cutover. Do not treat browser cache clearing as proof of persistent configuration.

Cypht 2.12.0 risks are listed in `docs/CYPHT_COMPATIBILITY.md`. Keep Cypht and Gateway on trusted networks. Do not claim later Cypht fixes are present.
