# plan-23: a compact text digest of the drawing for agents that cannot see

Proposed by `docs/plan/backlog.md` §9 (row 72; second Gemini input, idea 5, "compact spatial digest"). Size S-M. Wave 2. Depends on: plan-20 (an arm of the same runner); arch-std-02 helps (clearance findings) but is not needed.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-23-digest: received, starting on branch `claude/plan-23-digest`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only. An agent today must read the whole `mzdraw/1` document (or a picture) to learn the ids and where things are; the digest is a few hundred bytes that say it.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- A new module `crates/mzdraw/src/digest.rs`; `"out":"digest"` writes `digest/1`, keys in a fixed order, numbers through `mzlab_mol::fixed`, written by hand:
  - per piece, ordered left to right: `ids` (atom ids as ranges, `"1-12,15"`), `formula` (Hill), `box` (in bond lengths, one decimal), `labels` (`[[id,"N"],...]` for every atom that is not a plain carbon, and every abbreviation), `stereo` (`[[bond id,"wedge"|"hash"|"wavy", narrow-end atom id],...]`), `rings` (count);
  - `objects`: `[[id, kind, box]]`;
  - `clearance`: the smallest distance between two pieces, in bond lengths;
  - `std`: counts of `std/1` findings by key.
- A byte budget, tested: at most 1,200 bytes for a 30-atom structure with one arrow and one text.
- Add arm `digest` to plan-19's runner: the model may ask for the digest of its current state at any time; each digest is one replay request and counts as one call, like any other (no free reads); repeated digests are allowed and all counted, and the bytes of every digest answer count in the task's bytes.

Out (do not do here):
- Anything chemical beyond the formula (CIP labels come with arch-web-16 through the glue: add `cip` to the digest then, one line).
- Pictures.

## CLI twin (rule 15)
- `{"import":...,"log":[...],"out":"digest"}`; stdout the JSON, exit 0. Native replay test with the page's bytes (the page shows nothing new).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md`, `docs/plan/backlog.md` §9.
- `outline.rs` (the outline line already names pieces in words: reuse its piece order), `conform.rs`, `doc.rs`.

## Agreement needed before the PR
No `draw_edit/1` op. One row for the output `digest` in the `request` section of plan-01's table (D13).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-23-digest` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Digest writer.
2. Budget test, runner arm, doc section.

## Tests to add
- Digest of a fixed synthetic document is byte-identical to a golden; two runs give the same bytes.
- Budget test; a 200-atom structure still lists every hetero atom id.
- Every id in the digest exists in the document (property).
- **Fails without the change**: the golden test cannot pass before `digest.rs`.

## Independent proof
Correctness must not rest on our own claims. The formula of each piece equals RDKit's `CalcMolFormula` for the plan-03 set (in the tool); the box is checked against the SVG's own bounds by a test. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
Run plan-19 with arm `report+digest` against arm `report` on the same model(s). If pass@1 gains less than 15 points on the "surgical edits" and "several structures" tasks, or the median bytes per passing task rise, drop the runner arm and keep the output only for people reading logs (say so in `docs/agent-suite/`).

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
