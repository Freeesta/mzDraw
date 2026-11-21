# plan-07: a peak list in, the cuts that explain each peak out

Proposed by `docs/plan/backlog.md` (row 39; Gemini card 1 and bold idea 1 (reframed: engines `breaks` and `fragmenter` of mzLab)). Size M. Wave 4. Depends on: plan-06.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-07 cut-peaks: received, starting on branch `claude/plan-07-cut-peaks`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Evidence, never identification: a peak explained by a cut is a hypothesis, and the page says so in words.

## Scope
In:
- Input: a peak list as text (`m/z intensity` per line, or the `peaks` of a MassBank record pasted as text), a ppm tolerance (default 5) and the polarity. It is page and request state, not part of the document (the drawing does not carry spectra).
- Through the glue: every set of one or two single bonds is cut by the mzLab engine (`breaks` semantics: each piece takes a hydrogen per cut and may gain or lose a few), and each peak gets the list of cuts whose piece m/z lies within the tolerance, with the error in ppm.
- Display: a list (peak, explained by, error) in a panel; choosing a line shows that cut as a temporary cut object (not in the document until the user keeps it, then `obj.add` as plan-05).
- The count of peaks explained and of coincidental matches per peak are shown, so the false-positive density Gemini warns about is visible.

Out (do not do here):
- Live re-ranking of candidate isomers while dragging (deferred until this slice's numbers are in).
- Reading spectrum files (mzML, MGF): the host passes text.

## CLI twin (rule 15)
- `replay_glue` with `{"engines":[{"engine":"breaks","args":{"peaks":"...","ppm":5,"polarity":"+"}}]}`; mzLab's `msn graph --breaks` is the batch twin (say in the PR how its input differs).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- mzLab `src/chem/breaks.rs`, `fragmenter.rs` (read; do not change); if neither has a function that takes a plain peak list, stop and ask the coordinator (COO 16) rather than writing the matching here.

## Agreement needed before the PR
If the engine needs a new entry point in `mzlab-feat` for a plain peak list, that is a COO 16 change: ask first. No `draw_edit/1` change.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-07-cut-peaks` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Find or agree the engine entry point.
2. Panel, temporary cut, keep-as-cut command.
3. Synthetic peak list tests; the dataset run in tools.

## Tests to add
- A synthetic peak list made from three known cuts of a synthetic structure plus two noise peaks: the three are explained by their cuts, the noise by none, errors below 1 ppm.
- Parity wasm == native.
- e2e: paste a peak list, choose a line with the keyboard and by touch, keep it as a cut.
- **Fails without the change**: the noise-peak test fails if the tolerance is applied in Da instead of ppm at high m/z.

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/massbank.py` fetches a fixed list of 100 MassBank records (CC BY 4.0, release v2026.11, accession list committed, data not), runs the matching on each record's structure and peaks, and commits the explained-peak rate and the median number of matches per explained peak. The annotated peaks of records that carry annotations are the reference. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

