# tp-03: derive a transformation product from its parent (scaffold kept)

Proposed by `docs/study/tp-draw.md` (verdict V1, idea 1 "ghost-parent scaffold lock", part 1). Size M. Depends on: tp-01, tp-02, plan-11 (atom maps), plan-20. Wave: after plan-11 merges.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-03-derive: received, starting on branch `claude/tp-03-derive`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. A mass spectrometrist draws one parent and then 10 to 50 TPs that differ from it by one or two edits. The slow part is redrawing each TP, or copying it and re-aligning it by hand. Derive does this in one step: copy the parent with its exact coordinates, apply the edit, anchor an arrow, keep the atom correspondence. Nothing is predicted: the person (or the agent) names the edit.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- A new module `crates/mzdraw/src/tp.rs` with `fn derive(doc, of: atom id, patch: &[Op], at: Option<(f64, f64)>) -> Result<Vec<Op>, &'static str>`. It returns, as one list:
  1. **The copy** of the structure that holds `of`: atoms, bonds, labels and wedges. Data and annotation are **not** copied, because a TP is a new identification. The copy gets new ids from the document counter. It is translated so its box sits at `at`; when `at` is absent, one column to the right of the parent (`pathway.col_gap`, default 2.5 bond lengths). If that place is taken, it goes below the last TP of the same parent (`pathway.row_gap`, default 1.0).
  2. **Atom maps** (plan-11 `map`). If the parent atoms have none, they first get `1..n` in id order (`atom.set map`, in the same list). Each copy atom gets its source's map.
  3. **The `patch`**, written on the **parent's** ids and with coordinates relative to the parent:
     - parent ids are rewritten to the copy's ids;
     - coordinates are translated with the copy;
     - new atoms get new ids and the next free map numbers of the pathway (the largest map in the document + 1), so a map number names one atom across the whole pathway.
  4. **An arrow**: `obj.add` with `from` = the smallest atom id of the parent and `to` = the smallest of the copy, routed (tp-01).
- **Refusals.** A patch that leaves the copy invalid for `mzlab-mol` (a valence over the allowed set) is refused with `error.draw2.tpPatch` and changes nothing; an atom left alone after a cleavage is fine. A parent that needs maps but has more than 999 atoms is refused with `error.draw2.tpMaps`.
- **Command and page.** Command `tp:derive` is one undo step. The page has "Derive TP" in the context menu of a structure (`context.rs` item id `tp.derive`) and key `Shift+D` (arch-std-06's command table). The copy is made and selected, so the next edit (keyboard or mouse) applies to it, and the person then edits as usual.
- **Scaffold kept.** After a ring cleavage, every other atom stays where the parent had it; no clean-up runs.

Out (do not do here):
- The diff lens (tp-04), layout (tp-05), labels (tp-07), named steps (tp-10).
- A "rigidity slider" (dropped in the study, §3 V1); clean-up of a changed region is arch-std-15's.

## CLI twin (rule 15)
- `{"commands":[{"command":"tp:derive","args":{"of":1,"patch":[{"op":"atom.add","el":"O","x":1.5,"y":-0.866},{"op":"bond.add","a":7,"b":-1,"o":1}]}}]}`.
  - In a patch, `-1` names the first new atom of the patch, `-2` the second, and so on; write the rule in the request's docs.
  - The log gets the plain ops, so replaying the log alone gives the same bytes.
- Batch: several `tp:derive` in one `commands` list give a whole pathway in one request. A TP can be the `of` of the next one (TP1 → TP2).
- With `"report":true` the answer is `draw-result/1`: `created` lists the TP's atoms and the arrow. A refusal is exit 1 with `failed.stage` `command` and key `error.draw2.tpPatch` / `tpMaps`. `"dry_run":true` validates without applying.
- Request rows (plan-01 `request` section, D13): the command `tp:derive` with its args `of`, `patch`, `at`, and the negative-id rule of `patch`.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state"), `docs/plan/backlog.md` (sections 0, 4, 5, 9), `docs/study/tp-draw.md`.
- `editor_edit.rs` (copy and paste: reuse the id rewriting), `ops.rs`, `doc.rs`, plan-11 (`map`), tp-01 (`from`, `to`, `route`, `commands`), `mzlab-mol` `transform.rs` (the `trasf/1` map that plan-12 builds from these maps).

## Agreement needed before the PR
No `draw_edit/1` op or field: all exist after plan-11 and tp-01. Request rows only (D13). The negative ids of a patch are a convention of the request, not of `draw_edit/1`; if the coordinator prefers explicit ids, use those.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-03-derive` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Copy with id and map rewriting; placement (column, then below).
2. Patch rewriting (parent ids → copy ids, negative ids → new ids, coordinates translated); validation through the record of `mzlab-mol`.
3. Command, context-menu item, key.
4. Goldens: a synthetic parent with three TPs (hydroxylated, demethylated, ring-opened lactam).

## Tests to add
- Property: for every parent atom that the patch does not touch, the copy atom's position minus the parent's equals the translation (ε = 1e-9 before rounding; 3e-4 on the saved document), and its map equals the parent atom's map.
- Property: map numbers are unique per structure, and a number names the same parent atom in every TP of the pathway, including TP2 derived from TP1.
- Property: with an empty patch, the canonical SMILES of the copy equals the parent's.
- Undo of `tp:derive` returns the identical document, including the maps given to the parent.
- Refusals: a pentavalent carbon patch, a parent of 1,000 atoms; each is exit 1 with its key and nothing written.
- e2e: "Derive TP" by right-click (mouse), by long press (touch) and with `Shift+D` (keyboard); edit the copy; the SVG equals the golden.
- Screenshots at deviceScaleFactor 2 in the PR: the parent, and the derived TP before and after its edit.
- **Fails without the change**: `tp:derive` is an unknown command on `main` (exit 2).

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/derive.py` reads, in RDKit, our RXN V2000 (with maps, plan-11) of each parent → TP pair. It checks that RDKit's mapped atoms are the ones we mapped, and that the product formula minus the reactant formula equals our patch's delta (`mzlab-mol` `delta.rs`). Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

## Kill test
Add the velocity task "parent and 5 TPs (2 hydroxylations, 1 demethylation, 1 ring opening, 1 TP of a TP)" to plan-02. Keep `tp:derive` as the default way only if it needs at most half the actions of copy-paste-and-edit on the same task. Otherwise keep the command for the CLI route and drop the context-menu item and key in a follow-up.

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

