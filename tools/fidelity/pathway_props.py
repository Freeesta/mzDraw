#!/usr/bin/env python3
"""Independent check of the pathway layout (tp-05). No Rust is shared: it asks the native replay for `boxes/1` of each synthetic pathway of
tests/data/pathways/ after `pathway:tree`, re-derives the graph, the layers and the primary forest by the rules of the specification, and
re-checks P5, P6, P7, P9, P10, P11 and P12; for P8 it runs the replay under the keys `id` and `reverse-id` and compares.
Usage: python3 tools/fidelity/pathway_props.py [path/to/replay]   (default target/debug/examples/replay; build it with
`cargo build -p mzdraw --example replay`). Prints `skipped` and exits 0 when python3 is old or the replay is missing."""
import json, math, os, subprocess, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
REPLAY = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "target", "debug", "examples", "replay")
EPS = 3e-4
COL_GAP, ROW_GAP, MAX_ANGLE, MIN_ARROW, MARGIN = 2.5, 1.0, 40.0, 1.2, 0.35
NAMES = ["chain4", "star2", "star3", "star4", "star5", "star4_mz", "binary3", "unbalanced", "dag", "two_components",
         "mixed", "messy_star", "big200"]
TREES = [n for n in NAMES if n != "dag"]


def boxes(name, order="id"):
    doc = open(os.path.join(ROOT, "tests", "data", "pathways", name + ".json")).read()
    req = {"import": doc, "set": ["pathway.order=" + order], "commands": [{"command": "pathway:tree"}], "out": "boxes"}
    r = subprocess.run([REPLAY], input=json.dumps(req), capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(name + ": " + r.stderr)
    return json.loads(r.stdout)


class G:
    def __init__(self, b):
        self.box = {s["id"]: (s["x0"], s["y0"], s["x1"], s["y1"]) for s in b["structures"]}
        self.edges = [a for a in b["arrows"] if a["from"] and a["to"] and a["from"] != a["to"]]
        ids = sorted({e["from"] for e in self.edges} | {e["to"] for e in self.edges})
        self.ids = ids
        par = {v: [] for v in ids}
        for e in self.edges:
            par[e["to"]].append(e["from"])
        # layers: the longest directed path from a root
        self.L = {}
        def layer(v):
            if v not in self.L:
                self.L[v] = 0 if not par[v] else 1 + max(layer(u) for u in par[v])
            return self.L[v]
        sys.setrecursionlimit(10000)
        for v in ids:
            layer(v)
        # the primary parent: largest layer, then the smallest id (order key id)
        self.primary = {v: max(par[v], key=lambda u: (self.L[u], -u)) for v in ids if par[v]}
        self.kids = {v: sorted(c for c, p in self.primary.items() if p == v) for v in ids}
        # weakly connected components
        comp = {v: v for v in ids}
        def find(v):
            while comp[v] != v:
                comp[v] = comp[comp[v]]
                v = comp[v]
            return v
        for e in self.edges:
            comp[find(e["from"])] = find(e["to"])
        self.comp = {v: find(v) for v in ids}
        self.arrow = {(e["from"], e["to"]): e for e in self.edges}

    def c(self, v):
        b = self.box[v]
        return ((b[0] + b[2]) / 2, (b[1] + b[3]) / 2)

    def size(self, v):
        b = self.box[v]
        return (b[2] - b[0], b[3] - b[1])

    def shape(self, v):
        return (round(self.size(v)[0], 3), round(self.size(v)[1], 3), tuple(self.shape(k) for k in self.kids[v]))


def mirror(s):
    return (s[0], s[1], tuple(mirror(k) for k in reversed(s[2])))


def seg(e):
    return (e["x"], e["y"]), (e["x2"], e["y2"])


def check(name):
    g, bad = G(boxes(name)), []
    # P5 columns
    by = {}
    for v in g.ids:
        by.setdefault((g.comp[v], g.L[v]), []).append(v)
    for (cp, k), vs in by.items():
        xs = [g.c(v)[0] for v in vs]
        if max(xs) - min(xs) > EPS:
            bad.append(f"P5 column {k} of {cp}")
        nxt = by.get((cp, k + 1))
        if nxt:
            gap = g.c(nxt[0])[0] - g.c(vs[0])[0] - (max(g.size(v)[0] for v in vs) + max(g.size(v)[0] for v in nxt)) / 2
            if gap < COL_GAP - EPS:
                bad.append(f"P5 gap {gap} < col_gap")
            elif gap > COL_GAP + EPS:
                # raised: some primary arrow between the layers must sit on the slope or the length limit
                tight = False
                for v in nxt:
                    if v in g.primary and g.L[g.primary[v]] == k:
                        (x, y), (x2, y2) = seg(g.arrow[(g.primary[v], v)])
                        ang = math.degrees(math.atan2(abs(y2 - y), x2 - x))
                        tight |= abs(ang - MAX_ANGLE) < 0.05 or abs(math.hypot(x2 - x, y2 - y) - MIN_ARROW) < 1e-3
                if not tight:
                    bad.append(f"P5 gap {k} of {cp} raised for no reason")
    for u in g.ids:
        ks = g.kids[u]
        if not ks:
            continue
        ys = [g.c(k)[1] for k in ks]
        # P6 centring (first and last child)
        if abs(g.c(u)[1] - (ys[0] + ys[-1]) / 2) > EPS:
            bad.append(f"P6 {u}")
        # P7 and P9
        sh = [g.shape(k) for k in ks]
        m = len(ks)
        if all(sh[i] == mirror(sh[m - 1 - i]) for i in range(m)):
            for i in range(m):
                a, b = ks[i], ks[m - 1 - i]
                if abs(g.c(a)[1] + g.c(b)[1] - 2 * g.c(u)[1]) > EPS:
                    bad.append(f"P7 {u}")
                (p, q), (p2, q2) = seg(g.arrow[(u, a)]), seg(g.arrow[(u, b)])
                da = (q[0] - p[0], q[1] - p[1])
                db = (q2[0] - p2[0], q2[1] - p2[1])
                la, lb = math.hypot(*da), math.hypot(*db)
                if abs(da[0] / la - db[0] / lb) > EPS or abs(da[1] / la + db[1] / lb) > EPS:
                    bad.append(f"P9 {u}")
                if g.size(a) == g.size(b) and abs(la - lb) > EPS:
                    bad.append(f"P9 length {u}")
        # P10 even leaves
        if all(not g.kids[k] for k in ks) and len({g.size(k)[1] for k in ks}) == 1:
            h = g.size(ks[0])[1]
            for a, b in zip(ks, ks[1:]):
                if abs(g.c(b)[1] - g.c(a)[1] - h - ROW_GAP) > EPS:
                    bad.append(f"P10 {u}")
    # P11 slope and length of every primary arrow
    for v, u in g.primary.items():
        (x, y), (x2, y2) = seg(g.arrow[(u, v)])
        d = math.hypot(x2 - x, y2 - y)
        if abs(y2 - y) / d > math.sin(math.radians(MAX_ANGLE)) + EPS or d < MIN_ARROW - EPS:
            bad.append(f"P11 {u}->{v}")
    # P12 clearance boxes disjoint; primary arrows never cross
    ids = g.ids
    for i, a in enumerate(ids):
        for b in ids[i + 1:]:
            A, B = g.box[a], g.box[b]
            if max(B[0] - A[2], A[0] - B[2], B[1] - A[3], A[1] - B[3]) < ROW_GAP - EPS:
                bad.append(f"P12 boxes {a} {b}")
    prim = [seg(g.arrow[(u, v)]) for v, u in g.primary.items()]
    def orient(p, q, r):
        return (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    for i, s in enumerate(prim):
        for t in prim[i + 1:]:
            if orient(*s, t[0]) * orient(*s, t[1]) < -1e-12 and orient(*t, s[0]) * orient(*t, s[1]) < -1e-12:
                bad.append("P12 primary arrows cross")
    return bad


def check_mirror(name):
    a, b = G(boxes(name)), G(boxes(name, "reverse-id"))
    bad = []
    roots = {}
    for v in a.ids:
        if v not in a.primary:
            roots.setdefault(a.comp[v], v)
    for v in a.ids:
        r = roots[a.comp[v]]
        if abs((a.c(v)[1] - a.c(r)[1]) + (b.c(v)[1] - b.c(r)[1])) > EPS or abs((a.c(v)[0] - a.c(r)[0]) - (b.c(v)[0] - b.c(r)[0])) > EPS:
            bad.append(f"P8 {v}")
    return bad


def main():
    if sys.version_info < (3, 6) or not os.path.exists(REPLAY):
        print("skipped (no python 3.6+ or no replay example at " + REPLAY + ")")
        return 0
    failed = 0
    for n in NAMES:
        bad = check(n) + (check_mirror(n) if n in TREES else [])
        print(("ok    " if not bad else "FAIL  ") + n + ("" if not bad else ": " + "; ".join(bad[:5])))
        failed += bool(bad)
    print(f"{len(NAMES) - failed}/{len(NAMES)} pathways pass P5-P12 (independent)")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
