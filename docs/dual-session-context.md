# 双会话上下文模板（Agora + Keystone）

## Agora 会话启动模板

```text
【项目】
- 系统：Agora（Rust）
- 当前分支：<branch>
- 目标：<本轮目标>

【边界基线（必须遵守）】
- 单一事实源：docs/api-ownership.md
- Agora归属：任务调度/采集执行/SSE/运行态
- Keystone归属：后台管理/权限治理/审计报表

【本轮任务】
1. <task-1>
2. <task-2>

【验收标准】
- 功能：<2xx/401/403/字段契约等>
- 质量门禁：cargo test -p web_services && cargo clippy -p web_services --all-targets -- -D warnings && cargo fmt --all
- 提交规则：每个小功能单独提交

【协同约束】
- 若改动影响 Keystone：先更新 docs/api-ownership.md 的对应条目，再继续编码。
- 输出必须包含：变更摘要 / 影响接口 / 需Keystone配合项
```

## Keystone 会话启动模板

```text
【项目】
- 系统：Keystone（Java）
- 当前分支：<branch>
- 目标：<本轮目标>

【边界基线（必须遵守）】
- 单一事实源：docs/api-ownership.md
- Keystone归属：后台管理域API（用户、角色、权限、运营配置、审计）
- Agora归属：实时执行域API（调度、采集、SSE、任务运行）

【本轮任务】
1. <task-1>
2. <task-2>

【验收标准】
- 功能：<鉴权/字段/状态码/兼容性>
- 质量门禁：mvn test 或 gradle test + checkstyle/spotless（按项目标准）
- 提交规则：每个小功能单独提交

【协同约束】
- 任何跨系统接口变更必须先改 docs/api-ownership.md。
- 输出必须包含：变更摘要 / 对Agora影响 / 回滚方案
```

## API 归属上下文模板

```text
【API变更上下文】
- API名称：<name>
- 路径：<method + path>
- 当前归属：<Agora|Keystone>
- 目标归属：<Agora|Keystone|不变>
- 写主系统：<Agora|Keystone>
- 读侧系统：<Agora|Keystone|None>
- 鉴权模型：<JWT claims + 权限点>
- 契约变化：<新增/删除/字段变更>
- 兼容策略：<向后兼容期限>
- 测试要求：<最小回归矩阵>
```
