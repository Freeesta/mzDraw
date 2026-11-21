# arch-cd-02: CIP labels on the canvas (R/S, r/s, E/Z) from cip/1, live

Study rows: C02 of `docs/study/arch-cd.md`. Priority MUST. Size guess M. Depends on: nothing.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-02 cip-labels: received, working on claude/cd-02-cip-labels`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- a display setting `display.cip` (`off` default, `on`) in `mzdraw-settings/1`, a new section `display` next to `style`
- the labels computed by `cip/1` of `mzlab-feat` (feature `mzlab` of the bridge, like `check/1`) and painted by the scene next to their atom or the middle of their double bond, in the SVG and PNG too
- the same SVG bytes from the native replay: the request carries the labels (`"cip": [[atom id, "R"], ...]`, double bonds as `[[a, b], "E"]`)

Out (do not build here, say so in the PR if you meet it):
- M/P axial labels (C06: as far as `cip/1` gives them, they are painted; nothing new is computed here)
- enhanced-stereo labels (prompt 24)
- a new dependency of `crates/mzdraw` (it stays on `mzlab-mol`, `libm`, `serde`, `serde_json`)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `crates/mzdraw-wasm/src/lib.rs` `recheck`: how `check/1` is called on `to_molfile_for_check` and its atom indices mapped to document ids (molfile order = pool order); do the same for `mzlab_feat::chem::cip::cip_json`.
- `crates/mzdraw/src/scene.rs` `Overlay` (`warn_atoms`) and `settings.rs` (`FIELDS`, `Style::set`, `schema_json`): add the `display` section through the same table so the schema, the reader, the writer and `Editor::set("display.cip=on")` all know it.
- Study facts: ChemDraw shows R/S, r/s and E/Z through Show Stereochemistry and updates them while drawing (CD-UG17); MarvinSketch toggles R/S, E/Z and M/P from View > Stereo (MS/marvin_stereochemistry-in-marvinsketch.html); BIOVIA Draw shows meso, pseudo and unknown and has a time-out for large molecules (BD-DS24, BD-T104).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-02-cip-labels` from `origin/main`.

1. Add the `display` section with `cip` (`off`/`on`) to the settings table; unknown keys of the section are kept as for `style`. It is not part of the document and is no undo step.
2. `Editor::set_cip(labels: Vec<CipLabel>)` (an overlay, like the warnings); `scene::build` places each label: atoms at the free side (the widest gap, as for hydrogens), double bonds at the bond middle on the outer side; small italic text of the style's font, the colour of the style. Painted only when `display.cip=on`.
3. Bridge: after each change, when `display.cip=on`, call `cip_json` on the molfile and map indices to ids; skip and say why when `cip/1` answers `skip` (query, too big). Keep the call out of the hot path of a drag (after the release).
4. Replay (`crates/mzdraw/src/replay.rs`): accept `"cip"` in the request and give it to the overlay before `save("svg")`, so the CLI of mzLab (which has `mzlab-feat`) computes the labels and gets the same bytes. Document the field in the request description.
5. Add a cip step to `tests/data/mzdraw_log.json` (a synthetic chiral molecule, request with the labels) so the parity test covers the painted labels.

Tests to add (synthetic data, small):
- settings: `display.cip=on` round-trips, a bad value is refused with `error.draw2.styleValue`
- scene: labels sit at their atom, never on a bond line of that atom; the SVG of a synthetic (R)-centre and an (E)-alkene contains the labels; with `off` the SVG is byte-identical to before
- bridge test (wasm side, native unit test of the mapping): indices map to ids after deletions

UI: a "Stereo labels" toggle in the view settings; e2e: draw a centre with a wedge, switch the labels on, read `state().cip` and the SVG, flip the molecule, the label changes; keyboard reachable toggle. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

Docs: keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice: what it does, the op/setting names, what is not built); update the row(s) of `docs/study/arch-cd.md` to `built` in the same PR.

Size: the owner lifted the wasm size limit for now, but rule 8 still holds: measure with `tools/build_site.sh`, write the new size in `measured` of `tools/wasm-cap.toml` and the delta in the PR. If the module passes `cap`, send COO-MZDRAW the numbers and raise `cap` only when it answers, with the reason in the PR.

## Acceptance

All of these pass, and their output is quoted in the PR:
```
cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
tools/build_site.sh
cargo build --locked -p mzdraw --example replay && node tests/parity.mjs
node tests/e2e.mjs
```

If a golden output changes because of this slice (and only then), rewrite it with `MZDRAW_BLESS=1 cargo test -p mzdraw --test replay` and explain each changed file in the PR. Extend `tests/data/mzdraw_log.json` with the new op(s) so that the parity test replays them.

## Messages

- To COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) only. Start: the confirm line above. Blocked (a test you cannot make pass, a decision that is not yours, a host refused): one message with the exact error, then wait or continue with the rest.
- Done: one message with the PR link, the full head SHA, the module size before → after, and the study rows now built.
- The PR is in English, ready for review (not a draft), titled after the feature, its body following the repository's usual sections, with the screenshots for UI changes.

## Handoff at 200k tokens

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-02-cip-labels`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
