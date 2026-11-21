#!/usr/bin/env python3
"""Writes the synthetic pathways of tests/data/pathways/*.json (mzdraw/1 documents, structures written here, no real data).
Run from the repository root: python3 tools/gen_pathways.py"""
import json, math, random, os

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tests", "data", "pathways")

SMALL = [(0, 0, "C"), (0.87, 0.5, "C"), (1.73, 0, "O")]  # a bent chain C-C-O, every node of the equal-size trees
BENZENE = [(math.cos(math.pi / 3 * i), math.sin(math.pi / 3 * i), "C") for i in range(6)]


def tetracycle():
    atoms, rings = [], []
    for r in range(4):  # four fused six-membered rings in a row
        for i in range(6):
            x = math.cos(math.pi / 3 * i + math.pi / 6) + r * math.sqrt(3)
            y = math.sin(math.pi / 3 * i + math.pi / 6)
            atoms.append((round(x, 4), round(y, 4), "C"))
    uniq = []
    for a in atoms:
        if not any(abs(a[0] - b[0]) < 1e-3 and abs(a[1] - b[1]) < 1e-3 for b in uniq):
            uniq.append(a)
    return uniq


def ring_bonds(atoms):
    return [(i, j) for i in range(len(atoms)) for j in range(i + 1, len(atoms))
            if abs(math.hypot(atoms[i][0] - atoms[j][0], atoms[i][1] - atoms[j][1]) - 1.0) < 1e-2]


def chain_bonds(atoms):
    return [(i, i + 1) for i in range(len(atoms) - 1)]


def doc(nodes, edges, pos, data=None):
    """nodes: list of atom lists; edges: (u, v); pos: centre (x, y) per node; data: {node: [["MZ", "1"]]}."""
    atoms, bonds, objs, first, nid = [], [], [], [], 1
    for i, shape in enumerate(nodes):
        cx = sum(a[0] for a in shape) / len(shape)
        cy = sum(a[1] for a in shape) / len(shape)
        ids = []
        for k, (x, y, el) in enumerate(shape):
            a = {"id": nid, "el": el, "x": round(pos[i][0] + x - cx, 4), "y": round(pos[i][1] + y - cy, 4)}
            if data and i in data and k == 0:
                a["data"] = data[i]
            atoms.append(a)
            ids.append(nid)
            nid += 1
        first.append(ids[0])
        bs = ring_bonds(shape) if len(shape) in (6, 18) else chain_bonds(shape)
        for (p, q) in bs:
            bonds.append({"id": nid, "a": ids[p], "b": ids[q], "o": 1})
            nid += 1
    for (u, v) in edges:
        objs.append({"id": nid, "t": "arrow", "x": pos[u][0], "y": pos[u][1], "x2": pos[v][0], "y2": pos[v][1],
                     "from": first[u], "to": first[v]})
        nid += 1
    return {"format": "mzdraw/1", "next": nid, "atoms": atoms, "bonds": bonds, "objects": objs}


def scatter(n, seed, w=30.0, h=24.0):
    r = random.Random(seed)
    return [(round(r.uniform(0, w), 4), round(r.uniform(0, h), 4)) for _ in range(n)]


def save(name, d):
    with open(os.path.join(OUT, name + ".json"), "w") as f:
        json.dump(d, f, separators=(",", ":"))
        f.write("\n")


def same(n):
    return [SMALL] * n


save("chain4", doc(same(4), [(0, 1), (1, 2), (2, 3)], scatter(4, 1)))
for k in (2, 3, 4, 5):
    save(f"star{k}", doc(same(k + 1), [(0, i) for i in range(1, k + 1)], scatter(k + 1, 10 + k)))
# a star whose children are ordered by the data field MZ (and RT), one child without MZ
save("star4_mz", doc(same(5), [(0, i) for i in range(1, 5)], scatter(5, 77),
                     {1: [["MZ", "300.5"], ["RT", "4.0"]], 2: [["MZ", "120.25"], ["RT", "9.5"]],
                      3: [["MZ", "210.0"], ["RT", "1.5"]]}))
bin_edges = [(i, 2 * i + 1) for i in range(7)] + [(i, 2 * i + 2) for i in range(7)]
save("binary3", doc(same(15), bin_edges, scatter(15, 3)))
# a leaf, a subtree of five nodes, a leaf
save("unbalanced", doc(same(9), [(0, 1), (0, 2), (0, 3), (2, 4), (2, 5), (4, 6), (5, 7), (7, 8)], scatter(9, 4)))
# a DAG: a convergent TP (3 has two parents) and a long edge (0 -> 3, layer 0 to 2)
save("dag", doc(same(5), [(0, 1), (0, 2), (1, 3), (2, 3), (0, 3), (3, 4)], scatter(5, 5)))
save("two_components", doc(same(6), [(0, 1), (1, 2), (3, 4), (3, 5)], scatter(6, 6)))
# mixed sizes: a benzene, a tetracycle and small chains
tet = tetracycle()
save("mixed", doc([BENZENE, tet, SMALL, SMALL, BENZENE, tet], [(0, 1), (0, 2), (0, 3), (3, 4), (3, 5)], scatter(6, 8)))
# a messy star laid out by hand: uneven arrows, two children overlapping
save("messy_star", doc(same(4), [(0, 1), (0, 2), (0, 3)], [(0, 0), (6, 0.4), (6.5, 1.0), (4.5, 9)]))
# 200 nodes: a seeded random tree
r = random.Random(2024)
ed = [(r.randrange(0, i), i) for i in range(1, 200)]
save("big200", doc(same(200), ed, scatter(200, 9, 120, 80)))
print("ok")
