# arch-std-07: page audit against WCAG 2.2 AA, fixed where it fails

Proposed by the study `docs/study/arch-std.md` (feature rows 41; priority MUST). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-07 page-a11y-audit: received, starting on branch `claude/arch-std-07-page-a11y-audit`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Mostly tests and CSS; any fix that is logic goes into Rust.

## Scope
In:
- An e2e section `a11y` in `tests/e2e.mjs` (Playwright, Chromium, no new npm package) that measures and asserts: every button and input at least 24 × 24 CSS px or spaced per SC 2.5.8; at 320 CSS px width no horizontal page scroll and every control reachable (1.4.10); with text spacing overrides (line height 1.5, paragraph 2, letter 0.12, word 0.16 em) no control text clipped (1.4.12); the keyboard focus never fully hidden by the help, a popover or the autosave bar (2.4.11); portrait and landscape at a tablet size (1.3.4); contrast of the page's text tokens ≥ 4.5:1 and of control borders and focus outlines ≥ 3:1 (computed from the computed styles); `forced-colors: active` emulation keeps the canvas legible (bonds and labels drawn in `CanvasText` on `Canvas`, decided by a flag the page hands to the module) and `prefers-reduced-motion` removes any animation.
- Fix each failure found (CSS sizes and spacing, the forced-colours flag in Rust: the module picks the system colours pair when told the mode is on).
- The toolbar follows the APG toolbar pattern if it does not: one tab stop, arrow keys between buttons (page interface only).

Out (do not do here):
- A real screen-reader pass (cannot run here; list it as not done), focus on bonds and objects.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§2.6 table).
- WCAG 2.2, Recommendation 2024-12-12 (https://www.w3.org/TR/2024/REC-WCAG22-20241212/); CSS Color Adjustment Level 1, CR Snapshot 2025-12-16 (https://www.w3.org/TR/css-color-adjust-1/); Media Queries Level 5, WD 2026-02-19 (https://www.w3.org/TR/mediaqueries-5/); APG toolbar pattern (https://www.w3.org/WAI/ARIA/apg/patterns/toolbar/).
- `web/index.html`, `web/mzdraw.css`, `web/mzdraw.js`, `tests/e2e.mjs`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-07-page-a11y-audit` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Write the audit first and record every failure in the PR (SC, element, measured value).
2. Fix; re-run; screenshots at deviceScaleFactor 2 of the page at 320 px, at desktop size, and in forced colours.
3. A short "Accessibility" list in `docs/mzdraw.md`: which SC are tested by e2e and which are not.

## Tests to add
- The audit itself (above), with the measured numbers in the assertion messages.
- Unit (Rust) for the forced-colours flag: the display list uses only the two system colour indices when it is on.
- Keep the existing touch, pen and keyboard steps green.

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
