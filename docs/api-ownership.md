# API Ownership Registry（Agora × Keystone）

> 目标：明确每条 API 的唯一责任归属（Single Writer），杜绝双主写。  
> 原则：执行面在 Agora，管理面在 Keystone。跨系统协作只走契约，不走隐式耦合。

## 1. 责任边界总则

- **Agora（执行面）**
  - 调度执行（cron、重试、并发控制）
  - 采集/处理链路执行
  - 运行态与可观测（last_run/next_run/last_status）
  - 实时推送（SSE）
- **Keystone（控制面）**
  - 身份认证（登录、token 刷新）
  - 用户/角色/权限（RBAC）
  - 后台管理配置（运营配置、策略配置、审计）
  - 通用管理 API（可复用于多业务）

### 强约束

1. 单一写主：每类资源只能有一个系统负责写。
2. 另一侧如需操作：通过主系统 API 调用，不允许直写对方主数据。
3. 新增/变更 API 必须先更新本文件再开发。

## 2. 状态定义

- `owned`：已确定最终归属，不迁移。
- `migrate-planned`：已确定迁移目标，待实施。
- `migrating`：双边改造中（受控阶段）。
- `done`：已完成迁移，旧入口已只读/下线。

## 3. 路由级归属清单（首版）

| Domain | Method | Path | Current Owner | Target Owner | Writer | Batch | Status | Notes |
|---|---|---|---|---|---|---|---|---|
| health | GET | `/health` | Agora | Agora | Agora | P0 | owned | 服务健康检查，执行面基础能力 |
| auth | POST | `/register` | Agora | Keystone | Keystone | P1 | migrate-planned | 通用账号体系应统一到 Keystone |
| auth | POST | `/logout` | Agora | Keystone | Keystone | P1 | migrate-planned | 会话与令牌生命周期统一管理 |
| auth | POST | `/auth/token/refresh` | Agora | Keystone | Keystone | P1 | migrate-planned | token 刷新口径统一 |
| auth | GET | `/auth/oauth/github/login` | Agora | Keystone | Keystone | P1 | migrate-planned | 三方登录作为平台能力 |
| auth | GET | `/auth/oauth/github/callback` | Agora | Keystone | Keystone | P1 | migrate-planned | 三方登录回调统一治理 |
| profile | GET | `/api/me` | Agora | Keystone | Keystone | P1 | migrate-planned | 用户身份与权限数据由 Keystone 主导 |
| user-sync | GET | `/api/sync/me` | Agora | Keystone | Keystone | P1 | migrate-planned | 用户侧配置归管理面 |
| user-sync | POST | `/api/sync/me` | Agora | Keystone | Keystone | P1 | migrate-planned | 用户侧配置写入归 Keystone |
| ani | GET | `/api/anis` | Agora | Agora | Agora | P2 | owned | 业务查询能力，执行域数据服务 |
| ani | GET | `/api/anis/{id}` | Agora | Agora | Agora | P2 | owned | 同上 |
| ani-collect | GET | `/api/anis/collect` | Agora | Agora | Agora | P2 | owned | 业务域强相关，暂留 Agora |
| ani-collect | POST | `/api/anis/collect` | Agora | Agora | Agora | P2 | owned | 同上 |
| ani-collect | DELETE | `/api/anis/collect/{id}` | Agora | Agora | Agora | P2 | owned | 同上 |
| ani-collect | PATCH | `/api/anis/collect/{id}/watched` | Agora | Agora | Agora | P2 | owned | 同上 |
| news | GET | `/api/news` | Agora | Agora | Agora | P0 | owned | 新闻查询为执行域输出 |
| news | GET | `/api/news/items` | Agora | Agora | Agora | P0 | owned | 同上 |
| news | GET | `/api/news/events` | Agora | Agora | Agora | P0 | owned | 同上 |
| news | GET | `/api/news/events/{id}/items` | Agora | Agora | Agora | P0 | owned | 同上 |
| news-stream | GET | `/api/news/stream` | Agora | Agora | Agora | P0 | owned | SSE 实时链路必须在执行面 |
| task-list | GET | `/api/scheduledTasks` | Agora | Agora | Agora | P0 | owned | 执行配置读取与运行态在 Agora |
| task-write | POST | `/api/scheduledTasks` | Agora | Keystone | Keystone | P1 | migrate-planned | 管理配置写入将迁至 Keystone，Agora保留执行消费 |
| task-write | PUT | `/api/scheduledTasks/{id}` | Agora | Keystone | Keystone | P1 | migrate-planned | 同上 |
| task-write | PATCH | `/api/scheduledTasks/{id}/status` | Agora | Keystone | Keystone | P1 | migrate-planned | 同上 |
| task-write | DELETE | `/api/scheduledTasks/{id}` | Agora | Keystone | Keystone | P1 | migrate-planned | 同上 |
| task-sync | POST | `/api/sync/task_source` | Agora | Keystone | Keystone | P0 | migrate-planned | 作为首批迁移试点接口 |
| admin-task | PUT | `/admin/task/reload` | Agora | Agora | Agora | P0 | owned | 执行侧调度重载操作，留在 Agora |
| proxy | GET | `/api/proxy/image` | Agora | Agora | Agora | P2 | owned | 与内容抓取展示链路强耦合 |

## 4. P0 迁移试点（建议先做）

### 试点接口

- `POST /api/sync/task_source`

### 目标

- 写主从 Agora 迁移到 Keystone。
- Agora 对外仍保持兼容入口（短期可代理到 Keystone）。

### 最小验收

1. 状态码契约一致：`401 / 403 / 200`。
2. 输入校验一致：`name`、`cron`、`params.cmd`、`retryTimes`。
3. 审计可追踪：Keystone 记录操作人/时间/变更摘要。
4. 执行生效链路不回退：Agora 仍能及时 refresh 配置。

## 5. 变更流程（必须遵守）

1. 修改 API 前先更新本文件对应行（owner/status/notes）。
2. 同步更新契约文档（字段、错误码、鉴权要求）。
3. 提交必须带上迁移批次（P0/P1/P2）和影响范围说明。
4. 合并前通过跨系统契约回归。
