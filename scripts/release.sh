#!/usr/bin/env bash
set -euo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
readonly CLI_MANIFEST="${REPO_ROOT}/cli/Cargo.toml"
readonly OUTPUT_DIR="${REPLMUX_RELEASE_DIR:-${REPO_ROOT}/dist}"

staging_dir=""
fast_mode=false
static_mode=false
arch_flag=""

usage() {
  cat <<'EOF'
Usage: ./scripts/release.sh [--static] [--arch aarch64|x86_64] [--fast]

Options:
  --static  Build a fully-static musl Linux binary instead of the host
            binary. Target becomes <arch>-unknown-linux-musl. Requires
            clang/lld (macOS: brew install llvm lld); the Alpine musl sysroot
            for the target arch is downloaded on first use.
  --arch <aarch64|x86_64>
            Target architecture (implies --static; defaults to the host
            architecture).
  --fast    Package the existing release binary without cleaning, building, or
            testing.
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
      --static)
        static_mode=true
        ;;
      --arch)
        if (( $# < 2 )); then
          printf 'error: --arch requires an argument (aarch64|x86_64)\n' >&2
          exit 2
        fi
        arch_flag="$2"
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
      dependencies="$(otool -L "${release_binary}")"
      ;;
    Linux)
      require_command ldd
      dependencies="$(ldd "${release_binary}" 2>&1 || true)"
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

  local version target
  version="$(package_version)"
  target="$(host_target)"

  if [[ -n "${arch_flag}" && "${static_mode}" != true ]]; then
    printf 'error: --arch requires --static (cross-arch native builds are not supported)\n' >&2
    exit 2
  fi
  if [[ "${static_mode}" == true ]]; then
    # shellcheck source=lib-cross.sh
    source "${SCRIPT_DIR}/lib-cross.sh"
    local arch
    if [[ -n "${arch_flag}" ]]; then
      arch="$(normalize_arch "${arch_flag}")" || {
        printf 'error: --arch must be aarch64 or x86_64, got: %s\n' "${arch_flag}" >&2
        exit 2
      }
    else
      arch="$(uname -m | sed 's/^arm64$/aarch64/')"
    fi
    target="${arch}-unknown-linux-musl"
    export_cross_env "${target}"
  fi

  if [[ -z "${version}" || -z "${target}" ]]; then
    printf 'error: could not determine package version or Rust host target\n' >&2
    exit 1
  fi

  release_binary="${REPO_ROOT}/cli/target/${target}/release/replmux"

  if [[ "${fast_mode}" == true ]]; then
    if [[ ! -x "${release_binary}" ]]; then
      printf 'error: --fast requires an existing release binary: %s\n' "${release_binary}" >&2
      exit 1
    fi
    printf '\n==> Fast mode: using existing untested release binary\n'
  else
    require_command cargo
    require_command uv
    printf '\n==> Cleaning Rust CLI build artifacts\n'
    cargo clean --manifest-path "${CLI_MANIFEST}"
    if [[ "${static_mode}" == true ]]; then
      "${SCRIPT_DIR}/build-and-test.sh" --target "${target}"
    else
      "${SCRIPT_DIR}/build-and-test.sh"
    fi
  fi

  if [[ "${static_mode}" == true ]]; then
    verify_static_binary "${release_binary}"
  else
    verify_bundled_zmq
  fi

  local release_name archive_path bundle_dir
  release_name="replmux-v${version}-${target}"
  archive_path="${OUTPUT_DIR}/${release_name}.tar.gz"
  mkdir -p -- "${OUTPUT_DIR}"
  staging_dir="$(mktemp -d "${OUTPUT_DIR}/.${release_name}.XXXXXX")"
  bundle_dir="${staging_dir}/${release_name}"
  mkdir -p -- "${bundle_dir}"

  install -m 0755 "${release_binary}" "${bundle_dir}/replmux"
  install -m 0644 "${REPO_ROOT}/cli/assets/python_minimal_kernel.py" "${bundle_dir}/python_minimal_kernel.py"
  install -m 0644 "${REPO_ROOT}/cli/README.md" "${bundle_dir}/README.md"

  tar -C "${staging_dir}" -czf "${archive_path}.tmp" "${release_name}"
  mv -f -- "${archive_path}.tmp" "${archive_path}"
  write_checksum "${archive_path}"

  printf '\nRelease package created.\n'
  printf 'Archive:  %s\n' "${archive_path}"
  printf 'Checksum: %s\n' "${archive_path}.sha256"
  if [[ "${static_mode}" == true ]]; then
    printf '\nThe target system needs Python 3 with pyzmq (libzmq is bundled in the binary).\n'
  else
    printf '\nThe target system needs Python 3 with pyzmq to run local kernels.\n'
  fi
}

main "$@"
