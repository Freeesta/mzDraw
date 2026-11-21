# arch-std-12: enhanced stereo groups: document, operation, drawing

Proposed by the study `docs/study/arch-std.md` (feature rows 13; priority SHOULD). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-12 enhanced-stereo: received, starting on branch `claude/arch-std-12-enhanced-stereo`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. First of three slices (12 the model and the drawing, 19 the V3000 and KET files, 20 CXSMILES). Coordination: this slice adds to the log language `draw_edit/1` (the operation `stereo.group`); COO-MZDRAW asks COO 16 before queueing it, because `mzlab-mol` (the CLI route of mzLab) must accept the same language. Do not start before the coordinator confirms.

## Scope
In:
- The document keeps enhanced-stereo groups on stereocentres: `abs`, `and<n>`, `or<n>` (n ≥ 1), as the field `stg` of an atom (an older mzDraw keeps it as an unknown field and writes it back, as the format rule says).
- One new operation with an exact inverse: `{"op":"stereo.group","atoms":[...],"g":"and1"}`; `"g":""` takes the atoms out of any group; the inverse stores each atom's previous group.
- Drawn as the style says, key `stereo_labels`: `words` (default; IUPAC ST-6: "and enantiomer" under a structure whose centres are all in one `and` group, nothing for `abs`; never "rac" or "rel", which ST-6.5 does not accept), `groups` (`&1`, `or1` next to each centre: the CTfile / CXSMILES convention, which IUPAC does not cover, ST-0.8), `off`.
- Page: a small popover on a selected centre with Abs / And n / Or n, reachable from the keyboard; the outline says the group.

Out (do not do here):
- Reading and writing files (slices 19 and 20); stereo perception (the record decides which atoms are centres); InChI.

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state is what exists), `docs/study/arch-std.md` (§2.2 (ST-6 absolute and relative, ST-0.8)).
- IUPAC 2006, HTML edition https://iupac.qmul.ac.uk/drawing/stereo.html, ST-6.
- `crates/mzdraw/src/doc.rs`, `crates/mzdraw/src/ops.rs`, `crates/mzdraw/src/scene.rs`, `crates/mzdraw/src/settings.rs`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-12-enhanced-stereo` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). Size: no `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard). The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Document field, validation (only on atoms the record calls stereocentres; else refused with a new key `error.draw2.stereoGroup`), writer order.
2. Operation reader and writer by hand, inverse, property test.
3. Drawing of both label styles in `scene.rs` (canvas and SVG); popover in the page (interface only).

## Tests to add
- Unit: set, change and clear a group; undo gives the identical document; `words` writes "and enantiomer" only when every centre is in one `and` group.
- Parity: a log with `stereo.group`. e2e: set "and1" on two centres by mouse, by keyboard, by touch; screenshot at deviceScaleFactor 2 of both label styles.

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
