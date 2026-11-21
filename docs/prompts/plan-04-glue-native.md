# plan-04: engine glue callable natively, so parity covers mzLab engines

Proposed by `docs/plan/backlog.md` (row 12; finding F1; backlog D2). Size S. Wave 1. Depends on: none (CTX-MENU PR #37, which also touched the bridge, is merged).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-04 glue-native: received, starting on branch `claude/plan-04-glue-native`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. This slice moves code and adds a native door; it adds no feature. Every later slice that shows the answer of a mzLab engine (CIP, cuts, mechanism, highlight, selection values) builds on it.

## Scope
In:
- A new module `crates/mzdraw-wasm/src/glue.rs` (feature `mzlab`): `pub fn check(doc: &Document) -> Option<String>` (the existing `check/1` call moved here unchanged) and a small `pub fn run(engine: &str, doc: &Document, args: &str) -> Result<String, GlueError>` that builds the molfile with stable atom ids once (the same text `check` uses today) and calls the engine function of `mzlab_feat::chem` by name: `check`, `cip`, `cut`, `mech`, `hilite`, `sel` where the pinned features compile them. Engines not compiled return `error.draw2.engineMissing`.
- A map from the engine's atom numbers back to document ids (`glue::ids`), tested, so every engine answer is painted on the right atoms.
- A native example `crates/mzdraw-wasm/examples/replay_glue.rs`: the replay request of `crates/mzdraw` plus `"engines":[{"engine","args"}]`, whose answers are returned under `"engines"` in request order.
- `tests/parity.mjs`: runs `replay_glue` natively and the wasm bridge on the same request and compares the bytes (one case per engine that compiles).

Out (do not do here):
- Painting any engine answer (each feature slice paints its own).
- Any change in `mzlab-feat` (if a feature such as `sel` must be turned on in `Cargo.toml`, say so in the PR; turning on an existing feature is ours, changing mzLab code is not).

## CLI twin (rule 15)
- `cargo run -p mzdraw-wasm --example replay_glue < request.json` (stdout the JSON, exit 0; 2 with `error.draw2.engineMissing` on stderr).
- In mzLab the same functions are `mzlab-cli mol check|cip|cut|mech|hilite|sel`; a test feeds the molfile our glue builds to the CLI's function in-process and compares bytes.

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- `crates/mzdraw-wasm/src/lib.rs` (the current `check_json` call), `crates/mzdraw-wasm/Cargo.toml` (feature `mzlab`).
- mzLab `crates/mzlab-feat/Cargo.toml` (features `chem`, `check`, `sub`, `sel`) and `src/chem/{check,cip,cut,mech,hilite,sel}.rs` at the pinned commit `ccd4680` (read; do not change).

## Agreement needed before the PR
None: this slice adds no `draw_edit/1` operation or field and changes nothing in `mzlab-mol` / `mzlab-feat`. If you find you need one, stop and ask the coordinator (it asks COO 16).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-04-glue-native` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Move the check call into `glue.rs` with no byte change of the page state (golden parity must stay identical).
2. Add `run` and `ids`, the example and the parity cases.
3. List in the PR which engines compile under the pinned features and the module size of each one turned on.

## Tests to add
- `crates/mzdraw-wasm/tests/glue.rs` (native): for a synthetic drawing with ids that are not 0..n (after deletions), `ids` maps every engine atom number to the right document id.
- Each compiled engine: one synthetic drawing, the answer equals the one of the mzLab function called directly on the same molfile.
- Parity wasm == native for each case.
- **Fails without the change**: the id-map test with deleted atoms (ids 1, 4, 9) fails if the glue passes the molfile line numbers through unchanged.

## Independent proof
Correctness must not rest on our own claims. The engines are mzLab's own and have their own tests there; the independent check here is that our glue gives the engine exactly the molfile `mzlab-cli` gives it (bytes compared) and maps the answer back to the right atoms. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

