# Cypht Gateway 账户配置与生产级架构实施计划 (v0.6.0)

> **目标仓库**：`ashanzzz/cypht-gateway`  
> **基准分支**：`main`  
> **生产基线**：Cypht `v2.13.0` (兼容 `v2.12.x`)  
> **规划版本**：`v0.6.0`  
> **定位**：在遵循现有工程规范与安全铁律的前提下，全面升级 Cypht 生产基线，完善 Capability Handshake 契约，并为 AI (MCP/REST/CLI) 赋予稳健、安全、防凭据泄漏的 IMAP/SMTP 账户全生命周期管理能力。

---

## 1. 核心架构设计与优化原则

针对前序版本的实现偏差及深度评审意见，本实施计划全面落实以下关键安全与架构设计：

### 1.1 升级 Cypht 生产基线至 v2.13.0 并实行 Capability Handshake
- **生产基线升级**：将推荐生产镜像由 2.12.0 升级至官方最新稳定版 **Cypht v2.13.0**（2026-10-16 发布，包含 SMTP 附件路径、chunk 用户隔离及 Tag 作用域等安全补丁）。
- **能力握手契约（Capability Handshake）**：
  - 杜绝“版本号完全硬锁定”和“任何版本完全盲从”两个极端。
  - Bridge Ping 返回结构化能力清单（`capabilities: [...]`）与 `bridge_api_version`。
  - Gateway 根据能力声明决定功能开放；对于未验证的上游版本，只读能力正常放行，写配置能力默认保持安全锁定，除非声明具备 `user_config.durable.v1` 或显式启用 `CYPHT_ALLOW_UNTESTED_WRITES=true`。

### 1.2 凭据生命周期与 MCP 泄露防护（Secret Reference）
- **核心风险**：即使 Rust 内存中对密码即时执行 `Zeroize`，只要在 MCP Tool Schema 中暴露 `password`，密码就必然暴露在 **AI 对话上下文、MCP 客户端日志、Agent 运行追踪及 Telemetry** 中。
- **分层安全方案**：
  1. **REST / CLI / UI**：支持直接输入密码，CLI 通过安全输入（stdin / 无回显），内存即时擦除并对入参脱敏。
  2. **MCP 工具层**：Schema 禁止出现明文密码字段，改用凭据引用：
     ```json
     {
       "credential_ref": "sec_xxxx",
       "email": "user@company.com",
       "imap_host": "imap.company.com"
     }
     ```
  3. **透明短时换票**：UI / 客户端可在前端透明完成短时 `POST /api/v1/accounts/secrets` 取得 `sec_xxx` 引用，AI Agent 全程仅接触引用标识符，彻底封堵大模型上下文泄露凭据的通道。

### 1.3 依赖感知的账户删除模型（Dependency-Checked Account Bundles）
- **禁止无条件级联删除**：Cypht 的底层模型中，IMAP、SMTP 和 Profile 并不天然是一对一关系。多个账户或 Profile 完全可能共用一个企业 SMTP 发信服务器。
- **Managed Account Bundle**：
  - Gateway 在本地 SQLite 记录 `ManagedAccountBundle`（包含绑定的 imap_id, smtp_id, profile_ids 以及是否为 Gateway 托管标志）。
  - 执行删除时，先进行**引用依赖检测**：若底层关联的 SMTP 或 Profile 仍被其他活跃账户或 Profile 依赖，禁止删除并返回 `409 Conflict: account_has_dependencies`。
  - 仅当明确证明关联资源为当前账户独占时，才执行安全的级联清理。

### 1.4 修复已知的 P0 漏洞与安全加固
- **PAT Allowlist Fail-Closed**：彻底修复 `crates/gateway-storage/src/lib.rs` 中 `account_allowlist_json` 反序列化失败降级为 `[]`（导致全权提权）的漏洞，采用严格 `json_column` 报错安全失效。
- **敏感对象 Debug 脱敏**：为 `Credential` 和 `CyphtSession` 实现自定义 `std::fmt::Debug`，强制以 `[REDACTED]` 隐藏密码与 Session Token，防止打 Log 泄漏。
- **PAT Last-Used 高频写节流**：对 `touch_pat` 实施 60 秒限流，避免高频轮询引发的 SQLite WAL 高频写。
- **审计失败不掩盖业务成功**：解耦 `audit_write` 与业务返回值，记录 warning 告警，避免底层审计写入偶发异常导致已经成功的邮件或联系人操作返回假 500 进而引发客户端恶意重试。

