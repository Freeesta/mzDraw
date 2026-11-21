# plan-19: a fixed agent task suite, the external test of the agent route

Proposed by `docs/plan/backlog.md` §9 (row 68; second Gemini input `docs/input/gemini-strategy-2.md`, validation part "blind multi-model agent task suite"). Size M. Wave 1 tail, after plan-01. Depends on: plan-01 (the `"out":"schema"` text given to the models; until it merges, give the op list of `ops.rs` by hand).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-19-agent-suite: received, starting on branch `claude/plan-19-agent-suite`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust checker and a small Python runner under `tools/`; no product feature. It is the kill test of plan-20, plan-23 and plan-24: those prompts keep or drop their idea on its numbers.

## Scope
In:
- `tests/data/agent-suite/tasks.json` (`agent-suite-tasks/1`): 30 fixed tasks, synthetic, written by hand: 10 "build from words" (e.g. "draw 4-chlorophenol with the OH at the top"), 10 "surgical edits" on a given start document (e.g. "put a fluorine para to the chlorine on the product, touch nothing else"), 10 "several structures" (a reaction with an arrow and a condition text, three structures in a row, two salts). Each task: `id`, `kind`, `words` (the prompt the model sees), optional `import` (the start document), and `check`.
- `check` is read by Rust, never by Python: the canonical SMILES of `save("smiles")` per piece (from `mzlab-mol`), optional ids that must be untouched (surgical edits: the atoms and bonds outside the edit keep id, element and position), optional geometry (pieces ordered left to right, an arrow between them, no `std/1` finding of the listed keys).
- `crates/mzdraw/examples/suite_check.rs`: reads a task and a replay request, runs `mzdraw::replay` and the check, prints `{"task","pass","reason"}`; exit 0 pass, 1 fail.
- One hand-written reference solution (a replay request) per task in `tests/data/agent-suite/ref/`; `crates/mzdraw/tests/agent_suite.rs` checks that all 30 pass and that a broken copy of each (one atom changed) fails.
- `tools/agent-suite/run.py --model-cmd 'CMD' --arm plain --runs 1`: for each task gives CMD on stdin the words, the replay request format (fields of `replay.rs`) and the op table, reads back one or more replay requests (one per line; each line is one CLI call), checks each final request with `suite_check`, and writes `agent-suite/1` JSON: per task pass, number of calls, request bytes, and tokens when CMD reports them; overall pass@1 with a Wilson 95% interval, median calls, median bytes. `--arm` names the request features the model is told about. Syntax, defined once here: `plain`, or a `+`-joined set of feature names from a fixed list (`report`, `digest`, `select`; plan-20, plan-23 and plan-24 each add theirs); the set is order-free and written sorted in `agent-suite/1` (`digest+report`); an unknown name is a usage error (exit 2). The text the model reads for each feature lives in `tools/agent-suite/arms/<name>.md`; `plain` gives only the request format and the op table. Every replay request the model sends is one call, whatever it asks for.
- `docs/mzdraw.md`: a short section "Agent task suite" (how to run it, what is committed).

Out (do not do here):
- Running external models in CI or committing any key. The owner runs the suite against external models outside CI (coordinator, 2026-11-21); only the rates of `agent-suite/1`, the model name and the date are committed, under `docs/agent-suite/`.
- Any baseline on RDKit or Ketcher's API (later, if the owner asks).

## CLI twin (rule 15)
- `cargo run --example suite_check -- TASK.json REQUEST.json`: stdout one JSON line, exit 0 / 1.
- `python3 tools/agent-suite/run.py --model-cmd 'CMD' --arm plain > rates.json` (exit 0; 1 on a tool error; diagnostics on stderr).
- The Rust test runs with no Python and no model.

## Context (read first)
- `CLAUDE.md`, `docs/built/` (Built state), `docs/plan/backlog.md` §9.
- `crates/mzdraw/src/replay.rs` (the request), `crates/mzdraw/examples/replay.rs`, `tools/fidelity/` of plan-03 if merged (same layout for a Python tool).

## Agreement needed before the PR
None for `draw_edit/1`: this slice adds no operation or field and changes nothing in `mzlab-mol` / `mzlab-feat`. If you find you need one, stop and ask the coordinator (it asks COO 16).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-19-agent-suite` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. The 30 tasks and their checks.
2. The Rust checker, the example and the reference solutions.
3. The runner and the `agent-suite/1` format (keys in a fixed order).
4. Doc section.

## Tests to add
- All 30 reference solutions pass; each broken copy fails, with a reason.
- A surgical-edit check fails when an untouched atom changes id or moves.
- Determinism: `suite_check` output is byte-identical across two runs.
- **Fails without the change**: the test cannot compile before `suite_check` exists; once it does, a checker that compares only formulas fails the broken copies.

## Independent proof
Correctness must not rest on our own claims. The checker rests on the canonical SMILES of `mzlab-mol`; for the 30 targets, also write the InChIKey of each target as RDKit gives it (fetched at run time by `tools/agent-suite/ref.py`, never committed: the keys are committed with the RDKit version and date, the program is not) and check our SMILES of the reference solution gives the same key in the tool. If RDKit cannot be installed in the container, say so in the PR and leave the script runnable. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
This prompt is the kill test of others; its own: if two people reading a task cannot agree on one right answer, the task is rewritten or dropped before the PR (each task has one canonical target).

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
