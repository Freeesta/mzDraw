#!/usr/bin/env python3
"""Runs the agent task suite (tests/data/agent-suite/tasks.json) against a model command and writes `agent-suite/1` JSON on stdout.

    python3 tools/agent-suite/run.py --model-cmd 'CMD' --arm plain --runs 1 > rates.json
    python3 tools/agent-suite/run.py --reference --arm plain          # no model: the hand-written reference solutions answer

For each task CMD gets on stdin one text (the request format, the text of each arm feature, the table of the operations, the start
drawing of an edit task, the words) and answers on stdout with one replay request per line (each line is one call; the last is the
answer). A stdout line that is a JSON object with the one key "tokens" reports the tokens of the run; other lines that are not a JSON
object (blank lines, code fences) are ignored. The answer is judged by the Rust checker only (`crates/mzdraw/examples/suite_check.rs`):
this file decides nothing about chemistry. Diagnostics go to stderr. Exit 0; 1 on a tool error (build, checker, model command failed
to start); 2 on a usage error (unknown arm feature, arm text not written yet, bad option)."""
import argparse, datetime, json, math, pathlib, statistics, subprocess, sys, tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
SUITE = ROOT / "tests/data/agent-suite"
ARMS = pathlib.Path(__file__).resolve().parent / "arms"
# The request features a model can be told about; each has its text in arms/<name>.md, written by the slice that builds the feature.
FEATURES = ["report", "digest", "select", "plan", "grant"]
BIN = ROOT / "target/debug/examples"


def usage(msg):
    print(f"run.py: {msg}", file=sys.stderr)
    sys.exit(2)


def fail(msg):
    print(f"run.py: {msg}", file=sys.stderr)
    sys.exit(1)


def parse_arm(text, need_text=True):
    """`plain`, or a `+`-joined set of feature names; order-free, returned sorted. A usage error starts with a stable key:
    `arm_unknown:`, `arm_duplicate:` or `arm_text_missing:`."""
    if text == "plain":
        return "plain", []
    names = text.split("+")
    for n in names:
        if n not in FEATURES:
            usage(f"arm_unknown: {n!r} is not an arm feature; the list is plain or a + joined set of {', '.join(FEATURES)}")
    if len(set(names)) != len(names):
        usage("arm_duplicate: an arm feature is named twice")
    names = sorted(names)
    for n in names if need_text else []:
        if not (ARMS / f"{n}.md").exists():
            usage(f"arm_text_missing: the text of the arm feature {n!r} is not written yet (tools/agent-suite/arms/{n}.md is added by the slice that builds it)")
    return "+".join(names), names


def run(cmd, stdin=None, timeout=None):
    return subprocess.run(cmd, input=stdin, capture_output=True, text=True, timeout=timeout)


def replay(request):
    p = run([str(BIN / "replay")], json.dumps(request))
    if p.returncode != 0:
        fail(f"replay refused {json.dumps(request)[:120]}: {p.stderr.strip()}")
    return p.stdout


def prompt_for(task, features):
    parts = [(ARMS / "plain.md").read_text()]
    for f in features:
        parts.append((ARMS / f"{f}.md").read_text())
    parts.append("# The table of the operations (`draw-edit-schema/1`)\n\n" + replay({"out": "schema"}))
    if "import" in task:
        imp = task["import"] if isinstance(task["import"], str) else json.dumps(task["import"], separators=(",", ":"))
        parts.append("# The drawing to start from (an `mzdraw/1` document; pass its text, as a string, in `import`)\n\n" + replay({"import": imp, "out": "mzdraw"}))
    parts.append("# Task\n\n" + task["words"] + "\n")
    return "\n\n".join(parts)


def requests_of(stdout):
    reqs, tokens = [], None
    for line in stdout.splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            v = json.loads(line)
        except ValueError:
            reqs.append(line)  # one call that the checker will refuse
            continue
        if isinstance(v, dict) and list(v) == ["tokens"] and isinstance(v["tokens"], int):
            tokens = v["tokens"]
        else:
            reqs.append(line)
    return reqs, tokens


def judge(task, request, tmp):
    t, r = tmp / "task.json", tmp / "request.json"
    t.write_text(json.dumps(task))
    r.write_text(request)
    p = run([str(BIN / "suite_check"), str(t), str(r)])
    if p.returncode not in (0, 1):
        fail(f"suite_check failed on {task['id']}: {p.stderr.strip()}")
    try:
        v = json.loads(p.stdout)
    except ValueError:
        fail(f"suite_check wrote no JSON on {task['id']}")
    return v["pass"], v["reason"]


