# plan-16: an agent's proposal shown as a layer, accepted or rejected hunk by hunk

Proposed by `docs/plan/backlog.md` (row 48; Gemini bold idea 3, part 2). Size M. Wave 6. Depends on: plan-15, UI-2 PR 2.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-16 proposal-layer: received, starting on branch `claude/plan-16-proposal-layer`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. 

## Scope
In:
- `Editor::propose(patch)`: holds a patch (from an agent or a file) as a proposal, not in the document. The scene paints it over the drawing: added atoms and bonds in the accent token, removed ones struck in the warning token, changed ones outlined, each hunk numbered; the document is unchanged until accept.
- `accept(hunk)` and `reject(hunk)`; `accept_all`, `reject_all`. Each accept is one undo step of exactly the hunk's ops; reject drops it. The remaining hunks stay valid (plan-15 guarantees it).
- Keyboard: next and previous hunk, accept, reject (keys from arch-std-06's command table); touch: tap a hunk's number; the live line says what the hunk does (from its ops, in words).
- Page: a small panel listing the hunks (paint and forward); the host can call `propose` through the element API when arch-web-02 exists.

Out (do not do here):
- Any agent of our own; any network.
- Merging two proposals.

## CLI twin (rule 15)
- The replay request takes `"propose":PATCH` and `"accept":[hunk numbers]`; the result log has exactly the accepted hunks' ops, so a native replay gives the same bytes as the page.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- plan-15 `diff.rs` (`hunks`), `scene.rs` overlay painting, `Overlay` in the editor, UI tokens of UI-2.

## Agreement needed before the PR
None: this slice adds no `draw_edit/1` operation or field and changes nothing in `mzlab-mol` / `mzlab-feat`. If you find you need one, stop and ask the coordinator (it asks COO 16).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-16-proposal-layer` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Proposal state in the editor (not in the document, not in the log until accepted).
2. Overlay painting; panel; keys.
3. e2e.

## Tests to add
- Accept hunks 2 and 4 of a four-hunk patch: the document equals `a` plus those hunks, undo twice returns `a`.
- Reject all leaves the document identical and the log unchanged.
- Parity of the request with `propose` and `accept`; e2e with mouse, touch and keyboard; screenshots.
- **Fails without the change**: the accept test fails before `propose` exists.

## Independent proof
Correctness must not rest on our own claims. The result is checked as bytes against `apply` of the same hunks (plan-15's property); the review time study Gemini proposes is a person's experiment, written as a protocol in `docs/plan/velocity-protocol.md` (one section). Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

