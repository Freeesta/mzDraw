# arch-std-13: a gap in the rear bond where two bonds cross

Proposed by the study `docs/study/arch-std.md` (feature rows 4; priority SHOULD). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-13 crossing-gap: received, starting on branch `claude/arch-std-13-crossing-gap`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. arch-std-02 must be merged before you start (this slice reuses its crossing test); otherwise wait and tell the coordinator.

## Scope
In:
- A style key `crossing_gap` (pt, 0 = off, default 0 so files do not move; `slides` and `acs-1996` stay 0 unless the owner decides; when on, at least twice the line width, IUPAC ST-3.1, so the setter refuses a smaller non-zero value): where two bonds without a shared atom cross, the bond with the higher id (the one drawn later) is the front one and the other is cut by `crossing_gap` on each side of the crossing point, in the canvas and the SVG.
- The crossing test shares its geometry with `std.geom.crossing` of arch-std-02 (one function).

Out (do not do here):
- Choosing the front bond by depth or wedges; moving atoms to remove crossings.

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state is what exists), `docs/study/arch-std.md` (§2.1 crossings (IUPAC 2006 ST-3.1, read)).
- `crates/mzdraw/src/scene.rs`, `crates/mzdraw/src/conform.rs` (arch-std-02).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-13-crossing-gap` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). Size: no `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard). The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Find crossings with the grid of the report; split the rear line into two segments (double and triple bonds split every line).
2. Settings key with bounds 0-10 pt; schema; page field from the schema.

## Tests to add
- Unit: two crossing single bonds, a single over a double, three bonds crossing at one point, a crossing at an atom (no cut).
- Golden SVG with the gap on; default goldens unchanged. Parity with `"set":["style.crossing_gap=1.5"]`. e2e: draw a crossing with the mouse and with touch, turn the gap on from the page by keyboard; screenshot at deviceScaleFactor 2.

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
