# ani-updater

一个多源动漫数据聚合与订阅推送系统，支持定时任务、数据抓取、订阅管理等功能。

## 目录结构

```
ani-updater/
├── ani_spiders/      # 动漫数据爬虫库，支持多个平台
├── ani_subs/         # 订阅服务，提供 API 与任务调度
├── common/           # 公共工具库
├── timer_tasker/     # 定时任务调度与命令分发
├── configuration/    # 配置文件
├── docker/           # Docker 相关文件
├── migrations/       # 数据库迁移脚本
├── postgresql/       # 本地数据库数据与配置
├── scripts/          # 辅助脚本
└── ...
```

## 功能简介

- **ani_spiders**：聚合各大平台（如哔哩哔哩、爱奇艺、腾讯、优酷、Mikan、Age动漫等）动漫数据的爬虫库，统一数据结构输出。
- **ani_subs**：提供动漫订阅、推送、API 服务，支持数据库管理、定时任务、健康检查等。
- **timer_tasker**：定时任务调度器，负责定时拉取数据的lib。
- **common**：项目通用工具与基础设施代码。
- **数据库与迁移**：PostgreSQL 作为主数据库，支持自动建表与数据迁移。

## 快速开始

### 1. 环境准备

- Rust 1.86+
- PostgreSQL 17+
- Docker（可选，推荐用于本地开发）

### 2. 配置
配置文件位于 `configuration/` 目录下。
编辑 [`configuration/base.yaml`、`local.yaml`](configuration) 等文件，配置数据库、服务端口等参数。
环境变量配置
参照[github_oauth2.md](docs/github_oauth2.md)配置GitHub第三方登录所需的环境变量。

### 3. 数据库初始化(可选，现在仅仅CI测试时需要)

```bash
# 使用脚本初始化数据库
sh scripts/init_db.sh
# 或手动执行 migrations/ 下的 SQL 脚本
```

### 4. 构建与运行

目前可以使用 [material-kit-react-lovat](https://github.com/bruceblink/material-kit-react)作为前端项目用于测试使用

#### 方式一：本地构建

```bash
# 启动数据库
docker-compose -f docker/docker-compose.yml -p ani-updater up -d postgresql
# 构建所有子项目
cargo build --workspace
# 运行订阅服务
cargo run -p ani_subs

```

#### 方式二：Docker

```bash
# 启动数据库与应用服务
docker-compose -f docker/docker-compose.yml -p ani-updater up -d
```

### 5. API 说明

- 订阅服务 API 入口见 `ani_subs/src/startup.rs`
- 具体接口文档可参考代码注释或后续补充的 OpenAPI 文档
- 关于如何使用 GitHub 第三方登录请参考[GitHub第三方登录使用说明](docs/github_oauth2.md)

## 依赖说明

- [tokio](https://github.com/tokio-rs/tokio) 异步运行时
- [serde](https://github.com/serde-rs/serde) 序列化/反序列化
- [reqwest](https://github.com/seanmonstar/reqwest) HTTP 客户端
- [sqlx](https://github.com/launchbadge/sqlx) 异步数据库
- 其他依赖详见各项目[Cargo.toml](Cargo.toml)

## 贡献

欢迎 issue、PR 与建议！

## License

MIT [LICENSE](LICENSE)

