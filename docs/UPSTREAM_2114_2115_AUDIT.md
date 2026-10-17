# Cypht Gateway 借鉴 #2114 / #2115 自查与修改计划

> **目标仓库**：`ashanzzz/cypht-gateway`  
> **参考上游**：Cypht issue `#2114` / PR `#2115`  
> **日期**：2026-10-17  
> **用途**：用于仓库自查、代码修改、PR 拆分、Release Gate 和后续路线规划  
> **核心原则**：借鉴上游设计中成熟的安全、权限、工具目录、OAuth、超时和调度机制，但保持 `cypht-gateway` 作为**外置高可靠 Automation Control Plane**，不把核心控制逻辑重新塞回 PHP。

---

# 0. Executive Summary

## 0.1 我们从 #2114 / #2115 得到的核心提示

Cypht 上游现在已经出现完整的原生 MCP/REST 方案，覆盖：

- MCP Streamable HTTP
- REST `/api/v1`
- OAuth 2.1 / PKCE
- Personal Access Token
- per-user / per-connection 权限
- allowed accounts
- Mail read/search
- Draft / send / reply / forward
- Folder CRUD
- Tags / Contacts
- Scheduled Send Runner
- Activity Log
- OpenAPI
- IMAP request deadline / socket timeout
- SSRF / outbound download policy
- 统一 operation catalog

这说明 Cypht 面向 AI / Automation 的需求已经被上游确认，不再只是 Gateway 单独探索。

因此 `cypht-gateway` 后续不要定位成：

> “另一个 MCP server”

而应该定位成：

> **Cypht Automation Control Plane**

职责划分建议固定为：

```text
AI / Apps / Scripts
        │
        ├── MCP
        ├── REST
        ├── CLI
        └── OAuth
             │
       Rust Gateway
             │
   ┌─────────┼───────────┐
   │         │           │
 Auth     Policy      Audit
 OAuth    RateLimit   Scheduler
 Secrets  SSRF        Idempotency
 Health   Deadlines   Capability
   │
   ▼
Cypht Backend Adapter
   │
   ▼
 Cypht
   │
IMAP / JMAP / EWS / SMTP
```

---

# 1. 总体结论

## 1.1 应直接借鉴

- [ ] 单一 `Operation Catalog`
- [ ] MCP ToolAnnotations
- [ ] 用户级权限 ∩ Connection 权限 ∩ Token Scope ∩ Account Allowlist
- [ ] OAuth 2.1 + PKCE S256
- [ ] Integration Connection 一等资源
- [ ] refresh token rotation + replay revocation
- [ ] request deadline 传递到 IMAP socket
- [ ] Scheduled Send background runner
- [ ] safe permanent delete / UIDPLUS handling
- [ ] outbound network policy
- [ ] metadata-only activity log
- [ ] AI-safe message representation
- [ ] Host / Backend abstraction

## 1.2 不建议照搬

- [ ] 不把 MCP / OAuth / policy / audit 主逻辑迁回 PHP
- [ ] 不依赖 PHP-FPM 作为长期任务执行器
- [ ] 不默认给 AI `organize` / `drafts` 等写权限
- [ ] 不接受浏览器 stale session 覆盖 API durable writes
- [ ] 不把 Gateway REST 与 MCP 各自维护一套 schema
- [ ] 不在 MCP tool input 中长期保留明文邮箱密码
- [ ] 不把未来所有 Cypht 版本视为无条件可写兼容

---

# 2. P0：立即修复项

这些问题应在 Accounts CRUD / OAuth 大规模扩展之前完成。

---

## P0-01 修复当前 CI Build Breaker

### 现状

`gateway-cypht` 使用：

```rust
tracing::info!(...)
```

但 `crates/gateway-cypht/Cargo.toml` 缺少：

```toml
tracing.workspace = true
```

### 修改

文件：

```text
crates/gateway-cypht/Cargo.toml
```

增加：

```toml
[dependencies]
tracing.workspace = true
```

### 验收

- [ ] `cargo check --workspace`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] GitHub CI 全绿

---

## P0-02 Cypht Production Baseline 升级到 2.13.0

### 问题

Gateway 已放松版本检查，但私有 Cypht image 仍以 2.12.0 为默认基线。

### 建议

生产默认：

```text
Cypht 2.13.0
```

兼容测试矩阵：

```text
2.12.0
2.12.2
2.13.0
```

### 修改文件

```text
docker/Dockerfile.cypht
docs/CYPHT_COMPATIBILITY.md
docs/UNRAID.md
README.md
scripts/check-repository.py
CI workflow
```

### 验收

- [ ] Bridge 在 2.13.0 可启动
- [ ] login/ping 正常
- [ ] accounts/mailboxes/messages/search 正常
- [ ] contacts/tags/saved-searches durable write 正常
- [ ] mail send / move / delete E2E
- [ ] baseline 文档与 Docker tag 一致

---

## P0-03 不允许“任意未来 Cypht 版本无条件写配置”

### 问题

版本强 pin 被删除是正确方向，但：

