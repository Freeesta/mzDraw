#!/usr/bin/env bash
# Is the old name left in a renamed tree? tools/check_old_name_left.sh DIR OLD_SLUG   (exit 0: no; 1: yes, the places are listed)
#
# Looks for OLD_SLUG in any case (mzdraw, mzDraw, MZDRAW, mzDRAW, ...) in the text of every file and in every path of DIR, except what a
# rename keeps on purpose (see docs/built/rename-tool.md): the target, site, node_modules, tools/.bin and .git directories, docs/prompts
# and docs/input (written history), crates/*/tests/legacy (files of an earlier name), the lines marked `rename-keep` / `rename-legacy`,
# the `legacy_ids` line of brand.toml and the `*Old` lists of web/names.js, Cargo.lock, and inside a line the repository and the Pages
# address, `COO-<OLD>`, the first document id as `since`, and the key of the edit schema (`"<key>":true`). The values come from the
# brand.toml of DIR itself.
set -euo pipefail
[ $# -eq 2 ] || { echo "usage: tools/check_old_name_left.sh DIR OLD_SLUG" >&2; exit 2; }
cd "$1"; old=$2
toml() { sed -n "s/^$1 = \"\(.*\)\"[[:space:]]*\(#.*\)\{0,1\}\$/\1/p" brand.toml | head -1; }
first=$(toml first_document); key=$(toml schema_key); url=$(toml pages_url); base=$(toml pages_base)
[ -n "$first" ] && [ -n "$key" ] && [ -n "$url" ] && [ -n "$base" ] || { echo "check: no brand.toml in $1" >&2; exit 2; }
up=$(echo "$old" | tr 'a-z' 'A-Z')
esc() { printf '%s' "$1" | sed 's/[][\.*^$/#|(){}+?]/\\&/g'; }
skip='^\./(\.git|target|site|node_modules|tools/\.bin|docs/prompts|docs/input|crates/[^/]+/tests/legacy)(/|:)'
left=$(mktemp)
{
  # the text of the files, as path:line:text
  grep -rIin --exclude-dir=.git --exclude-dir=target --exclude-dir=site --exclude-dir=node_modules --exclude=Cargo.lock -e "$old" . || true
  # the paths
  find . \( -path ./.git -o -path ./target -o -path ./site -o -path ./node_modules \) -prune -o -iname "*$old*" -print || true
} | grep -Ev "$skip" \
  | grep -Ev 'rename-keep|rename-legacy|^\./brand\.toml:[0-9]+:legacy_ids = |^\./web/names\.js:[0-9]+:  (autosave|settings|clip)Old: ' \
  | sed -E "s#$(esc "$url")##g; s#$(esc "$base")##g; s#(Freeesta/|freeesta\\.github\\.io/)$old##Ig; s#COO-$up##g" \
  | sed -E "s#(\\\\?\"since\\\\?\":\\\\?\")$(esc "$first")##g; s#^(.*\\| \`[^\`]+\` \\| )$(esc "$first") \\|#\\1|#; s#(\\\\?\")$(esc "$key")(\\\\?\":(true|false))#\\1\\2#g" \
  | { grep -i -e "$old" || true; } > "$left"
if [ -s "$left" ]; then
  echo "the old name '$old' is left in $(wc -l < "$left") place(s) of $1:" >&2
  sed -n '1,40p' "$left" >&2
  rm -f -- "$left"
  exit 1
fi
rm -f -- "$left"
echo "no trace of the old name '$old' in $1 outside what a rename keeps"
