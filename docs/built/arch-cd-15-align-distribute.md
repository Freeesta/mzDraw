# Align, distribute, turn by a typed angle, scale, nudge (arch-cd-15), built

Five commands that arrange the selection (the whole drawing when nothing is selected), each one undo step, each only a list of existing operations (`move`, `rotate`): no new op, so a log from the page replays natively to the same bytes.

| Command | What it does | Ops |
|---|---|---|
| `align:left\|centre\|right\|top\|middle\|bottom` | puts every item on that edge, or centre line, of the box of the whole selection (`top` is the largest y of the document) | one `move` per item that is not there |
| `distribute:h\|v` | three or more items: the first and the last stay, the gaps between the boxes become equal | one `move` per item that moves |
| `rotate:<deg>` | turns the selected atoms about the centre of their box (the field "Turn by" of the Edit palette) | one `rotate` |
| `scale:<f>` | factor 0.01 to 100 about the centre of the box of the selection, bond lengths change with it; a structure keeps its shape, an object (arrow, plus, text) goes where its anchor lands and keeps its size | one `move` per atom and per object |
| `nudge:<dx>,<dy>` | moves the selection by that many bond lengths | one `move` |

An *item* is a connected structure (an atom in the selection stands for its whole structure, so a molecule is never torn) or one object; its box is the box of its anchor points (atoms, the two ends of an arrow, the anchor of a plus or a text). Items are ordered by position and then by id, every shift is rounded to the 4 decimals of the document, and the order of the selection does not matter: the same drawing always gives the same bytes. Arrows anchored to a structure that moves follow it as with any move (`following`), and the stereo guard speaks as for any geometry step. Arranging what is arranged already changes nothing and says so (`draw2.say.arrangedSame`); too few items is `error.draw2.arrangeTwo` (align) or `arrangeThree` (distribute); `rotate` with only objects selected is `arrangeAtoms`.

CLI route (rule 15): in a replay request `"commands":["align:left", {"command":"distribute:h","args":{"atoms":[1,4,8]}}]` (a bare name is a command without arguments; `args` takes `atoms` and `objects`, and with neither the command acts on everything). The golden file is `tests/data/log/arrange.json`. The page: eight buttons (six align, two distribute) and the fields "Turn by" and "Scale by" in the Edit palette (`ed.command("align:left")`, `ed.command("rotate:45")`).

Not built: `scale` does not lengthen an arrow or resize a text (no op sets those from a scale; `atoms.xy` of arch-std-15 is not in `main`, so a scale is one `move` per atom, which makes a long log but replays and undoes exactly); text and labels are boxed by their anchor point, not by their drawn extent; centre on the view; guides while dragging and groups (out of this slice); the keys for nudging and for 15° turns are arch-std-05's.
