#!/bin/sh
# Regenerate the white-paper PDF from its markdown source (GitHub renders the
# .md; the .pdf is the downloadable/printable form). Requires pandoc + xelatex.
set -e
cd "$(dirname "$0")"
pandoc white-paper.md --pdf-engine=xelatex \
  -V documentclass=article \
  -o white-paper.pdf
echo "built white-paper.pdf"
