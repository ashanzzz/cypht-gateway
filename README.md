# Cypht Gateway

Cypht Gateway is a Rust-first automation and AI gateway for Cypht. The project will expose Cypht capabilities through a stable REST API, MCP server, CLI, and management UI while leaving mail-provider protocol handling inside Cypht.

## Current milestone

`v0.1.0` establishes the repository and versioning foundation:

- Git/SemVer release model
- one product version shared by the Rust workspace
- embedded Git commit/tag/dirty build metadata
- `/healthz` and `/api/v1/meta/version`
- initial `cyphtctl` CLI
- OpenAPI seed contract
- Docker image foundation
- GitHub CI, multi-architecture GHCR image builds, and tag-driven releases
- compatibility placeholders for the future Cypht PHP bridge, MCP server, and frontend

Mail APIs and the Cypht bridge are intentionally not implemented in this milestone.

## Run locally

```bash
cargo run -p gatewayd
```

Then:

```bash
curl http://127.0.0.1:8080/healthz
curl http://127.0.0.1:8080/api/v1/meta/version
cargo run -p cyphtctl -- version
cargo run -p cyphtctl -- status --url http://127.0.0.1:8080
```

## Versioning

`VERSION` is the human-readable product version and must match `[workspace.package].version` in `Cargo.toml`.

Check consistency:

```bash
python3 scripts/check-version.py
```

Set a new version:

```bash
python3 scripts/set-version.py 0.2.0
```

Commit the change, then create an annotated tag:

```bash
git tag -a v0.2.0 -m "Cypht Gateway v0.2.0"
git push origin main --tags
```

Release builds require the Git tag to match `VERSION` exactly.

## Build metadata

The Rust build embeds:

- product version
- Git full SHA
- Git short SHA
- exact Git tag when available
- dirty-worktree state
- optional `BUILD_TIME` supplied by CI

This metadata is exposed by both the API and CLI.

## Repository layout

- `apps/gatewayd` HTTP daemon
- `apps/cyphtctl` CLI
- `crates/gateway-core` common product/build metadata
- `api` OpenAPI contract
- `cypht-module` future Cypht-side bridge
- `frontend` future management UI
- `docs` architecture and roadmap
- `.github/workflows` CI/build/release automation

See `docs/ARCHITECTURE.md` and `docs/ROADMAP.md` for the intended product architecture.