```text
unknown future version == automatically writable
```

不安全。

Gateway Bridge 使用 Cypht 内部：

```text
Hm_IMAP_List
Hm_SMTP_List
profiles
user_config
tags
saved_searches
calendar
```

未来结构变化可能造成数据损坏。

### 修改方向：Capability Handshake

Bridge Ping 增加：

```json
{
  "status": "ok",
  "bridge_version": "0.6.0",
  "bridge_api_version": 2,
  "cypht_version": "2.13.0",
  "capabilities": [
    "mail.read.v1",
    "mail.write.v1",
    "user_config.atomic.v2",
    "accounts.read.v1",
    "accounts.write.v1",
    "profiles.write.v1"
  ]
}
```

### Gateway 行为

Read：

```text
未知版本 + capability probe OK
→ 可继续
```

Write：

```text
未知版本 + accounts.write capability 缺失
→ CapabilityUnavailable
```

管理员 override：

```env
CYPHT_ALLOW_UNTESTED_WRITES=false
```

默认必须为 false。

### 相关文件

```text
cypht-module/gateway/handler_modules.php
crates/gateway-cypht/src/lib.rs
crates/gateway-core/
docs/CYPHT_COMPATIBILITY.md
```

---

## P0-04 SSO 内部地址不能硬编码 localhost

### 问题

Cypht Bridge 内部 SSO 当前使用类似：

```text
http://127.0.0.1:18080/api/v1/auth/sso
```

分容器部署时：

```text
127.0.0.1 = Cypht container
```

不是 Gateway。

### 修改

新增：

```env
GATEWAY_INTERNAL_URL=http://gateway:8080
```

PHP：

```php
$gateway_url = rtrim(
    env('GATEWAY_INTERNAL_URL', 'http://127.0.0.1:18080'),
    '/'
);
```

### 修改文件

```text
cypht-module/gateway/handler_modules.php
docker/cypht-gateway-config.php
deploy/docker-compose.yml
deploy/*.env.example
docs/UNRAID.md
README.md
```

---

## P0-05 `/api/v1/auth/sso` 必须从 Public Edge 隐藏

### 问题

SSO 是：

```text
Cypht → Gateway internal control-plane API
```

不是普通 public REST API。

### Nginx

增加：

```nginx
location = /api/v1/auth/sso {
    return 404;
}
```

Cypht 通过 Docker private network 调用：

```text
http://gateway:8080/api/v1/auth/sso
```

### 长期方案

Gateway 可拆：

```text
public listener
internal listener
```

例如：

```text
0.0.0.0:8080 public
private-net:8081 internal
```

---

## P0-06 SSO Session 创建的 PAT 需要重新设计

### 当前问题

SSO Principal 中：

```rust
Credential {
    username,
    password: ""
}
```

如果这个 Principal 创建 PAT，则 PAT 中持久化空 password。

Cypht Session 过期后：

```text
PAT
→ refresh Cypht session
→ login(username, "")
→ failure
```

### 方案 A：短期

SSO Principal 禁止创建 long-lived PAT：

```text
auth_kind == SSO_SESSION
→ create PAT requires re-auth
```

### 方案 B：正式

新增：

```http
POST /api/v1/auth/reauth
```

返回短生命周期：

```text
reauth_xxx
TTL 5 minutes
```

用于：

- 创建长期 PAT
- 修改 account credential
- 创建 account
- 更新 SMTP/IMAP password
- 高风险 user-config write

---

## P0-07 Sensitive Debug 必须 Redact

### 当前风险

以下类型不应 derive 普通 `Debug`：

```text
Credential
CyphtSession
Principal
AccountSecret
ReauthGrant
OAuthToken
RefreshToken
```

### 修改

自定义：

```rust
impl Debug for Credential {
    // username 可见，password 固定输出 [REDACTED]
}
```

### 同时修改 Vault

使用：

```rust
Zeroizing<Vec<u8>>
```

保护：

- JSON serialize plaintext
- decrypt plaintext
- temporary secret buffers

### 验收

添加测试：

```rust
assert!(!format!("{credential:?}").contains(&password));
assert!(!format!("{principal:?}").contains(&hm_session));
```

---

## P0-08 Audit Failure 不得改变已提交操作的客户端结果

### 当前风险

典型路径：

```text
Cypht write committed
→ SQLite audit failed
→ API returns 500
→ client retries
→ duplicate mutation
```

### 修改原则

区分：

```text
Operation Reliability
≠
Audit Observability
```

对于已经确认提交的 operation：

```text
audit failure
→ operation 仍返回成功
→ tracing error
→ health/metric 暴露 audit degraded
```

更高阶可增加 durable operation journal。

### 所有 write API 自查

- [ ] contact create/update/delete
- [ ] tags create/update/delete
- [ ] saved search create/update/delete
- [ ] calendar write
- [ ] feed write
- [ ] account create/update/delete
- [ ] folder write
- [ ] sieve write

---

# 3. P1：Operation Catalog 单一事实来源

这是 #2115 最值得直接借鉴的设计。

## 3.1 当前问题

