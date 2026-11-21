#!/usr/bin/env python3
"""Independent check of the pathway layout (tp-06): the same graphs go to Graphviz `dot -Grankdir=LR -Tjson`, a program that shares nothing
with this repository, and the answers are compared with ours.

  ranks      the rank of a node is the index of its x among the distinct x values of the node positions, rounded to 1 pt. Ours is read
             from OUR layout (the centres of the boxes of `boxes/1` after `pathway:tree`, at 30 pt per bond length), dot's from dot's
             `pos`; for every tree the two must be equal. No topology is computed here to stand for our rank.
  crossings  counted here, as pairwise proper intersections of the straight segments between node positions (ours: the real box
             centres; dot's: its node positions; dot's splines are not used; segments that share an end do not cross). For every
             tree (every edge is then a primary edge) our count must be <= dot's.
  DAG        a graph in which a node has two parents is not a tree: its rank differences and crossing counts are listed, never counted
             as a pass or a fail (dot minimises the total edge length, we use the longest path; our primary edges are not re-derived here).
  control    the same comparison on the messy star BEFORE the layout must fail: it shows the check can.

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


PT = 30.0     # points per bond length of the default style


def ours(name, laid=True):
    """boxes/1 of the pathway after our layout (or before it, for the control)."""
    doc = open(os.path.join(ROOT, "tests", "data", "pathways", name + ".json")).read()
    req = {"import": doc, "out": "boxes"}
    if laid:
        req["commands"] = [{"command": "pathway:tree"}]
    r = subprocess.run([REPLAY], input=json.dumps(req), capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(name + ": " + r.stderr)
    return json.loads(r.stdout)


def graph(b):
    """(node ids, edges as (from, to) with distinct ends, whether the graph is a forest: no node has two parents)."""
    edges = sorted({(a["from"], a["to"]) for a in b["arrows"] if a["from"] and a["to"] and a["from"] != a["to"]})
    ids = sorted({v for e in edges for v in e})
    return ids, edges, len({w for _, w in edges}) == len(edges)


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


def compare(name, laid=True):
    """The comparison of one pathway: (row, node ids)."""
    b = ours(name, laid)
    box = {s["id"]: (s["x0"], s["y0"], s["x1"], s["y1"]) for s in b["structures"]}
    ids, edges, forest = graph(b)
    centre = {v: (PT * (box[v][0] + box[v][2]) / 2, PT * (box[v][1] + box[v][3]) / 2) for v in ids}
    dpos = dot_layout(ids, edges, box)
    orank, drank = ranks(centre), ranks(dpos)
    differ = {v: [orank[v], drank[v]] for v in ids if orank[v] != drank[v]}
    mine, theirs = crossings(edges, centre), crossings(edges, dpos)
    return {"nodes": len(ids), "edges": len(edges), "kind": "tree" if forest else "dag", "ranks_equal": not differ,
            "rank_differences": {str(v): d for v, d in sorted(differ.items())},
            "crossings_ours": mine, "crossings_dot": theirs}


def main():
    if shutil.which("dot") is None:
        print("skipped: dot not installed")
        return 0
    if not os.path.exists(REPLAY):
        print("skipped: replay not built (cargo build -p mzdraw --example replay)")
        return 0
    version = subprocess.run(["dot", "-V"], capture_output=True, text=True).stderr.strip()
    print(version)
    failed, rows, trees = 0, {}, 0
    for name in NAMES:
        r = compare(name)
        rows[name] = r
        if r["kind"] == "tree":
            trees += 1
            ok = r["ranks_equal"] and r["crossings_ours"] <= r["crossings_dot"]
            failed += not ok
            verdict = "ok  " if ok else "FAIL"
            note = ""
        else:
            verdict = "list"
            note = f"  (DAG: listed, not counted; rank differences {r['rank_differences']})"
        print(f"{verdict} {name:15} {r['kind']:4} nodes {r['nodes']:3} edges {r['edges']:3}  ranks "
              f"{'equal' if r['ranks_equal'] else 'differ'}  crossings ours {r['crossings_ours']} dot {r['crossings_dot']}{note}")
    c = compare("messy_star", laid=False)
    caught = not c["ranks_equal"] or c["crossings_ours"] > c["crossings_dot"]
    failed += not caught
    print(f"{'ok  ' if caught else 'FAIL'} control: the messy star before the layout "
          f"{'is told apart' if caught else 'passes (the check cannot fail)'} (ranks {'equal' if c['ranks_equal'] else 'differ'}, "
          f"crossings ours {c['crossings_ours']} dot {c['crossings_dot']})")
    if WRITE:
        ref = {"format": "pathway-graphviz/1", "program": version, "date": datetime.date.today().isoformat(),
               "invocation": "dot -Grankdir=LR -Tjson", "pathways": rows, "control_messy_star_unarranged": c}
        with open(REFERENCE, "w") as f:
            json.dump(ref, f, indent=1)
            f.write("\n")
        print("wrote", os.path.relpath(REFERENCE, ROOT))
    print(f"{trees - (failed - (not caught))}/{trees} trees pass, control {'caught' if caught else 'missed'}")
    return 1 if failed else 0


sys.exit(main())
