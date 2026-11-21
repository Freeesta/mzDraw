#!/usr/bin/env bash
# Independent check of the PDFs of the golden cases (tests/data/log/pdf.json) with programs that are not ours: `qpdf --check`
# (structure and cross-reference table), `pdfinfo`, `pdfdetach` and `pdftotext` of poppler (one page, the embedded files, the text).
# Chromium's own PDF viewer is not in headless Chromium, so it is not part of this check. A program that is
# not installed is reported as "not measured", never as passed. Needs node (as the other tests do).
set -euo pipefail
cd "$(dirname "$0")/.."
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
node -e '
const fs = require("fs"), d = process.argv[1];
const f = JSON.parse(fs.readFileSync("tests/data/log/pdf.json", "utf8"));
f.pdf.forEach((c, i) => fs.writeFileSync(`${d}/case${i}.pdf`, Buffer.from(c.out, "base64")));
' "$dir"
fail=0
for tool in qpdf pdfinfo pdfdetach pdftotext; do
  command -v "$tool" >/dev/null || echo "$tool: not measured (not installed)"
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
