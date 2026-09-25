# Cypht Gateway All-in-One Distribution

The All-in-One distribution packages the complete Cypht 2.12.0 Webmail runtime and the Rust Gateway into a **single, zero-configuration Docker container**.

---

## Architecture Overview

```
                      +---------------------------------------+
                      |   Cypht All-in-One Docker Container   |
                      |                                       |
  [Port 8088] --------+--> Nginx (80) + PHP-FPM               |
 (Webmail UI)         |    Cypht 2.12.0 Engine                |
                      |    + modules/gateway (PHP Bridge)     |
                      |            ^                          |
                      |            | Local Fastcall (80)      |
                      |            v                          |
  [Port 18080] -------+--> gatewayd (Rust Daemon)             |
 (REST API & UI)      |    REST /api/v1 + Web UI              |
                      |            ^                          |
                      |            | Local REST API           |
                      |            v                          |
  [Port 8790] --------+--> cypht-mcp (Rust Server)            |
 (MCP AI Server)      |    Streamable HTTP /mcp               |
                      |                                       |
                      | Managed by container supervisord      |
                      +---------------------------------------+
```

### Key Highlights
- **Single Container Simplicity**: Unraid runs only one container. No complex Docker networks or multi-container wiring.
- **Zero-Config Secrets**: Generates cryptographically secure keys (`GATEWAY_MASTER_KEY`, `API_LOGIN_KEY`, `GATEWAY_BRIDGE_KEY`) on first boot and saves them to `/var/lib/hm3/app_data/gateway/`.
- **No Path Mounts Required**: All PHP bridge modules and ini configs are baked into the image.
- **Full Capabilities**: Exposes 42 REST endpoints, Model Context Protocol (MCP) streamable HTTP server, and native Webmail simultaneously.

---

## Port Mappings

| Port | Description | Target Use Case |
|---|---|---|
| `8088 -> 80` | Native Cypht Webmail UI | Browser webmail interface |
| `18080 -> 18080` | Gateway REST API & UI | Applications, REST integrations, Management UI |
| `8790 -> 8790` | MCP Server (`/mcp`) | AI Agents (Claude, Cursor, Codex, OpenCode) |

---

## Quick Start with Docker

```bash
docker run -d \
  --name cypht-aio \
  --restart unless-stopped \
  -p 8088:80 \
  -p 18080:18080 \
  -p 8790:8790 \
  -v /mnt/user/appdata/cypht/users:/var/lib/hm3/users \
  -v /mnt/user/appdata/cypht/attachments:/var/lib/hm3/attachments \
  -v /mnt/user/appdata/cypht/app_data:/var/lib/hm3/app_data \
  ghcr.io/ashanzzz/cypht-gateway:latest
```

---

## Unraid Installation

1. Copy `deploy/unraid-cypht-aio.xml` to `/boot/config/plugins/dockerMan/templates-user/my-cypht-aio.xml` on your Unraid flash drive.
2. In Unraid Web UI, go to **Docker** -> **Add Container**, select template **cypht-aio**.
3. All ports and persistent volume paths are pre-configured. Click **Apply** to launch!