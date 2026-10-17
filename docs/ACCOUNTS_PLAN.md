# Cypht Gateway 账户配置与无版本限制实施计划 (v0.6.0)

> **目标仓库**：`ashanzzz/cypht-gateway`  
> **基准分支**：`main`  
> **规划版本**：`v0.6.0-dev` / `v0.6.0`  
> **定位**：在遵循现有工程规范与安全铁律的前提下，彻底解除上游 Cypht 版本硬编码死锁，并为 AI (MCP/REST/CLI) 赋予极速、便捷、安全的 IMAP/SMTP 账户全生命周期管理能力。

---

## 1. 核心架构设计与优化原则

针对原有 71 节规划方案中的过度设计，本次落地计划进行了针对性精简与强化：

### 1.1 彻底解除上游 Cypht 版本锁定（Dynamic Upstream Compatibility）
- **原问题**：网关在 `crates/gateway-cypht/src/lib.rs` 中硬编码拦截 `ping.cypht_version != "2.12.0"`，导致上游 Cypht 一旦更新至 2.12.2、2.13.x 或最新镜像，网关直接拒绝工作。
- **新设计**：
  1. 移除 `validate_ping_versions` 中的版本强等比对报错。
  2. 采用**动态跟随与日志自省机制**：在 Bridge Ping 时记录上游实际版本（`tracing::info!`），允许任意上游原版版本即插即用。
  3. 兼容性判断以功能能力（如 `durable_user_config` 标志）为准，而非固定的版本字符串。

### 1.2 AI 友好的单步原子创建（Single-Shot Unified Setup）
- **原问题**：原有方案设计了复杂的两步“Secret Handoff”（先调用 Secret 接口换取临时 Token，再调用 Account 接口传 Token），并拆解了 IMAP、SMTP 和 Profile 的多步关联，对 AI Agent 和 MCP 极不友好，增加网络开销与超时隐患。
- **新设计**：
  1. 提供聚合型 API 与 MCP 工具：`mail_add_account`。
  2. 单次调用直接传入 `{ name, email, password, imap, smtp }`。
  3. **凭据安全铁律（Write-Only & In-Memory Zeroize）**：密码字段在 Rust 内存中即时擦除，严禁进入审计日志、严禁回显在 API 响应或 MCP 上下文中。
  4. **连接预检（Fail-Fast Verification）**：在正式写入配置前，PHP Bridge 先调用 `Hm_IMAP::connect` 与 `Hm_SMTP::connect` 进行真机握手测试。若认证失败直接返回 422 及清晰错误原因，防止 AI 保存无效配置。

### 1.3 修复已知的 P0 级提权隐患（PAT Allowlist Fail-Closed）
- **原问题**：`crates/gateway-storage/src/lib.rs` 中，`account_allowlist` 解析异常时使用 `unwrap_or_default()` 降级为空数组 `[]`，而在网关语义中空数组代表“允许访问所有账户”，导致受限 Token 被越权放大为全权 Token。
- **新设计**：
  1. 增加严格 JSON 转换函数 `json_column`，禁止静默容错。
  2. 反序列化异常时立即返回数据库错误，确保**严格安全失效（Fail-Closed）**。

### 1.4 多配置节原子事务（Multi-Section Durable Transaction）
- **设计要点**：Cypht 底层将收信（`imap_servers`）、发信（`smtp_servers`）和发信身份（`profiles`）分开存储。Bridge 层采用快照锁（Snapshot Lock）机制，在单次文件写锁生命周期内同时写入或回滚三个配置节，避免配置孤岛。

### 1.5 账户安全绑定与防 ID 复用（Account Binding & ID Retirement）
- **设计要点**：Cypht 内部使用自增整数作为服务器 ID。网关为每个外部公开账户生成全局唯一的 Opaque UUID，在本地 SQLite 中持久化映射关系。一旦账户被删除，该 Opaque ID 永久退役（Retired），防止 Cypht 内部自增 ID 复用时导致旧 Token 越权访问新账户。

