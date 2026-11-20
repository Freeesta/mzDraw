# arch-web-16: CIP labels drawn next to the stereocentres

Prompt for one DEV chat of mzDraw (`Freeesta/mzDraw`), proposed by the study `docs/study/arch-web.md` (§7, slice 16).

## Confirm message
Your first message, to COO-MZDRAW (the coordinator; its session id is in your launch message; use `send_message` with that id), one line:
`DEV-CIP: received; branch claude/arch-web-16-cip from origin/main; reading CLAUDE.md, docs/mzdraw.md, docs/study/arch-web.md.`

## Role
You are DEV-CIP, a development chat. You build one small slice, open one PR ready for review, and report. The coordinator merges with the full head SHA; you never merge.

## Scope
- In: setting `display.cip=on|off`; R / S at tetrahedral centres and E / Z at double bonds, placed by geometry beside the centre each time the display list is built (never stored, so a move cannot leave them behind), from the descriptors of the record.
- Out: enhanced-stereo labels, a CIP engine of our own.
- Anything else you notice: one line to COO-MZDRAW, not in this PR.

## Context (read first)
- `CLAUDE.md` (rules 1-14), `docs/mzdraw.md` (the section "Built state" is what exists; do not rebuild it), `docs/study/arch-web.md` (the rows of §4 named in §7 for slice 16, and §5 for the API).
- Ketcher shows CIP with `Mod+P` (Indigo); Marvin JS needs a server; Ketcher's regression tests cite labels left behind after a move (#7899) and wrong E/Z label placement (#4757) (`docs/study/arch-web.md` §3).
- `mzlab-feat` has `cip/1`; `crates/mzdraw` may depend only on `mzlab-mol`, `libm`, `serde`, `serde_json`. First check whether the pinned `mzlab-mol` gives the descriptors: if it does, compute them in `crates/mzdraw` and draw them in files too; if only `mzlab-feat` does, compute them in the bridge (feature `mzlab`) and hand them in as host marks of slice 15 (screen only, not in files), and ask COO-MZDRAW whether files should carry them (that needs the replay to link `mzlab-feat`).

## Rules
- English only, everywhere (code, comments, texts, commits, PR).
- All logic in Rust (`crates/mzdraw`, bridge `crates/mzdraw-wasm`); JavaScript only forwards pointer, wheel and key events, calls the module and paints the display list. No chemistry and no decision in JS.
- No bundler, no framework, no new crate or npm package unless the PR states the reason.
- Synthetic data only in tests and examples (molecules you write yourself as SMILES or by hand).
- Every new edit operation has an exact inverse and is added to the random sequence of the property test (`random_step` in `crates/mzdraw/tests/ops.rs`: apply then undo gives the identical document bytes, redo the identical result), and to the log reader and writer tests of the same file.
- Every page function has its scriptable twin: a `draw_edit/1` op, a `section.key=value` setting, or a field of the replay request; the native replay (`cargo run -p mzdraw --example replay`) gives the same bytes as the page (`tests/parity.mjs`); add a case to `tests/data/mzdraw_log.json` and bless it only after a wanted change (`MZDRAW_BLESS=1 cargo test -p mzdraw --test replay`).
- Never skip, disable or loosen a test. Never push to `main`, never rebase, never force-push; merge `main` into the branch only on a real conflict. The PR is opened ready for review and is never merged by you.
- Keep the size cap of `tools/wasm-cap.toml` as it is on `main` when you start; update `measured` with the size you measure and report the change in bytes in the PR.
- Keep the section "Built state" of `docs/mzdraw.md` true (one bullet for this slice, what is built and what is not), and mark the rows of `docs/study/arch-web.md` §4 that you build as `B`.

## Work
1. Rust: the setting in the settings table; label placement in the scene (outside the ring, away from bonds, the free side of the centre); the source of the descriptors as decided above.
2. Tests: synthetic molecules with known descriptors (write them yourself: a lactic acid enantiomer drawn with a wedge, an E alkene, a Z alkene, a centre with an undefined wedge gets no label); label position after a move; e2e toggling the setting.

## Tests
- As in Work; parity only if the labels are in files.
- Keep the touch, pen and keyboard steps of `tests/e2e.mjs` passing, and add your own steps in the same style (one `step(...)` per behaviour).
- UI change: screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), desktop 1280×720 and tablet 768×1024, light and dark if the theme tokens exist on `main`; attach them to the PR.

## Acceptance (all green, on your branch, before the PR)
```
cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
tools/build_site.sh
cargo build --locked -p mzdraw --example replay && node tests/parity.mjs
node tests/e2e.mjs
```
- The PR body says: what was built, what was not, the module size before and after (bytes), the new ops or settings with one example each, the test counts.

## Messages
- Only to COO-MZDRAW, by `send_message`. A decision you need: one message with the options (two at most) and your recommendation first, then keep working on what does not depend on it.
- When the PR is open: one message with the PR link, the full head SHA, the size change, and the test lines (`cargo test` summary, parity count, e2e steps).

## Handoff at 200k tokens
When your context reaches about 200k tokens: start nothing new; bring the branch to green or revert the unfinished step; commit and push (never force); send COO-MZDRAW one message: branch, last commit SHA, done, not done, the next step, open questions. The coordinator starts a successor with this prompt plus your message.
