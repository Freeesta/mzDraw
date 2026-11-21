# plan-30: a hash chain over the command log, its head in the saved picture

Proposed by `docs/plan/backlog.md` §10 (row 79; fourth external report, "provenance ledger"). Size S. Wave 3. Depends on: plan-20 (the verdict of `verify`).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-30-log-chain: received, starting on branch `claude/plan-30-log-chain`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only. Byte parity already makes a log reproducible; the chain makes it checkable: a picture says which log made it, and anyone can recompute that from the log.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- SHA-256 implemented in `crates/mzdraw/src/sha256.rs` (FIPS 180-4; about 120 lines; no new crate: the dependency guard of `docs/mzdraw.md` ("Leaving the repo") allows only `mzlab-mol`, `libm`, `serde`, `serde_json`). mzLab uses SHA-256 for its `prov/1` record (`docs/schemi/prov-1.json` at the pin), so the same algorithm.
- The chain: `h0` = SHA-256 of the empty string; `h(i)` = SHA-256 of `h(i-1)` (32 bytes) followed by the bytes of op `i` as `Op::write` gives them. The head is `h(n)` in lower-case hex.
- `"out":"chain"` writes `chain/1` (`{"format":"chain/1","alg":"sha256","ops":n,"head"}`); request field `"verify":"HEX"`: exit 0 when the log's head equals it, else exit 1 verdict (`failed.stage` `verify`).
- The SVG and PNG the editor saves carry the head of the log that made them (`metadata` element id `mzdraw-chain`; PNG `iTXt` keyword next to the document's); the SVG goldens move (bless and list them).

Out (do not do here):
- Signatures or keys (a hash chain proves integrity, not authorship).
- Storing the log in the picture (the document is already there; the log stays its own file).

## CLI twin (rule 15)
- `{"log":[...],"out":"chain"}`; `{"log":[...],"verify":"HEX"}`. Native replay test: the page's saved SVG carries the same head as the native replay.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` (slice 3b, pictures that open again; "Leaving the repo"), `docs/plan/backlog.md` §10.
- `svg.rs` (`carrying`), `png.rs`, `ops.rs` (`Op::write`), mzLab `docs/schemi/prov-1.json` at the pin.

## Agreement needed before the PR
No `draw_edit/1` op. Rows in the `request` section of plan-01's table: output `chain`, field `verify`, stage `verify` (D13). Ask the coordinator once whether COO 16 wants the head in mzLab's `prov/1`; that is mzLab's change, not this slice's.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-30-log-chain` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. SHA-256 with test vectors.
2. Chain, `out`, `verify`.
3. Head in SVG and PNG, goldens, doc section.

## Tests to add
- SHA-256 of the NIST example messages for FIPS 180-4 ("abc", the 448-bit message, one million `a`; read the values at csrc.nist.gov yourself) equal the published digests; and of 1,000 random byte strings equal Python `hashlib` in a `tools/` check (committed: the digests, not the program).
- The head changes when any op changes; equal logs give equal heads; the empty log gives `h0`.
- Page vs native: the head in the saved SVG equals `out:chain` of the same log (parity case).
- **Fails without the change**: `chain` is an unknown output before the slice.

## Independent proof
Correctness must not rest on our own claims. SHA-256 is checked against the FIPS 180-4 examples and Python's `hashlib`; the head of each parity case is recomputed by a 10-line Python script from the log file, outside Rust. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
If the head differs between the page, the CLI and the native replay for any parity case, or the module grows by more than 10 kB, stop and report. If COO 16 says `prov/1` will not record it and no other consumer is named, keep `out:chain` and `verify` and drop the head from the pictures (say so in the PR).

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
