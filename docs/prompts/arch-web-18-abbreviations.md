# arch-web-18: Abbreviations: expand and contract in place, user abbreviations

Prompt for one DEV chat of mzDraw (`Freeesta/mzDraw`), proposed by the study `docs/study/arch-web.md` (§7, slice 18).

## Confirm message
Your first message, to COO-MZDRAW (the coordinator; its session id is in your launch message; use `send_message` with that id), one line:
`DEV-ABBREVIATIONS: received; branch claude/arch-web-18-abbreviations from origin/main; reading CLAUDE.md, docs/mzdraw.md, docs/study/arch-web.md.`

## Role
You are DEV-ABBREVIATIONS, a development chat. You build one small slice, open one PR ready for review, and report. The coordinator merges with the full head SHA; you never merge.

## Scope
- In: ops `abbr.expand` (a label becomes its atoms, laid out at the attachment, the label kept on the group so it can be contracted) and `abbr.contract` (a group drawn as atoms folds back to its label), both with exact inverses; the `abbreviations` section of the settings file (name, SMILES with one attachment `*`, optional display text) read through the settings table; user names next to the built-in 30.
- Out: S-groups of other kinds, abbreviations with two attachments (later).
- Anything else you notice: one line to COO-MZDRAW, not in this PR.

## Context (read first)
- `CLAUDE.md` (rules 1-15), `docs/mzdraw.md` (the section "Built state" is what exists; do not rebuild it), `docs/study/arch-web.md` (the rows of §4 named in §7 for slice 18, and §5 for the API).
- `crates/mzdraw/src/label.rs` holds 30 abbreviations, expanded only in the record and the molfile with placeholder positions (`docs/mzdraw.md` Slice 2b).
- Ketcher has functional groups and salts as contractable abbreviations (expand, contract, remove abbreviation); CDK contracts only when more atoms are inside than outside (`docs/mzdraw-competitors.md`).

## Rules
- English only, everywhere (code, comments, texts, commits, PR).
- All logic in Rust (`crates/mzdraw`, bridge `crates/mzdraw-wasm`); JavaScript only forwards pointer, wheel and key events, calls the module and paints the display list. No chemistry and no decision in JS.
- No bundler, no framework, no new crate or npm package unless the PR states the reason.
- Synthetic data only in tests and examples (molecules you write yourself as SMILES or by hand).
- Every new edit operation has an exact inverse and is added to the random sequence of the property test (`random_step` in `crates/mzdraw/tests/ops.rs`: apply then undo gives the identical document bytes, redo the identical result), and to the log reader and writer tests of the same file.
- CLI first (CLAUDE.md rule 15): a feature without a CLI route and a test that replays it natively is incomplete. Every page function has its scriptable twin: a `draw_edit/1` op, a `section.key=value` setting, or a field of the replay request; the native replay (`cargo run -p mzdraw --example replay`) gives the same bytes as the page (`tests/parity.mjs`); add a case to `tests/data/mzdraw_log.json` and bless it only after a wanted change (`MZDRAW_BLESS=1 cargo test -p mzdraw --test replay`).
- Never skip, disable or loosen a test. Never push to `main`, never rebase, never force-push; merge `main` into the branch only on a real conflict. The PR is opened ready for review and is never merged by you.
- Keep the size cap of `tools/wasm-cap.toml` as it is on `main` when you start; update `measured` with the size you measure and report the change in bytes in the PR.
- Keep the section "Built state" of `docs/mzdraw.md` true (one bullet for this slice, what is built and what is not), and mark the rows of `docs/study/arch-web.md` §4 that you build as `B`.

## Work
1. Rust: the group record on the expanded atoms (which atoms came from which label), the two ops, positions from `Mol::layout_2d_fixed` with the attachment fixed (or a fallback placement, said in the PR), the settings section with validation keys.
2. Tests: unit tests (Boc expand and contract back to the identical document; user abbreviation from the settings file; a refused SMILES without `*`), property test, parity case with a settings file, e2e from the context menu or a key.

## Tests
- As in Work.
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
