# plan-21: stereo guard: a geometry edit that changes a configuration is reported, and a flip that keeps it

Proposed by `docs/plan/backlog.md` §9 (row 70; second Gemini input, idea 2 "stereo-invariant geometry", reduced). Size M. Wave 3. Depends on: plan-20 (the `expect` arm), arch-std-01 (built).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-21-stereo-guard: received, starting on branch `claude/plan-21-stereo-guard`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust; one command on the page (forwarded only). In mzDraw the wedges are the drawing and the configuration is derived at export, so moving or mirroring atoms can change the configuration without any stereo edit; "flip that keeps the configuration (optional)" is listed under "Still to build of slice 2" in `docs/mzdraw.md`.

## Scope
In:
- A new module `crates/mzdraw/src/stereo_guard.rs`: `signature(&Document) -> Vec<(piece, String)>`, the canonical isomeric SMILES of each piece as `mzlab-mol` writes it from the wedges as drawn (the same path as `save("smiles")`).
- After every undo step whose ops are only geometry (`move`, `rotate`, `flip`, `atoms.xy` when arch-std-15 has merged), the editor compares the signature before and after; a change is said in the live line ("configuration changed at atoms ...") and is never blocked in the page (coordinator, 2026-11-21: report-only in the page).
- The `expect` arm `{"stereo":"kept"}` of plan-20: false when any piece's signature differs between the start (after `import`) and the end of the log.
- Command `flip:keep`: the flip of the selection plus the swap solid ↔ hashed of every wedge whose narrow end is a flipped atom, as ONE undo step made of existing ops (`flip`, `bond.set`); keys and menu entry next to the plain flip (arch-std-06's table when merged).

Out (do not do here):
- Re-solving wedge placement automatically (IUPAC ST-1.1 preferred positions): DEFER.
- Atropisomers, allenes, enhanced-stereo groups (arch-std-12 keeps those).

## CLI twin (rule 15)
- `{"log":[...],"expect":[{"stereo":"kept"}],"report":true}`.
- The log of `flip:keep` is plain `flip` + `bond.set` ops, so any replay reproduces it; a native replay test gives the page's bytes.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` (wedges, slice 2a; `std/1`), `docs/plan/backlog.md` §9.
- `ops.rs` (`Flip`), `io.rs` (SMILES out), `conform.rs` (stereo checks), `mzlab-mol` `sites.rs` / `canon.rs` at the pin.

## Agreement needed before the PR
None for `draw_edit/1`: this slice adds no operation or field and changes nothing in `mzlab-mol` / `mzlab-feat`. If you find you need one, stop and ask the coordinator (it asks COO 16). Add one row for the `expect` arm `stereo` in the `request` section of plan-01's table (D13).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-21-stereo-guard` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Signature.
2. Guard after geometry steps (live line).
3. `flip:keep`.
4. `expect` arm, doc section.

## Tests to add
- A chiral centre: plain flip changes the signature (enantiomer), `flip:keep` does not; both are one undo step and undo returns the identical document (property test).
- Rigid move and rotate of a whole piece never change the signature (property over random documents with wedges).
- Dragging one neighbour of a wedged centre across the others changes it and the live line says so.
- Parity case for `flip:keep`; native replay test.
- **Fails without the change**: `flip:keep` does not exist; the signature test of a plain flip passes only once the guard compares.

## Independent proof
Correctness must not rest on our own claims. For the stereo structures of the plan-03 set: the InChI stereo layer (RDKit, in the tool) is unchanged after `flip:keep` and changed after a plain flip of a chiral piece. The guard's signature is `mzlab-mol`'s; RDKit is the outside check. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
Generate 1,000 random synthetic chiral documents and 5 random geometry edits each (natively, in a test or `tools/` script): the guard must flag 100% of plain flips of a chiral piece and 0 rigid moves/rotations. If it flags fewer than 1 in 1,000 realistic non-flip edits, keep `flip:keep` and the `expect` arm and drop the live-line guard (say so in the PR).

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
