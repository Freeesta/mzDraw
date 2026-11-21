# arch-cd-01: Clean Up Structure: the 2D layout of mzlab-mol on a drawing, stereo kept (superseded: do not run)

Study rows: D01, D04 of `docs/study/arch-cd.md`.

**Run arch-std-15 (clean-up, PR #45; op `atoms.xy`) instead.** It covers the same slice; this desktop-editor study adds the points below. COO-MZDRAW: paste them into the prompt you send, or into the PR review of that slice. A DEV chat that receives this file by mistake sends COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line, `arch-cd-01 is superseded by arch-std-15 (clean-up, PR #45; op `atoms.xy`)`, and stops.

## Additions from the desktop editors

- Stereo must survive: compare the record's stereo before and after the new positions (`Document::to_mol_stereo`); where a centre's descriptor would flip, turn its wedge (solid ↔ hashed) in the same step; where an E/Z would flip, keep that piece's old positions and say so. Test: the isomeric SMILES is the same before and after.
- A whole-drawing clean keeps each piece's centre where it was (no jump of the view); ChemDraw cleans rings only when fully selected (CD-UG17, 17.1) — consider the same rule for a selection clean.
- Key: ChemDraw and the new Marvin both use `Ctrl+Shift+K` for 2D clean (CD-UG17, M/marvin_shortcuts-and-tricks.html); Firefox keeps that key, so the button stays the main door.
