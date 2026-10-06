#!/bin/sh
# Regenerate the deck PDFs from their markdown sources (GitHub renders the .md;
# the .pdf is the downloadable/presentable form). Requires pandoc + xelatex.
set -e
cd "$(dirname "$0")"
for deck in overview technical; do
  pandoc "$deck.md" -t beamer --pdf-engine=xelatex \
    -V aspectratio=169 -V theme=Madrid -V fontsize=10pt \
    -o "$deck.pdf"
  echo "built $deck.pdf"
done
