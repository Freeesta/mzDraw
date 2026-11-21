# arch-cd-04: Double-bond position (centre, left, right) and bold / dashed bond display

Study rows: A03, A04 of `docs/study/arch-cd.md`. Priority MUST (A03), SHOULD (A04). Size guess S. Depends on: nothing.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-04 bond-display: received, working on claude/cd-04-bond-display`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- bond field `pos` (`auto` default, `c`, `l`, `r`: where the second line of a double bond goes, seen from atom `a` to `b`) and `d` (display: none, `bold`, `dash`, `bdash`, `hash`), both in `bond.set` with exact inverse
- auto: ring doubles inside the ring, terminal doubles centred, chain doubles on the side of more substituents (write the rule down)
- painted on screen and in the SVG; written to CDXML and KET where they have a place, otherwise listed as not written

Out (do not build here, say so in the PR if you meet it):
- stereo wedges (built)
- dative and hydrogen bonds (prompt 23)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `crates/mzdraw/src/doc.rs` `Bond` (`o`, `s`, `more`) and its hand-written JSON; `ops.rs` `BondSet`; `scene.rs` (how doubles are drawn now).
- Study facts: ChemDraw sets left/right/centre per bond with keys (CD-UG17); Marvin JS offers middle/left/right/auto (MJ/marvin-js_bond.html); MarvinSketch treats bold as a display style (MS/marvin_draw-bonds.html); ChemDoodle has bold, bold dashed, dashed doubles (CDD-G §2.2.4.3).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-04-bond-display` from `origin/main`.

1. Add `pos` and `d` to `Bond` (optional in `mzdraw/1`, left out when default: same document = same bytes) and to `BondSet` (log `{"op":"bond.set","id":5,"pos":"l"}`); a new order other than 2 drops `pos`. Write the op by hand like the others: `Op::write` (writer) and `Op::read` / `Op::from_value` (reader) in `crates/mzdraw/src/ops.rs`, defaults left out; the reader refuses a malformed op with its error key and changes nothing.
2. Scene: the offset line of a double on the chosen side, shortened at ring atoms as now; bold = the style's bold width; dash = the hash spacing as dash pattern.
3. Click on a double bond with the double-bond tool cycles `auto → l → c → r → auto` (as MarvinSketch cycles alignment); hover keys `L`, `C`... only if prompt 03 has merged (else leave a note).
4. CDXML out: `DoublePosition` and `Display` where the public CDX SDK pages define them; KET: leave out and list. Never guess an attribute: if a page does not say it, write the gap in the PR.

Tests to add (synthetic data, small):
- property test with `pos` and `d`; JSON round trip; a document with defaults is byte-identical to before
- SVG of a synthetic cyclohexene with `l` and `r` differs only in the second line; the golden SVGs of existing documents do not move

UI: a small bond-style group in the bond palette (bold, dashed); e2e: cycle a double bond by clicks and check `save("mzdraw")`; touch: the same with a tap. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-04-bond-display`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
