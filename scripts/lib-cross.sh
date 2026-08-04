#!/usr/bin/env bash
# Cross-compilation support for the replmux Rust CLI: builds fully-static musl
# Linux binaries with clang + an Alpine aarch64/x86_64 sysroot.
#
# Sourced by release.sh (--static/--arch) and build-and-test.sh (--target).
#
#   source "$SCRIPT_DIR/lib-cross.sh"
#   export_cross_env <rust-target-triple>   # sets CC/CXX/AR/RUSTFLAGS/... for
#                                           # the target (must be *-linux-musl)
#   cross_sysroot <arch>                    # -> sysroot path (downloads if absent)
#
# The zmq-sys crate vendors libzmq and compiles it from C++ for the target, so
# the host needs clang + lld + a musl sysroot. The sysroot is assembled from
# Alpine APK packages (dl-cdn.alpinelinux.org): musl-dev, linux-headers,
# libstdc++(-dev), libgcc, gcc. macOS: brew install llvm lld.

set -euo pipefail

readonly ALPINE_MIRROR="${ALPINE_MIRROR:-https://dl-cdn.alpinelinux.org/alpine}"
readonly ALPINE_VERSION="${ALPINE_VERSION:-v3.20}"
readonly CROSS_CACHE="${CROSS_CACHE:-${HOME}/.cache/replmux-cross}"

# Normalize an arch string (aarch64|arm64|x86_64|amd64) to the Alpine spelling.
normalize_arch() {
  case "$1" in
    aarch64|arm64) echo aarch64 ;;
    x86_64|amd64)  echo x86_64 ;;
    *)             return 1 ;;
  esac
}

triple_arch() {
  case "$1" in
    aarch64-*) echo aarch64 ;;
    x86_64-*)  echo x86_64 ;;
    *)         return 1 ;;
  esac
}

