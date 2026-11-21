# tp-04: the diff lens (what changed from the parent, shown on the TP)

Proposed by `docs/study/tp-draw.md` (verdict V1, idea 1, part 2). Size S. Depends on: tp-03. Wave: after tp-03.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-04-diff-lens: received, starting on branch `claude/tp-04-diff-lens`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. In a figure of TPs the reader looks for the change. Tinting it by hand is a chore, and the tint gets out of date when the TP is edited. Here the change is computed from the atom maps of the pathway, so it is always true.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- `fn lens(doc, arrow) -> Lens` in `tp.rs`. For an anchored arrow from structure P to structure T, compared by map number (plan-11):
  - **added**: atoms of T whose map is not in P, and bonds of T with at least one added end;
  - **changed**: atoms in both whose element, charge, isotope, radical or hydrogen count differ; bonds in both (same two maps) whose order differs;
  - **cleaved**: bonds of P between two maps that are both in T but are not bonded in T;
  - **lost**: atoms of P whose map is not in T (listed, not drawn on T).
  An atom or bond without a map, on either side, makes the lens refuse with the finding `std.tp.unmapped`; it never guesses.
- Display setting `display.lens` = `off` (default) | `tint` | `tint+ghost`:
  - `tint`: added and changed atoms and bonds of T in the accent token (one token from the UI-2 palette; arch-std-03 checks its contrast);
  - `tint+ghost`: also each cleaved bond, drawn on T as a thin dashed line between the two T atoms, in the muted token, behind the bonds.
- The lens is display only. It is not in the molfile, SMILES, KET, CDXML or RXN; it is in the SVG and PNG only when the setting is on at export.
- The "What is drawn" outline gets one line in words per TP: "from C17H19NO3: +O, 1 bond cleaved". The delta comes from `mzlab-mol` `delta.rs` on the two Hill counts.

Out (do not do here):
- Any comparison without maps (structure matching across different origins is plan-15's later question).
- Colours chosen by the user (the token is fixed here).

## CLI twin (rule 15)
- `{"set":["display.lens=tint+ghost"],"out":"svg"}` gives the tinted SVG.
- `{"out":"lens"}` gives the machine form, `lens/1`: `{"format":"lens/1","arrows":[{"arrow":ID,"added":[ids],"changed":[ids],"cleaved":[[a,b]],"lost":[maps],"delta":"+O"}]}`, with ids ascending and arrows by id. An unmapped pair is listed in `std/1`, never a partial lens.
- Both outputs are deterministic, with parity wasm == native.
- Request rows (plan-01 `request` section, D13): the output `lens`, the setting `display.lens`.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state"), `docs/plan/backlog.md` (sections 0, 4, 5, 9), `docs/study/tp-draw.md`.
- `scene.rs` (bond and atom painting, tokens), `settings.rs` (a `display` section: if arch-web-17 or plan-11 added one first, extend it; else add it), `outline.rs`, tp-03 `tp.rs`.

## Agreement needed before the PR
No `draw_edit/1` op or field. Request rows only (D13).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-04-diff-lens` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. `lens` on maps; the `lens/1` writer (by hand, keys in a fixed order).
2. Painting (tint, dashed ghost behind); outline line.
3. Goldens: the three TPs of tp-03 with the lens `off`, `tint` and `tint+ghost`.

## Tests to add
- Property: with an empty patch (TP = copy), the lens is empty; for a patch of k added atoms, `added` has exactly k atoms; for a ring opening, `cleaved` has exactly the removed bond.
- Property: P's formula plus the delta equals T's formula (`delta::add`), for 50 seeded random patches of add / remove / set ops.
- The molfile, SMILES and KET bytes are the same with the lens on and off.
- e2e: toggle the setting from the keyboard and by touch.
- Screenshots at deviceScaleFactor 2 in the PR: one TP with the lens `off`, `tint` and `tint+ghost`.
- **Fails without the change**: `"out":"lens"` is an unknown output on `main`.

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/lens.py` gives RDKit the RXN with maps of each pair and lists the added, changed and cleaved sets from RDKit's own mapped atoms and bonds. The committed reference equals our `lens/1` on the synthetic pairs. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

## Kill test
Show the owner (or two mass spectrometrists the owner names) the three goldens with the lens off and on. If they would not keep the tint in a figure, keep `out:lens` and the outline line, and set `display.lens` to stay `off` with no page toggle (say so in the PR).

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

