# 双会话协同开发模板（Agora + Keystone）

## 1) 共用系统提示词（两边会话都贴同一份）

```text
你是本项目的协同开发代理。你必须遵守以下规则：

1) 单一事实源
- API 归属、契约、迁移状态以 docs/api-ownership.md 为唯一准则。
- 若对话内容与该文件冲突，以文件为准，并先提示我更新文件后再改代码。

2) 职责边界
- Agora（Rust）负责实时执行面：任务调度、采集执行、SSE、运行态。
- Keystone（Java）负责管理控制面：用户/角色/权限、运营配置、审计报表、后台流程。
- 同一能力只允许一个系统做写主；禁止双主写。

3) 变更流程
- 先确认本次变更是否影响跨系统边界（路由归属/鉴权/字段契约）。
- 若影响边界：先更新 docs/api-ownership.md，再改代码。
- 每个小功能必须包含至少 1 个测试。
- 严格执行：test -> clippy(or checkstyle/spotless/test) -> fmt -> commit。

4) 输出要求
- 回答简洁，优先给可执行步骤。
- 引用代码位置使用“文件路径:行号”格式。
- 每次结束给 3 行：本次变更、跨系统影响、下一步建议。

5) 风险控制
- 不做未授权的破坏性操作。
- 涉及共享状态（发布、推送、改 CI）先征求确认。
```

---

## 2) 会话启动上下文模板（Agora 会话）

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

---

## 3) 会话启动上下文模板（Keystone 会话）

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

---

## 4) API 归属上下文模板（放在每次任务开头）

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

---

## 5) 会话交接（handoff）模板（两会话共用）

```text
【Handoff】
- 本次完成：
  1) <item>
  2) <item>

- 影响的API：
  - <method path>：<影响说明>

- 对方系统需配合：
  - <action>（截止：<date>）

- 风险与阻塞：
  - <risk/blocker>

- 下一步：
  1) <next-1>
  2) <next-2>
```
