# tp-05: pathway layout core: arrows that organise themselves (deterministic, symmetric)

Proposed by `docs/plan/backlog.md` §11 (row 85) and `docs/study/tp-draw.md` (verdict V2, part 3; the owner's first pain). Size M. Depends on: tp-01 (anchors, routing, `commands`), plan-20 (`draw-result/1`). Reads plan-13's `scheme.rs` if merged (box helpers), and writes its own module. Wave: right after tp-01. The page, the e2e test and the Graphviz proof are tp-06.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-05-pathway-layout: received, starting on branch `claude/tp-05-pathway-layout`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only, no page. A TP scheme is a tree, sometimes a DAG, of structures. The time goes on placing them and their arrows by hand so that branches are even, arrows have the same slope and nothing touches, and this has to be redone after every new TP. This slice adds one command that lays out the whole pathway from the graph of tp-01. It also defines "symmetric" as properties a test can check.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- A new module `crates/mzdraw/src/pathway.rs` (its own zone).
- `fn layout(doc, opts) -> Result<Layout, Refusal>`: the unrounded f64 positions plus the report.
- `fn ops(&Layout) -> Vec<Op>`: the rounded `move` ops, then tp-01's routing ops. No new op.
- Command `pathway:tree`: one undo step.
- Settings section `pathway`, through `settings::FIELDS`.

### The specification (normative; the tests below check every line)
**Terms.**
- Two inflations are named apart: the **route box** is a box inflated by `pathway.margin` (0.35, where tp-01 clips arrows); the **clearance box** is a box inflated by `row_gap/2` (what must not overlap).
- A **node** is a structure (connected piece) that is the `from` or `to` of a non-dangling anchored arrow.
- Its **box** is its bounding box with labels (the boxes of `scene.rs`), of width `w` and height `h`, with centre `c`.
- An **edge** is an anchored arrow `u → v`.
- Free arrows, plus signs and texts are not moved; the report lists them.

**Settings** (`pathway.*`, in bond lengths unless said). They are named apart from plan-13's `scheme:row` argument `gap` (1.0), which is a different spacing.
| key | values | default | meaning |
|---|---|---|---|
| `dir` | `right` \| `down` | `right` | direction of the layers |
| `col_gap` | number | 2.5 | least gap between columns |
| `row_gap` | number | 1.0 | least gap between two boxes of a column |
| `max_angle` | degrees | 40 | slope of the steepest primary arrow |
| `min_arrow` | number | 1.2 | least visible arrow length after clipping |
| `order` | `id` \| `mz` \| `rt` | `id` | sibling order key: the numeric data field `MZ` or `RT` of the node, missing values last |
| `margin` | number | 0.35 | box inflation for routing (tp-01) |

**Refusals.** Each is exit 1 with `failed.stage` `command`, and nothing changes:
- a directed cycle: `error.draw2.schemeCycle`; `failed.id` is the smallest arrow id on the cycle;
- two arrows with the same `from` and `to` structures: `error.draw2.schemeDuplicate`;
- no anchored arrow: `error.draw2.schemeEmpty`.

**Steps** (for `dir=right`; `down` swaps x and y on the way in and on the way out, and nothing else):
1. **Layers.** `L(v)` is the length of the longest directed path from any root (in-degree 0) to `v`.
2. **Forest.** A node with several parents keeps one **primary** parent: the parent with the largest `L`, ties by the order key, then by the smallest atom id. Primary edges form a forest `F`; the other edges are **secondary**.
3. **Order.** The children of a node in `F` are sorted by the order key, ties by the smallest atom id. The roots of a component are sorted the same way. Components (weakly connected) are sorted by their first root.
4. **Tidy placement in y**, per component, with the box heights as node sizes and `row_gap` as the least separation between neighbouring subtrees at every layer:
   - The algorithm is Walker's tidy tree as made linear by Buchheim, Jünger and Leipert (2002)`*`, with the roots of a component placed as children of a virtual root.
   - A parent's centre is the middle of its first and last child's centres.
   - Neighbouring subtrees are pushed together until their contours are `row_gap` apart.
   - Smaller subtrees that lie between two larger ones are spaced evenly.
   - If P7 or P8 fails for that algorithm on any test tree: place the tree twice (left-to-right and right-to-left sibling scan), take the mean of the two placements, and say so in the PR. With that fallback P13 is weakened, as P13 states.
5. **Columns in x, per component.** Columns are computed for each component on its own, so a raised gap in one component never moves another. In a component, all boxes of layer `k` share the centre `X_k`, and `W_k` is the widest box of layer `k` in that component. Then `X_{k+1} = X_k + W_k/2 + g_k + W_{k+1}/2`, and `X_0` is the x of the component's first root after step 6.
   - `g_k` is the least value ≥ `col_gap` for which every primary edge between layers `k` and `k+1` has a slope of at most `max_angle` (on the line between box centres) and a visible length of at least `min_arrow` after clipping. Once tp-07 exists, `g_k` must also leave room for the edge's label.
   - The report says, per component and `k`, whether `g_k` was raised and by which rule.
   - A primary arrow that, after this, still meets a route box other than its two ends (a narrow box next to a wide one in the same column) is reported as `std.scheme.crossing` (warn). No gap is raised for it, since a wider gap would not remove a crossing inside a column.
6. **Anchor and stacking**, one rule:
   - The **first** component is translated so that its first root's box centre stays where it was before the command.
   - Each later component is placed below the one before it, `2·row_gap` apart box to box. Its own first root has the same x as the first component's first root.
   - So only the first root is a fixed point.
7. **Arrows.** Every edge is routed by tp-01 (straight, on the centre line, clipped at the route boxes) on the **rounded** positions: the moves of step 8 are applied first and `route` reads the moved document, so the arrows are exactly what a later `pathway:route` would compute. Any edge, primary or secondary, whose visible segment meets a route box other than its two ends is reported as `std.scheme.crossing` (warn). Polyline routing is not in this slice.
8. **Output.**
   - One `move` per structure whose displacement is not zero, ordered by the structure's smallest atom id. The displacement is the rounded target minus the current position, both components at 4 decimals, the document's precision.
   - Then the arrow ops, ordered by arrow id.
   - The report goes into `std/1` findings: `std.scheme.gapRaised` (info, per raised `k`), `std.scheme.free` (info), `std.scheme.crossing` (warn).

**Properties** ("symmetric" and "organised", measured). Each is checked twice: on `layout`'s unrounded positions with ε = 1e-9, and on the saved document after `ops` with ε = 3e-4 (rounding of two coordinates of 5e-5 each, plus the clip). Angles are compared through direction vectors with the same ε. In P7 and P9, "mirror image in shape" means equal box sizes, recursively, in reversed sibling order.
- **P1 determinism**: the same document gives the same bytes of ops, in wasm and natively.
- **P2 order independence**: shuffling the atom, bond and object arrays of the document (same ids) gives the same ops.
- **P3 translation**: translating the input by (a, b), with a and b multiples of 1e-4 (the document's grid), translates the output by (a, b) and gives byte-identical ops. Other translations are not claimed, because rounding breaks them.
- **P4 idempotence**: `pathway:tree` on its own output gives no op (the layout of the rounded output rounds to the same positions, and routing already used rounded positions, step 7).
- **P5 columns**: in each component, every node of layer `k` has box centre x = `X_k` of that component. The gap between columns `k` and `k+1` is `g_k` ≥ `col_gap`, and equals `col_gap` when the report says it was not raised.
- **P6 centring**: every parent's centre y is the mean of its first and last primary child's centre y. A node with one primary child is on that child's line, so its arrow is horizontal.
- **P7 fan symmetry**: if the whole child sequence of `u` is mirror-symmetric in shape (for every `i`, the subtrees of `v_i` and `v_{m+1-i}` are mirror images, and for an odd `m` the middle child's subtree is its own mirror image), then `y(v_i) + y(v_{m+1-i}) = 2·y(u)` for every `i`, and for an odd `m` the middle child is on `u`'s line. Nothing is claimed for a sequence that is not mirror-symmetric as a whole.
- **P8 mirror**: for a component that is a tree (one root, every node with at most one parent, so no tie-break of step 2 is involved), reversing every sibling order (the test-only key `order=reverse-id`) reflects the component about its root's line, **relative to its root**: for every node, `y' − y'_root = −(y − y_root)` and `x' − x'_root = x − x_root`. For the first component the root does not move (step 6), so this is also absolute; later components may be stacked at another y, because their extents change under reversal. The order of components is not reversed. Multi-root components and DAGs are not claimed: under the reversed key, step 2's tie-break may choose another primary parent.
- **P9 arrow symmetry**: under P7, the arrows `u → v_i` and `u → v_{m+1-i}` have direction vectors (dx, dy) and (dx, −dy), and equal visible lengths when the two boxes are equal.
- **P10 even leaves**: when every child of a node is a leaf and all have equal box heights, neighbouring children have centre spacing `h + row_gap`.
- **P11 bounded slope and length**: every primary arrow has |slope angle| ≤ `max_angle` and a visible length ≥ `min_arrow`.
- **P12 no collision**: clearance boxes are pairwise disjoint (steps 4 and 5 enforce it). Primary arrows never cross each other (the forest is drawn in sibling order between adjacent columns). An arrow whose visible segment meets a route box other than its two ends is always reported as `std.scheme.crossing`; the claim is that the report is exact (every such meeting is listed, and nothing else), and each synthetic pathway's test states the crossings it expects (none for the trees of equal box sizes).
- **P13 compact**: for every node with at least two children, no two neighbouring child subtrees are closer than `row_gap` at any shared column, and the contour distance of some pair of child subtrees (not always neighbours: in large, small, large, the two large ones may touch past the small one) equals `row_gap`. Pairs spaced evenly between larger subtrees may be farther apart; P10 is the only spacing claimed for leaves. If step 4's two-pass mean is used, only the first half (no neighbouring pair closer than `row_gap`) is claimed, and the PR says so.
- **P14 locality**: adding a TP to component `k` leaves every node of components before `k` where it was. Components after `k` move only by one common translation in y (columns are per component, step 5).

Out (do not do here):
- The page button, the e2e test and the Graphviz proof (tp-06).
- Polyline (orthogonal or 45°) routing, lane pins for convergent pathways, a y axis by Δm/z or RT: the "metro" mode is a later slice once this one is used (study §3 V2b).
- Labels on arrows (tp-07); any change to a structure's shape.

## CLI twin (rule 15)
- `{"import":DOC,"set":["pathway.dir=right","pathway.order=mz"],"commands":[{"command":"pathway:tree"}],"out":"mzdraw"}` writes the moves in the log.
- `"out":"std"` lists raised gaps, free objects and crossings.
- `"dry_run":true` (plan-20) returns `draw-result/1` and applies nothing. A refusal is exit 1 with `failed.stage` `command`.
- `{"import":DOC,"out":"boxes"}` writes `boxes/1`: `{"format":"boxes/1","structures":[{"id":smallest atom id,"x0","y0","x1","y1"}],"arrows":[{"id","from","to","x","y","x2","y2"}]}`, in bond lengths, label-inclusive (the boxes of `scene.rs`), ordered by id, numbers at 4 decimals. It is the boxes the layout used, so an outside check needs no label metrics.
- Request rows (plan-01 `request` section, D13): the command `pathway:tree`, the output `boxes` (format `boxes/1`), the settings `pathway.*`, and the `std/1` keys `std.scheme.gapRaised`, `std.scheme.free`, `std.scheme.crossing`.

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state), `docs/plan/backlog.md` (sections 0, 4, 5, 9), `docs/study/tp-draw.md` (sections 3 and 4).
- tp-01 (`from`, `to`, `route`, `commands`), plan-13 `scheme.rs` (box helpers, read only), `scene.rs` (boxes), `settings.rs` (`FIELDS`), `conform.rs` (report rows), `outcome.rs` (plan-20).
- `*` Buchheim, Jünger, Leipert, "Improving Walker's algorithm to run in linear time", Graph Drawing 2002, LNCS 2528, doi:10.1007/3-540-36151-0_32. Read the public preprint if it opens; if it does not, say "not read" in the PR and implement from Walker's published description (Walker 1990, Software: Practice and Experience 20(7)). The properties above are the contract either way.

## Agreement needed before the PR
No `draw_edit/1` op or field: only moves and tp-01's fields. Request rows only (D13).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-05-pathway-layout` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Graph from anchors; cycle and duplicate checks; layers; forest; order.
2. Tidy placement (contours on box heights); columns with `g_k`; anchor and stacking.
3. Routing through tp-01; report; ops in the fixed order.
4. Synthetic pathways (structures you write yourself, `tests/data/pathways/*.json`):
   - a chain of 4;
   - stars 1→2, 1→3, 1→4, 1→5;
   - a full binary tree of depth 3;
   - an unbalanced tree (leaf, subtree of 5, leaf);
   - a DAG with a convergent TP (two parents → one TP) and a long edge (layer 0 → 2);
   - two components;
   - mixed sizes (benzene next to a tetracycle);
   - a messy star 1→3, laid out by hand with uneven arrows and one overlap;
   - 200 nodes (a seeded random tree, for the cost).

## Tests to add
- One test per property P1–P14 over all synthetic pathways, at both tolerances. P7–P9 run on the trees where their condition holds, and the test states which. Each refusal has a test.
- Undo of `pathway:tree` returns the identical document. Parity cases for three pathways.
- The 200-node pathway is laid out natively in less than 100 ms in release. The time is printed, not asserted in debug.
- **Fails without the change**: P6, P11 and P12 fail on the messy star as committed.

## Independent proof
Correctness must not rest on our own claims. Here: the properties are re-checked by an independent script on our output (tp-06 adds `tools/fidelity/pathway.py` with Graphviz). In this slice, `tools/fidelity/pathway_props.py` (about 200 lines of Python, no Rust shared) reads `boxes/1` from the native replay of each saved document (the label-inclusive boxes come from there; the script re-implements no label metrics), re-derives the graph from the anchors, the layers and the primary forest by the rules of steps 1–3, and re-checks P5, P6, P7, P9, P10, P11 and P12. For P8 it runs the native replay twice (key `id` and `reverse-id`) and compares. P1–P4, P13 and P14 stay with the Rust tests. It runs where Python 3 is installed; otherwise it says `skipped` and the PR lists it. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

## Kill test
If any of P6–P13 cannot hold together on the synthetic set, the spec is wrong and is fixed before the slice ships: stop and report which property and which pathway. Do not loosen ε or drop the property. If the tidy tree is slower than 100 ms on 200 nodes in release, report it; there is no fallback layout.

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

