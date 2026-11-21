#!/usr/bin/env python3
"""Independent check of the pathway layout (tp-06): the same graphs go to Graphviz `dot -Grankdir=LR -Tjson`, a program that shares nothing
with this repository, and the answers are compared with ours.

  ranks      dot's rank of a node is the index of its x among the distinct x values of the node positions (rounded to 1 pt). For every
             tree it must equal our layer. For the DAG the differences are listed, not failed: dot minimises the total edge length,
             we use the longest path.
  crossings  counted here, as pairwise proper intersections of the straight segments between node positions (dot's splines are not
             used; two segments that share an end do not cross). For every tree our primary-edge count must be <= dot's.

Our layout is the one of `pathway:tree` on each synthetic pathway of tests/data/pathways/ (read as `boxes/1` from the native replay;
build it with `cargo build -p mzdraw --example replay`). `dot` is installed or fetched by the caller (`apt-get install graphviz`); no
reference program or data is committed, only the reference output of our own inputs (tools/fidelity/pathway_graphviz.reference.json: the
program, its version, the date and the counts).

Usage: python3 tools/fidelity/pathway.py [--write-reference] [path/to/replay]
Exit 0 when every condition holds, 1 when one fails. Without `dot` (or without the replay binary) it prints `skipped: dot not installed`
(or `skipped: replay not built`) and exits 0."""
import datetime, json, os, shutil, subprocess, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
ARGS = [a for a in sys.argv[1:] if not a.startswith("--")]
WRITE = "--write-reference" in sys.argv
REPLAY = ARGS[0] if ARGS else os.path.join(ROOT, "target", "debug", "examples", "replay")
REFERENCE = os.path.join(ROOT, "tools", "fidelity", "pathway_graphviz.reference.json")
NAMES = ["chain4", "star2", "star3", "star4", "star5", "star4_mz", "binary3", "unbalanced", "dag", "two_components",
         "mixed", "messy_star", "big200"]


def ours(name):
    """boxes/1 of the pathway after our layout."""
    doc = open(os.path.join(ROOT, "tests", "data", "pathways", name + ".json")).read()
    req = {"import": doc, "commands": [{"command": "pathway:tree"}], "out": "boxes"}
    r = subprocess.run([REPLAY], input=json.dumps(req), capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(name + ": " + r.stderr)
    return json.loads(r.stdout)


def graph(b):
    """(node ids, edges as (from, to) with distinct ends, our layer of every node: the longest directed path from a root)."""
    edges = sorted({(a["from"], a["to"]) for a in b["arrows"] if a["from"] and a["to"] and a["from"] != a["to"]})
    ids = sorted({v for e in edges for v in e})
    parents = {v: [u for u, w in edges if w == v] for v in ids}
    layer = {}

    def of(v):
        if v not in layer:
            layer[v] = 0 if not parents[v] else 1 + max(of(u) for u in parents[v])
        return layer[v]

    sys.setrecursionlimit(10000)
    for v in ids:
        of(v)
    return ids, edges, layer


def dot_layout(ids, edges, box):
    """dot's node positions (points) for the graph, left to right."""
    lines = ["digraph G {", "node [shape=box, fixedsize=true];"]
    for v in ids:
        b = box[v]
        lines.append(f'{v} [width={(b[2] - b[0]) * 0.5:.4f}, height={(b[3] - b[1]) * 0.5:.4f}];')
    lines += [f"{u} -> {w};" for u, w in edges] + ["}"]
    r = subprocess.run(["dot", "-Grankdir=LR", "-Tjson"], input="\n".join(lines), capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError("dot: " + r.stderr)
    out = json.loads(r.stdout)
    pos = {}
    for o in out["objects"]:
        if "pos" in o:
            x, y = o["pos"].split(",")
            pos[int(o["name"])] = (float(x), float(y))
    return pos


def ranks(pos):
    """The index of the node's x among the distinct x values, after rounding to 1 pt."""
    xs = sorted({round(p[0]) for p in pos.values()})
    return {v: xs.index(round(p[0])) for v, p in pos.items()}


def crossings(edges, pos):
    """Pairwise proper intersections of the straight segments between node positions; segments that share an end are not counted."""
    def side(a, b, c):
        d = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
        return (d > 1e-9) - (d < -1e-9)

    n = 0
    for i, (a, b) in enumerate(edges):
        for c, d in edges[i + 1:]:
            if len({a, b, c, d}) < 4:
                continue
            p, q, r, s = pos[a], pos[b], pos[c], pos[d]
            if side(p, q, r) * side(p, q, s) < 0 and side(r, s, p) * side(r, s, q) < 0:
                n += 1
    return n


def main():
    if shutil.which("dot") is None:
        print("skipped: dot not installed")
        return 0
    if not os.path.exists(REPLAY):
        print("skipped: replay not built (cargo build -p mzdraw --example replay)")
        return 0
    version = subprocess.run(["dot", "-V"], capture_output=True, text=True).stderr.strip()
    print(version)
    failed, rows = 0, {}
    for name in NAMES:
        b = ours(name)
        box = {s["id"]: (s["x0"], s["y0"], s["x1"], s["y1"]) for s in b["structures"]}
        ids, edges, layer = graph(b)
        centre = {v: ((box[v][0] + box[v][2]) / 2, (box[v][1] + box[v][3]) / 2) for v in ids}
        dpos = dot_layout(ids, edges, box)
        drank = ranks(dpos)
        differ = {v: [layer[v], drank[v]] for v in ids if layer[v] != drank[v]}
        mine, theirs = crossings(edges, centre), crossings(edges, dpos)
        tree = name != "dag"
        ok = (not differ and mine <= theirs) if tree else True
        failed += not ok
        rows[name] = {"nodes": len(ids), "edges": len(edges), "kind": "tree" if tree else "dag", "ranks_equal": not differ,
                      "rank_differences": {str(v): d for v, d in sorted(differ.items())},
                      "crossings_ours": mine, "crossings_dot": theirs}
        note = "" if tree else f"  (listed, not failed: {len(differ)} rank difference(s) {differ})"
        print(f"{'ok  ' if ok else 'FAIL'} {name:15} nodes {len(ids):3} edges {len(edges):3}  ranks "
              f"{'equal' if not differ else 'differ'}  crossings ours {mine} dot {theirs}{note}")
    if WRITE:
        ref = {"format": "pathway-graphviz/1", "program": version, "date": datetime.date.today().isoformat(),
               "invocation": "dot -Grankdir=LR -Tjson", "pathways": rows}
        with open(REFERENCE, "w") as f:
            json.dump(ref, f, indent=1)
            f.write("\n")
        print("wrote", os.path.relpath(REFERENCE, ROOT))
    print(f"{len(NAMES) - failed}/{len(NAMES)} pathways pass")
    return 1 if failed else 0


sys.exit(main())
