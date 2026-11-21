# plan-31: the chain head carried in saved SVG and PNG

Proposed by `docs/plan/backlog.md` §10 (row 80; fourth external report, "provenance ledger", part 2; split from plan-30 at the coordinator's review). Size S. Wave 4. Depends on: plan-30, arch-std-16 (export profiles), arch-std-17 (export settings).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-31-chain-in-pictures: received, starting on branch `claude/plan-31-chain-in-pictures`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only. The saved picture already carries its `mzdraw/1` document (slice 3b); it now also says which chain made it.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- The SVG the editor saves carries `<metadata id="mzdraw-chain">` with `chain/1` (plan-30), next to the document's metadata. The PNG carries an `iTXt` chunk with its own keyword next to the document's.
- **Empty or absent log**: no element and no chunk, so pictures saved without edits stay byte-identical. Goldens whose request has a non-empty log and saves an SVG or PNG move: bless them and list them.
- **Open, then save**: when a picture that carries a chain is opened, the new session starts its own chain from the opened document (`h0` of plan-30). The carried head is kept as `parent` in the new `chain/1`, so the lineage is one hop back; nothing older is stored.
- **Clean export**: a setting `export.chain` (`on` by default, `off`), read through the settings table. The journal profiles of arch-std-16 set it to `off`, because a submitted figure carries no provenance metadata. With `off`, nothing is written.

Out (do not do here):
- Verifying a picture's head against a log (that is plan-30's `expect` arm on the log).
- Any change to how the document itself is embedded.

## CLI twin (rule 15)
- `{"log":[...],"out":"svg"}` carries the element; `"set":["export.chain=off"]` omits it. The page's saved SVG equals the native replay's bytes (parity case).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` (slice 3b), `docs/plan/backlog.md` §10.
- `svg.rs` (`carrying`), `png.rs`, `settings.rs` (the table), `chain.rs` of plan-30, arch-std-16's profiles.

## Agreement needed before the PR
No `draw_edit/1` op. One row for the setting `export.chain` in the settings table and one in the `request` section of plan-01's table, for the `chain` element of the `svg` and `png` outputs (D13).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-31-chain-in-pictures` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. The element and the chunk, empty-log rule.
2. `parent` on open.
3. `export.chain`, the profiles' value, goldens, doc section.

## Tests to add
- Empty log: the saved SVG and PNG are byte-identical to today's.
- Non-empty log: the element's head equals plan-30's `out:chain`.
- Open a picture that carries a chain, edit, save: the new `chain/1` has `parent` equal to the old head.
- `export.chain=off`, and an arch-std-16 journal profile: no element.
- **Fails without the change**: no element exists before the slice.

## Independent proof
Correctness must not rest on our own claims. `tools/chain/check.py` of plan-30 also reads the element from the saved SVG goldens and recomputes their heads from the logs. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
If a picture with the element fails to reopen in the survival matrix of plan-10 (any host that reopened it before), take the element out and keep the chain in the log only.

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
