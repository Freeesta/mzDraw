# Chain tool and the op `chain` (arch-web-08)

Built from `docs/prompts/arch-web-08-chain.md`. `arch-cd-05-chain-tool` (the desktop-editor study) is superseded by it and was not run; its points (count clamped, shown next to the pointer) are met here, with the limit 50 of this prompt.

## What is built
- **Op `chain`** (`draw_edit/1`): `{"op":"chain","n":5,"atom":6,"x":0,"y":0,"deg":30}`. `n` is 1 to 50 bonds (else `error.draw2.chain`); the chain starts on the atom `atom`, or at (`x`, `y`); the first bond leaves at `deg` degrees (0 = right, 90 = up), the next one turns by -60, the one after by +60: 120 degrees at every inner atom, bonds one length long. Like `ring` it expands in Rust (`build::chain`) to `atom.add` and `bond.add`; a point that falls on an existing atom reuses it, a bond that exists is kept, and the inverse is the inverse of the expanded operations (exact undo, in the property test `random_step`). A chain that adds nothing leaves no undo step.
- **Tool `chain`** (palette Bonds, in view; no key: the shortcut table of `arch-std-06` is not on `main`). A drag from an atom or from empty space draws the zigzag whose axis is the pointer direction snapped to 30 degrees, with as many bonds as the distance holds (one bond advances 0.866 along the axis), 1 to 50. The preview is drawn in the preview colour with the bond count next to the end (`Overlay::chain`); the drawing changes only on release, as one undo step. A click gives two bonds, from an atom at its sprout angle, from empty space at 30 degrees. Mouse, finger and pen alike.
- The CLI twin is the op itself: the page emits `chain` and the native replay gives the same bytes (golden `tests/data/log/chain.json`, parity).

## Not built
- Stroke recognition and chains along a curve (out of scope).
- A chain started on an atom that already has one bond takes the sprout angle on a click but keeps the fixed alternation of the op (-60 first), so the first turn can be the opposite of the zigzag of the bond before; the drag follows the pointer and is free of this.
- The first bond of a drag from an atom does not avoid the atom's other neighbours.
- No key binding.
