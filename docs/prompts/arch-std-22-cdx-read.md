# arch-std-22: read ChemDraw binary CDX files

Proposed by the study `docs/study/arch-std.md` (feature rows §2.8 CDX row; priority MUST). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-22 cdx-read: received, starting on branch `claude/arch-std-22-cdx-read`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Owner requirement: mzDraw opens the files of every program; CDX is the default file of ChemDraw and journals ask for it. One slice: the tag reader; writing CDX is another decision.

## Scope
In:
- `crates/mzdraw/src/cdx.rs`: a reader of the binary CDX file (header `VjCD0100`, then objects and properties as tagged records with lengths) that builds the same element tree the built CDXML reader works on, so every object the CDXML reader understands (fragments, nodes, bonds, wedges, captions, arrows, plus signs, R groups, abbreviations) is read by the same mapping, and everything else is counted and listed as for CDXML.
- Bounds as for every reader: lengths checked against the file, depth and object count bounded, no panic on a damaged file (fuzz-style tests with truncated and corrupted synthetic files).
- Open a file and paste (binary from a file only; the clipboard carries no CDX in the browser).

Out (do not do here):
- Writing CDX; CDX inside Office documents (a later slice); the Windows clipboard.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§2.8 (CDX row), §2.10 (placement (b))).
- CDX SDK documentation, public mirror https://chemapps.stolaf.edu/iupac/cdx/sdk/ (format overview, object and property pages, `CDXConstants.h` tag values as published there). Implement from the pages; copy no code of any program; ChemDraw files you did not write yourself are not test data.
- `crates/mzdraw/src/cdxml.rs`, `crates/mzdraw/src/xml.rs`, `crates/mzdraw/tests/cdxml.rs`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-22-cdx-read` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. A small writer of CDX in the tests only (to make synthetic files from the CDXML fixtures you already have), then the reader.
2. Map tag values to the CDXML object and property names the reader uses; unknown tags are skipped by length and counted.
3. Content sniffing on `VjCD0100`; `docs/mzdraw-formats.md` §1a CDX row; module size measured.

## Tests to add
- For each CDXML fixture of `crates/mzdraw/tests/cdxml.rs`, the CDX made by the test writer opens to the same document as the CDXML.
- Damaged files: every truncation point of one synthetic file is refused or read in part with a note, never a panic.
- Parity: a replay importing a CDX (base64 in the request if the request is text; add the field if needed). e2e: open a synthetic `.cdx` with the keyboard and by touch.

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
