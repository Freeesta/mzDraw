# tp-01: arrows that know their structures (anchors)

Proposed by `docs/study/tp-draw.md` (verdict V2, idea 2 "Δm/z metro map", part 1; the base of every automatic arrow). Size M. Depends on: plan-01 (the op table; the two new fields are rows of it). Wave: the first free slot after plan-01 merges.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-01 arrow-anchors: received, starting on branch `claude/tp-01-arrow-anchors`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only.

## Why
Today an arrow is two free points (`x, y, x2, y2`); nothing in the document says which structure it leaves and which it reaches, so no command can route, re-route or lay out arrows. A transformation-product (TP) scheme is a graph: parent → TP1 → TP2. This slice makes that graph explicit and stable, by id.

## Scope
In:
- Two optional fields of an `arrow` object in `mzdraw/1`: `from` and `to`, each the id of **one atom** of a structure (the structure is the connected piece that holds the atom; the same choice as `ann` and `data`, which also live on one atom). Absent or `0` = free end (today's arrows are unchanged, byte for byte).
- `obj.add` and `obj.set` take `from` and `to` (an `obj.set` with `"from":0` takes the anchor away); exact inverse in both directions.
- A **dangling** anchor (its atom was removed) is kept as written and treated as free when used; it is never rewritten by a remove, so undo stays exact. `std/1` gets one finding `std.scheme.anchor` (grade `warn`) listing such arrows.
- **Ends follow**: `fn route(doc, arrow) -> Option<Op>` computes the end points of an anchored arrow: the segment between the centres of the two structures' bounding boxes (label boxes of `scene.rs` included), clipped at each box inflated by `margin` (setting `scheme.margin`, default 0.35 bond lengths). Every editor command that moves, rotates, flips, adds to or removes from an anchored structure appends, in the same undo step, the `move` (tail) and `obj.set` (`x2`, `y2`) ops that put its arrows on their routed ends. The log carries those ops, so a native replay gives the same bytes without recomputing anything.
- Command `scheme:anchor` (on the selection: one arrow and the structures nearest its two ends, nearest by box distance within 1 bond length, ties by smallest atom id) and `scheme:route` (re-route every anchored arrow; one undo step; no op when nothing moves).
- The arrow tool: an arrow drawn from inside one structure's box to inside another's is anchored at once (`from` = smallest atom id of the start structure, `to` = of the end structure) and routed.
- KET, CDXML, RXN writers ignore the anchors (they have no such field, listed in `docs/mzdraw-formats.md` as "kept only in `mzdraw/1`"); `save("mzdraw")` and the pictures that carry the document keep them.

Out (do not do here):
- Layout of the whole scheme (tp-04), labels (tp-05), polyline arrows.

## CLI twin (rule 15)
- `{"op":"obj.add","t":"arrow","x":0,"y":0,"x2":2,"y2":0,"from":1,"to":12}` and `{"op":"obj.set","id":30,"to":0}` replay natively.
- `{"command":"scheme:route"}` in the replay request writes the ops in the log; `"out":"std"` lists dangling anchors.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5), `docs/study/tp-draw.md` (sections 3 and 4).
- `doc.rs` (`Obj`, `More`), `ops.rs` (`obj.add`, `obj.set`, `move`), `rxn.rs` (roles by place), `scene.rs` (label boxes), `conform.rs`.

## Agreement needed before the PR
Yes: `from` and `to` on `obj.add` / `obj.set` are new fields of `draw_edit/1`. Ask the coordinator first (it asks COO 16); the plan-01 table gets the two rows.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-01-arrow-anchors` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Fields, reader, writer (after the known fields, before `More`), op arms and inverses.
2. `route` with the box clip (a segment against an axis-aligned rectangle, written here with `libm`); the follow hook in the command layer (one place, not per tool).
3. Arrow tool anchoring; `scheme:anchor`, `scheme:route`; the `std.scheme.anchor` finding.
4. Goldens: a parent with two anchored arrows to two TPs, moved by keyboard (`Alt+Arrow`), the arrows following.

## Tests to add
- Property (ops.rs): apply then undo is the identity for `obj.add` / `obj.set` with every combination of `from` / `to` (set, change, clear), and for a `remove` of an anchored atom followed by undo (the anchor comes back pointing at the restored atom).
- Property: after any `move` / `rotate` / `flip` command on an anchored structure, each anchored arrow's tail lies on the inflated box of its `from` structure and its head on that of its `to` structure (within 1e-4, the document's 4 decimals), and the arrow lies on the line through the two box centres (within 1e-4).
- A document without anchors gives the same bytes as before (all existing goldens unchanged).
- e2e (Chromium): draw two structures, draw an arrow between them with the mouse, then with touch; move one structure with `Alt+Arrow`; the SVG equals the golden.
- **Fails without the change**: the follow property fails on `main` (the arrow stays where it was after the move).

## Independent proof
Correctness must not rest on our own claims. The clip is checked against a brute-force sampler in the test (10,000 points along the segment, the first and last outside the inflated boxes) on 200 seeded random box pairs; the parity of wasm and native (`tests/parity.mjs`) on the new log cases. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

