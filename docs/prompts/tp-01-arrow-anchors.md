# tp-01: arrows that know their structures (anchors and follow)

Proposed by `docs/study/tp-draw.md` (verdict V2, idea 2 "Δm/z metro map", part 1: the base of every automatic arrow). Size M. Depends on: plan-01 (op table and its `request` section), plan-20 (`draw-result/1`). Wave: the first free slot after both merge. Order with plan-13: see "Scope", last item.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-01-arrow-anchors: received, starting on branch `claude/tp-01-arrow-anchors`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only, no page change (the tool and the commands are tp-02). Today an arrow is two free points (`x, y, x2, y2`), and nothing says which structure it leaves or reaches. A transformation-product (TP) scheme is a graph (parent → TP1 → TP2); this slice makes the graph explicit and stable, by id.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- **Fields.** Two optional fields of an `arrow` object in `mzdraw/1`: `from` and `to`. Each is the id of **one atom** of a structure; the structure is the connected piece that holds the atom, the same choice as `ann` and `data`. Absent or `0` means a free end, so today's arrows are unchanged byte for byte.
- **Ops.** `obj.add` and `obj.set` take `from` and `to`; an `obj.set` with `"from":0` removes the anchor. Exact inverse both ways.
- **Dangling anchor.** When the anchored atom is removed, the anchor is kept as written and treated as free when used. A remove never rewrites it, so undo stays exact. `std/1` gets one finding `std.scheme.anchor` (grade `warn`) listing such arrows.
- **Routing.** `fn route(doc, arrow) -> Vec<Op>` gives the ops that put an anchored arrow on its routed ends: a `move` of the arrow object for the tail, then an `obj.set` of `x2`, `y2`; no op when the arrow is already there. The routed ends are the segment between the centres of the two structures' bounding boxes (labels included, the boxes of `scene.rs`), clipped at each box inflated by `pathway.margin` (a new setting, default 0.35 bond lengths, through `settings::FIELDS`).
- **Follow is a command behaviour, not an op behaviour.** Every editor *command* that moves, rotates or flips an anchored structure, or adds atoms to it or removes atoms from it, appends the `route` ops of its arrows in the same undo step. The log carries those ops. So:
  - a native replay of the page's log gives the page's bytes;
  - a bare `{"op":"move"}` in a request's `log` stays a bare move, natively and in the page alike, and leaves anchored arrows where they were;
  - an agent that writes raw ops runs `pathway:route` (tp-02) or sends its edit as a command (`commands`, below) to get the follow.
- **Commands in the request.** `"commands":[{"command":NAME,"args":{...}}]` runs after `log` and before `out`. Each command's ops are appended to the log, and a refused command stops the request: nothing is written, exit 1, `draw-result/1` with `failed.stage` `command`, `failed.op` the 0-based index in `commands`, `failed.key` the i18n key. plan-13 and plan-14 prompts write `{"command":...,"args":...}`. This slice adds the request rows `commands`, `command` and `args`, and plan-13/14 use the same rows. Whoever merges first adds them; the later prompt reuses them. In this slice the editor's existing `move`, `rotate` and `flip` commands become callable as commands, so a raw-op agent has a following move.
- **Order with plan-13.** plan-13's `scheme:row` sets arrow ends with `obj.set`.
  - If tp-01 merges first, plan-13 routes anchored arrows through `route` instead of setting their ends itself.
  - If plan-13 merges first, this slice makes `scheme:row` call `route` for anchored arrows (one test: an anchored arrow is on its routed ends after `scheme:row`).
  - Free arrows keep plan-13's behaviour either way.
  - The backlog edit E20 says the same.
- **Formats.** KET, CDXML and RXN have no such field; `docs/mzdraw-formats.md` lists the anchors as "kept only in `mzdraw/1`". `save("mzdraw")` and the pictures that carry the document keep them.

Out (do not do here):
- The arrow tool, `pathway:anchor`, `pathway:route`, the page (tp-02).
- Layout (tp-05, tp-06), labels (tp-07), polyline arrows.

## CLI twin (rule 15)
- `{"log":[{"op":"obj.add","t":"arrow","x":0,"y":0,"x2":2,"y2":0,"from":1,"to":12}]}` and `{"log":[{"op":"obj.set","id":30,"to":0}]}` replay natively.
- `{"import":DOC,"commands":[{"command":"move","args":{"atoms":[1,2,3],"dx":1,"dy":0}}],"out":"mzdraw"}` moves and routes. `"out":"std"` lists dangling anchors. With `"report":true` (plan-20), the answer is `draw-result/1`.
- Request rows (plan-01 `request` section, D13): `commands`, `command`, `args`, the stage `command` of `draw-result/1`. Op-table rows: the fields `obj.add.from`, `obj.add.to`, `obj.set.from`, `obj.set.to`, with `mzlab:false` until mzLab's reader adopts them.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4, 5, 9), `docs/study/tp-draw.md` (sections 3 and 4).
- `doc.rs` (`Obj`, `More`), `ops.rs` (`obj.add`, `obj.set`, `move`), `replay.rs`, `outcome.rs` (plan-20), `schema.rs` (plan-01), `scene.rs` (label boxes), `conform.rs`, plan-13's prompt (`scheme:row`).

## Agreement needed before the PR
Yes. `from` and `to` on `obj.add` / `obj.set` are new **fields of `draw_edit/1` ops**, so they need COO 16's agreement before the PR: ask the coordinator first. The request rows need only the table review (D13).

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
1. Fields, reader, writer (after the known fields, before `More`), op arms and inverses, op-table rows.
2. `route` with the box clip: a segment against an axis-aligned rectangle, written here with `libm`.
3. The follow hook in the command layer: one place, not per tool.
4. `commands` in `replay.rs`, stage `command`; the plan-13 order rule; the `std.scheme.anchor` finding.

## Tests to add
- Property (ops.rs): apply then undo is the identity for `obj.add` / `obj.set` with every combination of `from` / `to` (set, change, clear), and for a `remove` of an anchored atom followed by undo (the anchor comes back pointing at the restored atom).
- Property: after any `move` / `rotate` / `flip` *command* on an anchored structure, each anchored arrow's tail lies on the inflated box of its `from` structure and its head on that of its `to` structure, and the arrow lies on the line through the two box centres. Check before rounding with ε = 1e-9, and on the saved document with ε = 3e-4 (two rounded ends of 5e-5 each, plus the clip).
- Bare op: a `log` with a raw `move` gives the same bytes natively and in wasm, and leaves the arrow unrouted (documented behaviour).
- Parity: the page's log of a keyboard move (`Alt+Arrow`) replayed natively gives the page's bytes, including the route ops.
- A document without anchors gives the same bytes as before (all existing goldens unchanged).
- A refused command: exit 1, `failed.stage` `command`, nothing written.
- **Fails without the change**: the follow property fails on `main` (the arrow stays where it was after the move).

## Independent proof
Correctness must not rest on our own claims. The clip is checked against a brute-force sampler in the test (10,000 points along the segment; the first and last points outside the inflated boxes) on 200 seeded random box pairs. The parity of wasm and native (`tests/parity.mjs`) is checked on the new log cases. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

## Kill test
On the velocity task set (plan-02), add one task: "parent with three TPs and arrows; move the parent". Count the actions after this slice and tp-02. If following saves fewer than 2 actions per moved structure against re-drawing the arrows, keep the anchors (tp-05 needs them) and drop the follow hook in a follow-up (say so in the PR of tp-02).

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

