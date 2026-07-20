#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO="$(dirname "$SCRIPT_DIR")"
VENV_PYTHON="${SCRIPT_DIR}/.venv/bin/python"

# Fall back to system python if venv missing
if [ ! -x "$VENV_PYTHON" ]; then
  VENV_PYTHON="$(command -v python3)"
fi

KERNEL_NAME="${1:-lifecycle-test}"

# Clean up any stale kernel
python3 "${REPO}/jupyter_repl_cli.py" delete "$KERNEL_NAME" 2>/dev/null || true

exec "$VENV_PYTHON" "${SCRIPT_DIR}/lifecycle.py" "$KERNEL_NAME"