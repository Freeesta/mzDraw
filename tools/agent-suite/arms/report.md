# Arm: report

Three more fields of the request (plan-20, `draw-result/1`) tell you what a request did:

- `report: true`: the answer is one JSON document `draw-result/1` instead of the bare output: `ok`, `created` and `removed` (the ids of atoms, bonds and objects that the log made and took away), `failed` (the index of the first operation that was refused and its error key, when one is), and `out` (the output you asked for, when ok).
- `dry_run: true`: everything runs and is reported, nothing is written (no `out`).
- `expect`: a list of conditions on the final drawing: `{"pieces": N}` (N separate structures), `{"smiles": "..."}` (the SMILES of the drawing is exactly this text), `{"std": "none"}` (no `std/1` finding), `{"std_absent": ["key", ...]}`. A false condition makes `ok` false.

Use them to check a request before you commit to it; the request you send last is judged on its drawing, whether or not it carries these fields.
