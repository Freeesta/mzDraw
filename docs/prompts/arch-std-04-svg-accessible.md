# arch-std-04: saved SVG with a text alternative

Proposed by the study `docs/study/arch-std.md` (feature rows 27, 28; priority MUST). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-04 svg-accessible: received, starting on branch `claude/arch-std-04-svg-accessible`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. 

## Scope
In:
- The SVG the editor saves (`svg::to_svg_styled`, `svg::carrying`) gets `role="img"`, `aria-labelledby` pointing at a `<title>` and a `<desc>`, and `xml:lang="en"`: the title is the outline line of the drawing (Hill formulas of the structures from `outline.rs`, "and" between them, plus the arrow if there is one), the desc lists per structure its formula and, when annotated, its level (`L2b`). The text comes from Rust; English words come from a small fixed table in Rust because the file has no i18n layer (say so in the docs).
- A test that the writer uses only SVG 1.1 elements and attributes (a list of allowed element names checked over every golden SVG), so files open in older readers.

Out (do not do here):
- Accessibility of the live canvas (built: outline and live line), PNG text chunks.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§2.4 SVG row, §2.6 SC 1.1.1).
- SVG 1.1 Second Edition, W3C Recommendation 2011-08-16 (https://www.w3.org/TR/2011/REC-SVG11-20110816/), `title` and `desc`; WAI-ARIA Graphics Module, Recommendation 2018-10-02 (https://www.w3.org/TR/graphics-aria-1.0/); SVG-AAM Working Draft 2026-10-21 (https://www.w3.org/TR/svg-aam-1.0/).
- `crates/mzdraw/src/svg.rs`, `crates/mzdraw/src/outline.rs`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-04-svg-accessible` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Build the title and desc from the outline; escape XML text.
2. Keep the picture itself unchanged (only the new root attributes and two elements before the drawing); the carried document stays in `metadata`.
3. Update `docs/mzdraw-formats.md` §1a SVG row and `docs/mzdraw.md`.

## Tests to add
- Unit: title and desc for one structure, two structures with an arrow, an annotated structure, an empty drawing (title "Empty drawing").
- The allowed-element test over all golden SVGs. Opening a saved SVG still gives back the same document bytes.
- Bless the golden SVGs (every file changes by the new header) and list them. Parity unchanged in kind. e2e: save the SVG from the page (mouse, keyboard) and check `title` in the downloaded text.

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