当前 Gateway 的能力信息散落在：

```text
api/openapi.yaml
api/capability-registry.yaml
gateway-api routes
gateway-mcp tool schemas
cyphtctl commands
frontend scope list
README/docs
```

长期必然漂移。

## 3.2 建议升级 Capability Registry 为 Operation Catalog

建议定义：

```yaml
version: 2
operations:
  mail.send:
    permission: mail.send
    risk: external_write
    request:
      schema: SendMessageRequest
    response:
      schema: MailWriteResult
    rest:
      method: POST
      path: /api/v1/messages/send
    mcp:
      tool: mail_send
      annotations:
        readOnlyHint: false
        destructiveHint: false
        idempotentHint: false
        openWorldHint: true
    cli:
      command: mail send
    behavior:
      requires_idempotency: true
      requires_account_allowlist: true
      external_network: true
```

## 3.3 自动生成目标

Operation Catalog 应生成或验证：

- [ ] OpenAPI
- [ ] MCP tool schemas
- [ ] Tool annotations
- [ ] CLI help metadata
- [ ] UI scope list
- [ ] docs capability matrix
- [ ] `WRITE_TOOLS`
- [ ] repository consistency check

## 3.4 建议新增 crate

```text
crates/gateway-catalog/
```

或保留 YAML：

```text
api/operation-catalog.yaml
```

Rust build/test 阶段解析并校验。

---

# 4. P1：MCP ToolAnnotations

所有 MCP tool 应补：

```text
readOnlyHint
destructiveHint
idempotentHint
openWorldHint
```

## 建议映射

### read

```text
readOnlyHint=true
destructiveHint=false
idempotentHint=true
openWorldHint=false
```

### network probe

例如：

```text
mail_test_account
```

建议：

```text
readOnlyHint=true
destructiveHint=false
idempotentHint=true
openWorldHint=true
```

### update

```text
readOnlyHint=false
destructiveHint=false
idempotentHint=true
openWorldHint=false
```

### send

```text
readOnlyHint=false
destructiveHint=false
idempotentHint=false
openWorldHint=true
```

### delete

```text
readOnlyHint=false
destructiveHint=true
idempotentHint=true
openWorldHint=false
```

---

# 5. P1：权限模型升级为多层 Intersection

建议最终权限公式：

```text
effective_permission =
AdminPolicy
∩ UserPolicy
∩ IntegrationConnectionPolicy
∩ TokenScope
∩ AccountAllowlist
```

## 5.1 Admin Policy

管理员可规定：

```text
accounts.credentials.write = false
mail.delete = false
mail.send = true
```

任何 user/token 都不能突破。

## 5.2 User Policy

用户在 Settings 中定义：

```text
允许 AI 读取
允许整理
允许草稿
禁止发送
禁止永久删除
```

## 5.3 Connection Policy

每个 AI 客户端进一步缩小：

```text
ChatGPT:
  read=true
  organize=false

Codex:
  read=true
  organize=true
```

Connection 不得扩大 User Policy。

## 5.4 Token Scope

PAT / OAuth access token 继续作为运行时 scope。

## 5.5 Account Allowlist

继续保留 opaque account ID allowlist。

数据库损坏必须 fail closed。

---

# 6. P1：Integration Connection 成为一等资源

新增：

```text
IntegrationConnection
```

示例：

```json
{
  "id": "con_xxx",
  "name": "ChatGPT",
  "kind": "oauth",
  "permissions": ["mail.read", "mail.search"],
  "account_allowlist": ["acc_xxx"],
  "created_at": 0,
  "last_used_at": 0,
  "revoked_at": null
}
```

## REST

```text
GET    /api/v1/connections
POST   /api/v1/connections
GET    /api/v1/connections/{id}
PATCH  /api/v1/connections/{id}
DELETE /api/v1/connections/{id}
```

## UI

Settings 应展示：

```text
ChatGPT
Claude
Codex
Cursor
VS Code
Custom Script
```

每个 connection 单独显示：

- permissions
- allowed accounts
- auth kind
- last used
- revoke
- token rotation status

---

# 7. P1：OAuth 2.1 正式进入路线

## 7.1 支持目标

Remote MCP：

```text
OAuth 2.1 + PKCE S256
```

Local MCP / CLI：

```text
PAT
```

Web UI：

```text
Session
```

## 7.2 建议 AuthKind

```rust
enum AuthKind {
    Session,
    Pat,
    OAuthAccessToken,
    Runner,
}
```

## 7.3 OAuth 必需能力

- [ ] authorization endpoint
- [ ] token endpoint
- [ ] revoke endpoint
- [ ] PKCE S256
- [ ] state
- [ ] redirect URI strict matching
- [ ] short-lived access token
- [ ] rotating refresh token
- [ ] refresh replay revokes connection
- [ ] resource-bound token
- [ ] issuer metadata
- [ ] Dynamic Client Registration（可选）
- [ ] Client ID Metadata Document（可选）
- [ ] connection consent UI
- [ ] account restriction
- [ ] permission restriction

## 7.4 不要复制的部分

