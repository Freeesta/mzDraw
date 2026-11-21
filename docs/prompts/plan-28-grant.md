# plan-28: a scope grant checked on every op, and per-op contracts

Proposed by `docs/plan/backlog.md` §10 (row 77; third external report idea `grant`, merged with the fourth report's per-op contracts). Size M. Wave 4. Depends on: plan-20; plan-24 for a scope given by selector (optional).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-28-grant: received, starting on branch `claude/plan-28-grant`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only. A person lets an agent change one part of a drawing; nothing outside that part may change, and the editor checks it, rather than trusting the agent. **Not built in this slice**: the page part (a proposal of plan-16 held under a grant). plan-16 is wave 6; that part becomes a follow-up row then.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- A new module `crates/mzdraw/src/grant.rs`; request field `"grant":{...}`:
  - `atoms` (ids) or `select` (a plan-24 selector, after it merges): the scope (below);
  - `ops`: the op names or op classes allowed. This PR adds a `class` column to plan-01's `OpSpec` table (D22): classes over the ops `ops.rs` has today: `geometry` = `move`, `rotate`, `flip`; `atoms` = `atom.add`, `atom.set`, `atom.ann`, `atom.data`; `bonds` = `bond.add`, `bond.set`; `objects` = `obj.add`, `obj.set`; `structure` = `remove`, `ring`. Each later op names its class in its own row when it is added (E25): for example `atoms.xy` (arch-std-15) is `geometry`, `merge` (arch-web-09) is `structure`;
  - `max`: the most ops;
  - `each`: optional plan-20 `expect` arms checked after EVERY op (the fourth report's per-op contracts; no new op);
  - `settings`: `false` by default. Under a grant, the request fields that change anything outside the log (`style`, `set`) are refused (exit 2) unless `settings` is `true`. `import` is the document the grant is about.
- `Touch` and `fn touches(&Op, &Document) -> Touch` in `ops.rs`, one arm per op (append-only; every later op-adding prompt adds its arm, D23). `Touch` lists the atoms, bonds and objects an op writes:
  - the atom or bond it sets;
  - both end atoms of a bond it adds;
  - every atom it moves, rotates or flips;
  - on `remove`, the removed atoms, the bonds removed with them (cascade) and the objects;
  - objects (arrows, plus signs, texts) by id;
  - `document = true` for ops that change the drawing as a whole.
- Scope check: the scope is the given atoms, the bonds between two of them, and everything the log creates. An op passes when its `Touch` is inside the scope. An op with `document = true` passes only if its op is named explicitly in `ops`.
- A violation stops the request: nothing is written, exit 1 verdict with `failed.stage` `grant`, `failed.op`, key `error.draw2.grantScope`, `grantOp`, `grantMax` or `grantEach`. A goal patch (plan-27) is checked like the log.

Out (do not do here):
- The page part (above).
- Who may grant (the host's business; a `mzlab-cli` flag that takes the grant from the person rather than from the request is COO 16's to decide, listed in the PR).
- Grants stored in the document.

## CLI twin (rule 15)
- `{"import":DOC,"grant":{"atoms":[3,4,5],"ops":["atoms","bonds"],"max":5},"log":[...]}`; exit 0, or 1 with `draw-result/1`. The answer carries the request `id` (or the batch index `n`) as plan-20's second key (E26). Native replay test.
- Add feature `grant` to plan-19's runner (D21) with the adversarial set below.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md`, `docs/plan/backlog.md` §9 and §10.
- `ops.rs` (every op), `schema.rs` (plan-01), `outcome.rs` (plan-20), `select.rs` (plan-24).

## Agreement needed before the PR
No `draw_edit/1` op. Rows in the `request` section of plan-01's table: `grant` and its keys; the `class` column of the op table; the stage `grant` of `draw-result/1` (E21, D13).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-28-grant` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. `Touch` and `touches` for every op; the class column.
2. Grant check, `each`, `settings`, errors.
3. Adversarial set, runner feature, doc section.

## Tests to add
- Property: for random documents, grants and logs, when the replay is `ok` the whole `mzdraw/1` text with the scope's atoms, bonds and objects (and the created ones) removed is byte-identical before and after.
- Property: `touches` of each op equals the ids that differ between the documents before and after that op (so the table cannot lie).
- Each violation kind gives its key and the index of the first offending op; nothing is written.
- `style` or `set` under a grant without `settings` is exit 2.
- `each` with `{"std_absent":[...]}` stops at the op that creates the finding.
- Parity cases; native replay test.
- **Fails without the change**: `grant` is an unknown field before the slice.

## Independent proof
Correctness must not rest on our own claims. The property tests are the proof (exact bytes outside the scope; `touches` checked against real differences). The adversarial set is `tests/data/agent-suite/adversarial.json`: 100 instructions, synthetic, written by hand by this slice's DEV ("also fix the other ring", "rename everything", "move the arrow too"). The owner runs it against external models (D14). Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
Over the 100 adversarial instructions, any out-of-scope mutation that the grant lets through (an `ok` run with a change outside the scope) kills the design until it is fixed. If `each` costs more than 2× the replay time of the same log on the plan-19 tasks, drop `each` and keep the end-of-log `expect`.

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
