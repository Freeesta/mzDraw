# arch-std-21: texts with subscripts, superscripts and several lines

Proposed by the study `docs/study/arch-std.md` (feature rows 23; priority MUST). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-21 text-runs: received, starting on branch `claude/arch-std-21-text-runs`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Coordination: this slice adds to the log language `draw_edit/1` (`runs` on `obj.add` and `obj.set`); COO-MZDRAW asks COO 16 before queueing it, because `mzlab-mol` (the CLI route of mzLab) must accept the same language. Do not start before the coordinator confirms.

## Scope
In:
- A text object (and the text of an arrow) holds runs: `{"runs":[{"t":"H"},{"t":"2","sub":true},{"t":"O"}]}` with `sub`, `sup`, `b` (bold), `i` (italic) flags, and line breaks as runs `{"br":true}`. A plain one-line text stays readable as today (`text` field), so old documents open unchanged.
- Typing: in the text field, `_x` or `_{..}` makes a subscript and `^x` or `^{..}` a superscript (a small parser in Rust, so the page and the replay give the same runs); digits after an element symbol in a chemical formula become subscripts on request (a button "Formula").
- Drawn in the canvas and the SVG (subscripts at the style's `subscript_scale`, `tspan` with a baseline shift in the SVG); written to KET (its text runs carry sub/superscript, bold, italic) and CDXML (style runs), read back from both.
- The operation `obj.set` takes `runs`; its inverse stores the old runs.

Out (do not do here):
- Fonts other than the style's, colours of runs, rich-text paste from other programs.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§5 row 23; §2.4 KET row (text runs)).
- KET schema `textPart`, `textFont` (https://raw.githubusercontent.com/epam/ketcher/master/packages/ketcher-core/src/domain/serializers/ket/schema.json); the CDXML `t` and `s` objects (CDX SDK pages, https://chemapps.stolaf.edu/iupac/cdx/sdk/).
- `crates/mzdraw/src/doc.rs` (objects), `crates/mzdraw/src/ops.rs` (`obj.add`, `obj.set`), `crates/mzdraw/src/scene.rs`, `crates/mzdraw/src/ket.rs`, `crates/mzdraw/src/cdxml.rs`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-21-text-runs` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Document field `runs` with validation (bounded count and length), writer order, the old `text` kept as the one-run case.
2. The markup parser and its inverse (runs to markup for editing).
3. Drawing, then KET and CDXML in and out.

## Tests to add
- Unit: `H_2O`, `CH_3CO_2H`, `^{13}C`, `Pd(OAc)_2, 80 °C` give the expected runs and back; two lines; undo of `obj.set` gives the identical document; KET and CDXML round trips keep the runs.
- Golden SVG with sub/superscripts. Parity: a log with `obj.set` runs. e2e: type a text with subscripts by keyboard, place it by mouse and by touch, screenshot at deviceScaleFactor 2.

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
- Ask the coordinator only when blocked by a decision that is not yours (a cap raise, a scope question); otherwise decide and say why in the PR.
- When the PR is open: one message with the PR link, the full head SHA, the module size before and after (bytes), the test counts, and one line per acceptance command (pass/fail).

## Handoff at 200k tokens
At about 200k tokens of context, stop starting new work: commit what builds with a message that says what is left, push the branch (no force), and send the coordinator a handoff message: branch, last commit SHA, what is done, what is left as a numbered list, the exact next command to run, and any decision taken that the next chat must keep.
