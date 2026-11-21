# plan-08: live form check of curved arrows through `mechanism/1`

Proposed by `docs/plan/backlog.md` (row 52; Gemini card 6 (reframed: the engine is mzLab's `mechanism/1`)). Size S. Wave 6. Depends on: arch-cd-25 (anchored curved arrows and lone pairs), plan-04.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-08 mech-check: received, starting on branch `claude/plan-08-mech-check`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. The check says whether every arrow is well formed, never which mechanism is right.

## Scope
In:
- Through `glue::run("mech", ...)`: the curved arrows of arch-cd-25 (source: lone pair, bond or radical; sink: atom or bond; full or half head) and the drawn lone pairs, as `mechanism/1` takes them.
- Each problem the engine reports becomes a line in the "Drawing checks" panel of arch-std-01 (one key per kind, i18n `draw2.mech.<key>`), and clicking it selects the arrow.
- Recomputed after each change, never blocking.

Out (do not do here):
- Generating the next intermediate (deferred; a new mzLab engine).
- Electron bookkeeping beyond what `mechanism/1` checks.

## CLI twin (rule 15)
- `replay_glue` with `{"engines":[{"engine":"mech"}]}`; in mzLab `mzlab-cli mol mech` on the same input gives the same answer (compared in a test).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- mzLab `src/chem/mech.rs` (its input fields), the arch-cd-25 object fields, `conform.rs` panel of arch-std-01.

## Agreement needed before the PR
None: this slice adds no `draw_edit/1` operation or field and changes nothing in `mzlab-mol` / `mzlab-feat`. If you find you need one, stop and ask the coordinator (it asks COO 16).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-08-mech-check` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Map our arrows and lone pairs to the engine input; map answers back to ids.
2. Panel lines and selection.
3. Synthetic mechanisms.

## Tests to add
- Hydroxide attacking a methyl bromide carbon (SN2) drawn with two full arrows: no problem; the same with the first arrow starting at a carbon with no lone pair: one problem on that arrow.
- A fishhook from a bond: accepted as one electron.
- Parity wasm == native; e2e with keyboard and touch on a panel line.
- **Fails without the change**: the arrow from a carbon with no lone pair must produce one problem; without the glue the panel is empty.

## Independent proof
Correctness must not rest on our own claims. The answer of `mzlab-cli mol mech` on the same input is the reference (mzLab's engine, tested there); the synthetic cases are written from a textbook rule we state in our own words in the test. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

