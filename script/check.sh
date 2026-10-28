#!/usr/bin/env bash
# 本地质量检查：文档索引、Rust 格式、后端测试、前端构建。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export PATH="$HOME/.cargo/bin:$PATH"

echo "[1/4] docs/index.toml"
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

echo "[2/4] cargo fmt --check"
(cd backend && cargo fmt --check)

echo "[3/4] cargo test"
(cd backend && cargo test)

echo "[4/4] astro build"
npm --workspace frontend run build

echo "全部检查通过。"
