# flaky-tests: robust timing tests

Why: the CI jobs failed at random on tests that compare wall-clock times on a shared machine, and each failure cost a rerun. Nothing a test proves was loosened: the same claims (linear, not quadratic) are checked, with the same or a tighter ratio.

## What is built

- `crates/mzdraw/tests/support/timing.rs`: the clock of the scaling tests, included by `#[path]` (it is in no library and no binary; `mzdraw-wasm` and the unit test of `scene.rs` include the same file).
  - `Watch` / `best_of`: the CPU time of the running thread (`clock_gettime(CLOCK_THREAD_CPUTIME_ID)`, Linux 64-bit; the wall clock elsewhere, so the tests still run). The time a thread waits for a core on a busy machine, which landed on one of the two compared runs at random, is not counted.
  - `best_of(warm, runs, f)`: untimed warm-up runs, then the least of `runs`; noise only adds time.
  - `best_of_pair`: two workloads taken in turns, so a slow phase of the machine falls on both.
- Rust scaling tests moved to that clock (ratio limits kept; additive floors removed where the test had one for noise): `proposal.rs`, `steplabel.rs`, `values.rs`, `props.rs`, `text_runs.rs`, `pathway_gap.rs`, `tp.rs`, `tptable.rs`, `lens.rs`, `loss.rs`, `colour.rs`, `scene.rs` (unit), and `mzdraw-wasm/tests/values.rs`.
- Fixed here besides the clock:
  - `the_cost_per_arrow_does_not_grow_with_the_atoms`: the two runs are taken in turns (best of fifteen each) and the limit is now tighter: 999 arrows may cost at most 8% more than 250 over the same atoms (it was 25%). Its old limit was inside the run-to-run noise of the baseline (the same code, unchanged, failed it about one run in six on a quiet machine).
  - `the_cost_of_a_derive_grows_linearly_with_the_drawing` (`tp.rs`): the size compared was searched with a clock (the first size whose run took over 50 ms), so it moved with the speed of the machine, and above about 4000 structures the cost is not a pure scan (memory). The sizes are fixed at 2000 and 4000. Its fixed 0.02 s of request cost stays as it was.
- e2e (`tests/e2e.mjs`): every long press (`touch` and `pen`) waited a fixed 700 ms of the driver. The decision is Rust's, from the timestamps of the events, so the hold is now `hold(pg)`: it waits until the page's own clock (`performance.now()`, the clock of the timestamps) is 700 ms past the press. The atom-map-by-touch step tapped a field right after the palette was opened or scrolled, on a position read before the palettes laid themselves out again; it now waits for the section to be open and taps where the target has stopped moving (same box in two successive frames). No step was removed or weakened (touch, pen and keyboard stay). The two `waitForTimeout` that remain are not guesses about speed: one waits to show that nothing is written (an absence cannot be polled) and one is in `tools/check_pdf_viewer.mjs`.

## How each rewritten test was proven

Each was run against a planted slow mutation of the real code (made, run, and reverted with `git checkout`; none is in the PR):

- `grouping_and_diffing_scale_with_the_operations_and_the_ids`: a loop over all pairs of operations at the start of `diff::hunks`. The first case ("n operations in n hunks") failed with a ratio near the quadratic 16.
- `the_labels_of_a_thousand_arrows_scale_linearly_and_more_are_refused`: a scan of all atoms for each arrow at the start of `steplabel::plan`. It failed.
- `the_cost_per_arrow_does_not_grow_with_the_atoms`: a linear search of `doc.atoms` by id for the anchors of each arrow inside the loop of `plan`. One search per arrow failed most runs; two searches per arrow failed every run (four of four); the unmutated code passed every run.
- `the_glue_path_with_the_real_engine_scales_linearly_in_the_structures`: a loop over all pairs of structures (with a long inner spin) in `values::document_json`. It failed every run. This test stays a sanity check of the glue path (the real engine dominates its time; the scan mutations are caught by `crates/mzdraw/tests/values.rs`), as its comment says.

## Wall-clock tests of the repository and the verdict

- Fixed here (CPU clock, best of N, no additive floor or the old one stated above): the thirteen files listed above, plus `tp_steps.rs`, `digest.rs`, `stereo_guard.rs` (CPU clock; their own floors are as they were).
- Absolute bounds of seconds (a refusal is fast, a size is bounded): `cdx.rs`, `orient_limits.rs`, `tp_list.rs`, `tptable.rs`, `lens.rs`, `loss.rs`, `tp.rs`, `pathway_tools.rs`, `pathway.rs` (`two_hundred_nodes_in_time`: release only, now on the CPU clock), `proposal.rs` (500 ms bounds, now on the CPU clock). Left as bounds: they are two orders of magnitude over the real figure and a stall of a loaded machine cannot reach them.
- Not tests: `examples/stereo_bench.rs`, `tests/velocity.mjs` and `tools/velocity/*` (they print or guard figures with their own repeat logic; `velocity.mjs --check` is the guard of the page's drawing speed).
- Left as it is, with a reason: `pathway.rs` `p14_locality...` and `1000 nodes` only print their times and assert nothing about them.

## Not built

- No work counter in the library: counting work units would put test-only code in the editor core. The CPU clock plus interleaving gave a baseline spread of about 3% for the tightest test, which a mutation of one scan per arrow clears; if a future test needs more, the place for a counter is a `cfg(test)` unit test next to the code.
- A platform other than Linux 64-bit falls back to the wall clock in these tests.