---

## 2. API 与 MCP 能力契约规范

### 2.1 REST API v1 (`/api/v1/accounts`)

| 路径与方法 | 对应 Scope | 风险级别 | 说明 |
| :--- | :--- | :--- | :--- |
| `GET /api/v1/accounts` | `accounts.read` | read | 列表查询可见账户（脱敏，返回 Opaque ID） |
| `GET /api/v1/accounts/{id}` | `accounts.read` | read | 查询单个账户详情及连接状态（脱敏） |
| `POST /api/v1/accounts` | `accounts.write` | write | 一体化新增账户（含连接测试与原子提交） |
| `PATCH /api/v1/accounts/{id}` | `accounts.write` | write | 修改服务器参数或更新密码 |
| `DELETE /api/v1/accounts/{id}` | `accounts.write` | destructive | 删除账户（同步级联清理 IMAP、SMTP 和 Profile） |
| `POST /api/v1/accounts/{id}/test` | `accounts.write` | read | 手动触发服务器连通性测试 |

### 2.2 MCP 工具契约

1. **`mail_list_accounts`**（已有只读工具）：
   - 查询当前所有可见账户信息。
2. **`mail_get_account`**：
   - 依据 `account_id` 查询单个账户配置与健康状态。
3. **`mail_add_account`**（写操作，需 `--allow-write`）：
   - 入参：`email` (string), `password` (string), `name` (string), `imap_host` (string), `imap_port` (int), `smtp_host` (string), `smtp_port` (int) 等。
   - 具备内置预检与自动生成默认 Profile。
4. **`mail_update_account`**（写操作，需 `--allow-write`）：
   - 入参：`account_id` (string), 可选更新的字段。
5. **`mail_delete_account`**（破坏性操作，需 `--allow-write`）：
   - 入参：`account_id` (string), `confirm` (bool，必须为 true)。

---

## 3. GitHub 标准化实施路线图

### Milestone 1：版本解除与安全基线加固 (已就绪)
- [x] 解除 `crates/gateway-cypht/src/lib.rs` 中对 Cypht 2.12.0 的死锁拦截，支持任意原版版本。
- [x] 修复 `crates/gateway-storage/src/lib.rs` 中 PAT allowlist / scopes JSON 损坏导致全权提权的 P0 漏洞。
- [x] 更新 `scripts/check-repository.py`，提高仓库扫描效率并支持动态上游版本。
- [x] 更新 `AGENTS.md` 契约，明确动态跟随上游 Cypht 原版。

### Milestone 2：PHP Bridge 事务与账户适配
- [ ] 在 `cypht-module/gateway/` 中实现 `account_handlers.php`。
- [ ] 编写真机连接校验逻辑（`Hm_IMAP::connect` / `Hm_SMTP::connect`）。
- [ ] 实现 `gateway_commit_durable_user_config` 多节原子写入与级联删除。
- [ ] 编写 Bridge 单元测试覆盖创建、冲突与删除。

### Milestone 3：Gateway Domain 与 REST v1
- [ ] 在 `crates/gateway-core` 定义 `AccountCreateRequest`, `AccountUpdateRequest`。
- [ ] 在 `crates/gateway-domain` 实现账户管理服务，接入 `accounts.write` 权限与脱敏审计。
- [ ] 在 `crates/gateway-api` 暴露 `/api/v1/accounts` 完整 CRUD 端点。
- [ ] 更新 `api/openapi.yaml` 和 `api/capability-registry.yaml`。

### Milestone 4：MCP 工具与 CLI 对齐
- [ ] 在 `crates/gateway-mcp` 注册 `mail_add_account`, `mail_update_account`, `mail_delete_account`。
- [ ] 将写工具受控纳入 `WRITE_TOOLS` 白名单。
- [ ] 在 `apps/cyphtctl` 中增加 `cyphtctl account-create`, `cyphtctl account-delete` 命令。