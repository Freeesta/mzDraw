#!/usr/bin/env bash
# The size gate of the editor module: tools/wasm_cap.sh <file.wasm>; fails when the file is larger than `cap` of tools/wasm-cap.toml.
# It prints the real size (raw and gzip); nothing about the size is recorded in a file, so no PR edits a size figure.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
cap=$(sed -n 's/^cap = \([0-9]*\)$/\1/p' "$here/wasm-cap.toml")
[ -n "$cap" ] || { echo "wasm cap: no cap in tools/wasm-cap.toml" >&2; exit 2; }
size=$(wc -c < "$1")
gz=$(gzip -9 -c "$1" | wc -c)
line="wasm cap: $1 is $size B ($gz B gzip), cap $cap B"
echo "$line"
[ -z "${GITHUB_STEP_SUMMARY:-}" ] || echo "$line" >> "$GITHUB_STEP_SUMMARY"
[ "$size" -le "$cap" ] || { echo "wasm cap: over the cap by $((size - cap)) B" >&2; exit 1; }
