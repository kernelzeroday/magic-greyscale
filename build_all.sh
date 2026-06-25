#!/bin/bash
set -e
cd "$(dirname "$0")"

BIN="cargo run --release --"
OUT="output"
mkdir -p "$OUT"

echo "========================================="
echo " Magic Greyscale Proxy Sheet Builder"
echo "========================================="
echo ""

# Full art sets
echo "--- Mystical Archive (STA) - Japanese alt-art ---"
$BIN print "set:sta lang:ja" -o "$OUT/mystical_archive_jp.pdf"
echo ""

echo "--- Mystical Archive (STA) - English ---"
$BIN print "set:sta lang:en" -o "$OUT/mystical_archive_en.pdf"
echo ""

echo "--- Wilds of Eldraine Showcase ---"
$BIN print "set:wot frame:showcase" -o "$OUT/wilds_of_eldraine_showcase.pdf"
echo ""

echo "--- Special Guests ---"
$BIN print "set:spg" -o "$OUT/special_guests.pdf"
echo ""

echo "--- Multiverse Legends (MUL) ---"
$BIN print "set:mul" -o "$OUT/multiverse_legends.pdf"
echo ""

echo "--- Strixhaven Mystical Archive Japanese alt-art only ---"
$BIN print "set:sta lang:ja is:alternate" -o "$OUT/mystical_archive_jp_alt.pdf"
echo ""

# Power staples for high-power playtesting
echo "--- Power Staples (P9, fast mana, tutors, staples) ---"
$BIN print decks/power_staples.txt -o "$OUT/power_staples.pdf"
echo ""

echo "--- Fetchlands & Original Duals ---"
$BIN print decks/fetchlands_duals.txt -o "$OUT/fetchlands_duals.pdf"
echo ""

echo "========================================="
echo " Done! PDFs in $OUT/"
echo "========================================="
ls -lh "$OUT"/*.pdf
