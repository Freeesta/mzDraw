# plan-17: binary CDX reader: fragments, nodes and bonds

Proposed by `docs/plan/backlog.md` (row 19; Gemini do-not 5 (reframed: OLE dropped, CDX reading kept); built state: "Binary CDX is not read"). Size M. Wave 1. Depends on: none (backlog Q6).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-17 cdx-read-core: received, starting on branch `claude/plan-17-cdx-read-core`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. 

## Scope
In:
- A new module `crates/mzdraw/src/cdx.rs`: a reader of the CDX binary container (header, objects and properties as tag–length–value, nesting) with every length bounded and checked, depth and object count bounded as in `xml.rs`; no panics on hostile input.
- Maps the same content as the CDXML reader (`cdxml.rs`) for fragments, nodes (element, charge, isotope, radical, position), bonds (order, wedge display), and the labelled nodes (R groups, abbreviations of our table, else query atoms listed): build the same intermediate the CDXML reader builds and reuse its mapping, so the two readers cannot disagree.
- Detection by the magic bytes; `Editor::open` routes CDX to it; what is not read is counted and said (`draw2.import.other`) as for CDXML.

Out (do not do here):
- Text, arrows, graphics (plan-18).
- Writing CDX (deferred).

## CLI twin (rule 15)
- Opening a `.cdx` through the replay request `open` (bytes base64) gives the same document as the page.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- `cdxml.rs` (what is read and how it maps), the CDX SDK documentation read for slice 3d (the IUPAC copy; binary format pages: header, object and property tags, coordinates). Summarise, do not copy. Mark any tag meaning not stated on the pages read.
- Octet-Draw (Apache-2.0) reads CDX; do not read or copy its code.

## Agreement needed before the PR
None: this slice adds no `draw_edit/1` operation or field and changes nothing in `mzlab-mol` / `mzlab-feat`. If you find you need one, stop and ask the coordinator (it asks COO 16).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-17-cdx-read-core` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Container reader with bounds; property decoding for the tags needed.
2. Shared intermediate with `cdxml.rs` (a refactor inside `cdxml.rs` only where needed, by test unchanged output).
3. Synthetic CDX files: write them with a tiny writer in the test (from the documented layout), not from any program.

## Tests to add
- Five synthetic CDX files written by the test writer from the documented layout read to the same document as their CDXML twins written by hand.
- Hostile input: truncated lengths, nesting 10,000 deep, a length past the end: refused with a key, no panic.
- Parity; e2e: drop a `.cdx`.
- **Fails without the change**: opening a `.cdx` is refused today.

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/cdx.py`: Open Babel 3.1.1 (`obabel -icdx`) reads the same synthetic CDX files; the committed reference compares InChIKeys with ours (Open Babel is independent of us and of the SDK pages). Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

