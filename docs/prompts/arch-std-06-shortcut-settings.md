# arch-std-06: one-key shortcuts that can be turned off and remapped

Proposed by the study `docs/study/arch-std.md` (feature rows 40; priority MUST). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-06 shortcut-settings: received, starting on branch `claude/arch-std-06-shortcut-settings`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. This answers WCAG 2.2 SC 2.1.4 Character Key Shortcuts (A).

## Scope
In:
- The section `shortcuts` of `mzdraw-settings/1` (already reserved in `docs/mzdraw-formats.md` §4): `{"shortcuts":{"single_keys":true,"map":{"<key chord>":"<command id>"}}}`. `single_keys: false` turns every shortcut without a modifier off (typing an element symbol on the focused atom stays, as it is the text entry of the focused component; say so in the docs). `map` overrides the default map of `docs/mzdraw.md`; a chord is written `Ctrl+Shift+K` style; a conflict (two commands on one chord) is refused on load with `error.draw2.shortcut` and the key, never resolved silently.
- One table of command ids in Rust (the commands the keys reach today: tools, elements, orders, charge, undo, redo, copy, ...), so the settings, the help and the page read one list; `settings::schema_json()` lists the commands.
- Confirm by e2e that one-key shortcuts act only while the canvas has focus (typing in the SMILES field, the text field or the label field changes nothing on the canvas).
- `Editor::set("shortcuts.single_keys=false")` and `shortcuts.map.<chord>=<command>` through the same per-key setter.

Out (do not do here):
- Shortcuts for commands that do not exist yet; a key-recording UI (the page shows the map and a text field per command is enough).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§2.6 SC 2.1.4).
- WCAG 2.2, Recommendation 2024-12-12, SC 2.1.4 (https://www.w3.org/TR/2024/REC-WCAG22-20241212/#character-key-shortcuts).
- `crates/mzdraw/src/settings.rs` (unknown keys kept, `FIELDS`, `Style::set`), `crates/mzdraw/src/editor_keys.rs`, `crates/mzdraw/src/editor_tools.rs` (`key`), `docs/mzdraw.md` keyboard map.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-06-shortcut-settings` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Command table and the dispatch from a chord to a command in Rust; the current `match` on keys becomes a lookup in the default map plus the user map.
2. Settings reading/writing with key order fixed, unknown keys of the section kept (as `Style::unknown`).
3. Page: a "Keys" part in the settings panel built from the schema; "Single-key shortcuts" checkbox.
4. Docs: `docs/mzdraw.md` keyboard map says the map is the default of `shortcuts.map`.

## Tests to add
- Unit: default map gives today's behaviour for every key of the help (one table-driven test); `single_keys: false` disables `c`, `1`, `w` but not `Ctrl+Z`; a remap of `Ctrl+Alt+N` to the nitrogen tool works; a conflict is refused; round trip of a settings file with the section gives the same bytes.
- Parity: a replay request with `"set":["shortcuts.single_keys=false"]` and a key log has the same output natively and in wasm (if key events are not in the replay language, test the setter output: `style()` bytes).
- e2e: typing `N` in the SMILES field does not change the canvas; with single keys off, `N` on the canvas does nothing and `Ctrl+Z` still undoes; touch and pen unaffected.

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
