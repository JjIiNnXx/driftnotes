#!/usr/bin/env bash
# Enable the repository-local Git hooks. Run once per clone.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

git config core.hooksPath .githooks
chmod +x .githooks/pre-push

echo "Git hooks enabled: core.hooksPath=.githooks"
echo "Direct pushes to main are now blocked locally."
