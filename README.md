# Agora

> **Agora**（古希腊语：ἀγορά）是古希腊城邦的公共广场，是市民聚集、交流信息与讨论公共事务的核心场所。
> 以此命名，寓意本平台作为多源信息汇聚与交流的公共空间。

Agora 是一个基于 Rust 构建的多源新闻聚合与事件分析平台，提供新闻抓取、关键词提取、事件聚合、用户认证与 RBAC 权限管理等完整后端能力。

## 目录结构

```text
agora/
├── common/           # 公共工具库（结构体、工具函数）
├── configuration/    # 配置文件（base / local / production）
├── docker/           # Docker Compose 及 PostgreSQL 配置
├── docs/             # 附加文档（如 GitHub OAuth2 配置说明）
├── infra/            # 基础设施模块（数据库连接、日志、配置加载）
├── migrations/       # 数据库迁移脚本（sqlx）
├── scripts/          # 辅助脚本（数据库初始化等）
├── service/          # 业务整合层（新闻抓取、聚合逻辑）
├── timer_tasker/     # 定时任务调度器
└── web_services/     # Web 服务入口（Actix-Web，API 路由、认证）
```

## 功能简介

- **web_services**：Actix-Web HTTP 服务，提供新闻、动漫、用户管理 API，集成 GitHub OAuth2 登录、JWT 认证、健康检查与定时任务管理端点。
- **service**：聚合多源新闻与视频平台数据，归一化输出到数据库，支持关键词提取（TF-IDF）、新闻事件聚合与跨天合并。
- **timer_tasker**：基于 cron 表达式的异步定时任务调度器，任务配置持久化至数据库（`scheduled_tasks` 表），支持动态启停。
- **infra**：封装 sqlx 数据库连接池、`tracing` 日志初始化、配置解析等基础设施能力。
- **common**：项目通用类型定义、工具函数与错误处理。
- **migrations**：PostgreSQL 主数据库，使用 sqlx migrate 管理 Schema，支持全量初始化部署。

## 数据库设计

数据库使用 PostgreSQL，通过 `migrations/` 下的 sqlx 迁移脚本管理，共 **21 张表**，分为以下几个业务域：

### 番剧 / 视频

| 表名 | 说明 |
| --- | --- |
| `ani_info` | 番剧信息，唯一约束 `(title, platform, update_count)` |
| `ani_collect` | 用户番剧收藏，唯一约束 `(user_id, ani_item_id)` |
| `ani_watch_history` | 番剧观看历史，唯一约束 `(user_id, ani_item_id)` |

### 新闻聚合

| 表名 | 说明 |
| --- | --- |
| `news_info` | 原始新闻批次（来源×日期），唯一约束 `(news_from, news_date)` |
| `news_item` | 新闻条目，唯一约束 `(item_id, published_at)` |
| `news_keywords` | 新闻关键词（支持 tfidf / textrank / embedding），唯一约束 `(news_id, keyword, method)` |
| `news_event` | 新闻事件（热点聚合），支持 `parent_event_id` 跨天合并 |
| `news_event_item` | 事件-新闻关联（复合主键） |
| `news_event_pipeline_run` | 事件处理流水线运行记录 |

### 用户 / 认证

| 表名 | 说明 |
| --- | --- |
| `user_info` | 系统用户，含 SaaS 字段（tenant_id / plan）、安全字段（token_version / status / locked_until） |
| `user_identities` | 第三方登录绑定（GitHub 等），唯一约束 `(provider, provider_uid)` |
| `refresh_tokens` | Refresh Token，支持滑动窗口会话（`session_expires_at`） |
| `user_setting` | 用户个性化设置（JSONB） |

### RBAC 权限

| 表名 | 说明 |
| --- | --- |
| `roles` | 系统角色（admin / editor / user） |
| `permissions` | 权限/JWT Scopes（user:read / user:write / order:read 等） |
| `role_permissions` | 角色-权限关联 |
| `user_roles` | 用户-角色关联 |
| `plan_permissions` | 套餐-权限关联（free / pro / enterprise） |

### 通用

| 表名 | 说明 |
| --- | --- |
| `scheduled_tasks` | 定时任务配置（cron / JSONB params），唯一约束 `(name)` |
| `favorites` | 通用收藏 |
| `watch_history` | 通用观看历史 |

> 所有含 `updated_at` 的表均通过数据库触发器自动维护该字段。

## 快速开始

### 1. 环境准备

- Rust 1.88+
- PostgreSQL 17+
- Docker（可选，推荐用于本地开发）

### 2. 配置

配置文件位于 `configuration/` 目录下，编辑 [`base.yaml` / `local.yaml`](configuration) 配置数据库连接、服务端口等参数。

GitHub OAuth2 登录所需的环境变量配置请参考 [docs/github_oauth2.md](docs/github_oauth2.md)。

### 3. 数据库初始化

```bash
# 方式一：使用脚本（本地开发 / CI）
sh scripts/init_db.sh

# 方式二：直接执行 sqlx migrate
sqlx migrate run

# 在已有数据库上重新全量部署（数据将清空）
# psql -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
# sqlx migrate run
```

> **注意**：`migrations/` 下的脚本为全量合并版本，在已有数据库上运行前需先清空 `_sqlx_migrations` 表或重建 schema。

### 4. 构建与运行

可配合 [agora-frontend](https://github.com/bruceblink/agora-frontend) 前端项目使用。

#### 方式一：本地构建

```bash
# 启动数据库
docker-compose -f docker/docker-compose.yml -p agora up -d postgresql

# 构建所有子项目
cargo build --workspace

# 启动 Web 服务
cargo run -p web_services
```

#### 方式二：Docker Compose（含应用）

```bash
docker-compose -f docker/docker-compose.yml -p agora up -d
```

> 使用 Docker Compose 启动时，需在 `docker-compose.yml` 中配置 GitHub OAuth2 等环境变量，详见 [docs/github_oauth2.md](docs/github_oauth2.md)。

### 5. API 说明

- Web 服务路由入口见 `web_services/src/`
- 健康检查：`GET /health`
- 具体接口文档可参考代码注释或后续补充的 OpenAPI 文档
- GitHub 第三方登录使用说明：[docs/github_oauth2.md](docs/github_oauth2.md)

## 技术栈

| 类别 | 依赖 |
| --- | --- |
| 异步运行时 | [tokio](https://github.com/tokio-rs/tokio) |
| Web 框架 | [actix-web](https://github.com/actix/actix-web) |
| 数据库 | [sqlx](https://github.com/launchbadge/sqlx)（PostgreSQL + pgvector） |
| 序列化 | [serde](https://github.com/serde-rs/serde) / serde_json |
| HTTP 客户端 | [reqwest](https://github.com/seanmonstar/reqwest) |
| 认证 | [jsonwebtoken](https://github.com/Keats/jsonwebtoken) / [oauth2](https://github.com/ramosbugs/oauth2-rs) / bcrypt |
| 日志追踪 | [tracing](https://github.com/tokio-rs/tracing) + tracing-actix-web |
| 定时任务 | [cron](https://github.com/zslayton/cron) |
| 错误处理 | [anyhow](https://github.com/dtolnay/anyhow) / [thiserror](https://github.com/dtolnay/thiserror) |

其他依赖详见 [Cargo.toml](Cargo.toml)。

## 贡献

欢迎 issue、PR 与建议！

## License

MIT [LICENSE](LICENSE)
