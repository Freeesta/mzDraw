# tp-07: named transformation steps at a chosen site (+O, demethylation, ...)

Proposed by `docs/study/tp-draw.md` (verdict V5, idea 5 "arrow-pull mass-shift autocomplete", reduced). Size M. Depends on: tp-02 (derive). Wave: after tp-05 (second round of the TP work).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-07 named-steps: received, starting on branch `claude/tp-07-named-steps`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only.

## Why
Most TPs of a suspect screening differ from the parent by a few standard steps. Typing the step and pointing at the site is faster than drawing it, and the agent route gets the same steps by name. Nothing is predicted: the person or the agent chooses the step **and** the site; the editor only refuses a step that does not fit the site.

## Scope
In:
- A fixed table in `tp.rs` (`STEPS`: name, delta, site test, edit), each row citing in a comment the public list it was checked against (study §1): `hydroxylation` (+O; site: a C or aromatic C with at least one H; edit: add O–H), `n-demethylation` and `o-demethylation` (−CH2; site: a CH3 bonded to N or O; edit: remove it), `n-oxidation` (+O; site: a tertiary amine or pyridine-type N; edit: add O with N+ / O− charges), `dehydrogenation` (−H2; site: a C–C single bond with one H on each end; edit: order 2), `hydration` (+H2O; site: a C=C; edit: order 1, add O–H on the end with fewer H, ties by id), `dechlorination` / `debromination` (−Cl+H / −Br+H; site: the halogen; edit: remove it), `hydrolysis` (+H2O; site: an ester or amide C–O / C–N single bond next to C=O; edit: cleave, add O–H on the carbonyl C), `acetylation` (+C2H2O; site: an N–H or O–H; edit: label `Ac` on a new atom), `sulfation` (+SO3; site: an O–H; edit: label `SO3H` replacing H), `glucuronidation` (+C6H8O6; site: O–H, N–H, COOH; edit: a new atom labelled with a glucuronide abbreviation, added to `label.rs`'s table in this slice with its expansion), `decarboxylation` (−CO2; site: a CO2H carbon; edit: remove C and its two O).
- `fn sites(doc, structure, step) -> Vec<atom id>`: every atom where the site test holds, ascending id (a list to choose from, never a ranking).
- `tp:derive` (tp-02) takes `"step":"hydroxylation","at":7` in place of a `patch`: the step's edit becomes the patch; a site where the test fails is refused with `error.draw2.tpSite` and changes nothing. The delta of the result is checked against the table (`delta.rs`), so a wrong edit cannot ship.
- Page: on a structure, key `.` opens a step field (one-line input with the table as a list, filtered as you type, also matching the delta: `+O`, `+15.99`); the sites of the step are ringed (overlay); `Tab` is **not** used (the canvas never traps it, see Built state): `[` and `]` move to the previous / next site, `Enter` derives, `Escape` leaves. Touch: tap a ringed site.
- The preview (the ringed sites and the ghost of the added atoms on the focused site) is an overlay: no op until `Enter`.

Out (do not do here):
- Any ranking or likelihood of sites; any reaction-rule engine (enviPath, BioTransformer rules) or SMARTS (rule 13, query language).
- Typed deltas that match no row (shown as "no step", nothing derived); a free mass to formula search.

## CLI twin (rule 15)
- `{"command":"tp:derive","args":{"of":1,"step":"o-demethylation","at":14}}`; `{"out":"sites","args":{"of":1,"step":"hydroxylation"}}` returns `{"format":"sites/1","step":"hydroxylation","delta":"+O","atoms":[...]}`; `{"out":"steps"}` returns the table (name, delta, mass shift when plan-04 is there).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state"), `docs/plan/backlog.md` (sections 0, 4 and 5), `docs/study/tp-draw.md` (§1 for the sources of the table).
- tp-02 `tp.rs`, `label.rs` (abbreviation table), `mzlab-mol` `delta.rs`, `valence.rs`, `editor_keys.rs`.

## Agreement needed before the PR
None: steps expand to existing ops; `sites` and `steps` are `out` values. The new abbreviation is a row of `label.rs` (label zone: tell the coordinator).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-07-named-steps` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. `STEPS` with site tests on the record of `mzlab-mol` (hydrogen counts from the record, not from the drawing).
2. `sites`, `step` in `tp:derive`, the delta check.
3. Page field, overlay, keys and touch (paint and forward only: the list, the filter and the sites come from Rust).

## Tests to add
- For every row: on a synthetic parent that has the site, the derived TP's formula equals parent + delta; on one that lacks it, `sites` is empty and the derive is refused.
- `sites` is in ascending id order and equal to a brute-force check of the site test over all atoms.
- Undo of a step derive returns the identical document.
- e2e: `.`, type `+O`, `]` twice, `Enter` (keyboard); the same by touch; the SVG equals the golden.
- **Fails without the change**: `"step"` is an unknown argument on `main` (keyed error).

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/steps.py` applies, with RDKit, a reaction SMARTS written in the script for each row (in the tool only, never in the editor) to the same synthetic parents at the same atoms and compares the canonical SMILES of the product with ours. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

