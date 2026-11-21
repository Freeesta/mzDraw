# arch-cd-01: Clean Up Structure: the 2D layout of mzlab-mol on a drawing, stereo kept

Study rows: D01, D04 of `docs/study/arch-cd.md`. Priority MUST. Size guess M. Depends on: nothing.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-01 clean-up: received, working on claude/cd-01-clean-up`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- operation `place` (absolute coordinates for a list of atoms) with its exact inverse
- `layout::clean_up(doc, atoms)`: the selection is laid out again, every other atom stays where it is; nothing selected = the whole drawing
- the meaning of every wedge is kept (the isomeric SMILES before and after is the same), and of every double bond (E/Z)
- command `clean` (button, `Ctrl+Shift+K`), one undo step, announced in the live line

Out (do not build here, say so in the PR if you meet it):
- Clean Up Reaction (study row D02, a later slice)
- template-based layout (D03)
- changes to `mzlab-mol` itself: if the layout is wrong for a molecule, record it in the PR for mzLab

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `crates/mzdraw/src/layout.rs`: `clean_up` returns nothing today and `CLEAN_UP_STATUS` says `unavailable`; `computed` shows how the layout's 1.5 Å bonds become drawing units (`LAYOUT_BOND`).
- `mzlab_mol::Mol::layout_2d_fixed(&[Option<[f64; 2]>])` (pinned rev in the root `Cargo.toml`): atoms given in angstrom stay fixed, the rest is placed around them; `Mol::ez_mismatches` reports double bonds drawn against their record.
- `Document::to_mol` / `to_mol_stereo` in `crates/mzdraw/src/io.rs`: atom i of the record is atom i of the pool; the wedges are read by the molfile rules of `mzlab-mol`.
- Study facts: ChemDraw sets bonds to the document length, aims them at 15° steps and cleans rings only when fully selected (CD-UG17, 17.1); ChemDoodle's clean is stochastic (CDD-G §14.7.1.1) — ours must be deterministic, the same bytes in wasm and natively.

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-01-clean-up` from `origin/main`.

1. Add `Op::Place { atoms: Vec<u32>, xy: Vec<(f64, f64)> }`, log form `{"op":"place","atoms":[3,4],"xy":[[0.5,1],[1.366,1.5]]}`; refuse unknown ids, a length mismatch, duplicates and coordinates outside `MAX_COORD`; round to 4 decimals at the boundary. The inverse is one `Inv::SetAtom` per moved atom. Write the op by hand like the others: `Op::write` (writer) and `Op::read` / `Op::from_value` (reader) in `crates/mzdraw/src/ops.rs`, defaults left out; the reader refuses a malformed op with its error key and changes nothing.
2. Implement `layout::clean_up(doc, atoms) -> Vec<(u32, f64, f64)>`: fixed = atoms not in the set (in Å), run `layout_2d_fixed`, scale back by `LAYOUT_BOND`; for a whole-drawing clean keep each piece's centre where it was (no jump of the view). Return nothing when nothing moves.
3. Stereo: compare the stereo of the record before and after (`to_mol_stereo` on the document with the new coordinates, without applying it); where a centre's descriptor would change, turn its wedge (solid ↔ hashed on the same bond) in the same step; where a double bond would change E/Z, keep the old positions of that bond's piece and say so (`draw2.say.cleanKept`). Test it on synthetic chiral and E/Z molecules.
4. `Editor::clean()` builds one step (`place` + any `bond.set s`), `command("clean")` in `crates/mzdraw-wasm/src/lib.rs`, a button and `Ctrl+Shift+K` on the page (some browsers keep that key: the button is the fallback; rebinding comes with prompt 17). Set `CLEAN_UP_STATUS` to `layout`.
5. Add `place` to `random_step` of `crates/mzdraw/tests/ops.rs` and a clean step to `tests/data/mzdraw_log.json`.

Tests to add (synthetic data, small):
- apply then undo of `place` returns the identical document (property test) and the log round-trips
- clean of a crooked synthetic chain, ring, fused bicycle and two pieces: every bond 1.0 ± 0.01, the same isomeric SMILES before and after (`save("smiles")`), atoms outside the selection unchanged to the last decimal
- two runs give the same bytes (determinism); the native replay gives the same bytes as the page (parity)

UI: a Clean button next to the edit commands; e2e: draw a crooked chain with the mouse, press the button, check the bond lengths from `save("mzdraw")`, undo restores the drawing; one keyboard step (`Ctrl+Shift+K` where the browser lets it through). Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-01-clean-up`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
