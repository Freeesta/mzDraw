#!/usr/bin/env bash
# Independent check of the PDFs of the golden cases (tests/data/log/pdf.json) with programs that are not ours: `qpdf --check`
# (structure and cross-reference table), `pdfinfo`, `pdfdetach` and `pdftotext` of poppler (one page, the embedded files, the text).
# Chromium's own viewer is checked by tools/check_pdf_viewer.mjs. A program that is
# not installed FAILS the check (a quiet skip would be a test that passes by not running); `--allow-missing` skips it, for a manual
# run on a machine without them. CI installs qpdf and poppler-utils. Needs node (as the other tests do).
set -euo pipefail
cd "$(dirname "$0")/.."
allow=0; [ "${1:-}" = "--allow-missing" ] && allow=1
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
node -e '
const fs = require("fs"), d = process.argv[1];
const f = JSON.parse(fs.readFileSync("tests/data/log/pdf.json", "utf8"));
const cases = f.pdf || [];
if (!cases.length) { console.error("FAIL no cases: the `pdf` section of tests/data/log/pdf.json is empty or missing"); process.exit(1); }
cases.forEach((c, i) => fs.writeFileSync(`${d}/case${i}.pdf`, Buffer.from(c.out, "base64")));
' "$dir"
fail=0
for tool in qpdf pdfinfo pdfdetach pdftotext; do
  command -v "$tool" >/dev/null && continue
  if [ "$allow" = 1 ]; then echo "$tool: not measured (not installed, --allow-missing)"; else echo "FAIL $tool is not installed (--allow-missing skips it)"; fail=1; fi
done
for pdf in "$dir"/case*.pdf; do
  n=$(basename "$pdf")
  if command -v qpdf >/dev/null; then
    qpdf --check "$pdf" >"$dir/qpdf.txt" 2>&1 || { echo "FAIL qpdf --check $n"; cat "$dir/qpdf.txt"; fail=1; }
    grep -q "No syntax or stream encoding errors found" "$dir/qpdf.txt" || { echo "FAIL qpdf $n"; fail=1; }
  fi
  if command -v pdfinfo >/dev/null; then
    pdfinfo "$pdf" | grep -q "^Pages: *1$" || { echo "FAIL pdfinfo $n"; fail=1; }
  fi
  if command -v pdfdetach >/dev/null; then
    pdfdetach -list "$pdf" | grep -q "mzdraw.json" || { echo "FAIL pdfdetach $n: no mzdraw.json"; fail=1; }
    pdfdetach -savefile mzdraw.json -o "$dir/out.json" "$pdf" >/dev/null
    grep -q '"format":"mzdraw/1"' "$dir/out.json" || { echo "FAIL pdfdetach $n: the embedded document"; fail=1; }
  fi
  if command -v pdftotext >/dev/null && [ "$n" = case0.pdf ]; then
    # text is text: the labels of alanine are read back as characters
    pdftotext "$pdf" - | grep -q "^O" || { echo "FAIL pdftotext $n: no text"; fail=1; }
  fi
  echo "checked $n"
done
exit $fail
