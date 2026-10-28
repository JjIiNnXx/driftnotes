# 漂流笔记 · 个人博客

一个可直接在本地运行的博客 MVP：公开站点、后台写作、Markdown 预览、草稿/发布、标签、归档、RSS 和图片上传都已打通。

后端是 **Rust + Axum**，本地使用 **SQLite + 文件系统** 模拟生产环境的 **Cloudflare D1 + R2**；前端是 **Astro SSR + TypeScript**。生产目标仍是 Cloudflare Workers + D1 + R2，本地实现保持了 `handlers → services → repositories/storage` 的分层边界。

## 快速开始

环境要求：

- Rust 1.85+（`cargo` 在 PATH 中，需安装 `rustfmt` 与 `clippy` 组件）
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
bash script/check.sh     # 文档索引 + fmt + clippy + test + Astro 类型检查与构建
bash script/setup-git-hooks.sh  # 启用 pre-push，阻止直推 main
npm run dev:backend      # 只启动 Rust API（8787）
npm run dev:frontend     # 只启动 Astro（4321）
npm run build            # 构建前端
```

## CI 与分支保护

`main` 已开启 GitHub 分支保护，规则对管理员同样生效：

- 必须通过 PR 合入，禁止直接推送、force push 和删除分支；
- 必需检查 `ci` 必须通过，红灯时网页上的合并按钮不可用；
- 要求分支与 `main` 同步（strict），历史保持线性；
- 只允许 rebase merge，合并后自动删除 head 分支。

CI 只有一个工作流 `.github/workflows/ci.yml`，只在 PR 打开、更新、重新打开或转为 ready 时运行。PR 事件下检出的是 GitHub 生成的 merge commit，即“把该 PR 合入 `main` 后”的结果。

| 步骤 | 内容 |
|-|-|
| 密钥扫描 | gitleaks 固定版本并校验 SHA-256，扫描完整历史，日志脱敏 |
| 敏感文件 | 拒绝提交 `.env*`（`.env.example` 除外） |
| 文档索引 | `docs/index.toml` 中的路径必须存在 |
| 格式 | `cargo fmt --check` |
| 静态检查 | `cargo clippy --all-targets -- -D warnings` |
| 单元测试 | `cargo test` |
| 类型检查 | `astro check` |
| 构建 | `astro build` |

后五项由 `bash script/check.sh` 统一执行，本地与 CI 一致。

仓库同时开启 GitHub 自带的 secret scanning 与 push protection，推送含已知密钥的提交会被服务端直接拒绝。

PR 只能由维护者在 GitHub 网页手动点击 **Rebase and merge** 合入。Agent 不得使用 `gh pr merge`、合并 API 或 auto-merge；仓库已关闭 auto-merge。合并前确认 `ci` 为绿色。

本地 `pre-push` 仍会提前拦截直推 `main`，避免推到远端才被拒绝。首次克隆后执行一次：

```bash
bash script/setup-git-hooks.sh
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
