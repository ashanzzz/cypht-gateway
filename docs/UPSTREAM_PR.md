# Cypht Upstream Pull Request Guide

This guide documents the changes to submit to the official [cypht-org/cypht](https://github.com/cypht-org/cypht) repository to fix Docker environment loading for the `api_login` module and facilitate official Gateway integration.

---

## PR 1: Fix `API_LOGIN_KEY` environment variable in `config/app.php`

### Issue Summary
In the official Cypht 2.12.0 and later releases, `modules/api_login` allows programmatic SSO authentication via `POST /?page=process_api_login`. However, in `config/app.php` (line 991), the configuration setting:
```php
// 'api_login_key' => env('API_LOGIN_KEY'),
```
is commented out by default. In Docker deployments, even when users pass `API_LOGIN_KEY` via container environment variables, Cypht fails to load it, causing `process_api_login` requests to fail authentication unless an extra configuration file is manually injected.

### Proposed Code Diff
```diff
--- a/config/app.php
+++ b/config/app.php
@@ -988,7 +988,7 @@ return [
     | API login
     | ----------
     */
-    // 'api_login_key' => env('API_LOGIN_KEY'),
+    'api_login_key' => env('API_LOGIN_KEY'),

     /*
     | ----------------------------------------------------------------------
```

### Git Branch & Commit Instructions

```bash
# 1. Clone your fork of cypht
git clone https://github.com/ashanzzz/cypht.git
cd cypht
git remote add upstream https://github.com/cypht-org/cypht.git
git fetch upstream

# 2. Create feature branch from upstream master
git checkout -b fix/api-login-env-var upstream/master

# 3. Apply the fix
# Uncomment 'api_login_key' in config/app.php

# 4. Commit and push
git commit -am "fix(config): load API_LOGIN_KEY from environment in app.php"
git push origin fix/api-login-env-var
```

### Pull Request Title & Description (English)

**Title**: `fix(config): enable API_LOGIN_KEY environment variable in app.php`

**Description**:
```markdown
### Summary
This PR uncomments `'api_login_key' => env('API_LOGIN_KEY')` in `config/app.php`.

### Background
When deploying Cypht via Docker or containers, configuration is typically supplied through environment variables. The `api_login` module requires `api_login_key` to validate programmatic login requests to `process_api_login`. Currently, the line in `config/app.php` is commented out, preventing the `API_LOGIN_KEY` environment variable from being read at runtime without manual file injection.

### Changes
- Uncomment `'api_login_key' => env('API_LOGIN_KEY')` in `config/app.php`.
- Defaults to null when `API_LOGIN_KEY` is unset, preserving existing behavior and security.

### Verification
Tested in containerized Cypht runtime with and without `API_LOGIN_KEY` set. When set, `process_api_login` successfully verifies the secret key. When unset, API login requests fail closed with 401 as expected.
```

---

## PR 2: Community Module Set Contribution (`modules/gateway`)

For upstream maintainers interested in providing first-party REST and Model Context Protocol (MCP) support for Cypht:

- The `cypht-module/gateway` directory complies with Cypht's official module set conventions (`setup.php`, `modules.php`, `handler_modules.php`).
- It extends `Hm_Handler_Module` to provide structured JSON adapters for IMAP accounts, mailboxes, messages, contacts, calendar events, tags, and saved searches without modifying core modules.
- The module can be installed directly under `modules/gateway/` or packaged as a Composer package for Cypht installations.