#!/usr/bin/env bash
# Installs the two build tools into tools/.bin (git-ignored), from the official release tarballs with pinned SHA-256 (Linux x86_64):
# wasm-bindgen CLI (the version of Cargo.lock) and wasm-opt of binaryen version_133 (the one mzLab uses). A tool already on PATH at the
# right version is left alone.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd); bin="$here/.bin"; mkdir -p "$bin"
ver=$(grep -A1 'name = "wasm-bindgen"$' "$here/../Cargo.lock" | sed -n 's/version = "\(.*\)"/\1/p')
fetch() { # url sha256 file
  curl -sSfL --retry 4 -o "$3" "$1"
  echo "$2  $3" | sha256sum -c - >/dev/null
}
if ! "$bin/wasm-bindgen" --version 2>/dev/null | grep -q "$ver"; then
  [ "$ver" = "0.2.129" ] || { echo "install_tools: pin the SHA-256 of wasm-bindgen $ver here" >&2; exit 2; }
  t=$(mktemp -d)
  fetch "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$ver/wasm-bindgen-$ver-x86_64-unknown-linux-musl.tar.gz" \
    82d12bb940e2d4e72e0d5605387fc1b8ca179044e012b620f0ce4e7440e8320e "$t/wb.tgz"
  tar -xzf "$t/wb.tgz" -C "$t" && cp "$t/wasm-bindgen-$ver-x86_64-unknown-linux-musl/wasm-bindgen" "$bin/"; rm -rf "$t"
fi
if ! "$bin/wasm-opt" --version 2>/dev/null | grep -q "version 133"; then
  t=$(mktemp -d)
  fetch "https://github.com/WebAssembly/binaryen/releases/download/version_133/binaryen-version_133-x86_64-linux.tar.gz" \
    2dc9c7813f5375db93d96ead4b78222fcc3e2677bbb832297af4797782a37489 "$t/bn.tgz"
  tar -xzf "$t/bn.tgz" -C "$t" binaryen-version_133/bin/wasm-opt && cp "$t/binaryen-version_133/bin/wasm-opt" "$bin/"; rm -rf "$t"
fi
"$bin/wasm-bindgen" --version; "$bin/wasm-opt" --version
