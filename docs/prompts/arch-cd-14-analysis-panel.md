# arch-cd-14: Analysis panel: formula, MW, exact mass, m/z, elemental analysis of the selection (superseded: do not run)

Study rows: F01 of `docs/study/arch-cd.md`.

**Run arch-web-17 (values) instead.** It covers the same slice; this desktop-editor study adds the points below. COO-MZDRAW: paste them into the prompt you send, or into the PR review of that slice. A DEV chat that receives this file by mistake sends COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line, `arch-cd-14 is superseded by arch-web-17 (values)`, and stops.

## Additions from the desktop editors

- `mzlab-mol` has no mass table: the numbers come from `mzlab-feat` (`selection/1` with feature `sel`, `label/1`) in the bridge; `crates/mzdraw` cannot compute them. The native replay of `crates/mzdraw` therefore cannot give them: add a native test in `crates/mzdraw-wasm` (an rlib) that compares the panel JSON with `sel_json` on the same molfile, and say that `tests/parity.mjs` does not cover it.
- Copy the values as text (ChemDraw pastes Analysis values as captions, CD-UG17).
