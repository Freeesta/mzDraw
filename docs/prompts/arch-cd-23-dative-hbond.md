# arch-cd-23: Dative (coordinate) and hydrogen bonds

Study rows: A05, A06 of `docs/study/arch-cd.md`. Priority SHOULD. Size guess S. Depends on: prompt 04 (bond fields) if merged.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-23 dative-hbond: received, working on claude/cd-23-dative-hbond`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- bond orders 9 (dative, drawn as an arrow from donor `a` to acceptor `b`) and 10 (hydrogen bond, dashed), the V3000 molfile codes; tools in the bond group; `bond.add o` / `bond.set o`
- valence: a dative bond adds no valence to the donor's count in the check the way a covalent bond does (follow `mzlab-mol`'s handling of V3000 type 9; if it has none, list it as a gap for mzLab); a hydrogen bond adds none
- V2000 molfile: no code exists for them — write them as V3000 automatically or list them as lost (decide, state in the PR); KET and CDXML where defined

Out (do not build here, say so in the PR if you meet it):
- haptic / multicentre bonds (M05)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `doc.rs` `Bond.o`, `ops.rs` validation of orders, `io.rs` (molfile out through `mzlab-mol`), `scene.rs` (arrow heads exist for reaction arrows).
- Study facts: ChemDraw 25.0 treats dative and coordination as one type and writes MOL DISP COORD (CD-25.0*); MarvinSketch draws a coordinate bond as a line or an arrow (MS/marvin_draw-coordination-compounds.html); ChemDraw added a hydrogen-bond tool in 22.0 (CD-RN22).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-23-dative-hbond` from `origin/main`.

1. Orders 9 and 10 accepted by the op validation; the tool `bond:dative`, `bond:hbond`; a second click on a dative bond turns its direction (`rev`).
2. Molfile V3000 out and in; SMILES out: dative as the record allows, hydrogen bonds dropped and listed.

Tests to add (synthetic data, small):
- property test with the new orders; molfile V3000 round trip; check/1 gives no valence warning for a pyridine–BF3 adduct drawn with a dative bond (if `mzlab-mol` supports it; else the test documents the gap)

UI: two tools in the bond group; e2e: draw a dative bond, turn it, save molfile V3000; touch path. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

Docs: keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice: what it does, the op/setting names, what is not built); update the row(s) of `docs/study/arch-cd.md` to `built` in the same PR.

Size: the owner lifted the wasm size limit for now, but rule 8 still holds: measure with `tools/build_site.sh`, write the new size in `measured` of `tools/wasm-cap.toml` and the delta in the PR. If the module passes `cap`, send COO-MZDRAW the numbers and raise `cap` only when it answers, with the reason in the PR.

## Acceptance

All of these pass, and their output is quoted in the PR:
```
cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
tools/build_site.sh
cargo build --locked -p mzdraw --example replay && node tests/parity.mjs
node tests/e2e.mjs
```

If a golden output changes because of this slice (and only then), rewrite it with `MZDRAW_BLESS=1 cargo test -p mzdraw --test replay` and explain each changed file in the PR. Extend `tests/data/mzdraw_log.json` with the new op(s) so that the parity test replays them.

## Messages

- To COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) only. Start: the confirm line above. Blocked (a test you cannot make pass, a decision that is not yours, a host refused): one message with the exact error, then wait or continue with the rest.
- Done: one message with the PR link, the full head SHA, the module size before → after, and the study rows now built.
- The PR is in English, ready for review (not a draft), titled after the feature, its body following the repository's usual sections, with the screenshots for UI changes.

## Handoff at 200k tokens

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-23-dative-hbond`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
