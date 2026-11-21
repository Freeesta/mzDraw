# arch-cd-11: User abbreviations in the settings file, and composed labels

Study rows: B06, B04 of `docs/study/arch-cd.md`. Priority MUST. Size guess M. Depends on: prompt 10 (expand/contract) for the in-place use.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-11 user-abbreviations: received, working on claude/cd-11-user-abbreviations`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- section `abbreviations` of `mzdraw-settings/1`: lines `LABEL SMILES` with the attachment atom first (format of `docs/mzdraw-formats.md` §4), validated in Rust (the SMILES must parse, one attachment, label free of clashes with elements)
- "define from selection": the selected atoms with one bond out become an entry (the SMILES written by `mzlab-mol`)
- composed labels made of known parts: `CO2Me`, `NHBoc`, `CH2OH`, `OTBS` are parsed left to right into known groups and atoms, else refused with the part that failed

Out (do not build here, say so in the PR if you meet it):
- templates (prompts 12, 13)
- S-group abbreviations from files (listed on import)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `label.rs` (built-in table), `settings.rs` (sections, unknown keys kept, `Editor::set`), the replay request (`style`, `set`), `crates/mzdraw-wasm/src/lib.rs` (settings kept in OPFS by the page).
- Study facts: ChemDraw Define Nickname (CD-UG17); MarvinSketch `user.abbrevgroup` (MS/marvin_abbreviated-groups-superatom-group.html); ChemDoodle defines a label by name, SMILES and valency 0/1/2 (CDD-G §14.4.1.1); BIOVIA Draw lets users drag abbreviations onto the toolbar (BD-DS24).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-11-user-abbreviations` from `origin/main`.

1. Settings: the section, its validation with error keys, its schema entry; the label tool lists built-in + user entries; a user entry with a built-in name is refused (no silent override).
2. Expansion at export uses the user entry exactly as a built-in one; a document that uses a user label carries the definition inside (`more` of the document or a new optional `abbreviations` field), so it opens the same elsewhere — decide and state which.
3. Composed-label parser in Rust with a bounded length; the replay request takes `"set": ["abbreviations=..."]` or a settings object so the CLI gives the same bytes.

Tests to add (synthetic data, small):
- a user `Tf` (S(=O)(=O)C(F)(F)F) expands to the right record; a bad SMILES is refused with its key; a document using it opens on an editor without the setting and gives the same molfile
- composed labels: CO2Me, NHBoc, CH2OH parse; CO2Xx is refused

UI: "Define abbreviation" for the selection and the list in the settings panel; e2e: define one from a drawn group, use it from the label field, reload the page (OPFS), it is still there. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-11-user-abbreviations`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
