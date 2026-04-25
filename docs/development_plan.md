# Agora 功能完善实施计划

## Context
当前代码仓库的主干能力已具备（路由、OAuth/JWT、定时任务、DB 迁移），但与 README 描述的“完整后端能力”存在关键落差：健康检查端点缺失、本地登录闭环不完整、RBAC 仅鉴权未授权、定时任务配置与运行态闭环不足、关键安全路径测试覆盖薄弱。本次变更目标是先达成可稳定上线的最小可交付，再收敛一致性问题并提升可维护性。

## 推荐实施方案（单一路径）

### P0（先落地，满足最小可交付）
1. 补齐健康检查端点 `GET /health`
   - 修改文件：
     - `web_services/src/startup.rs`
     - `web_services/src/routes/mod.rs`
     - `web_services/src/routes/index.rs`（若复用/新增 handler）
   - 复用点：沿用 `startup.rs` 的路由注册模式。
   - 实现要点：公开路由返回轻量 200 JSON，不依赖重 DB 检查。

2. 补齐本地登录（当前 login 模块仅 logout）
   - 修改文件：
     - `web_services/src/routes/login/post.rs`
     - `web_services/src/startup.rs`
     - `infra/src/db/postgresql/user_info_table.rs`
   - 复用点：
     - token 签发与 cookie 策略复用 `web_services/src/routes/auth/token.rs`
     - 认证链路复用现有 `user_info_table` 查询能力。
   - 实现要点：支持账号/密码登录，统一错误响应，不泄露账号存在性。

3. RBAC 从“仅 JWT 鉴权”升级为“鉴权 + 授权”
   - 修改文件：
     - `web_services/src/middleware/auth_middleware.rs`
     - `web_services/src/routes/api/me.rs`
     - `web_services/src/routes/admin/scheduled_task_reload.rs`
   - 复用点：沿用中间件注入 claims 机制，并从现有 role/permission 表聚合权限。
   - 实现要点：
     - 管理端点加权限校验（至少 admin）
     - `/api/me` 返回真实 permissions（不再 `vec![]`）
     - 明确 401/403 边界。

4. 修复 `/api/sync/task_source` 写库后不生效
   - 修改文件：
     - `web_services/src/routes/api/sync/task_source.rs`
     - `web_services/src/task_manage.rs`
   - 复用点：对齐现有 scheduled_tasks create/update/toggle/delete 的 `refresh_config()` 模式。
   - 实现要点：写库成功后立即 refresh，失败返回明确错误并记录日志。

5. 回写 scheduled_tasks 运行态字段
   - 修改文件：
     - `timer_tasker/src/scheduler.rs`
     - `web_services/src/task_manage.rs`
     - `infra/src/db/postgresql/scheduled_tasks.rs`
   - 复用点：调度执行沿用 `scheduler`，状态落库集中在 `task_manage`/repository。
   - 实现要点：更新 `last_run` / `next_run` / `last_status`，保证 API 可观测。

6. 补关键测试并纳入 CI 门禁
   - 修改文件：
     - `.github/workflows/rust-ci.yml`
     - `web_services` 相关测试文件（新增/补充）
   - 复用点：沿用现有 CI 的 fmt/clippy/test 流程。
   - 实现要点：最小覆盖登录、鉴权/授权、task_source 生效、任务状态回写。

### P1（稳定后收敛一致性）
1. 修正 `tenant_id` 类型与 DB 一致
   - 修改文件：
     - `common/src/po.rs`
     - `common/src/dto.rs`
     - `infra/src/db/postgresql/user_info_table.rs`
   - 实现要点：统一 `tenant_id` 类型为与 schema 一致，补映射测试。

2. 修复 `/api/sync/me` SQL 调用方式
   - 修改文件：
     - `web_services/src/routes/api/sync/post.rs`
   - 实现要点：`query(...).execute(...)` 或补 `RETURNING` 后 `query_as`，二者择一并统一。

3. 对齐文档与真实 API
   - 修改文件：
     - `README.md`
     - `docs/api.md`
     - `docs/github_oauth2.md`
   - 实现要点：以 `startup.rs` 实际注册路由为准修订方法/路径/文件名。

### P2（可维护性增强）
1. 统一定时任务 `params` 协议（arg/url/cmd）
   - 修改文件：
     - `web_services/src/routes/api/sync/task_source.rs`
     - `web_services/src/task_manage.rs`
     - `timer_tasker/src/task.rs`
     - `service/src/timer_task_command.rs`
   - 实现要点：定义统一 schema；读取层做向后兼容（旧字段兜底，新字段优先）。

## 验证与验收
1. 编译与静态检查
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`

2. 端到端关键链路（本地运行 `cargo run -p web_services`）
   - `GET /health` 返回 200
   - 注册/登录/刷新/登出闭环可用
   - 普通用户访问 admin 受限接口返回 403
   - `/api/me` 返回真实 permissions
   - 调用 `/api/sync/task_source` 后任务配置即时生效
   - `/api/scheduledTasks` 中运行态字段随执行更新

3. 回归检查
   - OAuth 登录流程不受影响（开启 OAuth 配置时）
   - 现有 scheduled_tasks create/update/toggle/delete 行为保持兼容

## 交付顺序
- 先提交 P0（单次可上线）
- 再按 P1、P2 递进，避免一次性大改带来联调风险
