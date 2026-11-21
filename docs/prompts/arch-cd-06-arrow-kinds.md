# arch-cd-06: Arrow kinds, heads and conditions under the arrow

Study rows: G02, G03, G04 of `docs/study/arch-cd.md`. Priority MUST. Size guess S. Depends on: nothing.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-06 arrow-kinds: received, working on claude/cd-06-arrow-kinds`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- arrow field `k`: `forward` (default), `equilibrium`, `unbalanced` (equilibrium with a longer forward half), `retro`, `resonance` (double-headed), `failed` (crossed), `dashed`
- head fields `h1` / `h2` (`none`, `full`, `half-l`, `half-r`) and `arc` (bend in degrees, 0 = straight)
- text below the arrow (`below`, one or more lines), next to the built text above
- SVG, KET and CDXML out where they have a place (KET arrow `mode` values, CDXML `ArrowType` and heads, from their public schema/SDK pages); what has no place is listed; read back where the reader can

Out (do not build here, say so in the PR if you meet it):
- electron-pushing curved arrows anchored to atoms (prompt 25)
- several-step RXN (G08)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `doc.rs` `Obj` / `ObjKind::Arrow`, `ops.rs` `ObjAdd` / `ObjSet`, `scene.rs` arrow painting, `ket.rs`, `cdxml.rs` (their arrow writers and readers).
- Study facts: MarvinSketch gives chemical meaning to single, retrosynthetic, equilibrium and resonance arrows only (MS/marvin_creating-reactions.html); ChemDoodle composes an arrow from start head, connector and end head (CDD-G §8.3); the new Marvin adds unbalanced-equilibrium arrows (M/marvin_sketch-js-comparison.html).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-06-arrow-kinds` from `origin/main`.

1. Add `k`, `h1`, `h2`, `arc`, `below` to the arrow object (left out at default) and to `obj.add` / `obj.set` with exact inverse (`Inv::SetObj`). Write the op by hand like the others: `Op::write` (writer) and `Op::read` / `Op::from_value` (reader) in `crates/mzdraw/src/ops.rs`, defaults left out; the reader refuses a malformed op with its error key and changes nothing.
2. Scene: each kind drawn from the style (line width, head length); `below` set under the arrow middle like `text` above.
3. RXN out: only `forward`, `equilibrium` and `unbalanced` count as a reaction arrow; the others are refused by `save("rxn")` with a key of their own.
4. Arrow tool gets a kind picker (sub-palette, see prompt 18) and the text field writes below with `Shift` (or a below field).

Tests to add (synthetic data, small):
- property test with every field; round trip; each kind's SVG geometry (head points) checked numerically
- KET and CDXML round trip of kinds that have a place; the import report lists the rest

UI: kind picker next to the arrow tool; e2e: draw an equilibrium arrow with the mouse, write conditions below, save SVG; the same with touch. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-06-arrow-kinds`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
