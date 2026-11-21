# plan-13: lay out a one-line reaction scheme

Proposed by `docs/plan/backlog.md` (row 23; Gemini card 3 (reframed: no MCS engine, moves only)). Size M. Wave 2. Depends on: none.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-13 scheme-row: received, starting on branch `claude/plan-13-scheme-row`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. 

## Scope
In:
- A new module `crates/mzdraw/src/scheme.rs`: `fn row(doc, opts) -> Vec<Op>` that finds the structures, arrows, plus signs and texts of a drawing (or of the selection), orders them along the main direction, and returns `move` ops (and `obj.set` of arrow ends) that put them on one baseline: equal gaps of `gap` bond lengths (default 1.0), each structure's centre of its bounding box on the arrow line, arrow length fitted to its texts (above/below) plus margins, plus signs centred between neighbours; `wrap` (a width in bond lengths) breaks the row after an arrow and starts the next line under it.
- Command `scheme:row` (one undo step: the ops applied together); the log gets the `move` ops, so the CLI replays them with no new op.
- Page: a button in the objects palette (paint and forward).

Out (do not do here):
- Orienting cores (plan-14).
- Catalytic cycles (deferred); several rows with branching arrows.

## CLI twin (rule 15)
- The replay request takes `{"command":"scheme:row","args":{"gap":1,"wrap":0}}` and writes the moves in the log; `out` as usual.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- `objects` of `doc.rs`, `rxn.rs` (how a structure is reactant, agent or product by its place along the arrow: reuse it), `layout.rs`.

## Agreement needed before the PR
None: this slice adds no `draw_edit/1` operation or field and changes nothing in `mzlab-mol` / `mzlab-feat`. If you find you need one, stop and ask the coordinator (it asks COO 16).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-13-scheme-row` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Ordering and roles from `rxn.rs` logic; bounding boxes from the label boxes of `scene.rs`.
2. Placement; wrap.
3. Goldens for three schemes.

## Tests to add
- Property: after `scheme:row`, no two bounding boxes overlap, every gap equals `gap` within 1e-9, every structure's box centre lies on its arrow's line, and `save("rxn")` gives the same reactants and products as before.
- Undo returns the identical document.
- e2e: select a messy scheme with the keyboard and touch, press the button, the SVG equals the golden.
- **Fails without the change**: the overlap property fails on the messy synthetic scheme before the command exists (it is laid out by hand with overlaps).

## Independent proof
Correctness must not rest on our own claims. Geometry invariants checked against the RXN roles of `rxn.rs`, which RDKit reads back the same (`tools/fidelity` RXN rows); and the velocity task "two-component reaction" counted before and after (plan-02). Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