def wilson(k, n, z=1.96):
    if n == 0:
        return [0.0, 0.0]
    p = k / n
    d = 1 + z * z / n
    c = p + z * z / (2 * n)
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return [round(max(0.0, (c - h) / d), 4), round(min(1.0, (c + h) / d), 4)]


def median(xs):
    return statistics.median(xs) if xs else None


def main():
    ap = argparse.ArgumentParser(description="run the agent task suite")
    ap.add_argument("--model-cmd", help="shell command: prompt on stdin, replay requests (one per line) on stdout")
    ap.add_argument("--reference", action="store_true", help="no model: the reference solutions answer (a self-test of the pipeline)")
    ap.add_argument("--arm", default="plain", help="plain, or + joined features: " + ", ".join(FEATURES))
    ap.add_argument("--runs", type=int, default=1)
    ap.add_argument("--tasks", help="comma separated task ids (default all)")
    ap.add_argument("--model", default=None, help="the model name written in the result")
    ap.add_argument("--date", default=None, help="the date written in the result (default today, UTC)")
    ap.add_argument("--timeout", type=int, default=900, help="seconds for one model call")
    ap.add_argument("--parse-arm-only", action="store_true", help="print the sorted arm and exit (names are checked, texts are not)")
    ap.add_argument("--no-build", action="store_true", help="do not run cargo build")
    a = ap.parse_args()
    if a.parse_arm_only:
        print(parse_arm(a.arm, need_text=False)[0])
        return
    if bool(a.model_cmd) == a.reference:
        usage("give exactly one of --model-cmd and --reference")
    if a.runs < 1:
        usage("--runs must be at least 1")
    arm, features = parse_arm(a.arm)
    suite = json.loads((SUITE / "tasks.json").read_text())
    tasks = suite["tasks"]
    if a.tasks:
        want = a.tasks.split(",")
        unknown = [w for w in want if w not in {t["id"] for t in tasks}]
        if unknown:
            usage(f"unknown task {unknown[0]}")
        tasks = [t for t in tasks if t["id"] in want]
    if not a.no_build:
        p = run(["cargo", "build", "--locked", "-q", "-p", "mzdraw", "--example", "replay", "--example", "suite_check"])
        if p.returncode != 0:
            fail("cargo build failed: " + p.stderr.strip()[-400:])
    out_tasks, all_runs = [], []
    with tempfile.TemporaryDirectory() as d:
        tmp = pathlib.Path(d)
        for t in tasks:
            runs = []
            for i in range(a.runs):
                if a.reference:
                    reqs, tokens = [(SUITE / "ref" / f"{t['id']}.json").read_text().strip()], None
                else:
                    try:
                        p = run(["sh", "-c", a.model_cmd], prompt_for(t, features), a.timeout)
                    except subprocess.TimeoutExpired:
                        reqs, tokens = [], None
                        print(f"run.py: {t['id']} run {i + 1}: the model command timed out", file=sys.stderr)
                    else:
                        if p.returncode != 0:
                            print(f"run.py: {t['id']} run {i + 1}: the model command exited {p.returncode}: {p.stderr.strip()[-200:]}", file=sys.stderr)
                        reqs, tokens = requests_of(p.stdout)
                if reqs:
                    ok, reason = judge(t, reqs[-1], tmp)
                else:
                    ok, reason = False, "no request"
                runs.append({"pass": ok, "calls": len(reqs), "bytes": sum(len(r.encode()) for r in reqs), "tokens": tokens, "reason": reason})
                print(f"{t['id']} run {i + 1}: {'pass' if ok else 'FAIL ' + reason}", file=sys.stderr)
            out_tasks.append({"id": t["id"], "kind": t["kind"], "passes": sum(r["pass"] for r in runs), "runs": runs})
            all_runs += runs
    n, k = len(all_runs), sum(r["pass"] for r in all_runs)
    toks = [r["tokens"] for r in all_runs if r["tokens"] is not None]
    result = {
        "format": "agent-suite/1",
        "model": "reference solutions" if a.reference else a.model,
        "date": a.date or datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d"),
        "arm": arm,
        "runs_per_task": a.runs,
        "tasks": out_tasks,
        "overall": {
            "runs": n,
            "passes": k,
            "pass_at_1": round(k / n, 4) if n else None,
            "wilson95": wilson(k, n),
            "median_calls": median([r["calls"] for r in all_runs]),
            "median_bytes": median([r["bytes"] for r in all_runs]),
            "tokens": sum(toks) if toks else None,
        },
    }
    print(json.dumps(result, indent=1))


if __name__ == "__main__":
    main()
