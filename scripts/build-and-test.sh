#!/usr/bin/env bash
set -euo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

require_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    printf 'error: required command not found: %s\n' "${command_name}" >&2
    exit 127
  fi
}

run_service_tests() {
  printf '\n==> Testing FastAPI service\n'
  (
    cd "${REPO_ROOT}/service"
    uv sync --locked --dev
    uv run --locked pytest
  )
}

build_and_test_cli() {
  printf '\n==> Building Rust CLI (release)\n'
  (
    cd "${REPO_ROOT}/cli"
    cargo build --release --locked

    printf '\n==> Testing Rust CLI (release)\n'
    cargo test --release --locked
  )
}

main() {
  require_command uv
  require_command cargo

  run_service_tests
  build_and_test_cli

  printf '\nBuild and test commands completed.\n'
  printf 'CLI artifact: %s\n' "${REPO_ROOT}/cli/target/release/multirepl"
}

main "$@"
