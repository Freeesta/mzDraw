# tp-10: named transformation steps at a chosen site (table, sites, derive by step)

Proposed by `docs/plan/backlog.md` §11 (row 90) and `docs/study/tp-draw.md` (verdict V5, idea 5 "arrow-pull mass-shift autocomplete", reduced, part 1). Size M. Depends on: tp-03 (derive). Wave: after tp-09 (second round of the TP work).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-10-named-steps: received, starting on branch `claude/tp-10-named-steps`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only; the page field is tp-11. Most TPs of a suspect screening differ from their parent by a few standard steps. Typing the step and pointing at the site is faster than drawing the change, and the agent route gets the same steps by name. Nothing is predicted: the person or the agent chooses the step **and** the site, and the editor only refuses a step that does not fit the site.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- **The step table.** A fixed table `STEPS` in `tp.rs` (name, delta, site kind, site test, edit). Each row cites in a comment the public list it was checked against (study §1 T11; a row whose delta no source lists says "computed").

  The **site kind** is `atom` or `bond`: dehydrogenation, hydration and hydrolysis act on a bond, the others on an atom.

  | step | delta | site | edit |
  |---|---|---|---|
  | `hydroxylation` | +O | a C or aromatic C with at least one H | add O–H |
  | `n-demethylation` | −CH2 | a CH3 bonded to N | remove the CH3 |
  | `o-demethylation` | −CH2 | a CH3 bonded to O | remove the CH3 |
  | `n-oxidation` | +O | a tertiary amine or pyridine-type N | add O, with N+ / O− charges |
  | `dehydrogenation` | −H2 | a C–C single bond with one H on each end | set order 2 |
  | `hydration` | +H2O | a C=C | set order 1; add O–H on the end with fewer H, ties by id |
  | `dechlorination` | −Cl+H | the Cl | remove it |
  | `debromination` | −Br+H | the Br | remove it |
  | `hydrolysis` | +H2O | an ester or amide C–O / C–N single bond next to C=O | cleave it; add O–H on the carbonyl C |
  | `acetylation` | +C2H2O | an N–H or O–H | a new atom labelled `Ac` |
  | `sulfation` | +SO3 | an O–H | label `SO3H` replacing the H |
  | `glucuronidation` | +C6H8O6 | an O–H, N–H or COOH | a new atom labelled with a glucuronide abbreviation, added to `label.rs`'s table in this slice with its expansion |
  | `decarboxylation` | −CO2 | the carbon of a CO2H | remove the C and its two O |

- **Sites.** `fn sites(doc, structure, step) -> Sites` returns the step's site kind and every atom id (kind `atom`) or bond id (kind `bond`) where the site test holds, by ascending id. It is a list to choose from, never a ranking. Hydrogen counts come from the record of `mzlab-mol`, not from the drawing.
- **Derive by step.** `tp:derive` (tp-03) takes `"step":"hydroxylation","site":7` in place of a `patch`; `site` is an atom id or a bond id, as the step's site kind says (`at` stays tp-03's placement point), and the step's edit becomes the patch. A site where the test fails is refused with `error.draw2.tpSite` (exit 1, `failed.stage` `command`) and changes nothing. The delta of the result is checked against the table (`delta.rs`), so a wrong edit cannot ship.

Out (do not do here):
- The page field and its overlay (tp-11).
- Any ranking or likelihood of sites; any reaction-rule engine (enviPath, BioTransformer rules) or SMARTS (rule 13: query language).
- A typed delta that matches no row, and a free search from mass to formula.

## CLI twin (rule 15)
- `{"commands":[{"command":"tp:derive","args":{"of":1,"step":"o-demethylation","site":14}}]}`; `{"commands":[{"command":"tp:derive","args":{"of":1,"step":"hydrolysis","site":23}}]}` (23 a bond id)`.
- `{"out":"sites","args":{"of":1,"step":"hydroxylation"}}` returns `{"format":"sites/1","step":"hydroxylation","delta":"+O","kind":"atom","ids":[...]}`.
- `{"out":"steps"}` returns the table: name, delta, and the mass shift when arch-web-17 is there.
- Request rows (plan-01 `request` section, D13): the `tp:derive` args `step` and `site`; the outputs `sites` (and its arg) and `steps`; the formats `sites/1` and `steps/1`.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state"), `docs/plan/backlog.md` (sections 0, 4, 5, 9), `docs/study/tp-draw.md` (§1 T11 for the sources of the table).
- tp-03 `tp.rs`, `label.rs` (abbreviation table), `mzlab-mol` `delta.rs`, `valence.rs`.

## Agreement needed before the PR
No `draw_edit/1` op or field: steps expand to existing ops. Request rows only (D13). The new abbreviation is a row of `label.rs` (label zone): tell the coordinator.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-10-named-steps` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. `STEPS` with site tests on the record.
2. `sites`; `step` in `tp:derive`; the delta check.
3. `out` values `sites` and `steps`; docs of the table.

## Tests to add
- For every row: on a synthetic parent that has the site, the derived TP's formula equals the parent plus the delta; on one that lacks it, `sites` is empty and the derive is refused (exit 1).
- `sites` is in ascending id order and equals a brute-force check of the site test over all atoms (atom steps) or all bonds (bond steps).
- Undo of a step derive returns the identical document. Parity cases for three steps.
- **Fails without the change**: `"step"` is an unknown argument on `main` (exit 2).

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/steps.py` applies, with RDKit, one reaction SMARTS per row to the same synthetic parents at the same atoms. The SMARTS is written in the script, in the tool only, never in the editor. The canonical SMILES of each product equals ours. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

## Kill test
On the 12-row list of tp-08, count how many TPs one step at one site can make. If fewer than half, the table is too narrow to be worth a page field: ship the CLI route only, and tell the coordinator that tp-11 should not run.

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