cross_require_tools() {
  local missing=() name
  for name in clang llvm-ar curl; do
    command -v "$name" >/dev/null 2>&1 || missing+=("$name")
  done
  command -v rust-lld >/dev/null 2>&1 || command -v ld.lld >/dev/null 2>&1 || missing+=(ld.lld)
  # macOS: the LLVM toolchain ships in Homebrew's llvm/lld formulae.
  if [[ "$(uname -s)" == "Darwin" ]] && (( ${#missing[@]} > 0 )); then
    local llvm_bin lld_bin
    llvm_bin="$(brew --prefix llvm 2>/dev/null)/bin"
    lld_bin="$(brew --prefix lld 2>/dev/null)/bin"
    if [[ -x "${llvm_bin}/clang" ]]; then
      export PATH="${llvm_bin}:${lld_bin}:${PATH}"
      missing=()
    fi
  fi
  if (( ${#missing[@]} > 0 )); then
    printf 'error: cross-compilation requires: %s\n' "${missing[*]}" >&2
    printf '  macOS: brew install llvm lld\n' >&2
    exit 127
  fi
}

# Print the Alpine APK version for a package in <arch>/main.
apk_version() {
  local pkg="$1" arch="$2" idx="${CROSS_CACHE}/apkindex-${arch}"
  if [[ ! -f "$idx" ]]; then
    mkdir -p "$CROSS_CACHE"
    curl -fsSL "${ALPINE_MIRROR}/${ALPINE_VERSION}/main/${arch}/APKINDEX.tar.gz" -o "${idx}.tar.gz"
    tar -xzf "${idx}.tar.gz" -C "$CROSS_CACHE" APKINDEX
    mv -f "${CROSS_CACHE}/APKINDEX" "$idx"
    rm -f "${idx}.tar.gz"
  fi
  awk -v p="P:${pkg}" '$0 == p { found = 1; next } found && /^V:/ { print substr($0, 3); exit }' "$idx"
}

# Download + extract the musl sysroot for an arch into ${CROSS_CACHE}/<arch>.
cross_sysroot() {
  local arch="$1" dir="${CROSS_CACHE}/${arch}" p ver url
  normalize_arch "$arch" >/dev/null || {
    printf 'error: unsupported cross arch: %s (want aarch64|x86_64)\n' "$arch" >&2
    exit 2
  }
  if [[ -f "$dir/.complete" ]]; then
    echo "$dir"
    return
  fi
  cross_require_tools
  mkdir -p "$dir"
  printf '==> Assembling %s musl sysroot (Alpine %s)...\n' "$arch" "$ALPINE_VERSION"
  for p in musl-dev linux-headers libstdc++-dev libstdc++ libgcc gcc; do
    ver="$(apk_version "$p" "$arch")"
    if [[ -z "$ver" ]]; then
      printf 'error: no %s version in %s/main/%s\n' "$p" "$ALPINE_VERSION" "$arch" >&2
      exit 1
    fi
    curl -fsSL "${ALPINE_MIRROR}/${ALPINE_VERSION}/main/${arch}/${p}-${ver}.apk" -o "$dir/${p}.apk"
    tar -xzf "$dir/${p}.apk" -C "$dir"
    rm -f "$dir/${p}.apk"
  done
  touch "$dir/.complete"
  echo "$dir"
}

# Export the environment needed to build for a rust target triple
# (*-unknown-linux-musl). Requires clang/llvm-ar/rust-lld and the Alpine
# sysroot (downloaded on first use).
export_cross_env() {
  local triple="$1" arch envsuffix sysroot gccdir cxxinc cxxinc_arch
  arch="$(triple_arch "$triple")" || {
    printf 'error: unsupported cross target: %s (want aarch64-... or x86_64-...)\n' "$triple" >&2
    exit 2
  }
  case "$triple" in
    *-linux-musl) ;;
    *)
      printf 'error: static cross builds require a musl target, got: %s\n' "$triple" >&2
      exit 2
      ;;
  esac
  cross_require_tools
  local sysroot gccdir cxxinc cxxinc_arch pattern envsuffix
  sysroot="$(cross_sysroot "$arch")"
  # Resolve the versioned gcc/C++ include dirs via an unquoted pattern
  # (inline quotes around glob chars suppress pathname expansion).
  pattern="${sysroot}/usr/lib/gcc/"*"/"*"/"
  gccdir="$(echo ${pattern} | awk '{print $1}')"
  pattern="${sysroot}/usr/include/c++/"*"/"
  cxxinc="$(echo ${pattern} | awk '{print $1}')"
  pattern="${sysroot}/usr/include/c++/"*"/${arch}-alpine-linux-musl"
  cxxinc_arch="$(echo ${pattern} | awk '{print $1}')"
  envsuffix="${triple//-/_}"

  export "CC_${envsuffix}=clang --target=${triple} --sysroot=${sysroot}"
  export "CXX_${envsuffix}=clang++ --target=${triple} --sysroot=${sysroot} -isystem ${cxxinc} -isystem ${cxxinc_arch}"
  export "AR_${envsuffix}=llvm-ar"
  export "CFLAGS_${envsuffix}=-target ${triple}"
  export "CXXFLAGS_${envsuffix}=-target ${triple}"
  export PKG_CONFIG_ALLOW_CROSS=1
  export PKG_CONFIG_PATH="${sysroot}/usr/lib/pkgconfig"
  export PKG_CONFIG_SYSROOT_DIR="${sysroot}"
  export RUSTFLAGS="-C linker=rust-lld -C target-feature=+crt-static -C link-arg=-L${sysroot}/usr/lib -C link-arg=-L${gccdir}"
}

# Verify a built artifact is a fully static ELF (nothing dynamic at all, so
# libzmq is certainly bundled).
verify_static_binary() {
  local binary="$1" out
  out="$(file "$binary")"
  if grep -q "statically linked" <<<"$out"; then
    printf 'Verified: %s is fully static\n' "$binary"
  else
    printf 'error: expected a fully static binary, got: %s\n' "$out" >&2
    exit 1
  fi
}
