# AGENTS.md

> Audience: AI coding agents only. Human onboarding belongs in `README.md`.
> Project details live in `docs/`; `docs/index.toml` is the canonical index.

## Task Contract

Before editing, identify the requested deliverable and the affected boundary (frontend, backend, docs, scripts, or data). A task is complete when the requested behavior or document change is implemented, applicable checks pass, required documentation is updated, and the final response states changed files, validation results, and any known limitation or unrun check.

Agents may inspect the repository, create the required local `feature_<short-name>` branch, edit files, run local services, and run checks without pausing for approval. Stop and ask before destructive or irreversible actions: deleting or overwriting user-owned work, rewriting history, pushing to a remote, opening or merging an MR, publishing, or changing shared/production data. Unless the request already authorizes it, do not make product decisions the request does not determine. A routine self-check is not a user question; do not ask again for authorization already given.

Create a local commit only when the user requests it or when it is required by an explicitly requested push/MR workflow. Do not add AI/model/tool co-author trailers.

## Non-Negotiable Rules

| # | Rule | Constraint |
|---|---|---|
| 1 | Runtime | The backend MUST stay Rust (`backend/`); the frontend MUST stay Astro (`frontend/`). Do not introduce a Node API server, a second database, or a paid service for local development. |
| 2 | Layering | Backend flow is `handlers/` → `services/` → `repositories/` / `storage/`. HTTP concerns stay in handlers, business rules in services, SQL in repositories, object storage in storage. The frontend NEVER touches the database or filesystem. |
| 3 | Contract Sync | API responses use `{ "data": ... }` or `{ "error": { "code", "message" } }`; pagination fields are snake_case. Any route or field change MUST update both `backend/src/router.rs` and `frontend/src/lib/api.ts`. |
| 4 | Types | `frontend/tsconfig.json` extends Astro strict; do not weaken it or silence errors with `@ts-ignore` / `@ts-expect-error`. Do not add `as any` to bypass an API type; declare a narrow type instead. Rust request paths MUST return `ApiResult`/`ApiError`, not new panics. |
| 5 | Dead Code | Delete unused imports, variables, functions, and orphaned files in the same change. When replacing a feature, remove the entire old chain (route, handler, service, repository call, client, type, docs, test). |
| 6 | Cleanliness | NEVER leave commented-out code, `TODO`, `FIXME`, placeholder data, unused fallback paths, or broken user-visible flows. |
| 7 | Docs | Update the affected `docs/**/*.md` and `docs/index.toml` whenever structure, routes, fields, commands, schema, logging, or conventions change. `README.md` and `docs/` are written in Chinese. |
| 8 | Security | The local admin API has NO authentication and is local-only. NEVER bind it to a public interface or expose it. Do not commit secrets, tokens, cookies, or credentials. Production MUST add Access/HMAC and `no-store` as documented in `docs/03-开发规范/02-认证与安全.md`; local code MUST NOT pretend it is implemented. |
| 9 | Logging | Logs MUST NOT contain request bodies, Markdown content, cookies, `Authorization` headers, passwords, or keys. Daily files stay under `log/backend/` and `log/frontend/` with 3-day retention. |
| 10 | Content Integrity | Posts keep both `content_markdown` and ammonia-sanitized `content_html`. Updates use the `version` optimistic lock; the slug is immutable after publishing. Do not bypass these rules in handlers or client code. |
| 11 | Uploads | Accept only magic-byte-validated JPEG/PNG/GIF/WebP, deduplicate by SHA-256, and keep the 6 MB body limit in `backend/src/router.rs`. |
| 12 | Verification | `bash script/check.sh` is the only full gate: docs index → `cargo fmt --check` → `cargo test` → `astro build`. Run it before handoff and never claim a check you did not run. |
| 13 | Branching | NEVER commit or push directly to `main`. Every change MUST go through a `feature_<short-name>` branch and a merge request targeting `main`. |
| 14 | Merge Authority | Agents MUST NOT merge pull requests, enable auto-merge, or push to `main`. After CI passes, report the PR URL and stop; the repository owner merges in the GitHub web UI. |

## Baseline

