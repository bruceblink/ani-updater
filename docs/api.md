# API 文档

> 基础路径：`/`  
> 所有需要认证的接口须在 Cookie 中携带 `access_token`（JWT）。  
> 统一响应格式见 [响应结构](#响应结构) 章节。

---

## 目录

- [响应结构](#响应结构)
- [认证 / 鉴权](#认证--鉴权)
  - [注册](#post-register)
  - [登出](#post-logout)
  - [刷新 Token](#post-authtokenrefresh)
  - [GitHub OAuth 登录](#get-authoauthgithublogin)
  - [GitHub OAuth 回调](#get-authoauthgithubcallback)
- [用户信息](#用户信息)
  - [获取当前用户信息](#get-apime)
  - [获取用户配置](#get-apisyncme)
  - [保存用户配置](#post-apisyncme)
- [番剧信息](#番剧信息)
  - [分页查询番剧列表](#get-apianis)
  - [查询单条番剧](#get-apianisid)
- [番剧收藏](#番剧收藏)
  - [分页查询收藏列表](#get-apianiscollect)
  - [添加收藏](#post-apianiscollect)
  - [取消收藏](#delete-apianiscollectid)
  - [标记观看状态](#patch-apianiscollectidwatched)
- [新闻信息](#新闻信息)
  - [分页查询新闻列表](#get-apinews)
  - [分页查询新闻条目](#get-apinewsitems)
  - [分页查询热点事件](#get-apinewsevents)
  - [查询事件下的新闻条目](#get-apinewseventsidItems)
- [定时任务](#定时任务)
  - [分页查询定时任务](#get-apischeduledtasks)
  - [创建定时任务](#post-apischeduledtasks)
  - [更新定时任务](#put-apischeduledtasksid)
  - [切换启停状态](#patch-apischeduledtasksidstatus)
  - [删除定时任务](#delete-apischeduledtasksid)
  - [同步任务数据源](#post-apisynctask_source)
- [管理接口](#管理接口)
  - [重载任务调度器](#post-admintaskreload)
- [图片代理](#图片代理)
  - [代理图片请求](#get-apiproxyimage)

---

## 响应结构

### 成功响应

```json
{
  "status": "ok",
  "data": { }
}
```

### 失败响应

```json
{
  "status": "error",
  "message": "错误描述"
}
```

### 分页数据结构 `PageData<T>`

```json
{
  "status": "ok",
  "data": {
    "items": [],
    "totalCount": 100,
    "page": 1,
    "pageSize": 20,
    "totalPages": 5
  }
}
```

### 分页查询参数（通用）

所有分页接口均支持以下 Query 参数（`camelCase`）：

| 参数       | 类型     | 说明                 | 默认值 |
|----------|--------|--------------------| ---- |
| `page`   | number | 页码，从 1 开始         | 1    |
| `pageSize` | number | 每页条数             | 20   |
| `filter` | object | 过滤条件（各接口不同，见下文） | —    |

> `filter` 字段以 flat 形式展开在 Query 字符串中，例如：  
> `?page=1&pageSize=20&title=进击的巨人&platform=bilibili`

---

## 认证 / 鉴权

### POST `/register`

注册新用户（本地账号）。

**无需认证**

**请求体** `application/json`

```json
{
  "username": "alice",
  "password": "secret123",
  "email": "alice@example.com"
}
```

| 字段         | 类型     | 必填 | 说明               |
|------------|--------|----|------------------|
| `username` | string | ✓  | 用户名，不能为空        |
| `password` | string | ✓  | 密码，至少 8 位       |
| `email`    | string |    | 邮箱（可选）          |

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": {
    "userId": 1,
    "username": "alice"
  }
}
```

---

### POST `/logout`

登出，使当前 refresh token 失效，并清空 Cookie。

**无需认证**（依赖 Cookie 中的 `refresh_token`）

**请求体** 无

**响应** `200 OK`，同时清除 `access_token` / `refresh_token` Cookie。

---

### POST `/auth/token/refresh`

使用 Cookie 中的 `refresh_token` 换取新的 `access_token`。

**无需认证**

**请求体** 无（refresh token 从 Cookie 读取）

**响应** `200 OK`，通过 Set-Cookie 写入新的 `access_token`。

---

### GET `/auth/oauth/github/login`

发起 GitHub OAuth2 授权流程，重定向到 GitHub 授权页。

**无需认证**（需要在服务端配置 GitHub OAuth 应用）

**Query 参数**

| 参数            | 类型     | 必填 | 说明              |
|---------------|--------|----|-----------------|
| `redirect_uri` | string | ✓  | 授权成功后的回调前端地址   |

**响应** `302 Redirect` → GitHub 授权页

---

### GET `/auth/oauth/github/callback`

GitHub OAuth2 授权回调，由 GitHub 重定向至此。

**无需认证**

**Query 参数**

| 参数      | 类型     | 必填 | 说明                |
|---------|--------|----|-------------------|
| `code`  | string | ✓  | GitHub 返回的授权码    |
| `state` | string | ✓  | CSRF 防护 state 参数 |

**响应** `302 Redirect` → 前端回调地址，同时通过 Set-Cookie 写入 `access_token` / `refresh_token`。

---

## 用户信息

> 以下接口需认证（Cookie 携带 `access_token`）。

---

### GET `/api/me`

获取当前登录用户的基本信息。

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": {
    "id": 1,
    "username": "alice",
    "email": "alice@example.com",
    "avatar": "https://avatars.githubusercontent.com/...",
    "roles": ["user"],
    "permissions": []
  }
}
```

---

### GET `/api/sync/me`

查询当前用户的指定类型配置。

**Query 参数**

| 参数            | 类型     | 必填 | 说明                         |
|---------------|--------|----|----------------------------|
| `settingType` | string | ✓  | 配置类型，例如 `"theme"` `"layout"` |

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": {
    "data": { },
    "updatedTime": 1700000000
  }
}
```

> `data` 为任意 JSON 对象，`updatedTime` 为 Unix 时间戳（秒）。

---

### POST `/api/sync/me`

保存当前用户的指定类型配置（upsert）。

**请求体** `application/json`

```json
{
  "settingType": "theme",
  "data": {
    "colorScheme": "dark"
  }
}
```

| 字段            | 类型     | 必填 | 说明         |
|---------------|--------|----|------------|
| `settingType` | string | ✓  | 配置类型       |
| `data`        | object | ✓  | 任意 JSON 配置 |

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": "数据同步成功"
}
```

---

## 番剧信息

### GET `/api/anis`

分页查询番剧列表。

**Query 过滤字段**

| 参数         | 类型     | 说明              |
|------------|--------|-----------------|
| `title`    | string | 番剧标题（模糊匹配）     |
| `platform` | string | 平台名称（精确匹配）     |

**响应** `200 OK` → `PageData<AniInfoDto>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "id": 1,
        "title": "进击的巨人",
        "updateCount": "第25话",
        "updateInfo": "已完结",
        "imageUrl": "https://...",
        "detailUrl": "https://...",
        "updateTime": "2024-01-01T00:00:00Z",
        "platform": "bilibili"
      }
    ],
    "totalCount": 100,
    "page": 1,
    "pageSize": 20,
    "totalPages": 5
  }
}
```

---

### GET `/api/anis/{id}`

查询单条番剧详情。

**路径参数**

| 参数   | 类型   | 说明    |
|------|------|-------|
| `id` | i64  | 番剧 ID |

**响应** `200 OK` → `AniInfoDto`

```json
{
  "status": "ok",
  "data": {
    "id": 1,
    "title": "进击的巨人",
    "updateCount": "第25话",
    "updateInfo": "已完结",
    "imageUrl": "https://...",
    "detailUrl": "https://...",
    "updateTime": "2024-01-01T00:00:00Z",
    "platform": "bilibili"
  }
}
```

**错误**

| 状态码 | 说明      |
|-----|---------|
| 404 | 番剧不存在   |

---

## 番剧收藏

> 以下接口为用户私有数据，所有操作均隔离到当前登录用户。

---

### GET `/api/anis/collect`

分页查询当前用户的番剧收藏列表。

**Query 过滤字段**

| 参数         | 类型      | 说明              |
|------------|---------|-----------------|
| `aniTitle` | string  | 番剧标题（模糊匹配）    |
| `isWatched`| boolean | 是否已观看过滤        |

**响应** `200 OK` → `PageData<AniCollectDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "id": 10,
        "aniItemId": 1,
        "aniTitle": "进击的巨人",
        "collectTime": "2024-03-01T12:00:00Z",
        "isWatched": false
      }
    ],
    "totalCount": 5,
    "page": 1,
    "pageSize": 20,
    "totalPages": 1
  }
}
```

---

### POST `/api/anis/collect`

添加番剧到收藏。

**请求体** `application/json`

```json
{
  "aniItemId": 1,
  "aniTitle": "进击的巨人"
}
```

| 字段          | 类型     | 必填 | 说明       |
|-------------|--------|----|----------|
| `aniItemId` | i64    | ✓  | 番剧 ID    |
| `aniTitle`  | string | ✓  | 番剧标题（冗余） |

**响应** `201 Created` → `AniCollectDTO`

```json
{
  "status": "ok",
  "data": {
    "id": 10,
    "aniItemId": 1,
    "aniTitle": "进击的巨人",
    "collectTime": "2024-03-01T12:00:00Z",
    "isWatched": false
  }
}
```

**错误**

| 状态码 | 说明                          |
|-----|-----------------------------|
| 400 | 该番剧已收藏（UNIQUE 约束冲突）        |

---

### DELETE `/api/anis/collect/{id}`

取消收藏（仅可操作自己的收藏记录）。

**路径参数**

| 参数   | 类型  | 说明     |
|------|-----|--------|
| `id` | i64 | 收藏记录 ID |

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": null
}
```

**错误**

| 状态码 | 说明                 |
|-----|--------------------|
| 404 | 收藏记录不存在或不属于当前用户   |

---

### PATCH `/api/anis/collect/{id}/watched`

标记或取消观看状态。

**路径参数**

| 参数   | 类型  | 说明     |
|------|-----|--------|
| `id` | i64 | 收藏记录 ID |

**请求体** `application/json`

```json
{
  "isWatched": true
}
```

| 字段          | 类型      | 必填 | 说明             |
|-------------|---------|-----|----------------|
| `isWatched` | boolean | ✓   | `true`=已看 / `false`=未看 |

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": null
}
```

**错误**

| 状态码 | 说明                 |
|-----|--------------------|
| 404 | 收藏记录不存在或不属于当前用户   |

---

## 新闻信息

### GET `/api/news`

分页查询新闻信息源列表（原始抓取数据）。

**Query 过滤字段**

| 参数          | 类型      | 说明                    |
|-------------|---------|------------------------|
| `newsFrom`  | string  | 新闻来源（模糊匹配）           |
| `newsDate`  | string  | 新闻日期，格式 `YYYY-MM-DD` |
| `extracted` | boolean | 是否已提取                 |

**响应** `200 OK` → `PageData<NewsInfoDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "id": 1,
        "newsFrom": "36kr",
        "newsDate": "2024-03-01",
        "data": {},
        "createdAt": "2024-03-01T08:00:00Z",
        "updatedAt": "2024-03-01T09:00:00Z",
        "name": "36kr",
        "extracted": true,
        "extractedAt": "2024-03-01T09:05:00Z"
      }
    ],
    "totalCount": 200,
    "page": 1,
    "pageSize": 20,
    "totalPages": 10
  }
}
```

---

### GET `/api/news/items`

分页查询已提取的新闻条目列表。

**Query 过滤字段**

| 参数            | 类型      | 说明                      |
|-------------|---------|-------------------------|
| `source`    | string  | 新闻来源名称（模糊匹配）           |
| `publishedAt` | string | 发布日期，格式 `YYYY-MM-DD`    |
| `clusterId` | i64     | 聚类 ID（精确匹配）             |
| `extracted` | boolean | 是否已二次提取（关键词/事件）        |

**响应** `200 OK` → `PageData<NewsItemResponseDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "id": 100,
        "itemId": "abc123",
        "title": "某科技公司发布新产品",
        "url": "https://example.com/news/1",
        "source": "36kr",
        "publishedAt": "2024-03-01",
        "clusterId": 5,
        "extracted": true,
        "createdAt": "2024-03-01T08:00:00Z"
      }
    ],
    "totalCount": 500,
    "page": 1,
    "pageSize": 20,
    "totalPages": 25
  }
}
```

---

### GET `/api/news/events`

分页查询热点事件列表。

**Query 过滤字段**

| 参数          | 类型    | 说明                                                          |
|-------------|-------|-------------------------------------------------------------|
| `eventDate` | string | 事件日期，格式 `YYYY-MM-DD`                                       |
| `status`    | i16   | 事件状态：`0`=自动生成 / `1`=已确认 / `2`=已归档 / `3`=已合并 |

**响应** `200 OK` → `PageData<NewsEventDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "id": 1,
        "eventDate": "2024-03-01",
        "clusterId": 5,
        "title": "AI 大模型竞争加剧",
        "summary": "本周多家科技公司相继发布新一代 AI 模型...",
        "newsCount": 12,
        "score": 0.92,
        "status": 1,
        "parentEventId": null,
        "createdAt": "2024-03-01T10:00:00Z"
      }
    ],
    "totalCount": 30,
    "page": 1,
    "pageSize": 20,
    "totalPages": 2
  }
}
```

**`status` 枚举说明**

| 值 | 含义     |
|---|--------|
| 0 | 自动生成   |
| 1 | 已人工确认  |
| 2 | 已归档    |
| 3 | 已合并到其他事件 |

---

### GET `/api/news/events/{id}/items`

查询指定热点事件下关联的所有新闻条目（不分页）。

**路径参数**

| 参数   | 类型  | 说明     |
|------|-----|--------|
| `id` | i64 | 热点事件 ID |

**响应** `200 OK` → `Vec<NewsItemResponseDTO>`

```json
{
  "status": "ok",
  "data": [
    {
      "id": 100,
      "itemId": "abc123",
      "title": "某科技公司发布新产品",
      "url": "https://example.com/news/1",
      "source": "36kr",
      "publishedAt": "2024-03-01",
      "clusterId": 5,
      "extracted": true,
      "createdAt": "2024-03-01T08:00:00Z"
    }
  ]
}
```

---

## 定时任务

### GET `/api/scheduledTasks`

分页查询定时任务列表。

**Query 过滤字段**

| 参数          | 类型      | 说明              |
|-------------|---------|-----------------|
| `name`      | string  | 任务名称（模糊匹配）     |
| `isEnabled` | boolean | 是否启用            |

**响应** `200 OK` → `PageData<ScheduledTasksDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "id": 1,
        "name": "抓取36kr新闻",
        "cron": "0 */6 * * *",
        "params": {},
        "isEnabled": true,
        "retryTimes": 3,
        "lastRun": "2024-03-01T06:00:00Z",
        "nextRun": "2024-03-01T12:00:00Z",
        "lastStatus": "success"
      }
    ],
    "totalCount": 10,
    "page": 1,
    "pageSize": 20,
    "totalPages": 1
  }
}
```

---

### POST `/api/scheduledTasks`

创建新定时任务。

**请求体** `application/json`

```json
{
  "name": "抓取36kr新闻",
  "cron": "0 */6 * * *",
  "params": { "source": "36kr" },
  "isEnabled": true,
  "retryTimes": 3
}
```

| 字段           | 类型      | 必填 | 默认值   | 说明           |
|--------------|---------|-----|-------|--------------|
| `name`       | string  | ✓   | —     | 任务名称（唯一）    |
| `cron`       | string  | ✓   | —     | Cron 表达式    |
| `params`     | object  | ✓   | —     | 任务参数（任意 JSON）|
| `isEnabled`  | boolean |     | false | 是否立即启用      |
| `retryTimes` | number  |     | 3     | 失败重试次数      |

**响应** `201 Created` → `ScheduledTasksDTO`

---

### PUT `/api/scheduledTasks/{id}`

更新定时任务（所有字段均可选）。

**路径参数**

| 参数   | 类型  | 说明     |
|------|-----|--------|
| `id` | i64 | 任务 ID  |

**请求体** `application/json`

```json
{
  "name": "新名称",
  "cron": "0 8 * * *",
  "params": { "source": "36kr" },
  "retryTimes": 5
}
```

| 字段           | 类型     | 说明           |
|--------------|--------|--------------|
| `name`       | string | 任务名称         |
| `cron`       | string | Cron 表达式    |
| `params`     | object | 任务参数         |
| `retryTimes` | number | 失败重试次数      |

**响应** `200 OK` → `ScheduledTasksDTO`

**错误**

| 状态码 | 说明        |
|-----|-----------|
| 404 | 任务不存在     |

---

### PATCH `/api/scheduledTasks/{id}/status`

切换定时任务的启停状态。

**路径参数**

| 参数   | 类型  | 说明    |
|------|-----|-------|
| `id` | i64 | 任务 ID |

**请求体** `application/json`

```json
{
  "isEnabled": false
}
```

**响应** `200 OK` → `ScheduledTasksDTO`

---

### DELETE `/api/scheduledTasks/{id}`

删除定时任务。

**路径参数**

| 参数   | 类型  | 说明    |
|------|-----|-------|
| `id` | i64 | 任务 ID |

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": null
}
```

**错误**

| 状态码 | 说明      |
|-----|---------|
| 404 | 任务不存在   |

---

### POST `/api/sync/task_source`

同步（upsert）任务数据源配置。

**请求体** `application/json`

```json
{
  "name": "抓取36kr新闻",
  "cron": "0 */6 * * *",
  "params": { "source": "36kr" },
  "retryTimes": 3
}
```

| 字段           | 类型     | 必填 | 说明            |
|--------------|--------|----|---------------|
| `name`       | string | ✓  | 任务名称（ON CONFLICT 键）|
| `cron`       | string | ✓  | Cron 表达式     |
| `params`     | object | ✓  | 任意 JSON 参数   |
| `retryTimes` | number | ✓  | 重试次数          |

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": {
    "message": "同步成功"
  }
}
```

---

## 管理接口

> 需要认证，仅限管理员角色调用。

### POST `/admin/task/reload`

热重载任务调度器配置（无需重启服务）。

**请求体** 无

**响应** `200 OK`

---

## 图片代理

### GET `/api/proxy/image`

代理外部图片请求，防止跨域问题。仅支持白名单域名的 HTTP/HTTPS 图片。

**无需认证**

**Query 参数**

| 参数    | 类型     | 必填 | 说明                     |
|-------|--------|----|------------------------|
| `url` | string | ✓  | 要代理的图片 URL（需 URL 编码）  |

**响应** `200 OK`，`Content-Type: image/*`，返回图片二进制流。

**错误**

| 状态码 | 说明                    |
|-----|-----------------------|
| 400 | 缺少 url / url 格式非法 / 域名不在白名单 |

---

## 数据模型汇总

### `AniInfoDto`

| 字段           | 类型     | 说明       |
|--------------|--------|----------|
| `id`         | i64    | 番剧 ID    |
| `title`      | string | 番剧标题     |
| `updateCount`| string | 更新集数     |
| `updateInfo` | string | 更新说明     |
| `imageUrl`   | string | 封面图 URL  |
| `detailUrl`  | string | 详情页 URL  |
| `updateTime` | string | 最新更新时间   |
| `platform`   | string | 来源平台     |

### `AniCollectDTO`

| 字段            | 类型       | 说明          |
|---------------|----------|-------------|
| `id`          | i64      | 收藏记录 ID    |
| `aniItemId`   | i64      | 番剧 ID       |
| `aniTitle`    | string   | 番剧标题        |
| `collectTime` | datetime | 收藏时间（ISO 8601）|
| `isWatched`   | boolean  | 是否已观看      |

### `NewsInfoDTO`

| 字段            | 类型       | 说明            |
|---------------|----------|---------------|
| `id`          | i64      | 新闻源 ID       |
| `newsFrom`    | string   | 数据来源名称       |
| `newsDate`    | date     | 新闻日期         |
| `data`        | object   | 原始抓取 JSON 数据  |
| `name`        | string   | 来源名称         |
| `extracted`   | boolean  | 是否已提取条目      |
| `extractedAt` | datetime | 提取时间         |
| `createdAt`   | datetime | 创建时间         |
| `updatedAt`   | datetime | 更新时间         |

### `NewsItemResponseDTO`

| 字段           | 类型       | 说明             |
|--------------|----------|----------------|
| `id`         | i64      | 条目 ID          |
| `itemId`     | string   | 原始条目唯一标识      |
| `title`      | string   | 新闻标题           |
| `url`        | string   | 原文链接           |
| `source`     | string?  | 来源名称           |
| `publishedAt`| date     | 发布日期（`YYYY-MM-DD`）|
| `clusterId`  | i64?     | 聚类 ID          |
| `extracted`  | boolean  | 是否已做关键词/事件提取   |
| `createdAt`  | datetime? | 入库时间          |

### `NewsEventDTO`

| 字段              | 类型       | 说明                          |
|-----------------|----------|-----------------------------|
| `id`            | i64      | 事件 ID                       |
| `eventDate`     | date     | 事件日期                        |
| `clusterId`     | i64      | 聚类 ID                       |
| `title`         | string?  | 事件标题                        |
| `summary`       | string?  | 事件摘要                        |
| `newsCount`     | i32      | 关联新闻条数                      |
| `score`         | f32?     | 热度评分                        |
| `status`        | i16      | 状态（0=自动/1=确认/2=归档/3=合并）   |
| `parentEventId` | i64?     | 合并到的父事件 ID                  |
| `createdAt`     | datetime | 创建时间                        |

### `ScheduledTasksDTO`

| 字段           | 类型       | 说明             |
|--------------|----------|----------------|
| `id`         | i64      | 任务 ID          |
| `name`       | string   | 任务名称           |
| `cron`       | string   | Cron 表达式       |
| `params`     | object   | 任务参数 JSON      |
| `isEnabled`  | boolean  | 是否启用           |
| `retryTimes` | number   | 最大重试次数         |
| `lastRun`    | datetime? | 上次运行时间        |
| `nextRun`    | datetime? | 下次运行时间        |
| `lastStatus` | string   | 上次运行结果        |
