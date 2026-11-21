#!/usr/bin/env bash
# Tests of tools/queue_check.sh on a copy of docs/: the real tree is never touched.
#  1. the real tree passes; 2. a prompt file without a row fails and is named; 3. a row without a prompt file fails and is named;
#  4. a second row for one prompt fails.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd); repo=$(cd "$here/.." && pwd)
fail() { echo "FAIL: $*" >&2; exit 1; }
"$here/queue_check.sh" "$repo" >/dev/null || fail "the real tree does not pass"
echo "ok 1: the real tree passes"
tmp=$(mktemp -d); trap 'rm -rf -- "$tmp"' EXIT
mkdir -p "$tmp/docs"; cp -r "$repo/docs/prompts" "$repo/docs/plan" "$tmp/docs/"
q=$tmp/docs/plan/queue.md
expect_fail() {   # expect_fail NAME
  if out=$("$here/queue_check.sh" "$tmp" 2>&1); then fail "$1: the check passed"; fi
  echo "$out" | grep -qF -- "$1" || fail "$1 is not named: $out"
}
touch "$tmp/docs/prompts/zzz-new-prompt.md"; expect_fail zzz-new-prompt; rm "$tmp/docs/prompts/zzz-new-prompt.md"
echo "ok 2: a prompt file without a row is found"
cp "$q" "$q.keep"; printf '| `zzz-gone-prompt` | t | QUEUED | - | - | - |\n' >> "$q"; expect_fail zzz-gone-prompt; cp "$q.keep" "$q"
echo "ok 3: a row without a prompt file is found"
first=$(sed -n 's/^| `\([^`]*\)` |.*/\1/p' "$q" | head -1); printf '| `%s` | t | QUEUED | - | - | - |\n' "$first" >> "$q"; expect_fail "$first"; cp "$q.keep" "$q"
echo "ok 4: a duplicate row is found"
"$here/queue_check.sh" "$tmp" >/dev/null || fail "the restored copy does not pass"
