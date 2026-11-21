# plan-26: a flip that keeps the configuration

Proposed by `docs/plan/backlog.md` §9 (row 75; split from plan-21 at the coordinator's review). Size S. Wave 3, after plan-21. Depends on: plan-21 (the signature, for the tests).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-26-flip-keep: received, starting on branch `claude/plan-26-flip-keep`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust; one command on the page (forwarded only). "Flip that keeps the configuration (optional)" is listed under "Still to build of slice 2" in `docs/mzdraw.md`.

## Scope
In:
- Command `flip:keep` (horizontal and vertical, as the plain flip): the flip of the selection plus the swap solid ↔ hashed of every wedge whose narrow end (the stereocentre) is a selected atom, as ONE undo step made of existing ops (`flip`, `bond.set`). Wavy and plain bonds are left alone.
- **Selection that cuts a stereocentre**: if a stereocentre in the selection has a neighbour outside it (or a stereocentre outside it has a neighbour inside), mirroring part of its neighbourhood has no configuration-keeping answer: the command is refused, nothing changes, and the live line says why (key `error.draw2.flipKeepCut`, with the atom ids). A selection that holds whole pieces, or that cuts only at atoms that are not stereocentres, is accepted.
- Keys and menu entry next to the plain flip (arch-std-06's table when merged; the context menu item through `context.rs`).

Out (do not do here):
- Rotations (they never change a configuration).
- E/Z: a flip keeps E/Z in 2D; no change needed (test it).

## CLI twin (rule 15)
- The log of `flip:keep` is plain `flip` + `bond.set` ops, so any replay reproduces it; a native replay test gives the page's bytes. A refused `flip:keep` writes nothing to the log.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` (wedges, slice 2a), `docs/plan/backlog.md` §9.
- `ops.rs` (`Flip`), `editor_edit.rs` (the plain flip command), `context.rs`, `stereo_guard.rs` of plan-21.

## Agreement needed before the PR
None for `draw_edit/1`: this slice adds no operation or field and changes nothing in `mzlab-mol` / `mzlab-feat`. If you find you need one, stop and ask the coordinator (it asks COO 16).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-26-flip-keep` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. The command and the cut rule.
2. Keys, menu entry, live line.
3. Doc: "Built state" bullet; remove the item from "Still to build of slice 2".

## Tests to add
- A chiral centre: `flip:keep` keeps the signature, the plain flip does not; both are one undo step and undo returns the identical document (property).
- A selection cutting a stereocentre is refused and the document is unchanged; a selection cutting a chain at a non-stereo atom is accepted.
- E/Z unchanged by both flips.
- Parity case; e2e: the key and a touch/pen route to the menu entry.
- **Fails without the change**: the command does not exist.

## Independent proof
Correctness must not rest on our own claims. For the stereo structures of the plan-03 set: the InChI stereo layer (RDKit, in the tool) is unchanged after `flip:keep` of whole pieces. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
If the cut rule refuses more than half of the flips in the e2e scenarios written from the velocity tasks of plan-02 (people flip substituents, not pieces), list it for the coordinator: the command then ships for whole pieces only and the menu entry is hidden for a cutting selection.

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
