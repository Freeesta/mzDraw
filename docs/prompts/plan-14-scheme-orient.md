# plan-14: keep a shared core oriented the same way across a scheme

Proposed by `docs/plan/backlog.md` (row 41; Gemini card 3). Size M. Wave 3. Depends on: plan-13.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-14 scheme-orient: received, starting on branch `claude/plan-14-scheme-orient`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. 

## Scope
In:
- In `scheme.rs`: `fn orient(doc, from, to) -> Vec<Op>`: find the largest common substructure that `mzlab-mol`'s substructure search finds of the smaller of two neighbouring structures in the larger (or of their ring systems when that fails), fit the target's matched atoms onto the source's by a 2D rigid transform (rotation, translation; a flip only if it lowers the error and the target has no wedges, since a flip turns wedges into the other enantiomer), and return `rotate`/`flip`/`move` ops.
- Command `scheme:orient` on a scheme: each structure after an arrow is oriented on the one before it; one undo step.
- Atoms outside the match keep their place relative to the matched ones; if the result overlaps itself (any two atoms closer than 0.3 bond lengths), the slice reports it and does not apply (no clean-up here).

Out (do not do here):
- A maximum-common-substructure engine of our own (use what `mzlab-mol` has; if it is not enough, list the cases for COO 16).
- Clean-up of the new substituents (arch-web-10).

## CLI twin (rule 15)
- `{"command":"scheme:orient"}` in the replay request; moves in the log.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- `mzlab-mol` `substruct.rs` (what it matches, its cost; read at the pinned commit), `ops.rs` (`rotate`, `flip`, `move`), plan-13 `scheme.rs`.

## Agreement needed before the PR
If the substructure search is not reachable from `mzdraw` at the pinned features, that is a COO 16 question: ask first.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-14-scheme-orient` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Match, fit (Kabsch in 2D, written in Rust with `libm`), ops.
2. Self-overlap check.
3. Synthetic pairs: a substituted benzene → its nitro product; a steroid-like tetracycle → its hydroxylated form; a pair with no common ring (no change, reported).

## Tests to add
- For 10 synthetic pairs, the root-mean-square distance of matched atoms after the command is below 0.05 bond lengths; for the drawn-rotated input it was above 0.5.
- Wedged target: no flip is chosen.
- Undo; parity; e2e on a two-step scheme.
- **Fails without the change**: the RMS test fails before the command (inputs are rotated by hand by 70°).

## Independent proof
Correctness must not rest on our own claims. RDKit `rdDepictor.GenerateDepictionMatching2DStructure` on the same pairs (in `tools/fidelity/orient.py`) gives reference coordinates; the committed reference compares the angle of the core to ours within 2°. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

