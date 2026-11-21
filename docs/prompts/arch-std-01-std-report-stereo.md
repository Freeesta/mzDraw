# arch-std-01: conformance report `std/1`, stereo checks

Proposed by the study `docs/study/arch-std.md` (feature rows 11, 44; priority MUST). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-01 std-report-stereo: received, starting on branch `claude/arch-std-01-std-report-stereo`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. The report is read-only: it never changes the document and is no undo step.

## Scope
In:
- A new module `crates/mzdraw/src/conform.rs`: `fn report(doc: &Document) -> Report` and its JSON writer `std/1`: `{"format":"std/1","checks":[{"key","grade","atoms","bonds","objects"}]}`, checks ordered by key then by the smallest id, ids ascending, empty lists omitted, written by hand (no derived serde, no `f64` through `{}`: see the size notes in `docs/mzdraw.md`).
- The `std.stereo.*` checks of `docs/study/arch-std.md` §4: `wideEndCentre` (IUPAC ST-0.5), `notCentre`, `opposite` (marks within 10° of 180°, the IUPAC angle tolerance of ST-0.1), `sameKindThree`, `collinear` (170-190°), `wavyNotCentre`, `undefined`, `doubleCollinear`, `allene` (ST-1.6), `fischerCross` (four bonds within 10° of the axes at a stereocentre, GR-4.1.3), `reentrant` (a wedge inside a re-entrant ring angle, ST-1.2.3 / ST-1.4), `ringBond` (a wedge on a ring bond while an acyclic neighbour could carry it, ST-0.2), `hiddenH` (implicit H at a centre whose wide-end atom is itself a stereocentre, ST-1.2), `relLabel` (a text object reading `rac`, `rel`, `racemate` or `relative` next to a structure, ST-6.5). "Possible stereocentre" = the atoms of Table 8 of the InChI Technical Manual (C, Si, Ge, N+, P, P+, As, B-, Sn) with four different neighbours counting implicit H, decided on the `mol/1` record of `mzlab-mol` (use its stereo sites if it has them; do not write a second perception). `sameKindThree` uses the construction of the Technical Manual: equal bond lengths, wide end of a solid wedge 45° above the plane, of a hashed wedge 45° below, the centre outside the tetrahedron of its neighbours = undetermined.
- Replay: `"out":"std"` returns the report; `Editor::save("std")` is the one door.
- Page: a closed panel "Drawing checks" lists each check with its i18n text (`draw2.std.<key>` in `web/lang/mzdraw.js`) and the count; clicking a line selects its atoms and bonds (through the existing select command). Recomputed after each change, never blocking.

Out (do not do here):
- Geometry checks (slice arch-std-02), export checks (need the journal pages), CIP labels, any automatic fix.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§2.2 stereo, §4 checklist).
- IUPAC Recommendations 2006, Graphical representation of stereochemical configuration, HTML edition https://iupac.qmul.ac.uk/drawing/stereo.html (ST-0 to ST-6). Summarise, do not copy.
- InChI Technical Manual, software 1.07, revision 2024-01-19, §III.d "Stereochemical layer" (https://github.com/IUPAC-InChI/InChI, `INCHI-1-DOC/TechMan/InChI_TechMan.pdf`): the two wedge conventions, the 45° construction, the atom list. Summarise, do not copy.
- `crates/mzdraw/src/doc.rs` (wedge codes `WEDGE_UP` 1, `WEDGE_DOWN` 2, `WEDGE_WAVY` 3, narrow end at atom `a`), `crates/mzdraw/src/editor.rs` (`save`), `crates/mzdraw/src/replay.rs`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-01-std-report-stereo` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Write `conform.rs` with one function per check and a table `CHECKS: [(key, grade)]` so that the page, the docs and the tests read one list.
2. Wire `save("std")` and the replay `out`; add the report to the page state the bridge already returns (no decision in JS: the bridge hands over keys and ids).
3. Panel in `web/index.html` + `web/mzdraw.js` (paint and forward only), i18n texts.
4. Add a section "Drawing checks" to `docs/mzdraw.md` with the keys, grades and thresholds.

## Tests to add
- `crates/mzdraw/tests/conform.rs`: per check, one synthetic drawing that triggers it and one close drawing that does not (for example a wedge from C2 of butan-2-ol drawn correctly, the same wedge reversed onto a CH2, two wedges at 180°, (2R,3R)-butane-2,3-diol with one wedge joining both centres, E-but-2-ene with a methyl drawn collinear, penta-2,3-diene with one and with two opposite wedges).
- Determinism: the same document gives the same bytes; a property test that the report of a random document never panics and lists only ids that exist.
- Parity: add a request with `"out":"std"` to `tests/data/mzdraw_log.json` (or its sibling list) so `tests/parity.mjs` compares wasm and native bytes.
- e2e (`tests/e2e.mjs`): draw a wedge onto a CH2 with the mouse, see one line in the panel, activate it with the keyboard (Enter) and by touch, and check the atoms are selected.

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
