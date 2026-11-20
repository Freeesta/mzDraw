# mzDraw

A 2D molecule editor written in Rust and compiled to WebAssembly: https://freeesta.github.io/mzDraw/

mzDraw is the structure editor of [mzLab](https://github.com/Freeesta/mzlab), in a repository of its own. All the logic (document, edit operations with exact undo, tools, hit-testing, display list, import and export) is in the Rust crate `mzdraw`; the page only forwards pointer and key events and paints what the module returns.

Built so far (slice 1): atoms, bonds, ring templates, charges, select / move / erase, undo / redo, SMILES, molfile and SDF (V2000 / V3000), KET, CDXML and reaction files (RXN, RDF) in, molfile (V2000 / V3000) / SMILES / KET / CDXML / RXN / SVG and PNG that open again / own document `mzdraw/1` out, live valence check. The plan is in [`docs/mzdraw.md`](docs/mzdraw.md).

## Build and test

Needs Rust (the toolchain of `rust-toolchain.toml`) and Node 20+; the build tools are fetched by `tools/install_tools.sh` (Linux x86_64).

```
cargo test --locked
tools/build_site.sh        # builds site/ (open it with any static server)
cargo build --locked -p mzdraw --example replay && node tests/parity.mjs
node tests/e2e.mjs         # needs Playwright with Chromium
```

## Licence

MIT, see [`LICENSE`](LICENSE). If you use mzDraw in published work, please cite mzDraw and mzLab: see [`CITATION.cff`](CITATION.cff).
