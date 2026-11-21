# scheme:orient (plan-14)

Built: `scheme:orient` turns each structure after an arrow onto the one before it, by the core they share. One undo step; no new operation (it emits `flip` and `rotate`, which have exact inverses).

- **Module**: `crates/mzdraw/src/orient.rs` (`orient`, `solve`, `plan`). Kept as a module of its own, not inside plan-13's `scheme.rs` (now on `main`): `scheme.rs` is another slice's file and the two commands share only `commands.rs`; the pairs of structures come from anchors or from the position along the arrow (the same idea as `rxn::along`, but it picks the nearest structure on each side instead of classifying all of them). Folding it into `scheme.rs` later is a move of one file.
- **Core**: `SubQuery::matches` of `mzlab-mol` (public at the pinned commit; no feature needed): the smaller structure in the larger one; when that fails, the ring systems of either in the other, the largest first. At least 3 heavy atoms. Hydrogens drawn as atoms are left out of the query.
- **Fit**: 2D Kabsch with `libm` (`atan2` of the centred cross and dot sums). Every match is fitted; the least error wins, among equals the smallest turn. A mirror (`flip` about the vertical axis through the box centre of the target) is tried only when the target has no wedge, hash or wave bond, and is kept only when it lowers the error by more than 1e-3. A core that does not fit as a rigid piece (rms above `FIT` = 0.05 bond lengths, e.g. drawn bent) is treated as no core.
- **Place**: the turn is about the centre of the box of the target. The command never moves a structure (E28: placing is the layout's). The numbers in the tests are the root-mean-square distance of the core atoms with the shift taken away. The route of anchored arrows follows (`following`).
- **Pairs**: an arrow with both anchors (`from`, `to`) uses them; a free arrow uses the nearest structure before its tail and the nearest past its head (structures beside the arrow are agents and are left out). A chain A -> B -> C orients B on A, then C on the turned B. A structure that is the target of two arrows is oriented by the first only.
- **Refusals** (nothing changed): `error.draw2.orientOverlap` (some atom of a turned structure would come within 0.3 bond lengths of another atom that was not that close before; atomic: no pair is applied), `error.draw2.orientNone` (no pair has a core), `error.draw2.orientDone` (every pair is oriented already).
- **CLI twin**: `{"command":"scheme:orient"}` (no `args`; any argument is an invalid request, exit 2) in the replay request; the log holds `flip`/`rotate` and the route operations. Page: button `scheme-orient` in the edit palette, `command("scheme-orient")` of the module; same bytes (`tests/orient.rs`, golden `tests/data/log/scheme-orient.json`, parity).
- **Proof**: `tools/fidelity/orient.py` compares the angle of the core with RDKit `GenerateDepictionMatching2DStructure` (4 cases, difference 0.006 degrees at most, RDKit 2026.09.2, 2026-11-21). RDKit is installed by whoever runs the script; nothing of it is committed.

Not built:
- A per-pair report in the `draw-result/1` envelope: the Rust `orient::plan` returns one `Step` per pair (status, core size, turn, error before and after), the command only gives the refusal key. A scheme where some pairs have no core still applies the others, without saying which were skipped.
- `scheme:orient-series` (E18).
- A maximum-common-substructure engine: a core that is neither the smaller structure nor a ring system of one in the other (a shared chain, two different ring systems with a shared substituent) gives no core. Cases for COO 16: a shared acyclic chain; two ring systems that share only a part.
- A keyboard shortcut (the button is reached with Tab).
- Abbreviation labels: they match by the atoms of their expansion; a pair that differs only by a label is not tested.
- A flip of a target with wedges (refused as no core when only a mirror fits).
