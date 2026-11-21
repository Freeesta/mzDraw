#!/usr/bin/env bash
# Renames the product in one step: tools/rename_product.sh NEW_NAME [--dry-run] [--copy-to DIR] [--css-prefix XYZ] [--dir DIR]
#
# The name lives in brand.toml (and crates/mzdraw/src/brand.rs, which tests/brand.rs holds equal to it). This script moves everything
# else with it, from the values of brand.toml: the name (`mzDraw`), the slug (`mzdraw`: crates, paths, ids, formats, file names),
# the app id (`MZDRAW`), the PascalCase slug (`Mzdraw`) and the CSS prefix (`--mzd-`). The old slug is appended to `legacy_ids`, so
# files already written stay readable for ever. It never changes: the repository and the Pages URL (they follow the GitHub repository
# name, a decision of the owner), the ids that are not made from the name (`draw_edit/1`, `draw-bridge/1`), the history (lines marked
# `rename-keep`: `first_document` and the `since` of the edit schema), docs/prompts and docs/input (written history), crates/*/tests/
# legacy (files of an earlier name) and the name of the coordinator session `COO-MZDRAW`.
#   --dry-run     applies the rename to a temporary copy and prints what it did; nothing in DIR changes
#   --copy-to D   applies it to a new copy of the tree in D (CI: the real tree stays as it is); D must not exist
#   --css-prefix  the prefix of the colour tokens (default: the first three letters of the slug)
#   --dir DIR     the tree to rename (default: the repository this script is in)
# Needs GNU sed and find. Runs `cargo fmt` and `cargo update --workspace --offline` at the end when cargo is there (imports are
# sorted by name and Cargo.lock by package name).
set -euo pipefail
usage() { echo "usage: tools/rename_product.sh NEW_NAME [--dry-run] [--copy-to DIR] [--css-prefix XYZ] [--dir DIR]" >&2; exit 2; }
root=$(cd "$(dirname "$0")/.." && pwd); new=""; dry=0; copy=""; css=""
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) dry=1 ;;
    --copy-to) copy=${2:?}; shift ;;
    --css-prefix) css=${2:?}; shift ;;
    --dir) root=$(cd "${2:?}" && pwd); shift ;;
    -*) usage ;;
    *) [ -z "$new" ] || usage; new=$1 ;;
  esac
  shift
done
[ -n "$new" ] || usage
[[ "$new" =~ ^[A-Za-z][A-Za-z0-9]{1,30}$ ]] || { echo "rename: the name is letters and digits, starting with a letter: $new" >&2; exit 2; }

if [ "$dry" = 1 ] || [ -n "$copy" ]; then            # work on a copy: the tree in DIR is not touched
  if [ "$dry" = 1 ]; then copy=$(mktemp -d); else [ ! -e "$copy" ] || { echo "rename: $copy exists" >&2; exit 2; }; mkdir -p "$copy"; fi
  (cd "$root" && tar --exclude=./.git --exclude=./target --exclude=./site --exclude=./node_modules --exclude=./tools/.bin -cf - .) | tar -xf - -C "$copy"
  args=("$new" --dir "$copy"); [ -z "$css" ] || args+=(--css-prefix "$css")
  RENAME_DRY=$dry "$0" "${args[@]}"
  if [ "$dry" = 1 ]; then
    echo "--- dry run: what would change in $root (nothing was written)"
    diff -rq -x .git -x target -x site -x node_modules -x .bin -x Cargo.lock "$root" "$copy" | sed "s#$root#.#g; s#$copy#NEW#g" | sort | sed -n '1,25p' || true
    echo "... ($(diff -rq -x .git -x target -x site -x node_modules -x .bin "$root" "$copy" | wc -l) differences in all)"
    diff -u "$root/brand.toml" "$copy/brand.toml" | sed -n '3,$p' || true
    rm -rf "$copy"
  else
    echo "renamed copy: $copy (the tree in $root is untouched)"
  fi
  exit 0
fi

cd "$root"
toml() { sed -n "s/^$1 = \"\(.*\)\"[[:space:]]*\(#.*\)\{0,1\}\$/\1/p" brand.toml | head -1; }
old_name=$(toml name); old_slug=$(toml slug); old_app=$(toml app_id); old_css=$(toml css_prefix); first=$(toml first_document)
legacy=$(sed -n 's/^legacy_ids = \[\(.*\)\].*$/\1/p' brand.toml)
[ -n "$old_name" ] && [ -n "$old_slug" ] && [ -n "$old_app" ] && [ -n "$old_css" ] && [ -n "$first" ] || { echo "rename: brand.toml in $root is not the one of this repository" >&2; exit 2; }
slug=$(echo "$new" | tr 'A-Z' 'a-z'); app=$(echo "$slug" | tr 'a-z' 'A-Z'); pascal="$(echo "${slug:0:1}" | tr 'a-z' 'A-Z')${slug:1}"
[ -n "$css" ] || css=${slug:0:3}
old_pascal="$(echo "${old_slug:0:1}" | tr 'a-z' 'A-Z')${old_slug:1}"
[ "$slug" != "$old_slug" ] || { echo "rename: $new is already the name in use" >&2; exit 2; }
case ",$legacy," in *"\"$slug\""*) echo "rename: $slug is an earlier name (legacy_ids): renaming back is a decision for the owner" >&2; exit 2 ;; esac

