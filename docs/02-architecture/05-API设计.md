# API 设计

> 文档集索引：[index.toml](../index.toml) · 按当前本地 Rust API 实现整理

## 7. API 设计

所有接口使用 JSON，文件上传使用 `multipart/form-data`，时间统一为 RFC 3339 / ISO 8601 字符串。本地 API 默认监听 `http://127.0.0.1:8787`，前端通过 Astro SSR 直连或 Vite 代理访问。

### 7.1 公开接口

| 方法 | 路径 | 用途 |
|-|-|-|
| GET | `/api/health` | 健康检查，返回运行时与存储类型 |
| GET | `/api/posts` | 分页获取已发布文章 |
| GET | `/api/posts/:slug` | 获取单篇已发布文章及上下篇 |
| GET | `/api/tags` | 获取标签及已发布文章数 |
| GET | `/api/archive` | 按年 / 月分组的归档 |
| GET | `/api/settings` | 读取站点设置 |
| GET | `/images/*` | 读取本地上传图片，长期缓存 |

文章列表查询参数：

| 参数 | 默认值 | 说明 |
|-|-|-|
| `page` | `1` | 页码 |
| `page_size` | `10` | 每页数量，最大 `100`；兼容旧参数 `pageSize` |
| `tag` | 空 | 按标签 slug 或名称过滤 |
| `q` | 空 | 在标题、摘要、Markdown 正文中模糊搜索 |

分页响应字段使用 snake_case：

```json
{
  "data": {
    "items": [],
    "pagination": {
      "page": 1,
      "page_size": 10,
      "total": 8,
      "total_pages": 1
    }
  }
}
```

`/api/tags` 直接返回数组，每项包含 `id`、`name`、`slug`、`post_count`。`/api/archive` 返回年份与月份分组：

```json
{
  "data": [
    {
      "year": 2026,
      "months": [
        { "month": 10, "posts": [] }
      ]
    }
  ]
}
```

### 7.2 管理接口

| 方法 | 路径 | 用途 |
|-|-|-|
| GET | `/api/admin/stats` | 文章状态计数与图片统计 |
| GET | `/api/admin/posts` | 获取全部状态文章，支持 `status`、`q`、分页 |
| POST | `/api/admin/posts` | 新建草稿或直接发布 |
| GET | `/api/admin/posts/:id` | 获取编辑所需完整内容 |
| PUT | `/api/admin/posts/:id` | 更新文章，必须携带当前 `version` |
| DELETE | `/api/admin/posts/:id` | 归档文章（不物理删除） |
| POST | `/api/admin/render` | 使用后端同一渲染器预览 Markdown |
| GET | `/api/admin/assets` | 图片列表 |
| POST | `/api/admin/assets` | 上传图片，字段名 `file` |
| DELETE | `/api/admin/assets/:id` | 删除未被文章引用的图片 |

新建 / 更新文章的核心字段：

```json
{
  "title": "文章标题",
  "slug": "post-slug",
  "excerpt": "列表摘要",
  "content_markdown": "## 正文",
  "cover_image": "/covers/garden.svg",
  "tags": ["技术", "Rust"],
  "status": "draft",
  "version": 1
}
```

规则：

- `status` 只能是 `draft`、`published`、`archived`。
- 发布时若没有 `published_at`，服务端自动写入当前时间。
- 更新使用 `version` 乐观锁；版本不一致返回 `409 VERSION_CONFLICT`。
- 已发布文章的 `slug` 不可修改，返回 `409 SLUG_IMMUTABLE`。
- 标签按名称或 slug 查找，未知标签保存时自动创建。
- `cover_image` 只接受 `/covers/` 或 `/images/` 开头的本地路径。

### 7.3 统一响应结构

成功响应：

```json
{
  "data": {
    "post": {
      "id": 1,
      "title": "示例文章"
    }
  }
}
```

失败响应：

```json
{
  "error": {
    "code": "BAD_REQUEST",
    "message": "title is required and must be a non-empty string"
  }
}
```

当前错误码：

| 错误码 | HTTP | 说明 |
|-|-|-|
| `BAD_REQUEST` | 400 | 参数或业务规则不满足 |
| `INVALID_JSON` | 400 | JSON 解析失败 |
| `INVALID_SLUG` | 400 | slug 格式不合法 |
| `NOT_FOUND` | 404 | 文章、图片或路由不存在 |
| `SLUG_EXISTS` | 409 | slug 已被占用 |
| `SLUG_IMMUTABLE` | 409 | 已发布文章尝试改 slug |
| `VERSION_CONFLICT` | 409 | 乐观锁版本冲突 |
| `ASSET_IN_USE` | 409 | 图片仍被文章引用 |
| `PAYLOAD_TOO_LARGE` | 413 | 请求体或图片超过限制 |
| `INTERNAL_ERROR` | 500 | 服务端内部错误，不返回堆栈 |

### 7.4 本地与生产差异

本地管理接口没有认证，也没有 `Cache-Control: no-store`；生产部署时必须由 Cloudflare Access + 服务端 JWT/HMAC 校验保护，并补齐上述响应头。本地 `/api` 与 `/images` 由 Vite 代理到 Rust API；生产建议通过 Service Binding 在 Web Worker 与 API Worker 之间转发。