OAuth server 应在 Rust Gateway 中实现。

不要把 OAuth state/token/session storage 放进 Cypht PHP。

---

# 8. P1：端到端 Deadline Propagation

这是 #2115 很重要的可靠性提示。

## 8.1 当前问题

Rust request timeout 不等于 IMAP socket timeout。

可能出现：

```text
Rust gives up
↓
PHP worker still blocked
↓
IMAP read still blocked
```

## 8.2 目标

统一：

```text
Gateway deadline
↓
Bridge deadline
↓
IMAP/JMAP/EWS/SMTP network deadline
```

## 8.3 Header

例如内部使用：

```text
X-Cypht-Gateway-Timeout-Ms
```

传“剩余预算”而不是绝对 wall-clock 时间。

## 8.4 Bridge

Bridge 调用前：

```php
$deadline = microtime(true) + ($remaining_ms / 1000);
```

传递到支持的 Cypht protocol client。

## 8.5 默认预算建议

```text
account list          10s
messages list         20s
search                30s
account test          20s
send                  30s
unified inbox         30s
attachment            30s
```

单次 socket read：

```text
min(10s, remaining_budget)
```

## 8.6 测试

必须模拟：

- [ ] server accepts TCP but never replies
- [ ] partial IMAP line
- [ ] truncated literal
- [ ] stalled TLS
- [ ] auth timeout
- [ ] logout timeout
- [ ] response deadline exceeded
- [ ] Bridge request cancellation

---

# 9. P1：Scheduled Send 需要可靠后台执行

## 9.1 自查当前行为

必须确认：

```text
schedule_at
```

究竟是：

```text
Gateway background scheduler
```

还是：

```text
仅写 Cypht X-Schedule
```

如果后者，则浏览器关闭时可能不能按时发送。

## 9.2 建议 Gateway 自己持有 Scheduler

新增表：

```text
gateway_scheduled_jobs
```

字段：

```text
id
owner
message_ref
scheduled_at
status
claim_until
attempts
last_error
created_at
updated_at
```

## 9.3 状态机

```text
pending
  ↓
claimed
  ↓
sending
  ↓
sent
```

异常：

```text
claimed
  ↓ worker crash
claim_until expires
  ↓
pending
```

## 9.4 Exactly-once-ish

每个 send 使用：

```text
Idempotency-Key = scheduled_job_id
```

发送确认后标记 `sent`，不盲目 retry uncertain send。

## 9.5 可选兼容

继续读写 Cypht `X-Schedule`，但 Gateway scheduler 是 reliable executor。

---

# 10. P1：Safe Permanent Delete

删除邮件必须拆语义：

```text
trash
restore
delete_permanent
empty_folder
```

不要一个 `delete` 全包。

## 10.1 UIDPLUS

如果服务器支持 UIDPLUS，应优先精确 UID expunge。

## 10.2 非 UIDPLUS

如果 mailbox 中已有其他 `\Deleted`：

```text
拒绝普通 EXPUNGE
```

返回：

```text
409 unsafe_expunge
```

## 10.3 Gmail

Gmail folder ≈ label。

删除 folder 时需要明确：

```text
delete label
!=
delete messages from All Mail
```

## 10.4 测试矩阵

- [ ] UIDPLUS
- [ ] no UIDPLUS
- [ ] pre-existing deleted message
- [ ] Gmail
- [ ] trash
- [ ] restore
- [ ] delete permanent
- [ ] empty Trash
- [ ] empty Junk

---

# 11. P1：Account CRUD 仍是 Gateway 的关键差异化能力

#2115 覆盖很广，但并没有真正解决：

```text
IMAP Server provisioning
SMTP Server provisioning
Profile lifecycle
Credential rotation
Connection testing
Discovery
```

所以 Accounts v0.6 仍然应该继续。

## 11.1 第一版

支持：

```text
IMAP + SMTP
```

但公共 model 从第一天必须 protocol-neutral：

```rust
enum AccountProtocol {
    Imap,
    Jmap,
    Ews,
}
```

## 11.2 Managed Account Bundle

不要假设：

```text
1 IMAP = 1 SMTP = 1 Profile
```

定义：

```text
ManagedAccountBundle
```

记录：

```text
public_account_id
imap_internal_id
smtp_internal_id
profile_ids[]
generation
revision
managed_by_gateway
retired_at
```

## 11.3 删除前依赖检查

如果 SMTP/Profile 被其他资源使用：

```text
409 account_has_dependencies
```

默认禁止 cascade。

只有确定：

```text
Gateway-owned
AND
unreferenced
```

才自动删除。

---

# 12. P1：MCP 不应直接接收长期明文邮箱密码

## 推荐

UI / CLI / REST：

```text
password
↓
transient secret handoff
↓
sec_xxx
```

MCP：

```text
credential_ref
```

不要让明文 password 长期进入 tool args / agent trace。

## Secret API

```text
POST   /api/v1/account-secrets
DELETE /api/v1/account-secrets/{id}
```

响应：

```json
{
  "id": "sec_xxx",
  "expires_in": 600
}
```

