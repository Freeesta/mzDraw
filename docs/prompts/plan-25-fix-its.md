# plan-25: exact fix-its on drawing-check findings

Proposed by `docs/plan/backlog.md` §9 (row 74; second Gemini input, idea 5, "diagnostics with fix-it patches"). Size S. Wave 4. Depends on: arch-std-02 (geometry checks), arch-std-15 (`atoms.xy`), plan-20.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-25-fix-its: received, starting on branch `claude/plan-25-fix-its`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only (`conform.rs`). A finding of `std/1` may carry a patch that fixes it, but only when the fix is exact and needs no taste.

## Scope
In:
- An optional `fix` on findings of `std/1`: a list of `draw_edit/1` ops of the plan-01 table (no new op) that, applied, removes that finding and creates no other.
- A table in `docs/mzdraw.md` ("Drawing checks"): every `std.*` key, and either the exact fix or "no exact fix" with the reason (for example `std.geom.atomClash`: moving either atom is a choice, so no fix).
- Only keys whose fix is mechanical get one (candidates to check: a wedge whose narrow end is not the stereocentre, a bond length off the median through `atoms.xy`); decide per key and say why.

Out (do not do here):
- Fixes that need a layout choice; applying fixes automatically (an agent copies the ops into its log).

## CLI twin (rule 15)
- `{"out":"std"}` unchanged, with `fix` where given; replaying the `fix` ops then `"out":"std"` shows the finding gone (native replay test).
- **The `std/1` golden bytes move** for every golden with a fixable finding: rewrite them with `MZDRAW_BLESS=1 cargo test -p mzdraw --test replay` and list in the PR which goldens moved and that only `fix` was added.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Drawing checks"), `docs/plan/backlog.md` §9.
- `conform.rs`, `docs/study/arch-std.md` §4 (the checks and their sources).

## Agreement needed before the PR
No `draw_edit/1` op. The `fix` field changes the `std/1` output that `"out":"std"` gives through `mol draw`: add one row for it in the `request` section of plan-01's table (D13; an output field is part of the contract like an output name).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-25-fix-its` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Per key: exact fix or not, with the reason.
2. `fix` on those findings.
3. Table in the doc.

## Tests to add
- Property: for random documents, applying any `fix` removes its finding, adds no finding, and undo returns the identical document.
- Each key with a fix has a fixed synthetic case and a golden.
- **Fails without the change**: the property test has no `fix` to apply before the slice.

## Independent proof
Correctness must not rest on our own claims. After applying the fixes to the plan-03 set, the canonical isomeric SMILES (RDKit, in the tool) is unchanged: a fix changes the drawing, never the structure. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
If fewer than 3 keys get an exact fix, do not add the field: write the table only (as a doc change to arch-std-02's section) and say so in the PR.

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
