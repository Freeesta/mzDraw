# arch-cd-02: CIP labels on the canvas (R/S, r/s, E/Z) from cip/1, live (superseded: do not run)

Study rows: C02 of `docs/study/arch-cd.md`.

**Run arch-web-16 (cip) instead.** It covers the same slice; this desktop-editor study adds the points below. COO-MZDRAW: paste them into the prompt you send, or into the PR review of that slice. A DEV chat that receives this file by mistake sends COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line, `arch-cd-02 is superseded by arch-web-16 (cip)`, and stops.

## Additions from the desktop editors

- `cip/1` lives in `mzlab-feat`, not in `mzlab-mol`; `crates/mzdraw` may not depend on it. If the descriptors cannot come from the record in `crates/mzdraw`, let the bridge compute them (as `check/1`) and let the replay request carry them (`"cip": [[atom id, "R"], [[a, b], "E"]]`) so the native replay paints the same SVG bytes.
- Also paint r/s (pseudo-asymmetric) and M/P where `cip/1` returns them; BIOVIA Draw shows meso and unknown too and has a time-out for large molecules (BD-DS24, BD-T104): skip and say why when `cip/1` answers `skip`.
