# flaky-tests-4: the one-remove scaling test, small and large timed in turns

Why: `one_remove_naming_every_atom_of_a_big_drawing_is_held_marked_and_accepted` (crates/mzdraw/tests/proposal.rs) failed once in CI on PR #121 with "72.2 ms for 5000 atoms, 591.4 ms for 20000" (ratio 8.2, limit 8) and passed on a rerun. It took the least of three runs of the small size, then the least of three runs of the large size, one after the other. A slow phase of the CI machine (a neighbour, a frequency step) fell on all three large runs and on none of the small ones. Tests only; no production code changed.

## Is the code linear? (measured first)

The code is linear up to a log factor, not super-linear in a way that matters: the held proposal marks and words build `BTreeMap`s over the atoms and the bonds, and `propose` clones the document to check the patch. Debug build, this machine, one test thread, thread CPU clock. Ratio of the whole body (propose, marks, words, accept) for 4 times the atoms, 40 runs, one run each of the sizes:

| ratio, 20,000 against 5,000 atoms (limit 8) | min | median | max |
|---|---|---|---|
| whole body | 3.11 | 4.46 | 5.49 |
| propose alone | 3.14 | 4.51 | 5.60 |
| marks | 3.03 | 4.21 | 5.87 |
| words (`proposal_json`) | 2.90 | 4.45 | 6.32 |
| accept | 2.84 | 4.07 | 5.61 |

The median 4.46 is 4 times a factor 1.11 (the log factor of the maps and the cache); it is far from 6, so there is no code defect to fix and the limit stays 8 (quadratic gives 16). The small run is not dominated by a fixed cost: it takes about 55 ms (the 1 ms floor is never in use). The outliers above 5 (max 5.5) and the 3.1 ones (the small run itself was slow) are single-run noise on one side or the other, which is what the old method of three runs per size did not take out.

## What changed

- `best_pair_with` (new, next to `best_with`): the small and the large run are taken in turns, so a slow phase falls on both; two runs of each that are not timed, then the least of nine runs of each (was: warm-up 1, least of three, small and large one after the other). Thread CPU clock (`tests/support/timing.rs`) as before, setup outside the clock, what `run` returns dropped after the clock stopped.
- The small size has a floor of 1 ms against a tiny denominator (not in use here, as in flaky-tests-3).
- The limit is the same, 8 for 4 times the atoms. No absolute bound. Every functional assertion of the test is as before.

## Distribution of the ratio, the old method against the new, 60 runs each of the test body

| method | idle: min / median / max | 6 busy loops on 4 cores: min / median / max |
|---|---|---|
| old (least of 3, one size after the other) | 4.26 / 4.47 / 4.73 | 3.45 / 4.47 / 5.14 |
| new (least of 9, in turns) | 4.35 / 4.46 / 4.68 | 4.32 / 4.48 / 4.67 |

The new spread under load is as narrow as idle (0.35 wide against 1.7 for the old method), and its worst case stays well under 8. The CI figure 8.2 was not reproduced even by the old method under load here; the CI machine is noisier than this one, which is why the margin is taken by the method and not by the limit.

## Mutation proof (planted, run, reverted with `git checkout`; not in the PR)

A scan of all atoms for each atom in `Editor::propose` (quadratic in the atoms): the test fails, "224.7 ms for 5000 atoms, 3129.8 ms for 20000" (ratio 13.9; quadratic alone is 16). Reverted.

## The other ratio tests (scan)

Measured: `digest`, `loss`, `lens`, `pathway_gap`, `colour`, `values` (mzdraw-wasm) and `timing_guard`, each 6 runs with 6 busy loops on 4 cores, one test thread: no failure and no ratio message. Not shown to be at risk, so not touched.

Same pattern as the test fixed here (least of seven, small and large one after the other, limit 8) but not measured one by one: the `scales` tests of `proposal.rs` (`grouping_and_diffing_scale...`, `marking_and_saying_a_proposal_scales` and the other callers of `scales`). They passed in every run of the 20 acceptance runs and the loaded runs here, so they are not shown to be at risk; one row for them is in "Follow-ups without a prompt" of `docs/plan/queue.md`: if one fails in CI, give `scales` the same `best_pair_with` treatment.

## Not built

A ratio measured per test for every scaling test of the repository (the stress above is pass or fail); a hunk-count mutation lighter than quadratic (a light quadratic cost, as in flaky-tests-3, stays below limit 8); catching a large per-call cost that does not grow with the size.
