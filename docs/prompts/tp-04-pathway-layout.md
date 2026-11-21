# tp-04: pathway layout, arrows that organise themselves (deterministic, symmetric)

Proposed by `docs/study/tp-draw.md` (verdict V2, idea 2 "Δm/z metro map", part 2; the owner's first pain). Size M. Depends on: tp-01 (anchors and routing). Reads `scheme.rs` of plan-13 if merged (box helpers), writes its own module. Wave: right after tp-01.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-04 pathway-layout: received, starting on branch `claude/tp-04-pathway-layout`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only.

## Why
A TP scheme is a tree (sometimes a DAG) of structures. Placing them and their arrows by hand so that branches are even, arrows have the same slope and nothing touches is where the time goes, and it has to be redone after every new TP. This slice makes one command that lays the whole pathway out from the graph of tp-01, and defines "symmetric" as properties a test can check.

## Scope
In: a new module `crates/mzdraw/src/pathway.rs` (its own zone), command `scheme:tree` (one undo step: `move` ops of whole structures, then the tp-01 routing ops of the arrows; no new op), settings section `scheme` (fields below, through `settings::FIELDS`), and a button "Arrange pathway" in the objects palette (paint and forward).

### The specification (normative; the tests below check every line)
Terms. A **node** is a structure (connected piece) that is the `from` or `to` of a non-dangling anchored arrow; its **box** is its bounding box with labels (the boxes of `scene.rs`), of width `w` and height `h`, centre `c`. An **edge** is an anchored arrow `u → v`. Free arrows, plus signs and texts are not moved (listed in the report).

Settings (`scheme.*`, in bond lengths unless said): `dir` = `right` (default) | `down`; `gap` = 2.5 (least horizontal gap between columns); `vgap` = 1.0 (least gap between two boxes of a column); `max_angle` = 40 (degrees, the steepest primary arrow); `min_arrow` = 1.2 (least visible arrow length after clipping); `order` = `id` (default) | `mz` | `rt` (sibling order key: the numeric data field `MZ` or `RT` of the node, missing last); `margin` = 0.35 (tp-01).

Refusals (nothing changes, keyed error): a directed cycle (`error.draw2.schemeCycle`, naming its arrows); two arrows with the same `from` and `to` structures (`error.draw2.schemeDuplicate`); no anchored arrow (`error.draw2.schemeEmpty`).

Steps (for `dir=right`; `down` swaps x and y on the way in and on the way out, nothing else):
1. **Layers.** `L(v)` = the length of the longest directed path from any root (in-degree 0) to `v`.
2. **Forest.** Each node with several parents keeps one **primary** parent: the parent with the largest `L`, ties by the order key, then by the smallest atom id. Primary edges form a forest `F`; the others are **secondary**.
3. **Sibling order.** The children of a node in `F` are sorted by the order key, ties by the smallest atom id. Roots of a component the same way. Components are sorted by their first root.
4. **Tidy placement in y.** Each tree of `F` is placed by the tidy-tree algorithm of Walker as made linear by Buchheim, Jünger and Leipert (2002)`*`, with the box heights as node sizes and `vgap` as the least separation between neighbouring subtrees at every layer: a parent's centre is at the middle of its first and last child's centres; subtrees are pushed together until their contours are `vgap` apart; smaller subtrees between two larger ones are spaced evenly. If the mirror property P8 fails for that algorithm on any test tree, place the tree twice (left-to-right and right-to-left sibling scan), average the two, and say so in the PR.
5. **Columns in x.** All boxes of layer `k` share the centre `X_k`. `W_k` is the widest box of layer `k`. `X_{k+1} = X_k + W_k/2 + g_k + W_{k+1}/2`, where `g_k` is the least of the values ≥ `gap` such that every primary edge between layers `k` and `k+1` has a slope of at most `max_angle` (line between box centres) and a visible length of at least `min_arrow` after clipping (and, once tp-05 exists, room for its label). The report says, per `k`, whether `g_k` was raised and by which rule.
6. **Components** are stacked in y in their order, `2·vgap` apart (box to box).
7. **Anchor.** Each component is translated so that its first root's box centre stays where it was before the command.
8. **Arrows.** Every edge is routed by tp-01 (straight, centre line, clipped). A secondary edge whose segment meets an inflated box other than its two ends is listed as `std.scheme.crossing` (warn); polyline routing is not in this slice.
9. **Output.** One `move` per structure whose displacement is not zero (both components rounded to 4 decimals, the document's precision), ordered by the structure's smallest atom id; then the arrow ops ordered by arrow id.

Properties ("symmetric" and "organised", measured; ε = 1e-4 bond lengths, the 4 decimals of the document; angles compared through direction vectors with the same ε):
- **P1 determinism**: the same document gives the same bytes of ops, in wasm and natively (`tests/parity.mjs`).
- **P2 order independence**: shuffling the atom, bond and object arrays of the document (same ids) gives the same ops.
- **P3 translation**: translating the input by (a, b) translates the output by (a, b) and gives the same ops.
- **P4 idempotence**: `scheme:tree` on its own output gives no op.
- **P5 columns**: every node of layer `k` has box centre x = `X_k`; the gap between columns `k` and `k+1` is `g_k` ≥ `gap`, and equals `gap` when the report says it was not raised.
- **P6 centring**: every parent's centre y is the mean of its first and last primary child's centre y. A node with one primary child is on the same line as that child (its arrow is horizontal).
- **P7 fan symmetry**: if children `v_i` and `v_{m+1-i}` of `u` head subtrees that are mirror images in shape (box sizes, recursively), then `y(v_i) + y(v_{m+1-i}) = 2·y(u)`; for an odd `m` the middle such child lies on `u`'s line.
- **P8 mirror**: reversing every sibling order (key `order` with the test-only reversed key) reflects the layout about the first root's line: `y' = 2·y_root − y`, `x' = x`, for every node.
- **P9 arrow symmetry**: under P7, the arrows `u → v_i` and `u → v_{m+1-i}` have direction vectors (dx, dy) and (dx, −dy) and equal visible lengths when the two boxes are equal.
- **P10 even spacing**: neighbouring leaf siblings with equal box heights have equal centre spacing, `h + vgap`.
- **P11 bounded slope and length**: every primary arrow has |slope angle| ≤ `max_angle` and visible length ≥ `min_arrow`.
- **P12 no collision**: boxes inflated by `vgap/2` are pairwise disjoint; no primary arrow meets an inflated box other than its two ends; primary arrows never cross each other.
- **P13 compact**: for every pair of neighbouring sibling subtrees, the smallest vertical distance between their boxes in a shared column is exactly `vgap` (they were pushed together).
- **P14 locality**: nodes of other components do not move when a TP is added to one component.

Out (do not do here):
- Polyline (orthogonal or 45°) routing, lane pins for convergent pathways, a y axis by Δm/z or RT (the "metro" mode): a later slice once this one is used (study §4, V2b).
- Labels on arrows (tp-05); any change to a structure's shape.

## CLI twin (rule 15)
- `{"command":"scheme:tree","set":["scheme.dir=right","scheme.order=mz"]}` in the replay request writes the moves in the log; `"out":"std"` lists raised gaps, free objects and crossings; `--dry-run`: `"dry":true` returns the ops and the report without applying them (exit 0, or the keyed error with its exit code).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state"), `docs/plan/backlog.md` (sections 0, 4 and 5), `docs/study/tp-draw.md` (sections 3 and 4).
- tp-01 (`from`, `to`, `route`), plan-13 `scheme.rs` (box helpers, read only), `scene.rs` (boxes), `settings.rs` (`FIELDS`), `conform.rs` (report rows).
- `*` Buchheim, Jünger, Leipert, "Improving Walker's algorithm to run in linear time", Graph Drawing 2002, LNCS 2528, doi:10.1007/3-540-36151-0_32: read the public preprint if it opens; if not, say "not read" in the PR and implement from Walker's published description (Walker 1990, Software: Practice and Experience 20(7)) and the properties above, which are the contract either way.

## Agreement needed before the PR
None: no `draw_edit/1` op or field (moves and tp-01's fields only). The `scheme.*` settings are a new settings section; tell the coordinator in the PR.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-04-pathway-layout` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Graph from anchors; cycle and duplicate checks; layers; forest; order.
2. Tidy placement (contours on box heights); columns with `g_k`; components; anchor translation.
3. Routing through tp-01; report; ops in the fixed order; `dry`.
4. Synthetic pathways (structures you write yourself, `tests/data/pathways/*.json`): a chain of 4; stars 1→2, 1→3, 1→4, 1→5; a full binary tree of depth 3; an unbalanced tree (leaf, subtree of 5, leaf); a DAG with a convergent TP (two parents → one TP) and a long edge (layer 0 → 2); two components; mixed sizes (benzene next to a tetracycle); 200 nodes (seeded random tree, for the cost).

## Tests to add
- One test per property P1–P14 over all synthetic pathways (P7–P9 on the trees where the condition holds; the test states which), and each refusal.
- Undo of `scheme:tree` returns the identical document.
- The 200-node pathway is laid out natively in less than 100 ms in release (printed, not asserted in debug).
- e2e (Chromium): a messy star of 1→3 drawn with anchored arrows; "Arrange pathway" by mouse, then by touch on a reloaded page, then by the keyboard command; the SVG equals the golden each time; screenshots before/after at deviceScaleFactor 2 in the PR.
- **Fails without the change**: P6, P11 and P12 fail on the messy synthetic star as drawn (it is committed laid out by hand with uneven arrows and one overlap).

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/pathway.py` gives the same graphs to Graphviz `dot -Grankdir=LR -Tjson` (fetched at run time): for every tree the ranks equal our layers, and our primary-edge crossing count (0) is ≤ dot's; for the DAG the difference of ranks (dot minimises total edge length, we use the longest path) is listed, not failed. The P7/P8 checks are re-run in the script from our output alone, independent of the Rust test code. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

