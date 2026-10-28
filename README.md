# 漂流笔记 · 个人博客

一个可直接在本地运行的博客 MVP：公开站点、后台写作、Markdown 预览、草稿/发布、标签、归档、RSS 和图片上传都已打通。

后端是 **Rust + Axum**，本地使用 **SQLite + 文件系统** 模拟生产环境的 **Cloudflare D1 + R2**；前端是 **Astro SSR + TypeScript**。生产目标仍是 Cloudflare Workers + D1 + R2，本地实现保持了 `handlers → services → repositories/storage` 的分层边界。

## 快速开始

环境要求：

- Rust 1.85+（`cargo` 在 PATH 中）
- Node.js 22.23+ 与 npm

```bash
npm install
bash script/dev.sh
```

也可以使用：

```bash
npm run dev
```

首次启动会编译 Rust 后端，可能需要几分钟。启动后：

| 地址 | 说明 |
|-|-|
| http://127.0.0.1:4321 | 公开博客首页 |
| http://127.0.0.1:4321/posts | 文章列表 |
| http://127.0.0.1:4321/admin | 管理后台 |
| http://127.0.0.1:4321/admin/posts/new | 写新文章 |
| http://127.0.0.1:8787/api/health | Rust API 健康检查 |

按 `Ctrl+C` 会同时停止前后端。请求日志在 `log/` 下；后端编译输出直接打印在当前终端，Astro 诊断可用 `cd frontend && npx astro dev logs` 查看。

## 目录结构

```text
frontend/   Astro SSR 前端（公开站点 + 管理后台）
backend/    Rust + Axum API、SQLite、本地对象存储
docs/       方案、架构、开发规范、运维与评审记录（入口 index.toml）
log/        前后端按天日志
script/     一键启动与检查脚本
```

## 常用命令

```bash
bash script/dev.sh       # 一键启动前后端
bash script/check.sh     # 文档索引 + cargo fmt/test + Astro build
npm run dev:backend      # 只启动 Rust API（8787）
npm run dev:frontend     # 只启动 Astro（4321）
npm run build            # 构建前端
```

## 本地数据

默认数据目录是 `backend/data/`：

- `backend/data/blog.db`：SQLite 数据库（首次启动自动建表并写入示例文章）
- `backend/data/storage/`：上传图片

想恢复初始示例内容，停止服务后删除 `backend/data/` 再启动即可。也可以通过 `BLOG_DATA_DIR` 指定其他目录。

## 日志

日志按天分文件，前后端分开：

```text
log/backend/YYYY-MM-DD.log
log/frontend/YYYY-MM-DD.log
```

保留 3 天（今天 + 前两天），启动时清理一次，之后每 72 小时清理一次。请求日志包含方法、路径、状态码、耗时、大小、IP 和 User-Agent，不记录请求体、正文、Cookie 或密钥。查看方式：

```bash
tail -f log/backend/$(date +%F).log
tail -f log/frontend/$(date +%F).log
```

详细规范见 [日志规范](docs/03-开发规范/04-日志规范.md)。

## 已实现

- 公共：首页、文章列表/详情、标签、归档、关于、404、RSS、Sitemap
- 管理：概览统计、文章筛选/搜索、新建/编辑、实时 Markdown 预览、草稿/发布、归档
- 图片：JPEG/PNG/GIF/WebP 上传、Magic Bytes 校验、SHA-256 去重、引用检查
- 内容：服务端 Markdown 渲染 + HTML 净化、发布时固化 HTML、版本号乐观锁、发布后 slug 不可变
- 响应式布局与深浅色主题

## 安全说明

这是本地原型：管理接口 **没有认证**，仅在 `127.0.0.1` 上使用。部署到公网前必须补上 Cloudflare Access（或等价的身份校验）、CSRF/来源校验、`Cache-Control: no-store`，并把本地 SQLite/文件存储适配到 D1/R2。

## 文档

文档集按功能分目录，总索引是 [docs/index.toml](docs/index.toml)：

- [方案总览](docs/01-方案设计/01-方案总览.md)：目标、边界、核心决策
- [总体架构](docs/02-架构设计/01-总体架构.md)：生产架构与本地映射
- [API 设计](docs/02-架构设计/05-API设计.md)：当前接口契约与错误码
- [本地实现说明](docs/03-开发规范/01-本地实现说明.md)：分层、接口与生产迁移
- [日志规范](docs/03-开发规范/04-日志规范.md)：目录、格式与清理策略
- [本地开发与部署](docs/04-运维部署/01-本地开发与部署.md)：一键启动与部署顺序
