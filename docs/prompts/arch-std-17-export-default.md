# arch-std-17: downloads in every format, SVG on white by default

Proposed by the study `docs/study/arch-std.md` (feature rows 50, 51; priority MUST). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-17 export-default: received, starting on branch `claude/arch-std-17-export-default`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Owner requirement: the default download is the canvas image on a white background (study §2.9).

## Scope
In:
- The saved SVG gets a background `<rect>` the size of its view box, filled with the style's background (white by default; the key `background` of arch-std-03 if merged, else a new optional key `background` with default `#FFFFFF` and the value `none` for a transparent file). The PNG is drawn on the same ground (it is white today: make it come from the module, not from the page).
- Labels in the SVG are placed with an explicit `x` (start-anchored) instead of `text-anchor="end"` or `middle`, so Office places them right (study §2.9); font list `Arial, Helvetica, sans-serif` (built).
- One "Download" button whose default is SVG, with a menu of every format `Editor::save` writes (SVG, PNG, molfile V2000/V3000, SDF, SMILES, KET, CDXML, CML, RXN, `mzdraw/1`); the last choice is remembered by the page (a per-viewer convenience).
- Copy puts on the clipboard what the module returns for it: `image/png` and `text/plain` (molfile) always, `image/svg+xml` when the browser supports it (`ClipboardItem.supports`), the web custom formats in Chromium (built). The list of payloads and their order is decided in Rust; JS only checks `supports` and hands them over.

Out (do not do here):
- PDF (arch-std-18), EMF, journal sizes (arch-std-16).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§2.9 (evidence, measured sizes, recommendation)).
- Microsoft "Edit SVG images in Microsoft 365" (https://support.microsoft.com/en-us/office/edit-svg-images-in-microsoft-365-69f29d39-194a-4072-8c35-dbe5e7ea528c); Edge blog on SVG in the Async Clipboard API, 2024-07-11 (https://blogs.windows.com/msedgedev/2024/07/11/seamless-svg-copy-paste-on-the-web/); W3C Clipboard API and events, WD 2026-06-24 (https://www.w3.org/TR/clipboard-apis/).
- `crates/mzdraw/src/svg.rs`, `crates/mzdraw/src/png.rs`, `crates/mzdraw/src/editor.rs` (`save`), `web/mzdraw.js` (clipboard and downloads), `docs/mzdraw-formats.md` §2.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-17-export-default` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Background rect and start-anchored text in `svg.rs` (one code path; the display list for the canvas stays as it is).
2. A function that lists the clipboard payloads for a copy (`[(mime, text or bytes)]`), used by the bridge.
3. Download menu in `web/index.html` (real buttons, keyboard reachable); the page asks the module for the bytes and names the file.

## Tests to add
- Unit: the SVG starts with the white rect, no `text-anchor="end"` remains, the carried document reopens to the same bytes; `background=none` gives no rect.
- Golden SVGs move (the rect, the anchors): bless and list. Parity: unchanged in kind, new goldens compared.
- e2e: Download with the keyboard and by touch gives an SVG with the rect; the menu gives each format; Copy writes `image/png` (and `image/svg+xml` in Chromium); screenshot at deviceScaleFactor 2 of the menu.

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