# 1. the text of every file, except the history and what is not ours
skip=( -path ./.git -o -path ./target -o -path ./site -o -path ./node_modules -o -path ./tools/.bin -o -path ./docs/prompts -o -path ./docs/input
       -o -path './crates/*/tests/legacy' -o -name Cargo.lock )
sedf=$(mktemp)
{
  echo '/rename-keep/b'                                              # a line that is history is not touched
  # what must keep the old name: the repository and the Pages address, the coordinator session, the first document id in `since`
  echo "s#((Freeesta/|freeesta\\.github\\.io/)${old_slug:0:2})(${old_slug:2})#\\1\\x01\\3#Ig"
  echo "s#/${old_name:0:1}(${old_name:1}/)#/${old_name:0:1}\\x01\\1#g"
  echo "s#COO-${old_app:0:1}(${old_app:1})#COO-${old_app:0:1}\\x01\\1#g"
  # (`"since":"` plain, and `\"since\":\"` as JSON inside a JSON golden)
  echo "s#(\"since\\\\?\":\\\\?\")${first:0:1}(${first:1}\\\\?\")#\\1${first:0:1}\\x01\\2#g"
  echo "s#^(\\| \`[^\`]+\` \\| )${first:0:1}(${first:1} \\|)#\\1${first:0:1}\\x01\\2#"
  echo "s#${old_name}#${new}#g"; echo "s#${old_slug}#${slug}#g"; echo "s#${old_app}#${app}#g"; echo "s#${old_pascal}#${pascal}#g"
  echo "s#--${old_css}-#--${css}-#g"
  echo 's#\x01##g'
} > "$sedf"
changed=0
while IFS= read -r f; do
  grep -qI '' "$f" 2>/dev/null || continue                            # not text
  sed -E -i -f "$sedf" "$f" && changed=$((changed + 1))
done < <(find . \( "${skip[@]}" \) -prune -o -type f -print)
rm -f "$sedf"

# 2. brand.toml and brand.rs: the css prefix and the earlier names (the old slug joins the list, the older ones stay)
list="\"$old_slug\""; [ -z "$legacy" ] || list="$legacy, $list"
sed -i -E "s/^(legacy_ids = \\[).*(\\].*)\$/\\1$list\\2/; s/^(css_prefix = \")[^\"]*/\\1$css/" brand.toml
sed -i -E "s/^(pub const LEGACY_IDS: &\\[&str\\] = &\\[).*(\\];.*rename-legacy.*)\$/\\1$list\\2/; s/^(pub const CSS_PREFIX: &str = \")[^\"]*/\\1$css/" crates/*/src/brand.rs
# web/names.js is written from brand.rs: its lists of earlier names follow legacy_ids
if [ -f web/names.js ]; then
  olds() { local out="" s; for s in $(echo "$list" | tr -d '" ' | tr ',' ' '); do out="$out${out:+, }\"${1//@/$s}\""; done; echo "[$out]"; }
  sed -i -E "s#^(  autosaveOld: ).*,\$#\\1$(olds '@-autosave.json'),#; s#^(  settingsOld: ).*,\$#\\1$(olds '@-settings.json'),#; s#^(  clipOld: ).*,\$#\\1$(olds 'application/x-@+json'),#" web/names.js
fi
mv_list=$(find . \( "${skip[@]}" \) -prune -o -name "*$old_slug*" -print | sort -r)
renamed=0
while IFS= read -r p; do
  [ -n "$p" ] || continue
  d=$(dirname "$p"); b=$(basename "$p"); mv "$p" "$d/${b//$old_slug/$slug}"; renamed=$((renamed + 1))
done <<< "$mv_list"

# 3. Cargo.lock (the two crates are called otherwise) and the format of the sources
sed -i "s/\"$old_slug/\"$slug/g" Cargo.lock
if command -v cargo >/dev/null 2>&1; then
  cargo update --workspace --offline >/dev/null 2>&1 || echo "rename: cargo update --workspace --offline did not run: run it before --locked" >&2
  cargo fmt >/dev/null 2>&1 || echo "rename: cargo fmt did not run" >&2
fi
echo "$( [ "${RENAME_DRY:-0}" = 1 ] && echo would\ rename || echo renamed ) $old_name -> $new (slug $slug, app id $app, css prefix --$css-): $changed files read, $renamed paths moved, legacy_ids = [$list]"
