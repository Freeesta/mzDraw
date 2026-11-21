#!/usr/bin/env bash
# Tests of the rename tool on a renamed copy: tools/test_rename_product.sh COPY OLD_SLUG
# COPY is a tree made by `tools/rename_product.sh --copy-to COPY NEW_NAME` (CI: NEW_NAME is Zzz), OLD_SLUG the slug it had before.
#  1. the copy has no trace of the old name outside what a rename keeps (tools/check_old_name_left.sh);
#  2. the check is not blind: a leftover planted in a Rust source, in a page, in a document (other letter cases) and in a file name is
#     found, and the copy is clean again when it is taken away;
#  3. the tool refuses, with exit 2 and without touching the tree, a name that contains the old slug or the current one (mzdrawx,
#     xmzdraw, zzzx), an earlier name (renaming back) and the name in use.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd); copy=$(cd "$1" && pwd); old=$2
fail() { echo "FAIL: $*" >&2; exit 1; }
cur=$(sed -n 's/^slug = "\(.*\)".*/\1/p' "$copy/brand.toml" | head -1)
[ -n "$cur" ] && [ "$cur" != "$old" ] || fail "COPY is not a renamed copy (slug $cur, old $old)"

"$here/check_old_name_left.sh" "$copy" "$old" >/dev/null || fail "the renamed copy still names $old"
echo "ok 1: no trace of $old outside the keep-set"

src=$(ls "$copy"/crates/*/src/lib.rs | head -1)
planted() {   # planted FILE TEXT: appends TEXT, the check must fail and name FILE, then the file is as it was
  local f=$1 text=$2 keep; keep=$(mktemp); cp "$f" "$keep"; printf '%s\n' "$text" >> "$f"
  if out=$("$here/check_old_name_left.sh" "$copy" "$old" 2>&1); then cp "$keep" "$f"; rm -f -- "$keep"; fail "a leftover in $f was not found"; fi
  echo "$out" | grep -q "$(basename "$f")" || { cp "$keep" "$f"; rm -f -- "$keep"; fail "the leftover in $f is not named: $out"; }
  cp "$keep" "$f"; rm -f -- "$keep"
}
planted "$src" "pub const LEFT: &str = \"$old\"; // $old"
planted "$copy/web/index.html" "<!-- $(echo "$old" | sed 's/\(.\{2\}\)\(.*\)/\1\U\2/') -->"          # mzDRAW
planted "$copy/README.md" "$(echo "$old" | sed 's/^./\U&/') is the old name"                         # Mzdraw
touch "$copy/web/$old.txt"
if "$here/check_old_name_left.sh" "$copy" "$old" >/dev/null 2>&1; then rm -f -- "$copy/web/$old.txt"; fail "an old name in a file name was not found"; fi
rm -f -- "$copy/web/$old.txt"
"$here/check_old_name_left.sh" "$copy" "$old" >/dev/null || fail "the copy is not clean again after the planted leftovers are taken away"
echo "ok 2: leftovers in a source, a page, a document and a file name are found"

sum() { (cd "$copy" && find . -type f -not -path './target/*' -not -path './site/*' -not -path './.git/*' -print0 | sort -z | xargs -0 cksum | cksum); }
before=$(sum)
for name in "${old}x" "x${old}" "${cur}x" "$old" "$cur" "$(echo "$old" | sed 's/\(.\{2\}\)\(.*\)/\1\U\2/')x"; do
  set +e; msg=$("$here/rename_product.sh" --dir "$copy" "$name" 2>&1); rc=$?; set -e
  [ "$rc" = 2 ] || fail "the name $name was not refused (exit $rc): $msg"
done
[ "$before" = "$(sum)" ] || fail "a refused name changed the tree"
echo "ok 3: names that contain the old or the current slug, an earlier name and the name in use are refused, the tree is not touched"
