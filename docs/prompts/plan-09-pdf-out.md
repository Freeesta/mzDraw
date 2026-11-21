# plan-09: PDF out with the document embedded

Proposed by `docs/plan/backlog.md` (row 17; Gemini card 2 (reframed: SVG and PNG carrying the document are built)). Size M. Wave 1. Depends on: none (backlog Q7).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-09 pdf-out: received, starting on branch `claude/plan-09-pdf-out`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. 

## Scope
In:
- A new module `crates/mzdraw/src/pdf.rs`: a PDF 1.7 writer from the display list the SVG writer uses (lines, polygons, arcs as Bézier curves, text in the standard font Helvetica with WinAnsi encoding, colours of the style), one page the size of the drawing plus the margin of the style, in points, white background.
- The document `mzdraw/1` embedded as an embedded file (`/EmbeddedFiles`, name `mzdraw.json`, MIME `application/x-mzdraw+json`) and the V3000 molfile as a second one; `doc_from_pdf(bytes)` reads only PDFs we wrote (finds the embedded file by name; no general PDF parsing; bounds checked like the PNG reader).
- Deterministic bytes: fixed object order, no dates, no ID array from time (the `/ID` is a hash of the content), numbers through `mzlab_mol::fixed`.
- `Editor::save("pdf")`; the page offers PDF in the Save flyout; opening a PDF of ours adds its drawing; another PDF is refused with `error.draw2.pdfPlain`.

Out (do not do here):
- Fonts other than the standard 14; text outlined to paths (deferred).
- Multi-page output; PDF/A conformance claims.

## CLI twin (rule 15)
- `{"out":"pdf"}` in the replay request; the bytes are base64 in the JSON result as for PNG, and the example writes the raw file with `--raw`.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- `crates/mzdraw/src/svg.rs` (the display list to file), `png.rs` (how the PNG carries the document and how the reader is bounded).
- PDF 1.7 is ISO 32000-1; the free copy of the Adobe PDF Reference 1.7 (https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf): §7.3 objects, §7.5 file structure, §7.11.4 embedded files, §9.6.2.2 standard fonts. Summarise, do not copy.

## Agreement needed before the PR
None: this slice adds no `draw_edit/1` operation or field and changes nothing in `mzlab-mol` / `mzlab-feat`. If you find you need one, stop and ask the coordinator (it asks COO 16).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-09-pdf-out` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Writer and reader; size report.
2. Save door, replay out, page button and open path.
3. Goldens for three drawings (one with a reaction arrow and text).

## Tests to add
- document → PDF → document gives identical document bytes.
- Same document twice → identical PDF bytes.
- A PDF without our embedded file is refused with the key; a truncated one does not panic (fuzz-like test as for PNG).
- e2e: save PDF and reopen it by drop and by Open a file.
- **Fails without the change**: the round-trip test fails until the reader finds the embedded file.

## Independent proof
Correctness must not rest on our own claims. `tools/survival/pdf_check.sh`: `qpdf --check` reports no errors for the goldens, `pdfdetach -list` (poppler) lists `mzdraw.json`, and `pdftoppm` renders each golden (the raster is attached to the PR, not committed). Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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
- Ask the coordinator only when blocked by a decision that is not yours (a cap raise, a new op, a scope question); otherwise decide and say why in the PR.
- When the PR is open: one message with the PR link, the full head SHA, the module size before and after (bytes), the test counts, the independent-proof result, and one line per acceptance command (pass/fail).

## Handoff at about 150k tokens
At about 150k tokens of context, stop starting new work: commit what builds, push the WIP branch (no force) with a one-line state note at the top of the PR description or in the last commit message ("state: done X; next Y"), and send the coordinator: branch, last commit SHA, what is done, what is left as a numbered list, the exact next command to run, and any decision taken that the next chat must keep.

