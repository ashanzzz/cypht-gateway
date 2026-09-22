# Cypht Gateway bridge

This directory is reserved for the private Cypht-side bridge module.

The bridge will be intentionally small. It will load authenticated Cypht user context, call existing Cypht handlers/services, normalize results, and communicate only with the Rust gateway over a private channel.

It must not become a second public REST API.
