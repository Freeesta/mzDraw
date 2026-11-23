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

The median 4.46 is 4 times a factor 1.11 (the log factor of the maps and the cache); it is far from 6, so there is no code defect to fix and the limit stays 8 (quadratic gives 16). The small run is not dominated by a fixed cost: it takes about 55 ms in a debug build. The outliers above 5 (max 5.5) and the 3.1 ones (the small run itself was slow) are single-run noise on one side or the other, which is what the old method of three runs per size did not take out.

## What changed

- `best_pair_with` (new, next to `best_with`): the small and the large run are taken in turns, so a slow phase falls on both; two runs of each that are not timed, then the least of nine runs of each (was: warm-up 1, least of three, small and large one after the other). Thread CPU clock (`tests/support/timing.rs`) as before, setup outside the clock, what `run` returns dropped after the clock stopped.
- The small size has a floor of 1 ms against a tiny denominator. It is not in use in a debug build (the smallest small run of any test here is about 12 ms), but it can bind on a very fast release build. The verdict stays sound there: the floor only raises the small time, so it can only make the test more lenient for a run that is already below what the clock tells from noise, and quadratic work at the large size is still many times 8 x 1 ms (a quadratic mutation of the 4x size takes seconds in a release build, as the comments of the tests say).
- The limit is the same, 8 for 4 times the atoms. No absolute bound. Every functional assertion of the test is as before.

## Distribution of the ratio, the old method against the new, 60 runs each of the test body

| method | idle: min / median / max | 6 busy loops on 4 cores: min / median / max |
|---|---|---|
| old (least of 3, one size after the other) | 4.26 / 4.47 / 4.73 | 3.45 / 4.47 / 5.14 |
| new (least of 9, in turns) | 4.35 / 4.46 / 4.68 | 4.32 / 4.48 / 4.67 |

The new spread under load is as narrow as idle (0.35 wide against 1.7 for the old method), and its worst case stays well under 8. The CI figure 8.2 was not reproduced even by the old method under load here; the CI machine is noisier than this one, which is why the margin is taken by the method and not by the limit.

## Mutation proof (planted, run, reverted with `git checkout`; not in the PR)

A scan of all atoms for each atom in `Editor::propose` (quadratic in the atoms): the test fails, "224.7 ms for 5000 atoms, 3129.8 ms for 20000" (ratio 13.9; quadratic alone is 16). Reverted.

## The `scales` helper (the tests that failed on CI)

CI has failed on two of the scaling tests and passed on a rerun: `grouping_and_diffing_scale_with_the_operations_and_the_ids` (83.4 ms for 20,000 against 720.6 ms for 80,000, PR #98) and `the_cost_per_arrow_does_not_grow_with_the_atoms` (steplabel; PR #96 and PR #99). The steplabel test was already taken in turns with `best_of_pair`, fifteen runs. The first one, and the other tests that call the shared `scales` helper of `proposal.rs` (7 cases in 2 tests), timed the small and the large input one after the other, the least of seven each. Now `scales` times them in turns with `timing::best_of_pair`: two runs of each that are not timed, then the least of nine runs of each on the thread CPU clock, a floor of 1 ms for the small size, every limit as it was (8 for 4 times the input). `best` (least of seven, one size after the other) had no other caller and is removed.

Ratio of each case (4 times the input, limit 8), 40 runs each, min / median / max, debug build, one test thread. Idle, then with 6 busy loops on 4 cores:

| case | idle, old | idle, new | loaded, old | loaded, new |
|---|---|---|---|---|
| n operations in n hunks | 3.54 / 4.66 / 4.86 | 4.41 / 4.56 / 4.70 | 4.31 / 4.59 / 4.79 | 4.42 / 4.58 / 4.70 |
| n moves of n displacements | 3.98 / 4.52 / 4.73 | 4.37 / 4.51 / 4.67 | 4.24 / 4.45 / 5.02 | 4.36 / 4.51 / 4.73 |
| diff of n moved atoms | 4.23 / 4.60 / 4.97 | 4.20 / 4.42 / 4.56 | 4.09 / 4.58 / 4.88 | 4.34 / 4.49 / 4.77 |
| one move and one remove over n ids | 4.44 / 4.61 / 4.78 | 4.55 / 4.62 / 4.78 | 4.42 / 4.61 / 4.83 | 4.56 / 4.64 / 4.80 |
| a move of n atoms, applied and undone | 3.06 / 4.52 / 4.84 | 4.08 / 4.29 / 4.41 | 4.08 / 4.54 / 5.02 | 4.09 / 4.30 / 4.50 |
| marks and words of n hunks | 3.87 / 4.15 / 4.32 | 4.02 / 4.14 / 4.21 | 3.94 / 4.14 / 4.41 | 4.03 / 4.17 / 4.21 |
| marks and words of one hunk over n ids | 4.35 / 4.47 / 4.76 | 4.44 / 4.51 / 4.60 | 4.31 / 4.51 / 4.73 | 4.43 / 4.53 / 4.76 |

The code of these paths is linear (medians 4.1 to 4.7, nowhere near 6): no code defect, production code is not touched. The new method narrows the spread (the low outliers of the old one, down to 3.06, are a slow small run; the high ones up to 5.02 a slow large run) and the new maximum is 4.80.

Mutation proof (a scan of the atoms planted in the code path, a fraction of a full quadratic scan so that a run stays short; each run, then reverted with `git checkout`; none is in the PR): every case fails its test.

| mutation | case | message |
|---|---|---|
| `diff::hunks`, n operations without a move | n operations in n hunks | 199.1 ms for 20,000, 2697.7 ms for 80,000 |
| `diff::hunks`, n moves | n moves of n displacements | 213.4 ms, 2790.0 ms |
| `diff::diff` | diff of n moved atoms | 215.4 ms, 2884.7 ms |
| `diff::hunks`, one move and one remove | one move and one remove over n ids | 278.1 ms, 3220.2 ms |
| `Session::run` with a move | a move of n atoms, applied and undone | 42.7 ms for 10,000, 688.5 ms for 40,000 |
| `proposal_marks`, a scan of the atoms per hunk | marks and words of n hunks | 126.9 ms for 400, 1482.2 ms for 1,600 |
| `proposal_marks`, a scan of the atoms | marks and words of one hunk over n ids | 254.6 ms for 5,000, 3135.9 ms for 20,000 |

(A first mutation of the marks of n hunks that scanned a twentieth of the atoms was too small to count at that size, 1,600 atoms, and did not fail the test; the scan per hunk above does. A cost that does not grow with the size is not caught by any ratio.)

What is not shown: the two CI failures were never reproduced locally, neither before nor after, with up to 40 runs idle and loaded; the hardening follows the cause they share with the one fixed above (a slow phase on one of the two sizes), not a reproduced run. The other ratio tests that call neither `scales` nor `best_of_pair` (`digest`, `loss`, `lens`, `pathway_gap`, `colour`, `values` of mzdraw-wasm, `timing_guard`) had no failure in 6 loaded runs each, have no CI failure on record, and are not touched.

## Not built

A reproduction of the two CI failures of the `scales` tests; a ratio measured for the scaling tests outside `proposal.rs` and `steplabel.rs` (the stress above is pass or fail); a hunk-count mutation lighter than quadratic (a light quadratic cost, as in flaky-tests-3, stays below limit 8); catching a large per-call cost that does not grow with the size.
