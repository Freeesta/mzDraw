# plan-29: the stereo tier of each centre, never defaulted silently

Proposed by `docs/plan/backlog.md` §10 (row 78; fourth external report, "stereo tier ledger", reduced to what arch-std-12 does not cover). Size S. Wave 5. Depends on: arch-std-12 (`stg`), arch-std-19 (V3000 / KET collections), plan-22 (loss manifest), plan-23 (digest).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-29-stereo-tier: received, starting on branch `claude/plan-29-stereo-tier`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only. arch-std-12 stores the enhanced-stereo group of a centre (`stg`: `abs`, `and<n>`, `or<n>`). What stays implicit is a wedged centre with no group, and the V2000 chiral flag, which decides the meaning of every wedge in a file without being drawn.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- The tier of a stereocentre, derived, never stored separately: `none` (no stereo mark), `absolute` (`stg` `abs`), `relative` (`or<n>`: one of two enantiomers, unknown which), `racemic` (`and<n>`: both), `undefined` (wedged but no `stg`).
- Readers say what they assumed: when a reader sets `stg` from a default (V2000 chiral flag; V3000 centres with wedges outside every collection, where the counts-line flag applies), the import notes (rule 13, "opened with ... listed") list the centres and the rule used. CTfile 2026 (read): flag on = absolute for all wedged centres; flag off = a mixture of the drawn isomer and its enantiomer; collections take precedence over the flag.
- A `std/1` check `std.stereo.tierUndefined` (info): a wedged centre with no group.
- The tier per centre in the digest (`stereo` entries of plan-23 gain the tier) and an `expect` arm `{"stereo_tier":{"ID":"absolute",...}}` (plan-20).
- Writers: the V2000 chiral flag is computed from the tiers (all `absolute` → 1; all `racemic` in one group → 0); any other mix cannot be said in V2000 and is a `loss/1` item of plan-22 (`what`: `stereo-tier`).

Out (do not do here):
- Drawing the tier (arch-std-12's `stereo_labels`).
- New fields or ops (the tier is derived from `stg`).

## CLI twin (rule 15)
- `{"import":V2000,"out":"std"}`, `{"out":"digest"}`, `{"expect":[{"stereo_tier":{...}}]}`, `{"out":"loss:molfile"}`. Native replay tests with the page's bytes.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md`, `docs/plan/backlog.md` §10.
- `docs/study/arch-std.md` §2.2 (IUPAC ST-6), arch-std-12 and -19 prompts, `io.rs` readers and writers, `loss.rs`, `digest.rs`, `conform.rs`.

## Agreement needed before the PR
No `draw_edit/1` op or field. Rows in the `request` section of plan-01's table: the `expect` arm `stereo_tier` and the digest's tier key (D13). The `std/1` goldens with wedged centres and no group move: bless them and list them.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-29-stereo-tier` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Tier function.
2. Reader notes, `std.stereo.tierUndefined`.
3. Digest key, `expect` arm, V2000 flag and loss item.

## Tests to add
- Each tier from a synthetic V3000 file with all three collections; a V2000 file with flag 0 and flag 1 gives `racemic` and `absolute`, with the import note naming the rule.
- A document with mixed tiers written as V2000 lists a `stereo-tier` loss; as V3000 it lists none.
- Round trip V3000 → document → V3000 keeps every tier (property over random groups).
- **Fails without the change**: the check key does not exist before the slice.

## Independent proof
Correctness must not rest on our own claims. For synthetic files with each collection kind, the InChI stereo flags (absolute vs relative or racemic, through RDKit's InChI in the tool, reading V3000 enhanced stereo where the version does) agree with our tier; where the InChI version cannot tell, say so. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
If arch-std-12 and -19 already assign and report every default (no centre is left `undefined` after any reader, and the import notes already list the rule), ship only the digest key and the V2000 loss item and drop the check (say so in the PR).

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
- Ask the coordinator only when blocked by a decision that is not yours (a cap raise, a new op, a scope question); otherwise decide and say why in the PR.
- When the PR is open: one message with the PR link, the full head SHA, the module size before and after (bytes), the test counts, the independent-proof result, and one line per acceptance command (pass/fail).

## Handoff at about 150k tokens
At about 150k tokens of context, stop starting new work: commit what builds, push the WIP branch (no force) with a one-line state note at the top of the PR description or in the last commit message ("state: done X; next Y"), and send the coordinator: branch, last commit SHA, what is done, what is left as a numbered list, the exact next command to run, and any decision taken that the next chat must keep.
