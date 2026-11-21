# Prompt queue status (queue-status), built (docs and a shell check only; no product code)

`docs/plan/queue.md` is the one place that says what state each prompt of `docs/prompts/` is in: MERGED (PR number, short commit, and evidence in the code or in a `docs/built/` note), RUNNING, QUEUED, SUPERSEDED (and by what), WITHDRAWN, GATED (and the gate). A second table, "Follow-ups without a prompt", lists work named in `docs/plan/backlog.md` and in the "Not built" lines of `docs/built/*.md` that has no prompt file yet.

`tools/queue_check.sh [ROOT]` (plain bash) fails if a file of `docs/prompts/` has no row in `queue.md`, if a row names a file that does not exist, or if a prompt has two rows. It is a step of the `test` job of `.github/workflows/ci.yml`. `tools/test_queue_check.sh` runs the three failure modes on a copy of `docs/`, like `tools/test_rename_product.sh` does for the rename tool.

The status column of the table in section 4 of `docs/plan/backlog.md` is removed (it said RUN for things long merged); a line points to `queue.md`.

Not built: the check does not read the status words or verify the PR numbers (a row's evidence is written by the PR that changes the state); it does not ask GitHub which PRs are open, so RUNNING is kept by hand. `docs/built/still-to-build-slice-2.md` is partly stale (CDXML, RXN and CML are built) and is not touched here.
