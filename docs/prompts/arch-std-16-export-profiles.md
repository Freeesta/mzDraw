# arch-std-16: export profiles of the journals that could be read

Proposed by the study `docs/study/arch-std.md` (feature rows 32; priority SHOULD). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-16 export-profiles: received, starting on branch `claude/arch-std-16-export-profiles`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. The numbers come only from pages read for the study (§2.3); a publisher whose page could not be read gets no profile.

## Scope
In:
- A table of export profiles in Rust: `acs-joc` (one column ≤ 240 pt, two columns 300-504 pt, depth ≤ 660 pt, text ≥ 4.5 pt, lines ≥ 0.5 pt, fonts Arial/Helvetica), `springer` (84 / 174 mm, depth 234 mm; small journals 119 × 195 mm; text 8-12 pt; lines ≥ 0.3 pt; Helvetica or Arial), `elsevier` (30 / 90 / 140 / 190 mm, text ≥ 7 pt and sub/superscripts ≥ 6 pt, lines 0.10-1.5 pt, fonts Arial/Helvetica, Courier, Symbol, Times), `nature` (89 / 120-136 / 183 mm, depth ≤ 247 mm, lines 0.25-1 pt, sans-serif text 5-7 pt), `wiley` (80 / 180 mm). Each row cites its URL and date in a comment.
- A check `std.export.*` (in the `std/1` report of arch-std-01, `"out":"std"` with `"set":["export.profile=elsevier","export.width=single"]`): the drawing scaled to the width class puts text, sub/superscripts or lines below the minimum, exceeds the depth, or uses a font outside the list. The report gives the scale and the smallest text and line at print size.
- The SVG and PNG can be written at the size of a width class (`export.width`), so the file arrives at its print size.

Out (do not do here):
- RSC and Wiley-journal (Angewandte, Chemistry Europe) profiles: their pages were not readable; PDF output (arch-std-18).

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state is what exists), `docs/study/arch-std.md` (§2.3 table and rules).
- ACS JOC author guidelines PDF, last updated 2026-10-08 (https://researcher-resources.acs.org/publish/author_guidelines/pdf?coden=joceah), Appendix 2; Elsevier artwork sizing and types pages (https://www.elsevier.com/about/policies-and-standards/author/artwork-and-media-instructions/artwork-sizing); Nature final and initial submission pages; Wiley figure guide (March 2025). Re-read each page before pinning its numbers and note the date.
- `crates/mzdraw/src/settings.rs`, `crates/mzdraw/src/svg.rs`, `crates/mzdraw/src/png.rs`, `crates/mzdraw/src/conform.rs`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-16-export-profiles` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). Size: no `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard). The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Profiles table and the setting section `export` (`profile`, `width`) through the settings schema.
2. The check, computed from the style (font size, line width in pt) and the drawing's extent.
3. Page: a "For a journal" select next to Save; the report lines show under it.

## Tests to add
- Unit per profile: a small structure at `acs-1996` passes `acs-joc` single column; the same at `slides` scaled into 90 mm fails `elsevier` on text size; a wide scheme fails the depth or width.
- Parity with `"set":["export.profile=nature","export.width=double"]` and `"out":"std"`. e2e: choose a profile with the keyboard and by touch, read the report line.

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
