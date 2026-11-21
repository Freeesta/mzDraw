# arch-std-19: enhanced stereo groups in molfile V3000 and KET

Proposed by the study `docs/study/arch-std.md` (feature rows 13; priority SHOULD). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-19 enhanced-stereo-files: received, starting on branch `claude/arch-std-19-enhanced-stereo-files`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Second of three slices; arch-std-12 must be merged before you start, otherwise wait. Coordination: COO-MZDRAW asks COO 16 before queueing this prompt: the V3000 reading and writing of the groups goes through `mzlab-mol`, which must agree.

## Scope
In:
- Read the V3000 collections `MDLV30/STEABS`, `MDLV30/STERELn`, `MDLV30/STERACn` into the `stg` field of arch-std-12 (through what `mzlab-mol` already parses: it reads these groups for its record; do not write a second V3000 parser) and write them back; V2000 has only the chiral flag: written as absolute when every centre is `abs`, else the groups are listed as lost on write.
- KET: the enhanced-stereo flags of the schema on atoms, read and written.
- No group is ever assumed on read (D19, edit E20 of `docs/plan/backlog.md` §10.5). A V2000 chiral flag (0 or 1) and a V3000 wedged centre outside every collection leave the centre without a group (`undefined`). The import notes list those centres, the flag value and what CTfile 2026 says it means (flag on = absolute at every wedged centre; flag off = the drawn isomer and its enantiomer). The write rule above is unchanged.
- The import note "enhanced-stereo groups not kept" disappears for what is now kept; anything still lost is listed.

Out (do not do here):
- CXSMILES (slice 20), any new operation.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§2.2, §2.4 molfile and KET rows).
- The CTfile V3000 collection blocks: BIOVIA CTfile Formats, Chemistry 2026, public copy at https://www.3ds.com/assets/invest/2026-01/biovia_chemistry_2026_ctfile_formats.pdf (read by ARCH PLAN-2 on 2026-11-21: STEABS, STERACn, STERELn and the chiral flag; see `docs/plan/backlog.md` §10.1). Implement from what `mzlab-mol` reads and writes, and apply edit E20 (§10.5: no group is set from a default).
- KET schema (https://raw.githubusercontent.com/epam/ketcher/master/packages/ketcher-core/src/domain/serializers/ket/schema.json).
- `crates/mzdraw/src/io.rs`, `crates/mzdraw/src/ket.rs`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-19-enhanced-stereo-files` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- Rule 15 of `CLAUDE.md`, in full: 15. CLI-native and agent-first. mzDraw is born usable through the CLI (`mzlab-cli mol draw`) by people and above all by AI agents, to draw canvases of every kind as fast, as safely and as reproducibly as possible. Every feature is first a Rust command with a machine-readable form (`draw_edit/1` JSON ops, `--json` with stable keys, distinct exit codes, data on stdout and diagnostics on stderr, a `--dry-run` / validate mode, batch input and stdin streaming, deterministic bytes, no browser in the path); the page is one client of that same command set. A feature without a CLI route and a test that replays it natively is incomplete.
- From the CLI rules of the prompts: ids are stable, every operation reports the ids it created and removed, a batch of operations is atomic (all or nothing).
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. V3000 in and out, V2000 chiral flag, KET flags.
2. `docs/mzdraw-formats.md` §1a rows.

## Tests to add
- Synthetic V3000 and KET files written by hand with abs, and1 and or1 groups: read, write, read gives the same document.
- Parity: a replay with `"out":"molfile-v3000"` and `"out":"ket"` on a document with groups. e2e: open a V3000 file with groups (keyboard and touch), the labels show.

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
