#!/usr/bin/env bash
# Builds the site into site/: the page of web/ at the root and the editor module in site/pkg/ (cargo release build for wasm32,
# wasm-bindgen --target web, wasm-opt -O3 with the flags mzLab uses for its modules, then the size gate tools/wasm_cap.sh).
# Also writes target/pkg-node (wasm-bindgen --target nodejs of the same build) for the parity test tests/parity.mjs.
# The site that is tested (ci.yml) is the site that is published (pages.yml): both run this script.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd); cd "$root"
tools/install_tools.sh
PATH="$root/tools/.bin:$PATH"
cargo build --release --locked --target wasm32-unknown-unknown -p mzdraw-wasm
raw=target/wasm32-unknown-unknown/release/mzdraw_wasm.wasm
rm -rf site target/pkg-node && mkdir -p site/pkg
cp -r web/. site/
wasm-bindgen --target web --no-typescript --out-dir site/pkg "$raw"
wasm-bindgen --target nodejs --no-typescript --out-dir target/pkg-node "$raw"
# Never the relaxed SIMD proposal or --traps-never-happen: the results must be the same bytes everywhere.
wasm-opt -O3 --low-memory-unused --reorder-functions --strip-producers -o site/pkg/mzdraw_wasm_bg.wasm site/pkg/mzdraw_wasm_bg.wasm
touch site/.nojekyll
tools/wasm_cap.sh site/pkg/mzdraw_wasm_bg.wasm
