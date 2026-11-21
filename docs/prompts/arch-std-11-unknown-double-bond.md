# arch-std-11: double bonds of unknown geometry

Proposed by the study `docs/study/arch-std.md` (feature rows 12; priority SHOULD). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-11 unknown-double-bond: received, starting on branch `claude/arch-std-11-unknown-double-bond`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Coordination: this slice adds to the log language `draw_edit/1` (`bond.set` and `bond.add` with `s: 3` on a bond of order 2); COO-MZDRAW asks COO 16 before queueing it, because `mzlab-mol` (the CLI route of mzLab) must accept the same language. Do not start before the coordinator confirms. Note: this slice touches four formats (molfile V2000/V3000, SMILES, KET); if it does not fit one small PR, finish the document, operation and drawing first and leave the formats to a follow-up, said in the handoff.

## Scope
In:
- A double bond may carry the mark "either" (`s: 3` on a bond of order 2 in the document and in `bond.set`/`bond.add`; today `s` is for single bonds only). How it is drawn is the style's, key `either_double`: `wavy` (default: one single bond at one end of the double bond, the one with the lowest id, is drawn wavy) or `crossed` (the two lines of the double bond cross; offered only because other software reads it, and the stereo report gives `std.stereo.crossedDouble`, info, when it is used).
- Molfile V2000 out: bond stereo `3` on the double bond; V3000: `CFG=2`; SMILES: no `/` `\` for that bond; KET: the schema's double-bond stereo value; read back from all of them.
- The stereo check of arch-std-01 does not report `std.stereo.undefined` for a bond marked "either".

Out (do not do here):
- E/Z labels. A double bond in a ring of fewer than 8 atoms cannot be E/Z (our rule): the mark is refused there with a new key `error.draw2.eitherRing`.

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state is what exists), `docs/study/arch-std.md` (§2.2 unknown E/Z (IUPAC 2006 ST-4.4, read: linear or wavy preferred, the crossed double bond not acceptable except where software requires it), §2.4 molfile row).
- `crates/mzdraw/src/doc.rs` (wedge marks), `crates/mzdraw/src/ops.rs` (`BondSet`), `crates/mzdraw/src/scene.rs`, `crates/mzdraw/src/ket.rs`, the molfile marks code.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-11-unknown-double-bond` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). Size: no `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard). The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Allow `s: 3` on order 2 in validation; refuse other marks on double bonds.
2. Rendering of both forms; style key `either_double` through the settings table and setter.
3. Writers and readers of the four formats; how `mzlab-mol` derives the record stereo from it (unknown).

## Tests to add
- Unit: set, undo (property test covers `bond.set` with the new value), writers for each format and read back to the same document.
- Golden SVG for the crossed form. Parity: a log with `{"op":"bond.set","id":..,"s":3}` on a double bond. e2e: mark a double bond "either" with the wavy tool (mouse, touch, key `Y` on the focused bond if bond focus exists, else on the hovered bond).

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