- Two processes: Rust + Axum API in `backend/` on `:8787`; Astro 7 SSR in `frontend/` on `:4321`. npm workspaces contains only `frontend`; `backend` is plain Cargo.
- `bash script/dev.sh` (or `npm run dev`) starts both; `bash script/check.sh` (or `npm run check`) is the gate.
- Local data: `backend/data/blog.db` and `backend/data/storage/`. Logs: `log/backend/YYYY-MM-DD.log` and `log/frontend/YYYY-MM-DD.log`.
- Run the backend from `backend/` or set `BLOG_DATA_DIR`; the default data dir is relative to the current working directory. `script/dev.sh` pins it to `backend/data/`.
- Astro 7 daemonizes in non-interactive shells. Stop strays with `cd frontend && npx astro dev stop`; inspect them with `npx astro dev logs`.
- Single backend test: `cd backend && cargo test <name>`. Backend tests are inline `#[cfg(test)]` modules under `backend/src/services/`; there are no frontend tests.
- CI (`.github/workflows/check.yml`) runs `bash script/check.sh` on pull requests and `main`. Keep it green before merging; only the repository owner merges PRs in the web UI.

## Before Coding

1. Read `docs/index.toml` and only the docs that govern the affected boundary.
2. Search nearby files for existing patterns before adding an abstraction, dependency, or module.
3. Check the working tree first. Treat unfamiliar or modified files as user work; NEVER revert them without an explicit request.
4. For backend service or core logic changes, inspect existing tests and add focused coverage when the behavior is not covered.

## While Coding

- Keep edits scoped to the request; do not mix unrelated refactors.
- Use structured parsers/APIs when available; do not hand-parse structured data with string operations.
- Add an abstraction only when it removes real complexity or matches an established project pattern.
- When a change adds, removes, or renames a route, field, module, table, log target, command, or convention, update the docs and both sides of the contract in the same change.
- For Git commits, follow `Git Workflow` below.

## Git Workflow

- `main` is the stable branch. NEVER commit or push directly to `main`; every change goes through a local `feature_<short-name>` branch and an MR targeting `main`. Do not create `preview` or release branches.
- Start from the latest `main`: `git checkout main && git pull --rebase origin main && git checkout -b feature_<short-name>`.
- Keep history linear. Rebase onto `origin/main` before opening or updating the MR: `git fetch origin && git rebase origin/main`. Resolve conflicts during rebase; never merge `main` into the feature branch and never create a merge commit.
- Push the feature branch and open the MR only when the user asks to deliver/merge. If a pushed branch was rebased, update it with `git push --force-with-lease origin feature_<short-name>`; never force-push `main`.
- NEVER merge the MR yourself. CI runs the `Check` workflow when the PR is opened or updated, validating the merge result against `main`; after it passes, report the PR URL and stop. The repository owner merges in the GitHub web UI with rebase. Do not use `gh pr merge`, the merge API, or auto-merge.
- After the owner confirms the merge, update local `main` with `git checkout main && git pull --rebase origin main` and delete the local feature branch.
- Run `bash script/setup-git-hooks.sh` once per clone. The local `pre-push` hook blocks direct pushes to `main`; GitHub branch protection is unavailable for private repositories on the free plan, so CI also runs a `guard-main` check after push.
- When the user requests a commit, use a symbol-level message: a one-line purpose, then one bullet per added, modified, or deleted named symbol explaining why. File-level bullets are allowed only for artifacts without named code symbols (docs, SQL, config, generated assets).

```text
<type>(<scope>): <one-line purpose>

<Module or domain>:
- <AddedOrChangedSymbol>(): <why it changed>
- <DeletedSymbol>(): removed -- <reason>
- <file-or-artifact>: <why>  # only when no named symbol
```

Do not add AI/model/tool co-author trailers.

## Before Handoff

1. Run `bash script/check.sh`.
2. If it fails, fix the cause before reporting completion.
3. Report changed files, validation results, and any known limitation or unrun check. Do not describe the local MVP as production-ready.

## Documentation Routing

| If the task touches... | Read |
|---|---|
| Local runtime, layering, API contract | `docs/03-开发规范/01-本地实现说明.md` |
| Auth, secrets, upload safety, headers | `docs/03-开发规范/02-认证与安全.md` |
| Tests, quality gate, manual acceptance | `docs/03-开发规范/03-测试方案.md` |
| Logs, retention, format | `docs/03-开发规范/04-日志规范.md` |
| Architecture, data model, routes | `docs/02-架构设计/` |
| Startup, environment, deployment | `docs/04-运维部署/` |

`docs/index.toml` is the canonical index. When adding, deleting, renaming, or moving a document, update the index in the same change.
