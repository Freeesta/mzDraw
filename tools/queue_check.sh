#!/usr/bin/env bash
# Checks docs/plan/queue.md against docs/prompts/: tools/queue_check.sh [ROOT]   (ROOT defaults to the repository)
# Fails (exit 1, names on stderr) if a file of docs/prompts/ has no row in queue.md, or a row names a prompt file that does not exist.
# A row is a table line that starts with the prompt id in backticks: | `plan-01-op-table` | ...
set -euo pipefail
root=${1:-$(cd "$(dirname "$0")/.." && pwd)}
queue=$root/docs/plan/queue.md
[ -f "$queue" ] || { echo "queue_check: $queue is missing" >&2; exit 1; }
ids=$(sed -n 's/^| `\([^`]*\)` |.*/\1/p' "$queue" | sort)
files=$(cd "$root/docs/prompts" && ls -1 *.md | sed 's/\.md$//' | sort)
bad=0
for f in $files; do
  printf '%s\n' "$ids" | grep -qxF -- "$f" || { echo "queue_check: docs/prompts/$f.md has no row in docs/plan/queue.md" >&2; bad=1; }
done
for i in $ids; do
  printf '%s\n' "$files" | grep -qxF -- "$i" || { echo "queue_check: queue.md names $i, which is not a file of docs/prompts/" >&2; bad=1; }
done
dup=$(printf '%s\n' "$ids" | uniq -d)
[ -z "$dup" ] || { echo "queue_check: more than one row for: $dup" >&2; bad=1; }
[ "$bad" = 0 ] || exit 1
echo "queue_check: ok ($(printf '%s\n' "$files" | wc -l | tr -d ' ') prompts, one row each)"
