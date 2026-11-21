# arch-cd-10: Expand and contract an abbreviation in place

Study rows: B07 of `docs/study/arch-cd.md`. Priority MUST. Size guess M. Depends on: nothing.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-10 abbrev-expand: received, working on claude/cd-10-abbrev-expand`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- `label.expand {id}`: the labelled atom becomes the atoms and bonds of its group, placed by the 2D layout around the attachment, one undo step
- `label.contract {atoms, lab}`: a set of atoms that is exactly a known group with one attachment becomes one labelled atom again
- commands in the context of the selection (button, and prompt 20's menu when it exists)

Out (do not build here, say so in the PR if you meet it):
- user abbreviations (prompt 11, which will reuse these ops)
- S-groups in files (OUT; listed on import)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `crates/mzdraw/src/label.rs` (the table of 30 groups and how a label expands at export), `io.rs` `labelled` / `record_of`, `layout.rs` (with prompt 01, `layout_2d_fixed` places the new atoms; without it, use the expansion positions the export already uses).
- Study facts: MarvinSketch Structure > Group offers Contract, Expand, Ungroup (MS/marvin_marvinsketch_structure-menu.html); Marvin JS expands a contracted group when you edit inside it (MJ/marvin-js_abbreviations.html); ChemDoodle can collapse automatically (CDD-G §14.4.2); the CTfile keeps the expanded/contracted state (BD-CT26).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-10-abbrev-expand` from `origin/main`.

1. Both ops with exact inverses (expand stores the label atom; contract stores the removed atoms and bonds, as `remove` does). Write the op by hand like the others: `Op::write` (writer) and `Op::read` / `Op::from_value` (reader) in `crates/mzdraw/src/ops.rs`, defaults left out; the reader refuses a malformed op with its error key and changes nothing.
2. Contract recognises a group by its graph (the record of the selected atoms with the attachment as a dummy) against the label table; refuse with `error.draw2.notAGroup` when it does not match.
3. Positions: the expanded atoms keep the bond direction of the attachment; with prompt 01 merged, lay them out with the rest fixed.

Tests to add (synthetic data, small):
- expand then contract returns the identical document; property test with both ops; the molfile of an expanded and of a contracted Boc are the same record
- parity entry with expand/contract

UI: Expand / Contract buttons for the selection; e2e: place `label:Boc`, expand, change one atom, undo, contract; keyboard and touch paths. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-10-abbrev-expand`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
