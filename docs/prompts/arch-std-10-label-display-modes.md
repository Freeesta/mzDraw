# arch-std-10: carbon labels, hydrogen modes and the hash style

Proposed by the study `docs/study/arch-std.md` (feature rows 6, 10; priority SHOULD). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-10 label-display-modes: received, starting on branch `claude/arch-std-10-label-display-modes`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. 

## Scope
In:
- New optional style keys in `mzdraw-settings/1` (through `settings::FIELDS` or a sibling table for enumerations, read by `Style::set`): `carbon_labels` = `none` (default, today) | `terminal` (CH3, CH2 at chain ends) | `all`; `hydrogens` = `hetero` (default, today: H on heteroatoms) | `all` (also on every labelled carbon); there is no `none`, because a label without H means an atom without H (IUPAC GR-2.1.1); `hash_style` = `wedge` (default) | `lines` (parallel lines of equal length).
- The canvas and the SVG follow them through one code path; the presets keep their look (`acs-1996` stays as it is).
- IUPAC 2008 limits the choices: carbon is always labelled when isolated or between collinear bonds (GR-2.1.2) whatever `carbon_labels` says; `lines` hashing keeps 2-4 bond widths between hashes and at least 3 hashes per bond (GR-1.3), and ST-0.3 allows it only towards terminal atoms: the stereo report of arch-std-01 warns otherwise.

Out (do not do here):
- Explicit hydrogen atoms as atoms (the record decides), aromatic circles.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§2.1 labels, §2.2 hashed marks, §3 new keys).
- `crates/mzdraw/src/settings.rs`, `crates/mzdraw/src/scene.rs`, `crates/mzdraw/tests/settings.rs`, `crates/mzdraw/tests/labels.rs`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-10-label-display-modes` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Settings keys with bounds, schema output, setter; unknown values refused with `error.draw2.styleValue`.
2. Rendering: a terminal carbon gets a label box so bonds stop short of it (reuse the label code); `lines` hashing draws equal-length segments perpendicular to the bond.
3. Page fields from the schema as today.

## Tests to add
- Unit per key and value on a synthetic set (propan-1-ol, acetic acid, a wedge and a hash at one centre).
- Settings round trip bytes; golden SVGs unchanged for the default.
- Parity with `"set":["style.carbon_labels=terminal","style.hash_style=lines"]`. e2e: switch the keys from the page by keyboard and touch; screenshots at deviceScaleFactor 2.

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
