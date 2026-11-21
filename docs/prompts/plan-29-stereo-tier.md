# plan-29: the stereo tier of each centre, never assumed

Proposed by `docs/plan/backlog.md` §10 (row 78; fourth external report, "stereo tier ledger", reduced to what arch-std-12 and -19 do not cover). Size S–M. Wave 5. Depends on: arch-std-12 (`stg`), arch-std-19 with edit E20 (§10.5), plan-22 (loss manifest), plan-23 (digest).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-29-stereo-tier: received, starting on branch `claude/plan-29-stereo-tier`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only. arch-std-12 stores the enhanced-stereo group of a centre (`stg`: `abs`, `and<n>`, `or<n>`). Two things stay implicit: a wedged centre with no group, and the V2000 chiral flag, which changes the meaning of every wedge in a file without being drawn. IUPAC 2006 ST-6 says the chiral-flag behaviour is not followed (`docs/study/arch-std.md` §2.2), so mzDraw never assumes a tier: it notes what the file said.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- The tier of a stereocentre is derived and never stored separately: `none` (no stereo mark), `absolute` (`stg` `abs`), `relative` (`or<n>`), `racemic` (`and<n>`), `undefined` (wedged but no `stg`).
- Readers never set `stg` from a default (D19). A V2000 chiral flag (0 or 1) and a V3000 wedged centre outside every collection leave the centre `undefined`. The import notes (rule 13: "opened with ... listed") list those centres, the flag value, and what CTfile 2026 says it means (read, §10.1: flag on = absolute for all wedged centres; flag off = the drawn isomer and its enantiomer). The person or agent then sets the group with `stereo.group` (arch-std-12). E20 makes arch-std-19's reader follow this rule.
- A `std/1` check `std.stereo.tierUndefined` (info): a wedged centre with no group, with the import note's rule if there was one.
- The digest of plan-23 gains a new per-centre key `centres`: `[[atom id, tier], ...]`, ascending ids. The per-bond `stereo` entries are unchanged.
- An `expect` arm `{"stereo_tier":{"ID":"absolute",...}}` (plan-20).
- Writing V2000 adds no rule of its own: arch-std-19's write rule stands (chiral flag 1 only when every centre is `abs`; otherwise the groups are listed as lost), and plan-22's existing `enhanced-stereo` loss item reports it. No new loss kind.

Out (do not do here):
- Drawing the tier (arch-std-12's `stereo_labels`).
- New fields or ops (the tier is derived from `stg`).
- Any change to arch-std-19's write rule.

## CLI twin (rule 15)
- `{"import":V2000,"out":"std"}`, `{"out":"digest"}`, `{"expect":[{"stereo_tier":{...}}]}`. Native replay tests with the page's bytes.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md`, `docs/plan/backlog.md` §10.
- `docs/study/arch-std.md` §2.2 (IUPAC ST-6), arch-std-12 and -19 prompts, `io.rs` readers, `digest.rs`, `conform.rs`, `loss.rs`.

## Agreement needed before the PR
No `draw_edit/1` op or field. Rows in the `request` section of plan-01's table: the `expect` arm `stereo_tier` and the digest key `centres` (D13, D24). The `std/1` goldens with wedged centres and no group move: bless them and list them.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-29-stereo-tier` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Tier function.
2. Reader notes (with E20 in arch-std-19 if it has merged; otherwise in this slice's reader arm), `std.stereo.tierUndefined`.
3. Digest key `centres`, `expect` arm, doc section.

## Tests to add
- Each tier from a synthetic V3000 file with all three collections.
- A V2000 file with flag 0 and one with flag 1: both leave every wedged centre `undefined`, and the note names the flag and its CTfile meaning.
- A V3000 centre with wedges outside every collection is `undefined` and noted.
- Round trip V3000 → document → V3000 keeps every group (property over random groups).
- **Fails without the change**: the check key does not exist before the slice.

## Independent proof
Correctness must not rest on our own claims. For synthetic files with each collection kind, the InChI stereo layer flags given by RDKit's InChI in the tool are compared with our tier. **Unverified**: what InChI's `/s1`, `/s2` and `/s3` mean (`/s3` and the "absolute" flag are not in `docs/study`; the InChI FAQ answered 404 on 2026-11-21). The DEV reads the InChI technical manual (the version RDKit bundles) and writes down what it says. If it cannot be read, the comparison is marked "not verified" in the PR. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
If arch-std-12 and -19 (with E20) already leave no centre `undefined` without a note and already list the flag, ship only the digest key `centres` and the `expect` arm, and drop the check (say so in the PR).

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
