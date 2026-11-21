# plan-20: the result of a replay request: created and removed ids, the failing op, dry run and postconditions

Proposed by `docs/plan/backlog.md` §9 (row 69; second Gemini input, idea 5 "transactional canvas", part 1). Size M. Wave 1 tail, after plan-01. Depends on: plan-01 (the request rows of the table).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-20-result-envelope: received, starting on branch `claude/plan-20-result-envelope`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only (`replay.rs` and a new module), no page. The rule "every operation reports the ids it created and removed, a batch of operations is atomic" is stated in every arch-std prompt but is not built: `replay` returns only the output text or an error key.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- A new module `crates/mzdraw/src/outcome.rs` and three request fields in `replay.rs`:
  - `"report":true`, `"dry_run":true` or a non-empty `"expect"` each make the answer one `draw-result/1` JSON document instead of the bare output: `{"format":"draw-result/1","ok","out","created":{"atoms","bonds","objects"},"removed":{...},"failed":{"op","key","id"},"expect":[{"check","ok","detail"}]}` (keys in this order, empty lists left out, written by hand). `created` / `removed` are the id sets after minus before, per kind, ascending. `failed.op` is the 0-based index of the op that failed.
  - `"dry_run":true`: everything runs and is reported; `out` is left out, nothing else is printed.
  - With any of the three, stdout is exactly one `draw-result/1` document (no bare output before or after it); without them, stdout is the bare output as today.
  - `"expect":[{...}]`: postconditions checked on the final document, each a JSON object with one key: `{"std":"none"}` (no `std/1` finding), `{"std_absent":[keys]}`, `{"pieces":N}`, `{"smiles":"..."}` (canonical SMILES of the whole drawing equals). Later prompts add one arm each: `{"stereo":"kept"}` (plan-21), `{"lossless":"FORMAT"}` (plan-22). An unknown check is refused.
- `draw-result/1` is defined completely here; `mzlab-cli` follows it (COO 16, D13). Keys, in this order, every one stable:
  | key | type | when | meaning |
  |---|---|---|---|
  | `format` | string | always | `"draw-result/1"` |
  | `ok` | bool | always | every op ran and every expectation holds |
  | `dry_run` | bool | only when true | the request asked for a dry run |
  | `out` | string | `ok` and not a dry run | what `Editor::save(out)` wrote (the request's `out`, default `mzdraw`) |
  | `created` | object | always | `{"atoms":[ids],"bonds":[ids],"objects":[ids]}`, ascending; empty lists left out, `{}` if none |
  | `removed` | object | always | same shape |
  | `failed` | object | the log or a selector failed | `{"stage","op","key","id","candidates"}`: `stage` is `log` or `select` (plan-24); `op` the 0-based index in `log` (stage `log` only); `key` the i18n key; `id` the id it is about (0 when none); `candidates` the matched atom ids per match (stage `select` only) |
  | `expect` | array | the request had `expect` | one `{"check":the request object,"ok":bool,"detail":string}` per expectation, in request order; `detail` empty when `ok` |
  Exit codes (mzLab's table, D13), one rule for every replay output:
  - exit 0: `ok` is true;
  - exit 1 (verdict, `draw-result/1` on stdout): an op of the table could not be applied (unknown id, invalid value for that document), a selector matched none or not `count` (plan-24), or an expectation is false;
  - exit 2 (one `error/1` on stdout, from `mzlab-cli`): the request is invalid: not JSON, an unknown field, an op outside the table, an unknown `expect` arm, a malformed selector;
  - an unreadable `import` keeps the code `mol draw` gives an unreadable input today.
  `created` and `removed` are reported even when `ok` is false (they describe the attempted run; nothing is written). A JSON Schema of it goes in `docs/schemas/draw-result-1.json`, and a test validates every golden envelope against the key table (order included).
- Atomic: if an op fails or one expectation is false, the answer has `ok:false` and no `out`; nothing is written (today's replay already writes nothing on an op error; keep that and test it).
- `Editor::save` is unchanged; the envelope is only the replay door, so the page does not change.

Out (do not do here):
- A persistent server or session (the stdio server of the input is deferred: it would be mzLab's CLI).
- Fix-it patches (plan-25), the digest (plan-23), selectors (plan-24).

## CLI twin (rule 15)
- The fields above in the replay request (the route of `mzlab-cli mol draw`). Exit codes as in the rule above (0 ok; 1 verdict with `draw-result/1`; 2 invalid request with `error/1`). `mzlab-cli mol draw` passes the document through unchanged (D13).
- Add arm `report` to plan-19's runner (`--arm report`), telling the model about the three fields.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md`, `docs/plan/backlog.md` §9 and E11.
- `replay.rs`, `ops.rs` (`Session::run`), `conform.rs` (`std/1`), plan-01's `schema.rs`.

## Agreement needed before the PR
No `draw_edit/1` op. One row per request field (`report`, `dry_run`, `expect` and each `expect` arm) in the `request` section of plan-01's table; COO 16 reviews the table (D13). If plan-01 has not merged, add the section in this PR.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-20-result-envelope` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Id sets before and after; failing index.
2. `expect` arms.
3. Envelope writer, `docs/schemas/draw-result-1.json` and the doc section.

## Tests to add
- A log that adds 3 atoms and removes 1 reports exactly those ids; a log failing at op 4 reports `failed.op` = 4 and no `out`.
- `dry_run` gives the same envelope as the real run minus `out`, and `dry_run` or `expect` alone (no `report`) already gives the envelope.
- Each exit rule: an op on an unknown id is exit 1 with `failed.stage` `log`; an op outside the table is exit 2.
- Each `expect` arm true and false; an unknown arm is refused.
- Parity: two cases in `tests/data/mzdraw_log.json` with `report:true`; native replay test in `crates/mzdraw/tests/replay.rs`.
- **Fails without the change**: before the slice the request with `report` returns the bare document, so the envelope test fails.

## Independent proof
Correctness must not rest on our own claims. Property test over the random generator of `crates/mzdraw/tests/ops.rs`: `created` equals the ids present after and absent before, computed independently from the two `mzdraw/1` documents; for `{"smiles":...}` the canonical SMILES is compared with RDKit's for the plan-03 set in the tool. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
After the slice, run plan-19 with `--arm report` against `--arm plain` on the same model(s). If pass@1 gains less than 10 points AND median calls do not fall, keep only `created`/`removed` and `failed` and drop `expect` in a follow-up (say so in `docs/agent-suite/`).

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
