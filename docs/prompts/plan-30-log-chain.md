# plan-30: a hash chain over the command log

Proposed by `docs/plan/backlog.md` §10 (row 79; fourth external report, "provenance ledger", part 1). Size S. Wave 3. Depends on: plan-20 (the `expect` arm). Its twin plan-31 (the head in the saved pictures) follows in wave 4.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-30-log-chain: received, starting on branch `claude/plan-30-log-chain`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only. Byte parity already makes a log reproducible; the chain makes it checkable: a short hex string says which starting document and which log made a drawing, and anyone can recompute it.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- **SHA-256 first step: use what is linked.** `mzlab-mol` at the pin already depends on `sha2` 0.11 (its `Cargo.toml`; `codec.rs` uses it for `Mol::core_hash`). But it exposes only the hash of a record's core JSON, which ignores coordinates and ids, not a hash of bytes. Check the pin again when you start. If `mzlab-mol` then exposes a bytes hash, use it. Otherwise take ONE of these, in this order, and state the reason in the PR (rule 3):
  1. add `sha2` (the same version, already in `Cargo.lock`, so no new code in the module) as a direct dependency of `crates/mzdraw`, and add it to the dependency guard of `docs/mzdraw.md` ("Leaving the repo"). The guard is the owner's rule, so ask the coordinator before you change it (D25);
  2. ask the coordinator to ask COO 16 for `mzlab_mol::sha256(&[u8])` in a pin-move PR of its own (rule 9), and wait.
  Never write a second SHA-256 in mzDraw. Measure the module before and after: the expected cost is 0 B, since the code is already linked; any growth is reported. `mzlab-mol` is a direct dependency of `mzdraw`, so no `glue.rs` arm is needed (glue is for `mzlab-feat`).
- What the chain binds:
  - `h0` = SHA-256 of `mzdraw-chain/1` + a newline + the `mzdraw/1` bytes of the document after `import`, `style` and `set` (the empty document's bytes when there is no import), so the chain binds the starting document too;
  - `h(i)` = SHA-256 of `h(i-1)` (32 bytes) followed by the bytes of op `i` as `Op::write` gives them;
  - the head is `h(n)`, in lower-case hex.
- `"out":"chain"` writes `chain/1` (`{"format":"chain/1","alg":"sha256","ops":n,"start":h0,"head":h(n)}`).
- Checking is an `expect` arm, with no new field and no new stage (D20): `{"chain":"HEX"}` is true when the head equals HEX. A false arm is the usual plan-20 verdict (exit 1).

Out (do not do here):
- The head in saved pictures (plan-31).
- Signatures or keys (a hash chain proves integrity, not authorship).
- Recording the head in mzLab's `prov/1` (COO 16's decision; ask once through the coordinator).

## CLI twin (rule 15)
- `{"import":...,"log":[...],"out":"chain"}`; `{"log":[...],"expect":[{"chain":"HEX"}]}`. Native replay test with the page's bytes for the parity cases.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Leaving the repo", the dependency guard), `docs/plan/backlog.md` §10.
- `ops.rs` (`Op::write`), `replay.rs`, `outcome.rs` (plan-20); `mzlab-mol` `Cargo.toml` and `codec.rs` at the pin.

## Agreement needed before the PR
No `draw_edit/1` op. Rows in the `request` section of plan-01's table: the output `chain` and the `expect` arm `chain` (D13). A direct `sha2` dependency needs the coordinator's yes first (D25).

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
1. The SHA-256 decision (above), with the measured module size.
2. Chain, `out:chain`, `expect` arm.
3. Doc section (the definition of `h0` and `h(i)`, so another program can recompute it).

## Tests to add
- SHA-256 of the NIST example messages for FIPS 180-4 ("abc", the 448-bit message, one million `a`; read the values at csrc.nist.gov yourself) equals the published digests.
- The head changes when any op changes, and when the starting document changes; equal requests give equal heads; an empty log gives `h0`.
- The `expect` arm is true and false; parity cases; native replay test.
- **Fails without the change**: `chain` is an unknown output before the slice.

## Independent proof
Correctness must not rest on our own claims. A script `tools/chain/check.py` (ours, committed) recomputes `start` and `head` of every parity case from the request file with Python's `hashlib`, outside Rust. Its output (the digests) is committed as `tests/data/chain/expected.json` and checked by a Rust test without Python. No third-party program is committed. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
If the head differs between the page, the CLI and the native replay for any parity case, stop and report. If the module grows by more than 10 kB, also stop and report: the code should already be linked.

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
