# arch-cd-21: Embedding API: the editor in another page with a small, stable interface (superseded: do not run)

Study rows: L04 of `docs/study/arch-cd.md`.

**Run arch-web-01, arch-web-02, arch-web-03 (instances, element, host adapter) instead.** It covers the same slice; this desktop-editor study adds the points below. COO-MZDRAW: paste them into the prompt you send, or into the PR review of that slice. A DEV chat that receives this file by mistake sends COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line, `arch-cd-21 is superseded by arch-web-01, arch-web-02, arch-web-03 (instances, element, host adapter)`, and stops.

## Additions from the desktop editors

- Tool presets for embedders (`one-molecule`, `reaction`, `full`), as Marvin JS presets and ChemDoodle's `oneMolecule` / `includeToolbar` (study row K11); a `readonly` mode (arch-web-05).
- An editor removed from the page stops catching right-clicks and other events (Ketcher #5588; study §7 X16): a test that removes an element and right-clicks where it was.
