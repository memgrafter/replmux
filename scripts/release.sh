#!/usr/bin/env bash
set -euo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
readonly CLI_MANIFEST="${REPO_ROOT}/cli/Cargo.toml"
readonly RELEASE_BINARY="${REPO_ROOT}/cli/target/release/replmux"
readonly OUTPUT_DIR="${REPLMUX_RELEASE_DIR:-${REPO_ROOT}/dist}"

staging_dir=""
fast_mode=false

usage() {
  cat <<'EOF'
Usage: ./scripts/release.sh [--fast]

Options:
  --fast  Package the existing release binary without cleaning, building, or testing.
  -h, --help
          Show this help.
EOF
}

parse_arguments() {
  while (( $# > 0 )); do
    case "$1" in
      --fast)
        fast_mode=true
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

cleanup() {
  if [[ -n "${staging_dir}" && -d "${staging_dir}" ]]; then
    rm -rf -- "${staging_dir}"
  fi
}
trap cleanup EXIT

require_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    printf 'error: required command not found: %s\n' "${command_name}" >&2
    exit 127
  fi
}

package_version() {
  awk '
    /^\[package\]$/ { in_package = 1; next }
    /^\[/ { in_package = 0 }
    in_package && /^version = / {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' "${CLI_MANIFEST}"
}

host_target() {
  rustc -vV | awk '/^host:/ { print $2 }'
}

verify_bundled_zmq() {
  local dependencies=""
  case "$(uname -s)" in
    Darwin)
      require_command otool
      dependencies="$(otool -L "${RELEASE_BINARY}")"
      ;;
    Linux)
      require_command ldd
      dependencies="$(ldd "${RELEASE_BINARY}" 2>&1 || true)"
      ;;
    *)
      printf 'error: unsupported platform for static ZeroMQ verification\n' >&2
      exit 1
      ;;
  esac
  if grep -Eiq '(^|[/[:space:]])libzmq([.[:space:]]|$)' <<<"${dependencies}"; then
    printf 'error: release binary dynamically links libzmq; static bundle is required\n' >&2
    printf '%s\n' "${dependencies}" >&2
    exit 1
  fi
  printf 'Verified: libzmq is statically bundled in the Rust CLI.\n'
}

write_checksum() {
  local archive_path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$(dirname -- "${archive_path}")" && sha256sum "$(basename -- "${archive_path}")") \
      > "${archive_path}.sha256"
  elif command -v shasum >/dev/null 2>&1; then
    (cd "$(dirname -- "${archive_path}")" && shasum -a 256 "$(basename -- "${archive_path}")") \
      > "${archive_path}.sha256"
  else
    printf 'error: sha256sum or shasum is required\n' >&2
    exit 127
  fi
}

main() {
  parse_arguments "$@"

  require_command awk
  require_command grep
  require_command rustc
  require_command tar
  require_command uname

  if [[ "${fast_mode}" == true ]]; then
    if [[ ! -x "${RELEASE_BINARY}" ]]; then
      printf 'error: --fast requires an existing release binary: %s\n' "${RELEASE_BINARY}" >&2
      exit 1
    fi
    printf '\n==> Fast mode: using existing untested release binary\n'
  else
    require_command cargo
    require_command uv
    printf '\n==> Cleaning Rust CLI build artifacts\n'
    cargo clean --manifest-path "${CLI_MANIFEST}"
    "${SCRIPT_DIR}/build-and-test.sh"
  fi

  verify_bundled_zmq

  local version target release_name archive_path bundle_dir
  version="$(package_version)"
  target="$(host_target)"
  if [[ -z "${version}" || -z "${target}" ]]; then
    printf 'error: could not determine package version or Rust host target\n' >&2
    exit 1
  fi

  release_name="replmux-v${version}-${target}"
  archive_path="${OUTPUT_DIR}/${release_name}.tar.gz"
  mkdir -p -- "${OUTPUT_DIR}"
  staging_dir="$(mktemp -d "${OUTPUT_DIR}/.${release_name}.XXXXXX")"
  bundle_dir="${staging_dir}/${release_name}"
  mkdir -p -- "${bundle_dir}"

  install -m 0755 "${RELEASE_BINARY}" "${bundle_dir}/replmux"
  install -m 0644 "${REPO_ROOT}/cli/assets/minimal_kernel_clean.py" "${bundle_dir}/minimal_kernel_clean.py"
  install -m 0644 "${REPO_ROOT}/cli/README.md" "${bundle_dir}/README.md"

  tar -C "${staging_dir}" -czf "${archive_path}.tmp" "${release_name}"
  mv -f -- "${archive_path}.tmp" "${archive_path}"
  write_checksum "${archive_path}"

  printf '\nRelease package created.\n'
  printf 'Archive:  %s\n' "${archive_path}"
  printf 'Checksum: %s\n' "${archive_path}.sha256"
  printf '\nThe target system needs Python 3 with pyzmq to run local kernels.\n'
}

main "$@"
