# arch-std-14: identification scales on the page

Proposed by the study `docs/study/arch-std.md` (feature rows 35; priority SHOULD). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-14 ann-scales: received, starting on branch `claude/arch-std-14-ann-scales`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. 

## Scope
In:
- The annotation panel offers a scale: `schymanski-2014` (default: `1`, `2a`, `2b`, `3`, `4`, `5`) and `charbonnet-2022` (PFAS: `1a`, `1b`, `2a`, `2b`, `2c`, `3a`, `3b`, `3c`, `3d`, `4`, `5a`, `5b`); the level list follows the scale; the entry is written with that `scale`. One table in Rust (`ann.rs`) gives the scales and levels to the page; the meaning of each level is an i18n text (`draw2.ann.level.<scale>.<level>`), summarised in our own words.
- A level not in the chosen scale stays allowed (kept as read, as today); the panel shows it as "of the file".
- Picture: unchanged (`L` + level, so `L3b`, `L5a`).

Out (do not do here):
- Level-aware depiction of levels 4 and 5 (row 36, LATER), scores and source on the page.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§2.5).
- Charbonnet et al., Environ. Sci. Technol. Lett. 2022, 9(6), 473-481, DOI 10.1021/acs.estlett.2c00206, open access in PubMed Central PMC9202347 (read for the study; its Table 1 is an image).
- Schymanski et al., Environ. Sci. Technol. 2014, 48(4), 2097-2098, DOI 10.1021/es5002105 (text not read in the study).
- `crates/mzdraw/src/ann.rs`, `docs/mzdraw-annotations.md`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-14-ann-scales` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Scale table and its JSON for the page state; validation unchanged (any word of the level shape is kept).
2. Page: a scale select before the level select; keyboard and touch reachable.
3. `docs/mzdraw-annotations.md`: the scales and their levels.

## Tests to add
- Unit: the table lists both scales; `Editor::annotate` with `scale: charbonnet-2022`, level `3b` writes the entry and the picture shows `L3b`; SDF out and back keeps the scale.
- Parity: a log with an `atom.ann` of the PFAS scale and `"out":"sdf"`. e2e: choose the PFAS scale and level `2c` with the keyboard and by touch; the outline says the level.

## Acceptance (all green before the PR)
```
cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
tools/build_site.sh
cargo build --locked -p mzdraw --example replay && node tests/parity.mjs
node tests/e2e.mjs
```
If a golden output moves because of a WANTED change: `MZDRAW_BLESS=1 cargo test -p mzdraw --test replay`, and say in the PR which lines moved and why.

## Messages
- First message to the coordinator, before any work: the confirm line above, verbatim.
- Ask the coordinator only when blocked by a decision that is not yours (a cap raise, a scope question); otherwise decide and say why in the PR.
- When the PR is open: one message with the PR link, the full head SHA, the module size before and after (bytes), the test counts, and one line per acceptance command (pass/fail).

## Handoff at 200k tokens
At about 200k tokens of context, stop starting new work: commit what builds with a message that says what is left, push the branch (no force), and send the coordinator a handoff message: branch, last commit SHA, what is done, what is left as a numbered list, the exact next command to run, and any decision taken that the next chat must keep.
