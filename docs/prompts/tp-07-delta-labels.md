# tp-07: arrow labels computed from the two structures (+O, +15.9949)

Proposed by `docs/plan/backlog.md` §11 (row 87) and `docs/study/tp-draw.md` (verdict V2, part 5). Size S. Depends on:
- tp-01 (anchors);
- arch-web-17 (masses through the glue, E6: `selection/1` of `mzlab-feat` feature `sel`);
- arch-web-13 (the `below` text slot of an arrow; without it, the mass goes on a second line of `text`).

Wave: after tp-05 and arch-web-17.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-07-delta-labels: received, starting on branch `claude/tp-07-delta-labels`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. On a TP scheme each arrow says what changed: `+O`, `−CH2`, `+C6H8O6`, and often the mass shift. Labels typed by hand drift away from the structures; computed labels cannot.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- **Command `pathway:label`.** One undo step, written as `obj.set` of `text` (and of `below`, once arch-web-13 adds it). For each anchored arrow `P → T` it computes:
  - the **formula delta** `T − P`, in the canonical spelling of `mzlab-mol` `delta::print` on the two Hill counts of `outline.rs` (`+O`, `-CH2`, `-Cl+OH`); the drawing shows a true minus sign (U+2212), the data ASCII `-`;
  - optionally, the **monoisotopic mass shift** with its sign, to 4 decimals (`+15.9949`). It is the difference of the two monoisotopic masses that arch-web-17 takes from `selection/1` through the glue (E6: there is no mass table in `mzlab-mol`, and none is typed here). If arch-web-17 has not merged, the mass modes are refused with `error.draw2.labelMass` and `delta` works alone.
- **Setting `pathway.label`** = `delta` (default) | `mass` | `delta+mass` | `name+delta`. `name` comes from tp-10's table when the delta matches exactly one named step; otherwise only the delta is written. **Without tp-10** (not merged), `name+delta` is refused with `error.draw2.labelSteps` (exit 1, `failed.stage` `command`) and step names are not tokens (below).
- **Whose label it is, with no new field.** The label (`text`, and `below` when it exists) is split into tokens on line breaks and white space. A label is rewritten only when it is empty, or when **every** token is one this command writes:
  - a **signed** delta: a `+`, an ASCII `-` or a U+2212 minus first, then a text that `delta::parse` reads after the sign is mapped to ASCII. An unsigned formula is never a token, so reagent labels such as `H2O`, `O3` or `NaBH4` stay the person's;
  - a signed number with exactly 4 decimals (`+15.9949`, `−14.0157`);
  - a step name of tp-10's table, only when tp-10 is merged;
  - the `isomer` word of the page's language.
  Any other text is the person's and is kept; `"force":true` rewrites all labels. So no flag is stored, and no `draw_edit/1` field is added.
- **Isomers.** A TP whose formula equals its parent's is labelled `isomer` (i18n key) instead of an empty delta.
- **Room for labels.** tp-05's step 5 reserves room for the widest label in `g_k`; it is wired here, and the goldens of tp-05 with labels move (bless and list them).

Out (do not do here):
- Observed m/z and ppm errors (tp-09); naming a step without a match (no guess).

## CLI twin (rule 15)
- `{"set":["pathway.label=delta+mass"],"commands":[{"command":"pathway:label"}]}`. The log gets the `obj.set` ops. `"out":"lens"` (tp-04) already gives the delta in data form.
- A refusal is exit 1 with `failed.stage` `command` (`error.draw2.labelMass`).
- Request rows (plan-01 `request` section, D13): the command `pathway:label` with arg `force`, the setting `pathway.label`, and the keys `error.draw2.labelMass`, `error.draw2.labelSteps`.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state"), `docs/plan/backlog.md` (sections 0, 4, 5, 7 E6, 9), `docs/study/tp-draw.md`.
- `outline.rs` (counts), `mzlab-mol` `delta.rs`, `crates/mzdraw-wasm/src/glue.rs` and the native glue of plan-04, arch-web-17, tp-01, tp-05.

## Agreement needed before the PR
No `draw_edit/1` op or field: the ownership rule replaces a stored flag. The `below` slot is arch-web-13's field; without it, `text` holds two lines. Request rows only (D13). The mass arm in the glue is append-only.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-07-delta-labels` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Delta from counts; the label text; the ownership rule.
2. Mass shift from arch-web-17's numbers; rounding is half to even at 4 decimals, stated in the docs.
3. Goldens: the pathway of tp-05, labelled in the four modes.

## Tests to add
- Property: for 50 seeded random derive patches (tp-03), `delta::add(P, label) == T` on the formulas.
- Values: `+O` → `+15.9949`, `-CH2` → `-14.0157`, `+C6H8O6` → `+176.0321`, `+SO3` → `+79.9568`, `-H2` → `-2.0157`, `+H2O` → `+18.0106`. Each is checked in the test against the masses the glue returns, not only against these typed numbers (study §1 T11).
- Ownership: labels typed as `hydroxylation (CYP)`, `H2O`, `O3` and `+O, 37 °C` survive `pathway:label`, and `force` rewrites them. A label `+O` (ASCII), `−O` (U+2212) or `+O` newline `+15.9949` on an arrow whose TP changed is rewritten. Without tp-10, `name+delta` is refused.
- Undo returns the identical document.
- e2e: the command from the palette by mouse and touch, and from the keyboard; the SVG equals the golden.
- Screenshots at deviceScaleFactor 2 in the PR: the pathway in `delta` and in `delta+mass`.
- **Fails without the change**: `pathway:label` is an unknown command on `main`.

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/delta.py` computes, with RDKit (`rdMolDescriptors.CalcMolFormula`, `Descriptors.ExactMolWt`), the formula difference and the monoisotopic mass difference of every synthetic pair from our molfiles. The committed reference equals our labels, with masses within 1e-4 u. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

## Kill test
If the ownership rule rewrites a person's label in any e2e or synthetic case, the rule is wrong. Stop and ask the coordinator for a stored field instead (COO 16), and do not ship a rule that loses text.

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

