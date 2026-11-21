# tp-05: arrow labels computed from the two structures (+O, +15.9949)

Proposed by `docs/study/tp-draw.md` (verdict V2, idea 2, part 3). Size S. Depends on: tp-01 (anchors), plan-04 (engine glue callable natively, for masses), arch-web-13 (the `below` text slot of an arrow; without it, the mass goes on a second line of `text`). Wave: after tp-04.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-05 delta-labels: received, starting on branch `claude/tp-05-delta-labels`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only.

## Why
On a TP scheme each arrow says what changed: `+O`, `−CH2`, `+C6H8O6`, and often the mass shift. Typed by hand, these drift from the structures. Computed, they cannot.

## Scope
In:
- Command `scheme:label` (one undo step, `obj.set` of `text` and, with arch-web-13, `below`): for each anchored arrow `P → T`, the **formula delta** `T − P` in the canonical spelling of `mzlab-mol` `delta::print` on the two Hill counts of `outline.rs` (`+O`, `-CH2`, `-Cl+OH`; written with a true minus sign U+2212 on the drawing, ASCII `-` in the data), and optionally the **monoisotopic mass shift** to 4 decimals with its sign (`+15.9949`), from the nuclide masses of mzLab through the glue (plan-04 `glue.rs`, one engine arm: masses are not in `mzlab-mol`, and are never re-typed here).
- Setting `scheme.label` = `delta` (default) | `mass` | `delta+mass` | `name+delta` (`name` from tp-07's table when the delta matches exactly one named transformation, else just the delta).
- A label typed by a person is kept: the command fills only empty labels and labels it wrote itself (a flag `auto` in the arrow's `More`, kept across versions); `"force":true` rewrites all.
- A TP whose formula equals its parent's (an isomer) gets `isomer` (i18n key) instead of an empty delta.
- After tp-04, the column gap leaves room for the widest label (tp-04 step 5 rule, wired here).

Out (do not do here):
- Observed m/z, ppm errors (tp-06); naming transformations without a match (no guess).

## CLI twin (rule 15)
- `{"command":"scheme:label","set":["scheme.label=delta+mass"]}` in the replay request; the log gets the `obj.set` ops; `"out":"lens"` (tp-03) already gives the delta in data form.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state"), `docs/plan/backlog.md` (sections 0, 4 and 5), `docs/study/tp-draw.md`.
- `outline.rs` (counts), `mzlab-mol` `delta.rs`, `crates/mzdraw-wasm/src/glue.rs` (plan-04), arch-web-17 (values: the same mass arm), tp-01.

## Agreement needed before the PR
None if arch-web-13's `below` field exists; otherwise the two-line `text` is used and nothing is added. The engine arm in `glue.rs` is append-only.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-05-delta-labels` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Delta from counts; label text; the `auto` flag.
2. Mass arm in the glue; rounding (round half to even at 4 decimals, stated).
3. Goldens: the pathway of tp-04 labelled in the four modes.

## Tests to add
- Property: for 50 seeded random derive patches (tp-02), `delta::add(P, label) == T` on the formulas.
- Values: `+O` → `+15.9949`, `-CH2` → `-14.0157`, `+C6H8O6` → `+176.0321`, `+SO3` → `+79.9568`, `-H2` → `-2.0157`, `+H2O` → `+18.0106` (each checked in the test against the sum of the nuclide masses the glue uses, not against these typed numbers alone).
- A typed label survives `scheme:label`; `force` rewrites it; undo returns the identical document.
- e2e: the command from the palette by mouse and touch, and from the keyboard; the SVG equals the golden.
- **Fails without the change**: the delta property has no command on `main`.

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/delta.py` computes, with RDKit (`rdMolDescriptors.CalcMolFormula`, `Descriptors.ExactMolWt`), the formula difference and the monoisotopic mass difference of every synthetic pair from our molfiles; the committed reference equals our labels (mass within 1e-4 u). Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

