# The replay request

You draw by sending replay requests. A request is one JSON object on one line; the tool runs it on an empty drawing (or on the drawing in `import`) and gives back the output named in `out`. Each request you send is one call and is independent of the others: it starts again from `import`, so a request must say everything it needs.

Fields (any other field is refused):

- `import`: a text to start from, a string: a SMILES, a molfile, or the text of an `mzdraw/1` document. A structure that arrives without coordinates is placed by the layout of the tool (pieces side by side, largest first). A document keeps its ids and coordinates.
- `log`: the list of operations (see the table below), applied in order.
- `out`: what to write: `mzdraw` (the document, default), `smiles`, `molfile`, `svg`, `std`, `schema`, ...
- `style`, `set`: settings; you do not need them.

Coordinates are in bond lengths, y up. Ids of atoms, bonds and objects (arrows, plus signs, texts) share one counter: the next id is the counter's value. An operation that names an id that does not exist is refused.

Answer with the final request only (or several, one per line, if you want to look at an output first; the last one is the answer). One JSON object per line, no explanation, no code fences. The last request must produce the drawing that the task asks for.
