#!/usr/bin/env bash
set -euo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

target=""
vm_name=""

usage() {
  cat <<'EOF'
Usage: ./scripts/build-and-test.sh [--target <triple>] [--vm <name>]

Options:
  --target <triple>  Cross-compile for a *-unknown-linux-musl triple (e.g.
                     aarch64-unknown-linux-musl) instead of the host. Builds
                     the release binary AND compiles the test suite for the
                     target (cargo test --no-run), and verifies the binary is
                     fully static.
  --vm <name>        After a cross build, push the binary into a running
                     sandmux VM named <name> and run a live smoke test
                     (replmux --version + kernel create/exec). Requires the
                     sandmux daemon; skipped with a warning if unavailable.
  -h, --help         Show this help.
EOF
}

require_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    printf 'error: required command not found: %s\n' "${command_name}" >&2
    exit 127
  fi
}

parse_arguments() {
  while (( $# > 0 )); do
    case "$1" in
      --target)
        if (( $# < 2 )); then
          printf 'error: --target requires an argument\n' >&2
          exit 2
        fi
        target="$2"
        shift
        ;;
      --vm)
        if (( $# < 2 )); then
          printf 'error: --vm requires an argument\n' >&2
          exit 2
        fi
        vm_name="$2"
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        printf 'error: unknown argument: %s\n' "$1" >&2
        usage >&2
        exit 2
        ;;
    esac
    shift
  done
}

prepare_kernel_environment() {
  printf '\n==> Preparing Rust kernel test environment\n'
  uv sync --project "${REPO_ROOT}/cli/tests" --locked
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
  if [[ -n "${target}" ]]; then
    # shellcheck source=lib-cross.sh
    source "${SCRIPT_DIR}/lib-cross.sh"
    export_cross_env "${target}"
    local binary="${REPO_ROOT}/cli/target/${target}/release/replmux"
    printf '\n==> Building Rust CLI (release, cross target %s)\n' "${target}"
    (
      cd "${REPO_ROOT}/cli"
      cargo build --release --locked --target "${target}"
      printf '\n==> Compiling Rust CLI tests for %s (--no-run)\n' "${target}"
      cargo test --release --locked --target "${target}" --no-run
    )
    verify_static_binary "${binary}"
    if [[ -n "${vm_name}" ]]; then
      live_smoke_test "${vm_name}" "${binary}"
    fi
  else
    printf '\n==> Building Rust CLI (release)\n'
    (
      cd "${REPO_ROOT}/cli"
      cargo build --release --locked

      printf '\n==> Testing Rust CLI (release)\n'
      cargo test --release --locked
    )
  fi
}

# Push a cross-built binary into a running sandmux VM and smoke-test it:
# transfer over HTTP via the VM's NAT gateway, then replmux --version and a
# kernel create/exec/delete round-trip. Skipped (with a note) when sandmux or
# the VM is unavailable.
live_smoke_test() {
  local vm="$1" binary="$2"
  local sandmux="${SANMUX_BIN:-sandmux}"
  if ! command -v "${sandmux}" >/dev/null 2>&1; then
    printf '\n==> Live smoke test: skipping (sandmux not on PATH; set SANMUX_BIN)\n'
    return 0
  fi
  if ! "${sandmux}" vm list 2>/dev/null | grep -qw "${vm}"; then
    printf '\n==> Live smoke test: skipping (sandmux VM "%s" not running)\n' "${vm}"
    return 0
  fi

  local gateway port dir http_pid version
  gateway="$("${sandmux}" vm exec "${vm}" ip route | awk '/default via/{print $3; exit}')"
  if [[ -z "${gateway}" ]]; then
    printf '\n==> Live smoke test: skipping (no default route in VM)\n'
    return 0
  fi

  port="${SANMUX_SMOKE_PORT:-8127}"
  dir="$(mktemp -d)"
  cp "${binary}" "${dir}/replmux"
  (cd "${dir}" && exec python3 -m http.server "${port}" --bind 0.0.0.0 >/dev/null 2>&1) &
  http_pid=$!
  sleep 1

  printf '\n==> Live smoke test in sandmux VM "%s" (via %s)\n' "${vm}" "${gateway}"
  "${sandmux}" vm exec "${vm}" --timeout 60 wget -qO /usr/local/bin/replmux "http://${gateway}:${port}/replmux"
  "${sandmux}" vm exec "${vm}" --timeout 60 chmod +x /usr/local/bin/replmux
  version="$("${sandmux}" vm exec "${vm}" --timeout 30 replmux --version)"
  printf '    %s\n' "${version}"
  "${sandmux}" vm exec "${vm}" --timeout 60 sh -c "'replmux kernel create smoke && replmux kernel exec smoke \"print(6*7)\" && replmux kernel delete smoke'"

  kill "${http_pid}" 2>/dev/null || true
  rm -rf -- "${dir}"
}

main() {
  parse_arguments "$@"
  # Allow SANMUX_VM as a default for --vm (mirrors SANMUX_BIN for the binary).
  if [[ -z "${vm_name}" && -n "${SANMUX_VM:-}" ]]; then
    vm_name="${SANMUX_VM}"
  fi

  require_command uv
  require_command cargo

  prepare_kernel_environment
  run_service_tests
  build_and_test_cli

  printf '\nBuild and test commands completed.\n'
  if [[ -n "${target}" ]]; then
    printf 'CLI artifact: %s\n' "${REPO_ROOT}/cli/target/${target}/release/replmux"
  else
    printf 'CLI artifact: %s\n' "${REPO_ROOT}/cli/target/release/replmux"
  fi
}

main "$@"
