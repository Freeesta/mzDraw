# arch-std-05: move, rotate and select without dragging

Proposed by the study `docs/study/arch-std.md` (feature rows 39; priority MUST). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-05 drag-free-editing: received, starting on branch `claude/arch-std-05-drag-free-editing`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. This answers WCAG 2.2 SC 2.5.7 Dragging Movements (AA): what a drag does must also be possible with single pointer actions or keys.

## Scope
In:
- Keys on a selection (canvas focused, a selection present): `Alt+Arrow` moves it by 0.1 bond length, `Alt+Shift+Arrow` by 1 bond length (one `move` op each, consecutive presses coalesced into one undo step as a drag is); `Alt+[` / `Alt+]` rotate it by 15° about its centre (`rotate` op); `Shift+Arrow` with the keyboard focus extends the selection to the neighbour walked to.
- Single-pointer alternative to drag-moving: with the select tool, a click on an empty spot while a selection exists and the "Move here" mode is on (a toggle button, key `M`) moves the selection so its centre lands on the click; one `move` op.
- The live line announces each step (`draw2.announce.moved`, `draw2.announce.rotated` with the amount).

Out (do not do here):
- New operations (the existing `move` and `rotate` suffice), band selection by keys (Shift+Arrow covers it), focus on bonds and objects (another slice).

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state is what exists), `docs/study/arch-std.md` (§2.6 SC 2.5.7 and 2.1.1).
- WCAG 2.2, Recommendation 2024-12-12, SC 2.5.7 (https://www.w3.org/TR/2024/REC-WCAG22-20241212/#dragging-movements).
- `crates/mzdraw/src/editor_keys.rs` (keys and the focus walk), `crates/mzdraw/src/editor_tools.rs`, `crates/mzdraw/src/ops.rs` (`Move`, `Rotate`), the help list of keys in `web/index.html`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-05-drag-free-editing` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). Size: no `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard). The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Key handling in Rust (`editor_keys.rs`); the page forwards `Alt` in the modifier bits if it does not already.
2. Coalescing: consecutive moves by key within one selection are one undo step until another command or a 1 s pause (time comes from the event stamp the page forwards, as touch does; no clock in Rust).
3. "Move here" toggle as a real `<button aria-pressed>`.
4. Add the keys to the help and to `docs/mzdraw.md` keyboard map.

## Tests to add
- Unit: each key gives the expected ops in the log; undo after three `Alt+Right` gives the identical document; rotate by 24 presses of 15° returns to the start within rounding of the document (4 decimals).
- Parity: a log with the generated `move` and `rotate` ops.
- e2e: move a selection by keys only, by "Move here" with the mouse and with touch, rotate by keys; check positions and the announcements.

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