规则：

- [ ] owner-bound
- [ ] encrypted
- [ ] TTL
- [ ] no GET
- [ ] one-time consume
- [ ] audit 不记录 secret
- [ ] MCP 不提供 create-secret tool

---

# 13. P1：OutboundNetworkPolicy 统一化

统一应用到：

- [ ] account discovery
- [ ] IMAP/SMTP test
- [ ] OAuth client metadata
- [ ] attachment remote download
- [ ] feed fetch
- [ ] calendar URL
- [ ] webhook
- [ ] future remote resource

## Policy

必须包括：

```text
scheme allowlist
host allowlist
DNS resolve
check every resolved IP
deny loopback
deny link-local
deny multicast
deny unspecified
deny RFC1918 by default
deny IPv6 ULA by default
redirect revalidation
DNS rebinding mitigation
size limit
time limit
redirect count
```

管理员可配置 private network allowlist：

```env
GATEWAY_OUTBOUND_PRIVATE_CIDRS=
```

AI 不能在 request 中临时 `allow_private=true`。

---

# 14. P1：AI-safe Message Representation

邮件是 untrusted input。

建议 Message response 区分：

```json
{
  "body": {
    "text": "...",
    "html": "..."
  },
  "ai": {
    "text": "...",
    "trust": "untrusted"
  }
}
```

AI text 可做：

- [ ] HTML → text
- [ ] remove hidden HTML
- [ ] remove zero-width/invisible chars
- [ ] tracking pixel metadata removal
- [ ] mark quoted content
- [ ] mark signature
- [ ] max size
- [ ] explicit `untrusted` marker

不要修改或替代原始用户可见邮件，AI-safe 只是额外 representation。

---

# 15. P1：Durable User Config 必须继续严格

上游方案自己承认 stale browser session 可能覆盖 API 对 tags/contacts 的修改。

我们的 Gateway 不应该接受这个限制。

所有 durable write 必须满足：

```text
exclusive lock
↓
fresh load
↓
stale compare
↓
mutation
↓
atomic save
↓
fresh readback
↓
verify expected state
↓
release lock
```

Accounts 创建涉及：

```text
imap_servers
smtp_servers
profiles
```

必须 multi-section transaction。

建议 PHP API：

```text
gateway_begin_write_sections(...)
gateway_commit_write_sections(...)
gateway_abort_write()
```

旧单-section wrapper 保持兼容。

---

# 16. P1：Operation Idempotency 泛化

不要只给 send 系列做 idempotency。

需要覆盖：

- [ ] contacts create
- [ ] tags create
- [ ] saved search create
- [ ] calendar create
- [ ] feed create
- [ ] account create
- [ ] profile create
- [ ] folder create
- [ ] scheduled job create

明确状态，不再用空字符串隐式表示：

```text
pending
completed
failed_before_commit
uncertain
```

Bridge response lost 时，对 create/delete：

```text
禁止自动盲目 replay
```

应 read-after-uncertain → reconcile。

---

# 17. P1：Backend / Host Abstraction

建议 trait：

```rust
trait MailBackend {
    // accounts/mailboxes/messages/search/send/mutate
}

trait AccountProvisioner {
    // create/update/delete/test/discover
}

trait UserConfigBackend {
    // durable transaction
}
```

当前实现：

```text
CyphtBridgeBackend
```

未来：

```text
NativeCyphtAutomationBackend
```

如果上游将来提供稳定 automation API，Gateway 不需要大改 Domain。

---

# 18. P2：统一收件箱性能和 Partial Error

当前 unified inbox 不应串行逐账户等待。

改进：

```text
4-8 accounts bounded concurrency
```

Rust 可使用类似：

```text
buffer_unordered(4)
```

Response 不应静默忽略失败 account：

```json
{
  "messages": [],
  "partial": true,
  "account_errors": [
    {
      "account_id": "acc_x",
      "code": "authentication_failed"
    }
  ]
}
```

---

# 19. P2：PAT last_used 写放大

当前每次 PAT request 更新 `last_used_at` 会制造不必要 WAL 写。

改成节流，例如：

```sql
UPDATE gateway_pats
SET last_used_at=?1
WHERE id=?2
AND (
    last_used_at IS NULL
    OR last_used_at < ?1 - 60
)
```

一分钟更新一次足够。

---

# 20. P2：SQLite Migration 正式版本化

不要只：

```sql
CREATE TABLE IF NOT EXISTS
```

改用：

```text
PRAGMA user_version
```

建议：

```text
crates/gateway-storage/src/migrations.rs
```

事务执行每个 migration。

---

# 21. P2：Activity / Audit Privacy

允许记录：

```text
operation
resource type
resource opaque id
account ids
count
recipient domains
success
error code
latency
```

禁止记录：

```text
message body
subject
full recipients
password
OAuth token
refresh token
raw Cypht error containing secret
```

---

# 22. P2：UI Scope 不再硬编码

新增：

```http
GET /api/v1/meta/capabilities
```

返回：

