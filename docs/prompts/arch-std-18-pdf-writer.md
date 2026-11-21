# arch-std-18: PDF out: vector, text as text, the document attached

Proposed by the study `docs/study/arch-std.md` (feature rows 30; priority SHOULD). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-18 pdf-writer: received, starting on branch `claude/arch-std-18-pdf-writer`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Decision 4 of `docs/mzdraw-plan.md` recommends an own small writer on the display list; confirm with the coordinator before starting that the owner has taken it.

## Scope
In:
- `crates/mzdraw/src/pdf.rs`: one page of the drawing's size in pt, white ground, lines and polygons as path operators, text as text in the base-14 font Helvetica (and Helvetica-Oblique if needed) with WinAnsi encoding, element colours as RGB; the `mzdraw/1` document as an embedded file (`/EmbeddedFiles`, `mzdraw.json`) so the PDF reopens in mzDraw; a cross-reference table with exact offsets; streams uncompressed or stored, so no compressor is needed; the same bytes from the page and the replay (`"out":"pdf"`, no date unless the request gives one).
- Reading back: a PDF written by mzDraw opens (find the embedded file); any other PDF is refused in words (`error.draw2.pdfPlain`).

Out (do not do here):
- Embedded TrueType fonts, PDF/A, reading other programs' PDFs, multi-page output.

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state is what exists), `docs/study/arch-std.md` (§2.3 (every publisher read accepts PDF), §2.4 PDF row, §2.9).
- ISO 32000-1:2008 (PDF 1.7) as published free by Adobe, or ISO 32000-2:2020 (PDF 2.0, free through the PDF Association): read the sections on file structure, content streams, the standard 14 fonts and embedded file streams; if you cannot read them, stop and tell the coordinator (the study did not read them).
- `crates/mzdraw/src/svg.rs` (the display list to a file: follow its shape), `crates/mzdraw/src/png.rs` (a binary writer with tests).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-18-pdf-writer` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). Size: no `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard). The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Writer from the display list; text widths from the advance-width tables already used for the SVG.
2. Embedded document and the reader that finds it.
3. "PDF" in the Download menu; `docs/mzdraw-formats.md` §1a PDF row; size measured.

## Tests to add
- Unit: the xref offsets are exact (parse the file back by hand in the test), the embedded document reopens to the same bytes, the text of a label is present as text, the file is the same for the same document.
- A check in `tools/` or the e2e that Chromium's PDF viewer opens the file without error (no new npm package).
- Parity: `"out":"pdf"` in the golden cases. e2e: download the PDF by keyboard and touch and reopen it with Open a file.

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
