# tp-02: derive a transformation product from its parent (scaffold kept)

Proposed by `docs/study/tp-draw.md` (verdict V1, idea 1 "ghost-parent scaffold lock", part 1). Size M. Depends on: tp-01 (anchors), plan-11 (atom maps). Wave: after both merge.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-02 derive: received, starting on branch `claude/tp-02-derive`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only.

## Why
A mass spectrometrist draws one parent and then ten to fifty TPs that differ from it by one or two edits. Redrawing each TP, or copying and re-aligning it by hand, is the slow part. Derive = copy the parent with its exact coordinates, apply the edit, anchor an arrow, keep the atom correspondence. Nothing is predicted: the person (or the agent) names the edit.

## Scope
In:
- A new module `crates/mzdraw/src/tp.rs`: `fn derive(doc, of: atom id, patch: &[Op], at: Option<(f64, f64)>) -> Result<Vec<Op>, &'static str>` that returns, as one list:
  1. the copy of the structure holding `of` (atoms, bonds, labels, wedges, data and annotation **not** copied: a TP is a new identification), with new ids from the document counter, translated so its box sits at `at` or, when `at` is absent, one column to the right of the parent (gap `scheme.gap`, default 2.5 bond lengths; below the last TP of the same parent if that place is taken, spacing `scheme.vgap`, default 1.0);
  2. atom maps (plan-11 `map`): if the parent atoms have none, they get `1..n` in id order first (`atom.set map`, part of the same list); each copy atom gets its source's map;
  3. the `patch`, written on the **parent's** ids and coordinates relative to the parent: ids of the parent are rewritten to the copy's ids, coordinates are translated with the copy, new atoms get new ids and the next free map numbers of the pathway (the largest map in the document + 1, so a map number names one atom across the whole pathway);
  4. an `obj.add` arrow with `from` = the smallest atom id of the parent, `to` = the smallest of the copy, routed (tp-01).
- A patch that leaves the copy invalid for `mzlab-mol` (valence over the allowed set, an atom left alone after a cleavage is fine) is refused with `error.draw2.tpPatch` and changes nothing.
- Command `tp:derive` (one undo step) and the page: "Derive TP" on the context menu of a structure (`context.rs` item id `tp.derive`) and key `Shift+D` (in arch-std-06's command table): the copy is made and selected so the next edit (from the keyboard or the mouse) applies to it; the person then edits as usual. A ring cleavage keeps every other atom where the parent had it (scaffold kept); no clean-up runs.
- When the parent with more than 999 atoms would need maps, the derive is refused with `error.draw2.tpMaps`.

Out (do not do here):
- The diff lens (tp-03), the layout (tp-04), the labels (tp-05), named transformations (tp-07).
- A "rigidity slider" (dropped in the study: not measurable, see §5); clean-up of the changed region is arch-std-15's.

## CLI twin (rule 15)
- `{"command":"tp:derive","args":{"of":1,"patch":[{"op":"atom.add","el":"O","x":1.5,"y":-0.866},{"op":"bond.add","a":7,"b":-1,"o":1}]}}`: `-1` names the first new atom of the patch, `-2` the second, and so on (write the rule in the request's docs); the log gets the plain ops, so replaying the log alone gives the same bytes.
- Batch: `"commands":[...]` of several `tp:derive` give a whole pathway in one request; a TP can be the `of` of the next (TP1 → TP2).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state"), `docs/plan/backlog.md` (sections 0, 4 and 5), `docs/study/tp-draw.md`.
- `editor_edit.rs` (copy and paste: reuse the id rewriting), `ops.rs`, `doc.rs`, plan-11 (`map`), tp-01 (`from`, `to`, `route`), `mzlab-mol` `transform.rs` (the `trasf/1` map that plan-12 builds from these maps).

## Agreement needed before the PR
None for the ops (all exist after plan-11 and tp-01). The negative ids of a patch are a convention of the request, not of `draw_edit/1`; if the coordinator prefers explicit ids, use those.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-02-derive` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Copy with id and map rewriting; placement (column, then below).
2. Patch rewriting (parent ids → copy ids, negative ids → new ids, coordinates translated); validation through the record of `mzlab-mol`.
3. Command, context-menu item, key; goldens for a synthetic parent with three TPs (hydroxylated, demethylated, ring-opened lactam).

## Tests to add
- Property: for every atom of the parent that the patch does not touch, the copy atom's position minus the parent's equals the translation (within 1e-4), and its map equals the parent atom's map.
- Property: map numbers are unique per structure and a number names the same parent atom in every TP of the pathway (TP2 derived from TP1 included).
- Property: the canonical SMILES of the copy with an empty patch equals the parent's.
- Undo of `tp:derive` returns the identical document (maps given to the parent included); apply then undo for the plain ops is already covered.
- Refusals: a pentavalent carbon patch, a parent of 1,000 atoms.
- e2e: right-click (mouse) and long press (touch) "Derive TP", then `Shift+D` from the keyboard; edit the copy; the SVG equals the golden.
- **Fails without the change**: the translation property has no command to run on `main`; the test file is added first and fails to build (say so in the PR).

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/derive.py` reads our RXN V2000 of each parent → TP pair (with maps, plan-11) in RDKit and checks that RDKit's mapped atoms are the ones we mapped and that the product's formula minus the reactant's equals our patch's delta (`mzlab-mol` `delta.rs`). Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

