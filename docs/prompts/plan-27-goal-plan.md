# plan-27: one goal in, one canonical reversible patch out (`goal`)

Proposed by `docs/plan/backlog.md` §10 (row 76; third external report, idea `plan`). Size M. Wave 4. Depends on: plan-01 (op table), plan-20 (`draw-result/1`; edit E21 of §10.5). Each goal kind is gated on the prompt that builds its engine (below).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-27-goal-plan: received, starting on branch `claude/plan-27-goal-plan`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only. An agent today sends many ops to draw one thing; here it sends one goal and gets back the ops it would have written, as a patch it can read, apply or refuse.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- A new module `crates/mzdraw/src/plan.rs` and a request field `"goal":{...}` (a JSON object, one key = the goal kind) plus optional `"budget":N` (most ops allowed in the patch; default 500).
- Order inside a request: `import`, `style`, `set`, then the goal's patch, then `log` (so a log may use the ids the patch created; they are in `created`). A request with `goal` always answers `draw-result/1` (as `dry_run` and `expect` do, E21), with the key `patch` (the ops, in `draw_edit/1`) before `out`. With `dry_run` nothing is applied; otherwise the patch is applied and `out` is written as usual.
- Goal kinds of v1, each only when its engine exists (a kind whose engine is not merged is refused as unsupported):
  - `{"draw":"SMILES","at":[x,y]}`: the structure laid out by `mzlab-mol` (the import path), as `atom.add` / `bond.add` ops with new ids; always available;
  - `{"row":["SMILES",...],"arrow":true}`: a one-line scheme; after plan-13;
  - `{"attach":{"at":ID,"template":NAME}}`: a template or abbreviation on an atom; after arch-web-19 (`tmpl`) or arch-web-18;
  - `{"swap":{"select":{...},"el":"F"}}`: element change on the atoms a plan-24 selector resolves to; after plan-24. The selector is a parameter of this goal only: the patch holds plain ids, and selectors inside ops stay deferred (§9.3).
- Canonical: the same goal on the same document gives byte-identical patches (ids from the document's counter, ops in a fixed order).
- A grant (plan-28) checks the patch's ops like any op of the log.
- Errors, by plan-20's rule: an unknown or not yet supported goal kind is an invalid request (exit 2, `error/1`, key `error.draw2.planUnsupported`, the supported kinds listed in `params`); a supported goal that needs more ops than `budget` is a verdict (exit 1, `failed.stage` `plan`, key `error.draw2.planBudget`).

Out (do not do here):
- Natural-language goals or any model (DROP, backlog §1.2 do-not 4).
- More than one goal per request (batch through mzLab's stdin streaming).

## CLI twin (rule 15)
- `{"goal":{"draw":"c1ccccc1O"},"dry_run":true}` gives the patch; `{"goal":...}` applies it. Native replay test: replaying `patch` as a `log` gives the same bytes as the goal request. The answer carries the request `id` (or the batch index `n`) as plan-20's second key (E26).
- Add feature `plan` to plan-19's runner (`--arm` list; D21).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md`, `docs/plan/backlog.md` §9 and §10.
- `replay.rs`, `outcome.rs` (plan-20), `io.rs` / `layout.rs` (the import and layout path), `schema.rs` (plan-01).

## Agreement needed before the PR
No `draw_edit/1` op (the patch uses table ops only). Rows in the `request` section of plan-01's table: `goal`, `budget`; the `draw-result/1` key `patch` and stage `plan` are plan-20's key table, edited by E21 (D13, D20).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-27-goal-plan` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. `draw` goal, canonical patch, request order.
2. Budget, errors, the gated kinds that exist at your start.
3. Runner feature, doc section (the goal kinds and their gate).

## Tests to add
- Canonical: two runs of every goal give the same bytes; the patch replayed as a `log` gives the same document.
- Every patch is inverted exactly by undo (the property test of `ops.rs` over goal patches).
- A log after a goal can address the atoms the goal created.
- An unsupported kind is exit 2 with the supported list; a budget of 3 for benzene is exit 1 with stage `plan`.
- Parity cases; native replay test.
- **Fails without the change**: `goal` is an unknown field before the slice.

## Independent proof
Correctness must not rest on our own claims. The canonical SMILES (RDKit, in the tool) of the document after each `draw` and `row` goal equals that of the goal's SMILES for the plan-03 set. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
Run plan-19 with the feature `plan` against the same arm without it. Keep only if median calls per passing task fall by at least 50%, pass@1 stays within 5 points, and every replay of a passing run is byte-identical; otherwise drop the field (say so in `docs/agent-suite/`).

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
