# plan-05: a cut line across bonds, as an object of the document

Proposed by `docs/plan/backlog.md` (row 22; Gemini card 1 (reframed: the engine is mzLab's `cut/1`)). Size M. Wave 2. Depends on: plan-01 (table entry).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-05 cut-object: received, starting on branch `claude/plan-05-cut-object`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. This slice draws and stores the cut; the masses come in plan-06.

## Scope
In:
- Object kind `cut` in `mzdraw/1` (`objects`): `{"t":"cut","bonds":[[a,b],...],"mode":"homolytic"|"heterolytic"|"hydrogen"}`; bonds are named by their two atom ids as everywhere in the log.
- Log: `obj.add` with `t:"cut"` and `obj.set` of `bonds` / `mode`, each with its exact inverse; a removed bond removes itself from every cut (the inverse restores it).
- Tool `cut` (`editor_tools.rs`, one arm): a drag across bonds makes one cut of the bonds the stroke crosses; a click on a bond with a selection of bonds cuts those. Key: none (std-06 assigns one later).
- Painting: a dashed line across each cut bond, perpendicular, two bond-widths past each side; drawn on screen and in SVG/PNG, not in molfile, KET or CDXML (listed under what the format does not hold).
- A cut whose bonds do not split the structure (a single ring bond) is kept and marked `open` in the page state so plan-06 can tell the user; a ring opening needs two bonds of the ring.

Out (do not do here):
- Masses, formulas, callouts (plan-06).
- Peak lists (plan-07).

## CLI twin (rule 15)
- `{"op":"obj.add","id":40,"t":"cut","bonds":[[3,4]],"mode":"hydrogen"}` in a log replayed natively gives the same SVG as the page.
- An op whose bond does not exist is refused with `error.draw2.cutBond` and changes nothing.

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- `crates/mzdraw/src/doc.rs` (objects), `ops.rs` (`obj.add`, `obj.set`), `scene.rs` (object painting), `editor_tools.rs`.
- mzLab `crates/mzlab-feat/src/chem/cut.rs` (`cut/1`: how mzLab names a cut and its modes; use the same mode words).

## Agreement needed before the PR
New object kind `cut` and its fields in `draw_edit/1`: send the coordinator the field list from the plan-01 table before the PR and wait for COO 16's agreement.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-05-cut-object` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Add the kind, the ops and their inverses; add them to the plan-01 table.
2. Tool and hit-test of a stroke against bond segments in Rust.
3. Painting on screen and in the SVG writer; goldens.
4. Page: the tool in the objects palette (icon from the UI icon list; paint and forward only).

## Tests to add
- `crates/mzdraw/tests/cut.rs`: a stroke across two bonds of a chain makes one cut with those bonds in log order; removing a cut bond and undoing restores the cut exactly.
- Property test of `ops.rs` extended with cut objects.
- Golden SVG of ethyl acetate with one cut.
- e2e: draw a cut with the mouse, with touch and from a keyboard selection of bonds plus the tool button.
- **Fails without the change**: the undo test (remove a bond that a cut names, undo) fails if the cut does not record the bond in the inverse.

## Independent proof
Correctness must not rest on our own claims. The same molfile and bond pair given to `mzlab-cli mol cut` gives pieces whose atoms are the two sides our cut separates (compared in a test through the plan-04 glue). Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

