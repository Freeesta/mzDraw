# flaky-tests: robust timing tests

Why: the CI jobs failed at random on tests that compare wall-clock times on a shared machine, and each failure cost a rerun. Nothing a test proves was loosened: the same claims (linear, not quadratic) are checked, with the same or a tighter ratio.

## What is built

- `crates/mzdraw/tests/support/timing.rs`: the clock of the scaling tests, included by `#[path]` (it is in no library and no binary; `mzdraw-wasm` and the unit test of `scene.rs` include the same file).
  - `Watch` / `best_of`: the CPU time of the running thread (`clock_gettime(CLOCK_THREAD_CPUTIME_ID)`, Linux 64-bit; the wall clock elsewhere, so the tests still run). The time a thread waits for a core on a busy machine, which landed on one of the two compared runs at random, is not counted.
  - `best_of(warm, runs, f)`: untimed warm-up runs, then the least of `runs`; noise only adds time.
  - `best_of_pair`: two workloads taken in turns, so a slow phase of the machine falls on both.
- Rust scaling tests moved to that clock (ratio limits kept; the additive floors that were there for noise removed, see the verdict list below): `proposal.rs`, `steplabel.rs`, `values.rs`, `props.rs`, `text_runs.rs`, `pathway_gap.rs`, `tp.rs`, `tptable.rs`, `lens.rs`, `loss.rs`, `colour.rs`, `scene.rs` (unit), and `mzdraw-wasm/tests/values.rs`.
- Fixed here besides the clock:
  - `the_cost_per_arrow_does_not_grow_with_the_atoms`: the two runs are taken in turns (best of fifteen each) and the limit is now tighter: 999 arrows may cost at most 8% more than 250 over the same atoms (it was 25%). Its old limit was inside the run-to-run noise of the baseline (the same code, unchanged, failed it about one run in six on a quiet machine).
  - `the_cost_of_a_derive_grows_linearly_with_the_drawing` (`tp.rs`): the size compared was searched with a clock (the first size whose run took over 50 ms), so it moved with the speed of the machine. The sizes are now fixed at 1000 and 4000 structures and the bound is a pure ratio of 10 for four times the structures (a linear cost is 4, a quadratic one 16; the additive 0.02 s is gone). The measured ratio is a little over linear and moves from run to run (about 4 to 8, in debug and in release), so the old formulation (twice the size, at most three times) sat on the noise of the code itself.
- e2e (`tests/e2e.mjs`): every long press (`touch` and `pen`) waited a fixed 700 ms of the driver. The decision is Rust's, from the timestamps of the events, so the hold is now `hold(pg)`: it waits until the page's own clock (`performance.now()`, the clock of the timestamps) is 700 ms past the press. The atom-map-by-touch step tapped a field right after the palette was opened or scrolled, on a position read before the palettes laid themselves out again; it now waits for the section to be open and taps where the target has stopped moving (same box in two successive frames). No step was removed or weakened (touch, pen and keyboard stay). The two `waitForTimeout` that remain are not guesses about speed: one waits to show that nothing is written (an absence cannot be polled) and one is in `tools/check_pdf_viewer.mjs`.

## Mutation proof: what was run and what was not

A planted slow mutation of the real code was made, the test run, and the code reverted with `git checkout` (none is in the PR). Proven this way, and only these:

- `grouping_and_diffing_scale_with_the_operations_and_the_ids`: only its first case ("n operations in n hunks"), with a loop over all pairs of operations at the start of `diff::hunks`; it failed with a ratio near 16. The other four cases of that test and the two cases of `marking_and_saying_a_proposal_scales` were not mutated.
- `the_labels_of_a_thousand_arrows_scale_linearly_and_more_are_refused`: a scan of all atoms for each arrow at the start of `steplabel::plan`; it failed.
- `the_cost_per_arrow_does_not_grow_with_the_atoms`: one linear search of `doc.atoms` by id per arrow failed most runs (two of three); two searches per arrow failed every run (four of four); unmutated code passed every run.
- `the_glue_path_with_the_real_engine_scales_linearly_in_the_structures`: a loop over all pairs of structures with a long inner spin in `values::document_json`; it failed every run. A lighter version of the same mutation (a shorter spin) did not fail it: this test stays a sanity check of the glue path, as its comment says; the scan mutations are caught by `crates/mzdraw/tests/values.rs`.
- `the_cost_of_a_derive_grows_linearly_with_the_drawing`: a loop over all pairs of atoms, ten times over, at the start of `tp::derive`; it failed in release and in debug (ratio above 14).

Not mutation-proven (moved to the CPU clock only, ratio limits unchanged, and they passed): the tests of `values.rs`, `props.rs`, `text_runs.rs`, `pathway_gap.rs`, `tptable.rs`, `lens.rs`, `loss.rs`, `colour.rs`, the unit test of `scene.rs`, `tp_steps.rs`, `digest.rs` and `stereo_guard.rs`, and the other cases of `proposal.rs`. For these the change is the clock; what they prove is what they proved before.

Known residual risk: the limit of `the_cost_per_arrow_does_not_grow_with_the_atoms` is 8% and the spread of the unmutated code was about 3% in the runs made here; on a machine with a noisier thread clock (heavy SMT contention, a virtual machine with a coarse CPU clock) that margin could fail it. If it does, the first thing to look at is the spread of the two runs, not the limit.

## Wall-clock tests of the repository and the verdict

- Scaling tests (a ratio between two sizes), now on the CPU clock: `proposal.rs`, `steplabel.rs`, `values.rs`, `props.rs`, `text_runs.rs`, `pathway_gap.rs`, `tp.rs`, `tptable.rs`, `lens.rs`, `loss.rs`, `colour.rs`, `scene.rs` (unit), `mzdraw-wasm/tests/values.rs`, `tp_steps.rs`, `digest.rs`, `stereo_guard.rs`.
  - Additive floor removed: `proposal.rs`, `steplabel.rs`, `tp.rs`, `tptable.rs`, `lens.rs`, `scene.rs`.
  - Guard against a zero small time kept (`small.max(1)` on integer microseconds, in `values.rs`, `props.rs` and the wasm `values.rs`).
  - Floor or guard kept as it was: `tp_steps.rs` (0.03 s and 0.02 s), `digest.rs` (0.02 s), `stereo_guard.rs` (1 ms), `loss.rs` (the ratio is checked only when the small run took over 0.02 s). Only their clock changed.
- Absolute bounds of seconds (no ratio: a refusal is fast, a size is bounded), left as bounds: `cdx.rs`, `orient_limits.rs`, `tp_list.rs`, `pathway_tools.rs`, and the bounds inside `tptable.rs`, `lens.rs`, `loss.rs` and `tp.rs` that are not ratios. They are about two orders of magnitude over the real time, so a stall of a loaded machine cannot reach them. Two absolute bounds that were tight were moved to the CPU clock: `proposal.rs` (500 ms) and `pathway.rs` (`two_hundred_nodes_in_time`, release only).
- Not tests: `examples/stereo_bench.rs`, `tests/velocity.mjs` and `tools/velocity/*` (they print or guard figures with their own repeat logic; `velocity.mjs --check` is the guard of the page's drawing speed).
- Left as it is: `pathway.rs` `p14_locality...` only prints its times and asserts nothing about them.

## Not built

- No work counter in the library: counting work units would put test-only code in the editor core. The CPU clock plus interleaving gave a baseline spread of about 3% for the tightest test, which a mutation of one scan per arrow clears; if a future test needs more, the place for a counter is a `cfg(test)` unit test next to the code.
- A platform other than Linux 64-bit falls back to the wall clock in these tests.
