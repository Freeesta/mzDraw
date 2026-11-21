# arch-std-20: enhanced stereo groups from and to CXSMILES

Proposed by the study `docs/study/arch-std.md` (feature rows 13, 21; priority SHOULD). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-20 enhanced-stereo-cxsmiles: received, starting on branch `claude/arch-std-20-enhanced-stereo-cxsmiles`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Third of three slices; arch-std-12 must be merged before you start, otherwise wait. Coordination: COO-MZDRAW asks COO 16 before queueing this prompt: the CXSMILES writer belongs in `mzlab-mol` (study §2.10), so COO 16 must agree where it is written. Writing CXSMILES needs a CXSMILES writer: by the placement rule of the study (§2.10) it belongs in `mzlab-mol`; if it is not there, do the reading part only and report the writer to the coordinator.

## Scope
In:
- Read the stereo-group fields of the CXSMILES extension block (`a:`, `&n:`, `on:`; `mzlab-mol` already reads them for its record) into the `stg` field.
- Write them, with the coordinates, when `mzlab-mol` offers a CXSMILES writer at the pinned commit (`"out":"cxsmiles"`); otherwise list them as lost on SMILES output.

Out (do not do here):
- Other CXSMILES fields (atom labels, coordinates on read beyond what `mzlab-mol` gives, S-groups), a CXSMILES writer of our own.

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state is what exists), `docs/study/arch-std.md` (§2.4 CXSMILES row, §2.10).
- Chemaxon CXSMILES documentation (https://docs.chemaxon.com/latest/formats_chemaxon-extended-smiles-and-smarts-cxsmiles-and-cxsmarts.html), the enhanced stereo section.
- `mzlab-mol/src/cxsmiles.rs` at the pin, `crates/mzdraw/src/io.rs`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-20-enhanced-stereo-cxsmiles` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). Size: no `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard). The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Read path; 2. write path if the writer exists; 3. `docs/mzdraw-formats.md` §1a SMILES/CXSMILES row.

## Tests to add
- Synthetic CXSMILES with each group kind; a SMILES out of a document with groups lists them as lost.
- Parity: a replay importing a CXSMILES. e2e: paste a CXSMILES with groups (keyboard and touch); the labels show.

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
