# arch-std-03: element palettes with tested contrast, dark background

Proposed by the study `docs/study/arch-std.md` (feature rows 33; priority MUST). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-03 palette-contrast: received, starting on branch `claude/arch-std-03-palette-contrast`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. 

## Scope
In:
- The colours of `scene.rs` (`colour::N`, `O`, `S`, `HALOGEN`, `P`, ink, select, hover, warn, preview, focus) become per-element colours of a palette chosen by the style: new optional keys `palette` (`light` default, `dark`, `mono`; `mono` = the built `colour: mono`, keep reading `colour`) and `background` (a `#RRGGBB`, default from the palette) in `mzdraw-settings/1`, through `settings::FIELDS` and `Style::set`.
- The palettes of `docs/study/arch-std.md` §2.7 (light on #FFFFFF, dark on #121212), with one colour per element for C, H, N, O, S, P, F, Cl, Br, I, B, Si and one for other metals and one for other non-metals; the canvas and the SVG use the same colours (the display list carries colour indices; the page maps index to colour from a table the module gives, never its own).
- A preset `mzlab-dark` (= `mzlab` with `palette: dark`).
- A unit test that computes the WCAG 2.2 contrast ratio (relative luminance of sRGB, `(L1 + 0.05) / (L2 + 0.05)`) of every label colour against its background (≥ 4.5) and of bonds, selection, hover, warning and focus colours (≥ 3.0).

Out (do not do here):
- Highlight and evidence colours, a colour picker, per-atom colours in files.

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state is what exists), `docs/study/arch-std.md` (§2.6 (1.4.3, 1.4.11), §2.7 (tables and the reason one palette cannot serve both grounds), §3).
- WCAG 2.2, W3C Recommendation 2024-12-12 (https://www.w3.org/TR/2024/REC-WCAG22-20241212/), SC 1.4.3 and 1.4.11 and the definition of relative luminance.
- `crates/mzdraw/src/settings.rs`, `crates/mzdraw/src/scene.rs` (`mod colour`), `crates/mzdraw/src/svg.rs`, `web/mzdraw.js` (paint).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-03-palette-contrast` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). Size: no `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard). The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Palette table in Rust (one `const` table per palette), lookup by element symbol; `Metrics` carries the palette.
2. Settings keys and the preset; the schema `settings::schema_json()` lists the new keys and values; the page's "Your own values" fields come from the schema as today.
3. Page: the canvas background follows the style; the page CSS keeps its own tokens but the canvas colours come only from the module.
4. Docs: the palette table in `docs/mzdraw.md`.

## Tests to add
- The contrast test above for both palettes; a test that `mono` gives one ink for all.
- Settings round trip: a file with `palette` and `background` reads and writes the same bytes; an unknown palette name is refused with `error.draw2.styleValue`.
- Golden SVGs: the default look may change colour values only; bless and list them.
- Parity: one request with `"style":"mzlab-dark"`. e2e: switch to the dark preset with the keyboard and by touch, take a screenshot at deviceScaleFactor 2 of a structure with N, O, S, Cl, Br in both palettes.

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
