# migration-plan-v0.4.x（Agora × Keystone）

> 目标：将当前“糅合架构”收敛为“Keystone 控制面 + Agora 执行面”的可维护形态。  
> 时间窗口：v0.4.x 周期内完成 P0/P1 主体，P2 持续收口。

## 一、范围与目标

### 目标

1. Keystone 成为通用管理平台（认证、RBAC、配置、审计）。
2. Agora 成为执行引擎（调度、执行、SSE、运行态）。
3. API 不再重叠写主，杜绝双主写。

### 非目标（本阶段不做）

- 不做一次性全量迁移。
- 不做大规模表结构重塑。
- 不在同一迭代引入多套新中间件框架。

## 二、里程碑

## M1（P0，1~2 周）：边界定责 + 试点迁移

### 任务包

1. 建立并冻结 `docs/api-ownership.md`（路由级归属清单）。
2. 选定首个试点：`POST /api/sync/task_source`。
3. Keystone 落地等价写接口（统一鉴权与参数校验）。
4. Agora 增加兼容层（保留原入口，内部代理/转发到 Keystone）。
5. 补齐契约回归：`401 / 403 / 2xx + 核心字段断言`。

### 验收标准

- 试点接口在生产路径只有 Keystone 写主。
- Agora 兼容入口可平滑服务旧调用方。
- 无权限回退（非管理员不得写）。

## M2（P1，2~4 周）：管理域批量迁移

### 任务包

1. 迁移认证相关 API 到 Keystone：
   - `/register`
   - `/logout`
   - `/auth/token/refresh`
   - `/auth/oauth/github/*`
2. 迁移用户配置相关 API 到 Keystone：
   - `/api/me`
   - `/api/sync/me`
3. 迁移任务配置写接口到 Keystone：
   - `POST/PUT/PATCH/DELETE /api/scheduledTasks*`
4. Agora 保留：
   - `GET /api/scheduledTasks`
   - `PUT /admin/task/reload`
   - `/api/news/stream` 与执行链路相关接口。

### 验收标准

- 管理型写接口由 Keystone 完整接管。
- Agora 仅保留执行型能力与必要只读能力。
- API 文档与实际路由归属一致。

## M3（P2，持续）：遗留收口 + 平台化

### 任务包

1. 下线已迁移旧入口（经过兼容窗口后）。
2. 清理重复 DTO/权限判断/配置写入逻辑。
3. 完成统一观测：
   - Keystone：管理操作审计
   - Agora：执行链路运行态
4. 形成可复用 Keystone 接入规范（供其他业务系统复用）。

### 验收标准

- 不再出现同业务能力双系统并行实现。
- 新业务接入 Keystone 不需要复制管理代码。

## 三、实施策略

### 1) 迁移策略

- **Strangler Fig**：新流量先到 Keystone，旧入口在 Agora 做受控兼容。
- **小步快跑**：每次只迁一个业务切片，每片都可独立回滚。
- **契约优先**：先固化请求/响应/错误码，再动实现。

### 2) 流量策略

- 阶段灰度：10% -> 50% -> 100%。
- 失败回退：网关路由一键回切 Agora 旧实现。

### 3) 质量门禁

- Agora：`cargo test -p web_services` -> `cargo clippy -p web_services --all-targets -- -D warnings` -> `cargo fmt --all`
- Keystone：`mvn/gradle test` + 静态检查 + 格式化
- 跨系统：契约回归测试必须全绿。

## 四、角色分工

- Keystone 负责人：认证/RBAC/管理配置/审计 API。
- Agora 负责人：执行链路、兼容层、运行态一致性。
- 架构 owner：维护 `api-ownership.md`，审批跨系统边界变更。

## 五、风险与缓解

1. 风险：双系统权限口径不一致。  
   缓解：统一 claims + 权限点命名，契约测试覆盖 401/403。

2. 风险：迁移期间调用方受影响。  
   缓解：Agora 兼容入口保留一个版本周期，灰度+回滚开关。

3. 风险：改动面大导致发布不稳。  
   缓解：按接口切片分批迁移，每批独立验收。

## 六、发布节奏建议（v0.4.x）

- `v0.4.1`：完成 P0（试点迁移 + 契约门禁）。
- `v0.4.2`：完成 P1 第一批（认证与用户配置）。
- `v0.4.3`：完成 P1 第二批（scheduledTasks 写接口迁移）。
- `v0.4.4+`：P2 收口与旧入口下线。
