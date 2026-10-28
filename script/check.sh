#!/usr/bin/env bash
# 本地质量检查：文档索引、Rust 格式、Clippy、后端测试、前端类型检查与构建。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export PATH="$HOME/.cargo/bin:$PATH"

echo "[1/6] docs/index.toml"
python3 - <<'PY'
import pathlib
import tomllib

root = pathlib.Path("docs")
index = tomllib.loads((root / "index.toml").read_text(encoding="utf-8"))
missing = [
    str(root / document["path"])
    for group in index["groups"]
    for document in group["documents"]
    if not (root / document["path"]).is_file()
]
if missing:
    raise SystemExit("文档索引指向不存在的文件：" + ", ".join(missing))
print(f"  {len(index['groups'])} 个分组，{sum(len(g['documents']) for g in index['groups'])} 篇文档")
PY

echo "[2/6] cargo fmt --check"
(cd backend && cargo fmt --check)

echo "[3/6] cargo clippy"
(cd backend && cargo clippy --all-targets -- -D warnings)

echo "[4/6] cargo test"
(cd backend && cargo test)

echo "[5/6] astro check"
npm --workspace frontend run check

echo "[6/6] astro build"
npm --workspace frontend run build

echo "全部检查通过。"
