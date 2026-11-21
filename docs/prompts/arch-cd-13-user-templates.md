# arch-cd-13: User templates: save the selection, rename, remove, share as a file

Study rows: A15 of `docs/study/arch-cd.md`. Priority MUST. Size guess M. Depends on: prompt 12.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-13 user-templates: received, working on claude/cd-13-user-templates`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- section `templates` of `mzdraw-settings/1`: named `mzdraw/1` fragments with an attachment atom or bond (as `docs/mzdraw-formats.md` §4)
- "Save selection as template" (asks a name and, when there is more than one candidate, the attachment), rename, remove, order
- kept with the settings (OPFS on the page, the settings file in the CLI and the desktop app); imported and exported as one file

Out (do not build here, say so in the PR if you meet it):
- a template editor with its own canvas (LATER)
- team libraries on a server (OUT: offline)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- prompt 12's placement code; `settings.rs` (sections, unknown keys kept, schema); the settings file the page keeps in OPFS (`mzdraw-settings.json`).
- Study facts: ChemDraw adds a saved template document to its Templates palette (CD-KB/4408239673748*); MarvinSketch My Templates via the Template Library Manager (MS/marvin_templates-in-marvinsketch.html); ChemDoodle add, remove, rename, update in its Templates widget (CDD-G §4.12); BIOVIA installs libraries per user or for all users (BD-T65).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-13-user-templates` from `origin/main`.

1. Validation: a template is a valid `mzdraw/1` fragment within the bounds; names unique; at most 500 entries and a size bound; refusal keys.
2. The palette shows user templates after the built-in ones; the CLI replay request can carry them (`"set"` or a settings object) so placing a user template replays to the same bytes.

Tests to add (synthetic data, small):
- save, place, rename, remove; a template survives a settings round trip byte for byte; a bad template file is refused with its key and the old list is kept

UI: user group in the template palette; e2e: draw a scaffold, save it, reload the page, place it; touch: place it with a tap. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-13-user-templates`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
