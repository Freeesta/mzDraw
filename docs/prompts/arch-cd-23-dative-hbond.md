# arch-cd-23: Dative (coordinate) and hydrogen bonds (superseded: do not run)

Study rows: A05, A06 of `docs/study/arch-cd.md`.

**Run arch-web-20 (bond-kinds) instead.** It covers the same slice; this desktop-editor study adds the points below. COO-MZDRAW: paste them into the prompt you send, or into the PR review of that slice. A DEV chat that receives this file by mistake sends COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line, `arch-cd-23 is superseded by arch-web-20 (bond-kinds)`, and stops.

## Additions from the desktop editors

- V2000 molfiles have no code for dative or hydrogen bonds: write V3000 automatically or list them as lost, and say which in the PR.
- Check that `check/1` does not count a dative bond as a covalent one on the donor (follow `mzlab-mol`'s handling of V3000 type 9; a gap goes to mzLab).