```json
{
  "scopes": [],
  "permissions": [],
  "default_ai_scopes": [],
  "operations": []
}
```

UI 动态生成 scope / presets。

---

# 23. P2：Scopes 进一步细分

当前 `mail.modify` 长期过粗。

建议迁移到：

```text
mail.read
mail.search
mail.organize
mail.drafts
mail.send
mail.trash
mail.delete
folders.read
folders.write
tags.read
tags.write
contacts.read
contacts.write
accounts.read
accounts.test
accounts.write
accounts.credentials.write
accounts.delete
```

保留一段时间 legacy mapping：

```text
mail.modify → legacy alias
```

---

# 24. P2：Feed / Folder / Sieve 功能补齐

## Folder

新增：

```text
POST   /accounts/{id}/folders
PATCH  /folders/{id}
DELETE /folders/{id}
PUT    /accounts/{id}/special-folders
```

## Sieve

不要只有 status：

```text
GET    /sieve/rules
POST   /sieve/rules
PATCH  /sieve/rules/{id}
DELETE /sieve/rules/{id}

GET    /sieve/scripts
POST   /sieve/scripts
PUT    /sieve/scripts/{id}/activate
```

## Feeds

补：

```text
feed article list/read
feed create/delete
OPML import
```

所有远端 URL 走 `OutboundNetworkPolicy`。

---

# 25. OAuth / Connection 数据模型建议

建议表：

```text
gateway_connections
gateway_oauth_clients
gateway_oauth_codes
gateway_oauth_tokens
gateway_refresh_tokens
gateway_reauth_grants
```

Refresh token 必须：

```text
store hash only
family_id
generation
used_at
revoked_at
```

Replay：

```text
old refresh token reused
→ revoke entire token family
→ revoke connection
```

---

# 26. Encryption Upgrade 可选方案

目前 Gateway master key vault 可以继续使用。

长期可升级：

```text
Master KEK
  ↓ wraps
Connection DEK
  ↓ encrypts
connection secrets
```

好处：

```text
revoke connection
→ destroy wrapped DEK
→ crypto-erasure
```

---

# 27. Rate Limiting

统一限流维度：

```text
source IP
user
connection
token
operation
```

建议起始默认值：

```text
Login:        5/min IP + 5/min username
Account test: 10/min connection, 30/hour user
Send:         30/hour connection, 200/day user
Discovery:    10/min connection
OAuth register: low-frequency
```

具体默认值后续应通过压测和生产反馈调优。

---

# 28. Cypht Brute-force 与 Gateway Login E2E

Cypht 2.13 brute-force 使用 `REMOTE_ADDR` + username 计数。

分容器时所有 Gateway login 可能从同一个 Gateway container IP 到达 Cypht。

必须测试：

```text
连续错误密码
→ 是否锁整个 Gateway container IP
```

如果确认：

- Gateway 自己执行真实 client IP + username rate limit
- 内部 API-login 不应因共享 Gateway IP 锁死所有用户
- username lock 继续保留
- 不直接信任公网 `X-Forwarded-For`

---

# 29. Release Status 重新定义

不要一个 `production ready` 覆盖所有含义。

建议：

```text
Implementation Status
CI Validation
Live Integration Validation
Production Validation
```

示例：

```yaml
accounts:
  implementation: complete
  ci: complete
  live_integration: partial
  production_validation: not_validated
```

---

# 30. 建议版本路线

## v0.5.x Stabilization

- [ ] CI build fix
- [ ] Cypht 2.13 baseline
- [ ] capability handshake
- [ ] SSO internal URL
- [ ] hide SSO on edge
- [ ] SSO reauth
- [ ] sensitive Debug redact
- [ ] audit false-failure fix
- [ ] PAT write throttle
- [ ] request deadline foundation

## v0.6 Operation Catalog + Accounts

- [ ] operation catalog
- [ ] generated/validated MCP/OpenAPI
- [ ] ToolAnnotations
- [ ] account binding
- [ ] IMAP CRUD
- [ ] SMTP CRUD
- [ ] Profile CRUD
- [ ] account test
- [ ] discovery
- [ ] credential handoff
- [ ] account dependency graph
- [ ] multi-section durable transaction
- [ ] account idempotency states

## v0.7 OAuth + Connections

- [ ] IntegrationConnection
- [ ] User Policy
- [ ] OAuth 2.1
- [ ] PKCE
- [ ] refresh rotation
- [ ] consent UI
- [ ] connection-specific account restrictions
- [ ] connection activity log

## v0.8 Mail Organization

- [ ] Folder CRUD
- [ ] special folders
- [ ] safe trash
- [ ] safe permanent delete
- [ ] restore
- [ ] Sieve CRUD
- [ ] scope migration from mail.modify

## v0.9 Scheduler

- [ ] reliable Gateway scheduler
- [ ] scheduled queue
- [ ] cancel
- [ ] reschedule
- [ ] claim lease
- [ ] crash recovery
- [ ] send idempotency

## v0.10 Protocol Expansion

