# flaky-tests-3: the accepts timing test without a subtracted baseline

Why: `two_thousand_separate_accepts_through_the_request_are_fast_and_all_or_none` (rewritten in flaky-tests-2) failed in about one rerun in three on an unmodified `main`. It timed the whole request (import of a 20,000-atom drawing, then the hunks) and took off the time of the same request with one hunk: `least[2] - base` against `least[1] - base`. The import is large and noisy in a debug build, so a small noise on `base` moved the ratio of two differences (unmutated about 5.4, limit 8). Tests only; no production code changed.

## What changed (`crates/mzdraw/tests/proposal.rs`)

- The accepts test now times `Editor::accept_hunks` alone, with `best_with`: the drawing of 20,000 atoms and the held proposal are made by `setup`, outside the clock, for every run; only the accepting (the check on a copy and the accept of each hunk, which is what the request does after `propose`) is on the thread CPU clock. There is no baseline to subtract. 500 against 2,000 hunks, at most 8 times as much (linear 4, quadratic 16); each size is the least of nine runs after one that is not timed (was: four rounds of a whole request).
- The request of 2,000 hunks still runs once through `replay`, and every functional assertion (all-or-none, the key `error.draw2.proposalHunk`, the `failed` report) is as before.
- The move test (`one_move_of_all_atoms...`): the limit stays 8 for each of held, accepted and undone; the small size now has a floor of 1 ms (`a.max(1.0)`), which only matters for a phase under a millisecond (a fast machine or a release build; the debug figures are 3 ms and more, so it was not in use in the 30 runs below).

## Unmutated ratio, 30 runs, this machine (debug build, one test thread)

| ratio (4 times the items, limit 8) | min | median | max |
|---|---|---|---|
| accepts, 2,000 against 500 hunks | 4.27 | 4.56 | 4.79 |
| move held, 20,000 against 5,000 atoms | 3.06 | 4.59 | 5.26 |
| move accepted | 2.79 | 4.42 | 5.11 |
| move undone | 2.55 | 4.20 | 4.81 |

All 30 runs passed. The smallest phase of the move test took 3.0 ms.

## Mutation proof (planted, run, reverted with `git checkout`; none is in the PR)

- A scan of all hunks 200 times for each accepted hunk, in `Editor::accept_hunk`: the test fails (ratio 10.9: 705 ms for 500 hunks, 7708 ms for 2,000).
- The same scan 40 times does NOT fail it (ratio 7.3), as with the old test (about 7): a light quadratic cost in the hunks is below the limit of 8. The unmutated maximum is now 4.8, so the margin to the limit is wide, but the limit was not lowered here.

## Not caught by a ratio

As in flaky-tests-2: a large cost per accept that does not grow with the hunks (linear in the hunks).

build_site ok, under the cap.
