#!/usr/bin/env bash
# 一键启动本地博客：Rust API + Astro SSR。
# 用法：bash script/dev.sh（或 npm run dev）
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
mkdir -p "$ROOT/log/backend" "$ROOT/log/frontend"

export PATH="$HOME/.cargo/bin:$PATH"
export BLOG_DATA_DIR="${BLOG_DATA_DIR:-$ROOT/backend/data}"
export BLOG_LOG_DIR="${BLOG_LOG_DIR:-$ROOT/log}"
export FRONTEND_LOG_DIR="${FRONTEND_LOG_DIR:-$ROOT/log/frontend}"
BACKEND_URL="${BACKEND_URL:-http://127.0.0.1:8787}"
FRONTEND_URL="${FRONTEND_URL:-http://127.0.0.1:4321}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "未找到 cargo。请先安装 Rust：https://rustup.rs/" >&2
  exit 1
fi
if ! command -v npm >/dev/null 2>&1; then
  echo "未找到 npm。请先安装 Node.js 22.23 或更高版本。" >&2
  exit 1
fi

if [ ! -d "$ROOT/node_modules" ]; then
  echo "首次运行：安装前端依赖…"
  npm install
fi

BACKEND_PID=""
FRONTEND_PID=""
FRONTEND_STARTED=0

cleanup() {
  trap - EXIT INT TERM
  echo
  echo "正在停止本地服务…"
  if [ "$FRONTEND_STARTED" = "1" ]; then
    [ -n "$FRONTEND_PID" ] && kill "$FRONTEND_PID" 2>/dev/null || true
    # Astro 7 在非交互环境会把 dev server 放入后台；即使 PID 已退出，
    # 也要通过官方命令停掉后台进程。
    (cd "$ROOT/frontend" && npx --no-install astro dev stop >/dev/null 2>&1) || true
  fi
  [ -n "$BACKEND_PID" ] && kill "$BACKEND_PID" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

if curl -fsS "$BACKEND_URL/api/health" >/dev/null 2>&1; then
  echo "检测到后端已在 $BACKEND_URL 运行，将直接复用。"
else
  echo "启动 Rust 后端（首次编译可能需要几分钟）…"
  (cd backend && cargo run) &
  BACKEND_PID=$!

  for _ in $(seq 1 900); do
    if curl -fsS "$BACKEND_URL/api/health" >/dev/null 2>&1; then
      break
    fi
    if ! kill -0 "$BACKEND_PID" 2>/dev/null; then
      echo "后端启动失败，请查看上方 cargo 输出。" >&2
      exit 1
    fi
    sleep 1
  done

  if ! curl -fsS "$BACKEND_URL/api/health" >/dev/null 2>&1; then
    echo "等待后端超时，请查看上方 cargo 输出。" >&2
    exit 1
  fi
fi

if curl -fsS "$FRONTEND_URL/" >/dev/null 2>&1; then
  echo "检测到前端已在 $FRONTEND_URL 运行，将直接复用。"
else
  echo "启动 Astro 前端…"
  (cd frontend && npm run dev) &
  FRONTEND_PID=$!
  FRONTEND_STARTED=1
fi

for _ in $(seq 1 120); do
  if curl -fsS "$FRONTEND_URL/" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! curl -fsS "$FRONTEND_URL/" >/dev/null 2>&1; then
  echo "等待前端超时，请运行 cd frontend && npx astro dev logs 查看诊断日志。" >&2
  exit 1
fi

cat <<EOF

  漂流笔记 · 本地开发环境已启动

  博客首页   $FRONTEND_URL
  管理后台   $FRONTEND_URL/admin
  Rust API   $BACKEND_URL/api/health

  SQLite     $BLOG_DATA_DIR/blog.db
  图片目录   $BLOG_DATA_DIR/storage
  后端日志   $BLOG_LOG_DIR/backend/YYYY-MM-DD.log
  前端日志   $FRONTEND_LOG_DIR/YYYY-MM-DD.log

  日志按天分文件，保留 3 天，每 72 小时清理一次。
  按 Ctrl+C 停止全部服务。
EOF

if [ -n "$BACKEND_PID" ]; then
  wait "$BACKEND_PID"
else
  while sleep 3600; do :; done
fi
