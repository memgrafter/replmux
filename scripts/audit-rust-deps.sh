#!/usr/bin/env bash
set -uo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
readonly CLI_DIR="${REPO_ROOT}/cli"

failures=0

now() {
  date -u '+%Y-%m-%dT%H:%M:%SZ'
}

ensure_tool() {
  local subcommand="$1"
  local package="$2"
  if cargo "${subcommand}" --version >/dev/null 2>&1; then
    printf '[%s] FOUND %s\n' "$(now)" "${package}"
    return 0
  fi

  printf '[%s] MISSING %s; installing once\n' "$(now)" "${package}"
  run_timed "install ${package}" cargo install --locked "${package}"
  cargo "${subcommand}" --version >/dev/null 2>&1
}

write_deny_policy() {
  cat > "${CLI_DIR}/deny.toml" <<'EOF'
[graph]
all-features = true

[advisories]
ignore = []

[licenses]
allow = [
  "Apache-2.0",
  "Apache-2.0 WITH LLVM-exception",
  "BSD-3-Clause",
  "CDLA-Permissive-2.0",
  "ISC",
  "MIT",
  "Unicode-3.0",
  "Unlicense",
]
confidence-threshold = 0.8

[bans]
multiple-versions = "warn"
wildcards = "deny"
highlight = "simplest-path"
workspace-default-features = "allow"
external-default-features = "allow"
allow = []
deny = []
skip = []
skip-tree = []

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []

[sources.allow-org]
github = []
gitlab = []
bitbucket = []
EOF
}

initialize_policies() {
  if [[ -f "${CLI_DIR}/deny.toml" ]]; then
    printf '[%s] FOUND cli/deny.toml\n' "$(now)"
  else
    run_timed 'initialize cargo-deny policy' write_deny_policy
  fi

  if [[ -f "${CLI_DIR}/supply-chain/config.toml" ]]; then
    printf '[%s] FOUND cli/supply-chain/config.toml\n' "$(now)"
  else
    run_timed 'initialize cargo-vet policy' cargo vet init --locked
  fi
}

run_timed() {
  local label="$1"
  shift

  local started_at started_epoch finished_at finished_epoch status duration
  started_at="$(now)"
  started_epoch="$(date +%s)"
  printf '\n[%s] START %s\n' "${started_at}" "${label}"

  "$@"
  status=$?

  finished_at="$(now)"
  finished_epoch="$(date +%s)"
  duration=$((finished_epoch - started_epoch))
  if ((status == 0)); then
    printf '[%s] PASS  %s (%ss)\n' "${finished_at}" "${label}" "${duration}"
  else
    printf '[%s] FAIL  %s (%ss, exit %s)\n' \
      "${finished_at}" "${label}" "${duration}" "${status}" >&2
    failures=$((failures + 1))
  fi
}

main() {
  local missing=0
  command -v cargo >/dev/null 2>&1 || {
    printf '[%s] MISSING cargo\n' "$(now)" >&2
    exit 127
  }

  ensure_tool audit cargo-audit || missing=1
  ensure_tool deny cargo-deny || missing=1
  ensure_tool vet cargo-vet || missing=1
  if ((missing != 0)); then
    printf '\n[%s] One or more required audit tools could not be installed.\n' "$(now)" >&2
    exit 127
  fi

  cd "${CLI_DIR}"
  initialize_policies
  if [[ ! -f deny.toml || ! -f supply-chain/config.toml ]]; then
    printf '[%s] Required dependency policy initialization failed.\n' "$(now)" >&2
    exit 1
  fi

  printf '[%s] Rust dependency security checks: %s\n' "$(now)" "${CLI_DIR}"

  run_timed 'cargo audit' cargo audit
  run_timed 'cargo deny check' cargo deny check
  run_timed 'cargo vet' cargo vet

  printf '\n[%s] Completed Rust dependency security checks (%s failure(s)).\n' \
    "$(now)" "${failures}"
  ((failures == 0))
}

main "$@"
