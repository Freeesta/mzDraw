# flaky-tests-2: no absolute time bound in the proposal tests

Why: `one_remove_naming_every_atom_of_a_big_drawing_is_held_marked_and_accepted` failed on loaded CI runners ("21290 ms for one remove of 20000 atoms", "20218 ms against a 20000 ms limit"). It was already on the thread CPU clock (PR #102), so the CPU clock alone is not enough: on a loaded runner the CPU time of a thread is inflated too. A bound in milliseconds is a guess about the machine; a ratio between two sizes of the same run is not. Nothing a test proves was loosened: each one still fails on a planted quadratic cost (below).

## What is built (tests only; no production code changed)

`crates/mzdraw/tests/proposal.rs`: the helper `best_with(warm, runs, setup, run)` (least CPU time of the thread over `runs` runs after `warm` untimed ones; each run gets a fresh input made outside the clock). The functional assertions (held, marked, accepted, all-or-none, undo returns the same bytes, the keys of the refusals) are the same as before.

- `one_remove_naming_every_atom_of_a_big_drawing_is_held_marked_and_accepted`: the result is checked once on 20,000 atoms (held as one hunk, `scene()` and the words made, accepted, no atom left). The cost is `propose` + `proposal_marks` + `proposal_json` + `accept_hunk`, timed for 5,000 and for 20,000 atoms: at most 8 times as much for 4 times the atoms (linear 4, quadratic 16).
- `one_move_of_all_atoms_is_one_hunk_and_the_ids_are_capped`: held, accepted and undone are timed apart for 5,000 and 20,000 atoms, each at most 8 times as much; the refusal of more ids than `MAX_IDS` must cost no more than holding a move of the 20,000 atoms of the drawing (the work it saves).
- `two_thousand_separate_accepts_through_the_request_are_fast_and_all_or_none`: the request of 2,000 hunks runs once; the cost of the hunks is `(request of n hunks) - (the same request with one hunk)` over the same 20,000-atom drawing, for 500 and 2,000 hunks, at most 8 times as much; the three requests are taken in turns, the least of four rounds each. The all-or-none checks are untouched.
- `a_patch_that_does_not_apply_or_is_too_big_is_refused_and_nothing_is_held`: refusing `MAX_OPS + 1` operations must cost no more than holding `MAX_OPS` of them.
- `stereo_guard.rs`: `a_geometry_step_on_a_big_wedge_free_drawing_is_cheap` (it was 5 ms absolute, now 5,000 against 20,000 atoms, at most 8 times as much) and `a_signature_of_a_100_atom_piece_with_10_wedges_is_fast` (the wall clock moved to the CPU clock, least of twenty; its bound of 100 ms is still absolute, see below).
- `crates/mzdraw/tests/timing_guard.rs`: scans `crates/*/tests/*.rs`. (1) No line may contain `as_millis()` (how a figure in milliseconds was made to compare). (2) A test file that reads `Instant::now` (the wall clock) must be in the list `WALL_CLOCK` of that file, each with its reason, and a listed file that no longer does must be taken off. Limit: it cannot tell a scaling claim from a size bound, and it does not see an assertion built another way (`as_secs_f64() < 5.0` on `Watch`); a reviewer still reads a new timing test. The guard was shown to fail by planting `as_millis()` and `Instant::now` in another test file.

## A finding, not fixed here (production code, outside this slice)

`Editor::scene()` of a plain chain of 20,000 atoms with no proposal at all is not linear in a debug build: about 0.8 s for 5,000, 2.6 s for 10,000, 10.7 s for 20,000 atoms. The marks of the proposal are not the cause (they take 18 ms at 20,000). So the old 20,000 ms bound was mostly the cost of drawing the chain, which is why it sat near the limit on a quiet machine and over it on a loaded one. The remove test therefore calls `scene()` for its result and leaves it out of the clock; the proposal's own share of the scene (`proposal_marks`) and its words are timed. A queue row asks for the cause of the superlinear scene (or captions) to be found. The import of a 20,000-atom document by `replay` is superlinear in the same way (about 0.14 s for 5,000, 1.1 s for 20,000 atoms), which is why the accepts test takes that off.

## Mutation proof: what was run and what was not

A planted slow mutation of the real code, the test run, the code restored with `git checkout` (none is in the PR):

- remove test and move test (held): a scan of all atoms for every id at the end of `propose`: both failed (ratio about 13 and 14).
- accepts test: a scan of all hunks for each accepted hunk in `accept_hunk`, 200 times: failed (ratio about 11.5). The same scan 40 times did NOT fail it (the ratio was about 7 against the limit 8): the test sees a quadratic cost in the hunks that is heavy, not a light one. The unmutated ratio is about 5.4 (a little over linear 4) in two runs.
- refusal of too many operations: a loop over the operations 20 times for each operation before the refusal: failed (7.5 s against 18 ms).
- refusal of too many ids: the touched ids summed 3,000 times before the refusal: failed (1.2 s against 0.38 s). 40 times did NOT fail it.
- `a_geometry_step_on_a_big_wedge_free_drawing_is_cheap`: a scan of a 400th of the atoms for each bond at the start of `stereo_guard::before`: failed (ratio about 12). Unmutated it is about 3.9.

Not mutation-proven: the accepted and undone phases of the move test (only the held phase was mutated); `a_signature_of_a_100_atom_piece_with_10_wedges_is_fast` (moved to the CPU clock only).

Not caught by a ratio, by its nature: a cost per accept that does not grow with the hunks but is large (a clone of the whole drawing for each accept): it is linear in the hunks. That needs a work counter (not built, as in `flaky-tests.md`).

## Wall-clock and absolute bounds that remain (and why)

- Absolute bounds in whole seconds, left as in `flaky-tests.md`: `cdx.rs`, `orient_limits.rs`, `tp_list.rs`, `pathway_tools.rs`, `loss.rs`, `lens.rs`, `tp.rs`, `tptable.rs`. They are listed with their reasons in `timing_guard.rs`. Not rewritten here: some are close to their real time in a debug build (the `pathway_tools.rs` bounds of 20 s and 10 s, the `lens.rs` and `tp.rs` bounds of 5 s), so they may need the same treatment if one of them fails on CI; the evidence so far is only for the proposal tests.
- `pathway.rs` `two_hundred_nodes_in_time` (release only, 100 ms on the CPU clock) and the 100 ms bound of the signature test of `stereo_guard.rs` are absolute figures on the CPU clock, not rewritten.
