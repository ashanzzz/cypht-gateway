# Cypht Upstream Integration & PR Blueprint

本文件是向 Cypht 官方仓库（[cypht-org/cypht](https://github.com/cypht-org/cypht)）提交代码合并（Pull Request）与推进官方原生采纳的**权威行动路线图**。

---

## 愿景与策略（Upstream First Strategy）

### 为什么官方会非常欢迎这项增强？
1. **开源邮件生态的技术空白**：在开源自建邮箱客户端中（Roundcube、Rainloop、Snappymail、Cypht），目前**没有任何一家原生支持现代 AI Agent 协议（Model Context Protocol / MCP）与规范化的 REST API**。
2. **零侵入官方核心**：我们设计的 `modules/gateway` 遵循 Cypht 原生 Module Set 规范，对 Cypht 原有的 40 多个核心模块**零改动、零破坏**；如果未启用该模块，对原有 Cypht 网页版性能和行为产生 0 影响。
3. **架构解耦**：Cypht 继续做它擅长的 IMAP/SMTP/MIME 邮件协议引擎与用户认证，Rust Gateway 负责对外暴露高性能流式长连接（MCP SSE）与 Token 权限控制。

---

## 第一步：提交基础修复 PR（The Door Opener Bugfix）

这是最容易被官方立即合并的高价值微小修复，解决官方 Docker 镜像无法读取环境变量的问题。

### 1. 提交目标文件
`config/app.php`（第 991 行）

### 2. 代码变更（Diff）
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

### 3. Git 操作流程（在你的 GitHub Fork 上操作）
```bash
# 1. 在 GitHub 网页点击 cypht-org/cypht 的 Fork 按钮
# 2. 克隆你的 Fork 仓库
git clone https://github.com/ashanzzz/cypht.git
cd cypht
git remote add upstream https://github.com/cypht-org/cypht.git
git fetch upstream

# 3. 创建独立修复分支
git checkout -b fix/api-login-env-key upstream/master

# 4. 修改 config/app.php 取消注释，并提交
git commit -am "fix(config): load API_LOGIN_KEY from environment in app.php"
git push origin fix/api-login-env-key
```

### 4. PR 标题与英文描述（可直接复制粘贴到 GitHub PR）

**Title**: `fix(config): enable API_LOGIN_KEY environment variable in app.php`

**Description**:
```markdown
### Summary
This PR uncomments `'api_login_key' => env('API_LOGIN_KEY')` in `config/app.php`.

### Background / Motivation
When running Cypht inside Docker or cloud container environments, configuration is supplied through environment variables rather than hardcoded configuration files.

While `modules/api_login` is a native Cypht module providing programmatic authentication via `POST /?page=process_api_login`, the setting in `config/app.php` is commented out by default. As a result, setting `API_LOGIN_KEY=...` in the container environment is ignored, and API login requests cannot authenticate without mounting or editing internal files.

### Changes
- Uncomment `'api_login_key' => env('API_LOGIN_KEY')` in `config/app.php`.
- When `API_LOGIN_KEY` is not set in the environment, `env()` gracefully returns `null`, preserving existing security and behavior.

### Verification
Verified in PHP 8.1 container deployment. Setting `API_LOGIN_KEY` allows `process_api_login` to validate the secret key. When unset, requests fail closed with 401.
```

---

## 第二步：在官方 GitHub Discussions 发起 RFC 讨论

在提交完整的 `gateway` 模块前，先在官方社区建立共识，向作者（Jason Munro）介绍 MCP 与 REST API 的价值。

**讨论分类**：Ideas / Architecture

**讨论标题**：
`[RFC] Native REST API and Model Context Protocol (MCP) support for Cypht`

**讨论正文（可直接复制）**：
```markdown
Hi Cypht team and community!

Cypht is one of the most reliable and lightweight self-hosted webmail engines available. With the rapid rise of AI agents (Claude, Cursor, Codex, OpenCode) and modern client ecosystems, there is growing demand for:
1. Standardized, headless REST API access to mailbox data (without parsing HTML).
2. Native support for Model Context Protocol (MCP) so AI coding agents and assistants can safely read, search, and manage mail and contacts.

### Architecture Proposal
We have developed a non-intrusive extension that keeps Cypht's proven IMAP/SMTP/MIME core untouched:

1. **A standard Cypht Module Set (`modules/gateway`)**:
   - Implements standard `Hm_Handler_Module` classes.
   - Provides internal AJAX endpoints (`ajax_gateway_*`) that serialize mailboxes, messages, contacts, tags, and calendar events into structured JSON.
   - Guarded by session authentication and an internal bridge secret (`X-Cypht-Gateway-Key`).
   - Zero modifications to Cypht core modules.

2. **External Gateway Daemon**:
   - An asynchronous daemon (written in Rust) that speaks to the internal bridge.
   - Handles public REST v1 endpoints, scoped PAT tokens, idempotency protection, and Streamable HTTP MCP server.

We would love to contribute `modules/gateway` upstream as an official/optional module set, making Cypht the very first open-source webmail natively accessible by AI agents.

Looking forward to your thoughts and feedback!
```

---

## 第三步：提交 `modules/gateway` 模块 PR

一旦讨论获得积极回应，提交 `modules/gateway` 作为一个完整的独立 Module Set：

* **代码目录**：`modules/gateway/`
* **符合规范**：
  - `setup.php`：规范声明 `allowed_pages`、`allowed_post`、`allowed_get`、`allowed_server`；
  - `modules.php`：规范声明模块加载；
  - `handler_modules.php`：标准继承 `Hm_Handler_Module`；
  - 完全由 `CYPHT_MODULES` 开关控制，默认不影响任何原生功能。