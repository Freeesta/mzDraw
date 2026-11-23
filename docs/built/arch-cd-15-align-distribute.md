# Align, distribute, turn by a typed angle, nudge (arch-cd-15), built

Four commands that arrange the selection (the whole drawing when nothing is selected), each one undo step, each only a list of existing operations (`move`, `rotate`): no new op, so a log from the page replays natively to the same bytes.

| Command | What it does | Ops |
|---|---|---|
| `align:left\|centre\|right\|top\|middle\|bottom` | puts every item on that edge, or centre line, of the box of the whole selection (`top` is the largest y of the document) | one `move` per item that is not there |
| `distribute:h\|v` | three or more items: the first and the last stay, the gaps between the boxes become equal | one `move` per item that moves |
| `rotate:<deg>` | turns the selected atoms about the centre of their box (the field "Turn by" of the Edit palette) | one `rotate` |
| `nudge:<dx>,<dy>` | moves the selection by that many bond lengths | one `move` |

An *item* is a connected structure (an atom in the selection stands for its whole structure, so a molecule is never torn) or one object; its box is the box of its anchor points (atoms, the two ends of an arrow, the anchor of a plus or a text). Items are ordered by position and then by id, every shift is rounded to the 4 decimals of the document, and the order of the selection does not matter: the same drawing always gives the same bytes. Arrows anchored to a structure that moves follow it as with any move (`following`), and the stereo guard speaks as for any geometry step. Arranging what is arranged already changes nothing and says so (`draw2.say.arrangedSame`); too few items is `error.draw2.arrangeTwo` (align) or `arrangeThree` (distribute); `rotate` with only objects selected is `arrangeAtoms`.

CLI route (rule 15): in a replay request `"commands":["align:left", {"command":"distribute:h","args":{"atoms":[1,4,8]}}]` (a bare name is a command without arguments; `args` takes `atoms` and `objects`, and with neither the command acts on everything). The golden file is `tests/data/log/arrange.json`. The page: eight buttons (six align, two distribute) and the field "Turn by" in the Edit palette (`ed.command("align:left")`, `ed.command("rotate:45")`).

Not built: `scale:<f>`: it needs the op `atoms.xy` of arch-std-15, which is not in `main`; the queue row of arch-std-15 stays the follow-up (decision of COO-MZDRAW; the study row I08 stays open). Text and labels are boxed by their anchor point, not by their drawn extent; centre on the view; guides while dragging and groups (out of this slice); the keys for nudging and for 15° turns are arch-std-05's.