- [ ] JMAP provisioning
- [ ] EWS provisioning
- [ ] OAuth mail providers
- [ ] advanced search
- [ ] feed full parity
- [ ] CardDAV/Gmail/LDAP contact adapters

---

# 31. 建议代码目录调整

```text
crates/
  gateway-catalog/
    src/lib.rs

  gateway-domain/
    src/
      accounts.rs
      connections.rs
      oauth.rs
      scheduler.rs
      network_policy.rs
      deadlines.rs
      operation_journal.rs

  gateway-cypht/
    src/
      backend.rs
      accounts.rs
      mail.rs
      user_config.rs
      capabilities.rs

  gateway-api/
    src/
      accounts.rs
      connections.rs
      oauth.rs
      meta.rs

  gateway-mcp/
    src/
      generated_tools.rs
      transport.rs
```

---

# 32. Repository 自查脚本应增加的规则

`scripts/check-repository.py` 增加：

- [ ] every MCP tool exists in Operation Catalog
- [ ] every public REST mutation exists in Operation Catalog
- [ ] OpenAPI scopes == auth scopes == catalog scopes
- [ ] write tools annotations correct
- [ ] destructive tools have `destructiveHint=true`
- [ ] external side effect tools have `openWorldHint=true`
- [ ] secret/password fields never appear in MCP schemas
- [ ] `/auth/sso` not reachable via edge
- [ ] Cypht Docker baseline belongs to tested matrix
- [ ] capability handshake version exists
- [ ] default AI scopes contain no write scope
- [ ] no unsafe `Debug` on secret-bearing types
- [ ] no `unwrap_or_default()` on security-sensitive DB JSON
- [ ] all create mutation capabilities declare idempotency behavior
- [ ] version docs consistent

---

# 33. Tests：必须新增

## Security

- [ ] malformed PAT scopes fail closed
- [ ] malformed allowlist fail closed
- [ ] restricted connection cannot broaden permission
- [ ] restricted user cannot create broader token
- [ ] admin-disabled permission cannot be enabled by user
- [ ] MCP schemas do not expose password field
- [ ] secret TTL
- [ ] secret ownership
- [ ] secret one-time consumption
- [ ] OAuth refresh replay
- [ ] OAuth redirect mismatch
- [ ] PKCE mismatch
- [ ] SSO endpoint blocked through public edge

## Reliability

- [ ] Bridge timeout before commit
- [ ] Bridge timeout after commit
- [ ] audit DB unavailable after successful Cypht write
- [ ] SQLite busy
- [ ] process crash during scheduled claim
- [ ] duplicate scheduler workers
- [ ] idempotency pending
- [ ] idempotency uncertain
- [ ] readback mismatch
- [ ] stale user-config writer

## Network

- [ ] loopback reject
- [ ] RFC1918 reject
- [ ] IPv6 ULA reject
- [ ] DNS multiple IP check
- [ ] redirect to private IP reject
- [ ] DNS rebinding scenario
- [ ] oversized remote file
- [ ] slow remote file
- [ ] IMAP silent server
- [ ] SMTP silent server

## Mail Delete

- [ ] UIDPLUS exact delete
- [ ] non-UIDPLUS safe case
- [ ] non-UIDPLUS unsafe expunge refused
- [ ] Gmail label delete semantics
- [ ] trash / restore
- [ ] empty folder permission

## Account CRUD

- [ ] create IMAP only
- [ ] create IMAP+SMTP+Profile
- [ ] failed IMAP test -> no save
- [ ] failed SMTP test -> no save
- [ ] multi-section commit
- [ ] rollback
- [ ] update password
- [ ] update host
- [ ] dependency conflict delete
- [ ] retire public account ID
- [ ] internal ID reuse cannot revive old public ID
- [ ] old PAT allowlist cannot gain access to replacement account

---

# 34. Performance Gate

建议 release gate：

```text
GET /accounts           p95 < 150ms excluding upstream probes
GET /meta/capabilities  p95 < 50ms
OAuth token validation  p95 < 20ms local DB
Account test            hard deadline <= 20s
Search                  hard deadline <= 30s
Unified inbox           bounded concurrency
```

禁止 `GET /accounts` 自动做网络 health probe，Health 使用 cache。

---

# 35. 最终目标架构

```text
                         ┌────────────────────┐
                         │ Operation Catalog  │
                         └─────────┬──────────┘
                                   │
              ┌────────────────────┼─────────────────────┐
              │                    │                     │
             REST                 MCP                   CLI
              │                    │                     │
              └────────────────────┴─────────────────────┘
                                   │
                            GatewayService
                                   │
           ┌───────────────────────┼────────────────────────┐
           │                       │                        │
         Auth                    Policy                 Operation Journal
   Session/PAT/OAuth      Admin/User/Connection        Idempotency/Uncertain
           │                       │                        │
           ├───────────────────────┼────────────────────────┤
           │                                                │
      Secret Vault                                    Scheduler
           │                                                │
           └───────────────────────┬────────────────────────┘
                                   │
                              MailBackend
                                   │
                    ┌──────────────┴──────────────┐
                    │                             │
             CyphtBridgeBackend          NativeCyphtBackend
                    │                             │
                    └──────────────┬──────────────┘
                                   │
                                  Cypht
                                   │
                        IMAP / JMAP / EWS / SMTP
```

