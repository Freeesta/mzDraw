# plan-10: which hosts keep the document inside our pictures

Proposed by `docs/plan/backlog.md` (row 32; Gemini card 2 and experiment 2; the PDF writer is arch-std-18). Size S. Wave 3. Depends on: arch-std-18 (PDF writer).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-10 survival-matrix: received, starting on branch `claude/plan-10-survival-matrix`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Tools and a report only; no editor code. It tells the owner, with evidence, where a picture still reopens as a drawing.

## Scope
In:
- `tools/survival/run.sh`: takes the goldens (SVG, PNG, PDF) and passes each through the programs that can run headless in the container: `rsvg-convert` (SVG→PDF, SVG→PNG), Inkscape (SVG→PDF, SVG plain save), LibreOffice headless (insert the image into a Writer document, save as ODT and DOCX, export PDF), ImageMagick and `optipng` (PNG re-encode), `pngcrush -rem allb` as the hostile case; then tries to reopen each output with the native replay `open` and records whether the document comes back byte-identical.
- `docs/plan/survival.md`: the matrix (host × format × kept / stripped / reopened), program versions and date; plus a checklist for a person to run the same three files through Word and PowerPoint on macOS and Windows, Google Docs and Typst, with a row to fill per host.
- If a common re-encoder strips the PNG `iTXt` chunk but keeps `tEXt` or `zTXt`, say so: that is a finding for the coordinator, not a change here.

Out (do not do here):
- Changing the writers (findings go to the coordinator).

## CLI twin (rule 15)
- `tools/survival/run.sh > survival.json` (exit 0; a missing program is reported as `not run`, never as kept).

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- `png.rs`, `svg.rs` (`svg::carrying`), `pdf.rs` of arch-std-18.
- Office keeps an inserted SVG as an `svgBlip` with a PNG fallback (MS-ODRAWXML; whether `<metadata>` survives is not documented: this is what the person's checklist answers).

## Agreement needed before the PR
None: this slice adds no `draw_edit/1` operation or field and changes nothing in `mzlab-mol` / `mzlab-feat`. If you find you need one, stop and ask the coordinator (it asks COO 16).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-10-survival-matrix` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Install what the container allows (apt), pin and record versions.
2. Write the runner and the reopen check.
3. Write the matrix and the checklist.

## Tests to add
- The runner's own check: on an unmodified golden it reports `reopened` for all three formats (a test in `tests/survival_self.sh`, run in CI only if the programs are present).
- **Fails without the change**: `pngcrush -rem allb` must report `stripped` for the PNG; if it reports `reopened`, the reopen check is broken.

## Independent proof
Correctness must not rest on our own claims. The hosts are independent programs; the reopen test is our reader on their output, with byte comparison of the recovered document. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