### 1.5 原生 SSO 部署 Bug 修复与边缘隐藏
- **动态解析 Gateway 地址**：PHP Bridge 中通过 `GATEWAY_INTERNAL_URL` 动态解析内部 Gateway 地址，解决容器网络下写死 `127.0.0.1:18080` 导致跨容器 SSO 失败的问题。
- **Edge 边缘隐藏**：在 Nginx 反向代理中对 `/api/v1/auth/sso` 配置 `return 404;`，杜绝内部交换凭据端点暴露至公网端口。

---

## 2. API 与 MCP 契约规划

### 2.1 账户与凭据端点 (`/api/v1`)

| 路径与方法 | 对应 Scope | 风险级别 | 说明 |
| :--- | :--- | :--- | :--- |
| `POST /api/v1/accounts/secrets` | `accounts.write` | credential | 提交临时敏感凭据，返回短时 `credential_ref` (300s 有效) |
| `GET /api/v1/accounts` | `accounts.read` | read | 列表查询可见账户（脱敏，返回 Opaque ID） |
| `GET /api/v1/accounts/{id}` | `accounts.read` | read | 查询单个账户详情及健康状态（脱敏） |
| `POST /api/v1/accounts` | `accounts.write` | write | 一体化新增账户（支持 direct password 或 credential_ref） |
| `PATCH /api/v1/accounts/{id}` | `accounts.write` | write | 修改服务器配置或更新凭据引用 |
| `DELETE /api/v1/accounts/{id}` | `accounts.write` | destructive | 依赖感知删除（支持 `force=true` 参数显式解绑） |
| `POST /api/v1/accounts/{id}/test` | `accounts.write` | read | 手动触发服务器真实连通性测试 |

### 2.2 MCP 工具契约

1. **`mail_list_accounts`**：查询所有可用邮件账户。
2. **`mail_get_account`**：依据 `account_id` 查询单个账户配置。
3. **`mail_add_account`**（写操作，需 `--allow-write`）：
   - 入参：`email`, `name`, `imap_host`, `imap_port`, `smtp_host`, `smtp_port`, `credential_ref` (引用短时凭据，不出现 raw password)。
4. **`mail_update_account`**（写操作，需 `--allow-write`）：
   - 入参：`account_id`, `credential_ref` (更新密码时使用), 及其他可选配置。
5. **`mail_delete_account`**（破坏性操作，需 `--allow-write`）：
   - 入参：`account_id`, `confirm: true`, 可选 `force: false`。

---

## 3. 标准化实施里程碑 (Milestones)

### Milestone 1：基础设施加固与版本基准对齐 (已完成)
- [x] 修复 `crates/gateway-cypht/Cargo.toml` 缺少 `tracing.workspace` 导致的 CI 编译中断。
- [x] 将生产 Dockerfile 基准升级为 `cypht/cypht:2.13.0`。
- [x] 引入 Bridge 能力清单握手（`capabilities`）。
- [x] 修复 Native SSO 动态内部寻址（`GATEWAY_INTERNAL_URL`）与 Edge 端口屏蔽（`/api/v1/auth/sso`）。
- [x] 敏感凭据对象实现自定义 `[REDACTED]` Debug 脱敏。
- [x] SQLite `touch_pat` 实施 60s 节流优化。
- [x] 解耦 `audit_write`，杜绝审计偶发故障引发误导性客户端重试。

### Milestone 2：PHP Bridge 账户适配与多配置节事务
- [ ] 在 `cypht-module/gateway/` 中实现 `account_handlers.php`。
- [ ] 编写真机连接校验逻辑（`Hm_IMAP::connect` / `Hm_SMTP::connect`）。
- [ ] 实现基于快照锁的多配置节（IMAP + SMTP + Profiles）原子提交与事务回滚。
- [ ] 实现引用依赖检测与独立删除保护逻辑。

### Milestone 3：Domain 账户服务与 REST v1
- [ ] 在 `crates/gateway-core` 定义协议中立模型与 `ManagedAccountBundle`。
- [ ] 在 `crates/gateway-domain` 实现短时凭据引用存储（`SecretVault`）与账户绑定。
- [ ] 在 `crates/gateway-api` 暴露 `/api/v1/accounts` 与 `/api/v1/accounts/secrets` 端点。
- [ ] 更新 `api/openapi.yaml` 和 `api/capability-registry.yaml`。

### Milestone 4：MCP 工具与 CLI 工具对齐
- [ ] 在 `crates/gateway-mcp` 注册不含明文密码字段的 `mail_add_account` 等工具。
- [ ] 将新写工具受控纳入 `WRITE_TOOLS` 白名单。
- [ ] 在 `apps/cyphtctl` 中增加 `account-create`, `account-delete`, `account-test` 命令。