---

# 36. PR 拆分建议

## PR 1 — Build / Baseline

```text
fix(ci): restore gateway-cypht build
chore(cypht): move tested baseline to 2.13.0
```

## PR 2 — Internal SSO

```text
fix(auth): make Cypht SSO endpoint internal and configurable
```

## PR 3 — Secret Hygiene

```text
security(auth): redact credential/session Debug and zeroize plaintext buffers
```

## PR 4 — Mutation Reliability

```text
fix(domain): decouple audit failures from committed mutations
feat(storage): explicit idempotency states
```

## PR 5 — Capability Handshake

```text
feat(bridge): negotiate bridge capabilities instead of trusting version strings
```

## PR 6 — Operation Catalog

```text
feat(catalog): unify REST MCP CLI capability definitions
```

## PR 7 — Deadline Propagation

```text
feat(cypht): enforce end-to-end protocol request deadlines
```

## PR 8 — Accounts

```text
feat(accounts): add managed IMAP SMTP profile lifecycle
```

## PR 9 — OAuth / Connections

```text
feat(auth): add OAuth connections with PKCE and refresh rotation
```

## PR 10 — Folders / Safe Delete

```text
feat(mail): add folder lifecycle and safe permanent deletion
```

## PR 11 — Scheduler

```text
feat(scheduler): add reliable scheduled mail execution
```

---

# 37. Definition of Done

一个新 capability 只有满足全部条件才算完成：

- [ ] Core model
- [ ] Domain implementation
- [ ] Auth scope
- [ ] account allowlist behavior
- [ ] Operation Catalog
- [ ] REST endpoint
- [ ] OpenAPI
- [ ] MCP tool
- [ ] ToolAnnotations
- [ ] CLI
- [ ] audit
- [ ] idempotency if mutation
- [ ] rate limit if external/network
- [ ] timeout/deadline
- [ ] tests
- [ ] negative tests
- [ ] docs
- [ ] CI
- [ ] live Bridge validation
- [ ] release status updated

---

# 38. 最后自查问题

每次准备 merge / release 前逐项回答。

## Architecture

- [ ] 这项能力是否仍让 Cypht 做 protocol engine？
- [ ] 是否在 Rust 里重复实现 IMAP/JMAP/EWS/SMTP？
- [ ] Domain 是否依赖 transport？
- [ ] MCP 是否仍只是 canonical operation 的一个入口？

## Security

- [ ] AI 是否看到不必要的 password/token？
- [ ] scope 是否可能 fail-open？
- [ ] account allowlist 是否可能因损坏扩大？
- [ ] outbound URL 是否经过统一 policy？
- [ ] 日志是否可能包含 secret？
- [ ] Public edge 是否暴露 internal endpoint？

## Reliability

- [ ] mutation 的 commit point 在哪里？
- [ ] response 丢失时是什么状态？
- [ ] client retry 是否安全？
- [ ] audit 失败会不会制造 false failure？
- [ ] protocol socket 是否有 hard deadline？
- [ ] stale session 是否可能覆盖 durable write？

## AI / MCP

- [ ] ToolAnnotations 是否正确？
- [ ] write tool 是否默认隐藏？
- [ ] destructive tool 是否要求 explicit permission？
- [ ] output 是否明确 untrusted mail content？
- [ ] schema 是否 bounded？
- [ ] 是否支持 partial failures 而不是静默丢弃？

## Cypht Compatibility

- [ ] 依赖了哪些 Cypht internal structures？
- [ ] capability handshake 是否覆盖？
- [ ] 新 Cypht baseline 是否经过 E2E？
- [ ] 未知版本的 write 是否默认 fail closed？

---

# 39. 一句话方向

`cypht-gateway` 后续最有价值的方向不是和 Cypht 原生 `modules/mcp` 比谁“工具更多”，而是成为：

> **一个稳定、可审计、可扩展、可嵌入、支持多种认证和多种 Cypht backend 的 Automation Control Plane。**

上游 #2114/#2115 可以借鉴它的：

```text
Catalog
OAuth
Connection Permissions
Timeouts
Scheduler
Safe Delete
SSRF
Activity Privacy
```

但 Gateway 应继续保留自身优势：

```text
Rust control plane
strict durable writes
separate security boundary
low-invasion Cypht integration
multi-client API
strong idempotency
explicit uncertain-state handling
future backend abstraction
```

这应作为 v0.6+ 的主线。

---

# 40. 参考

- Cypht issue #2114 — Optional MCP server and REST API with OAuth and per-connection permissions
- Cypht PR #2115 — `modules/mcp` implementation
- Cypht issue #2108 — Gateway proposal
- Cypht PR #2109 — Gateway module proposal

> 注意：#2115 在本计划生成时仍为 open / 未合并，因此应作为设计参考与兼容目标，而不是已稳定的上游 API 合同。
