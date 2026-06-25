#!/bin/bash
set -e
cd "$(dirname "$0")"

BIN="cargo run --release --"
OUT="output"
COCKATRICE_DECKS="$HOME/Library/Application Support/Cockatrice/Cockatrice/decks"
mkdir -p "$OUT"

# Toner save: 0=off (default, full black), 20=light save, 40=moderate, 60=heavy
TONER_SAVE="${TONER_SAVE:-0}"

echo "========================================="
echo " Master Proxy Sheet Builder"
echo " toner_save=$TONER_SAVE"
echo "========================================="
echo ""
echo "Cockatrice decks: $(ls "$COCKATRICE_DECKS"/*.cod 2>/dev/null | wc -l | tr -d ' ') decks"
echo ""

$BIN print \
    "set:sta number<64"                    \
    "set:sta number>=64"                   \
    "set:wot"                              \
    "set:spg"                              \
    "set:mul"                              \
    "set:otp"                              \
    "set:big"                              \
    "set:spm frame:showcase"               \
    "set:rex"                              \
    decks/power_staples.txt                \
    decks/fetchlands_duals.txt             \
    decks/timeless_staples.txt             \
    "$COCKATRICE_DECKS"                    \
    -o "$OUT/master.pdf"                   \
    --cockatrice "$OUT/master.cod"          \
    --toner-save "$TONER_SAVE"

echo ""
echo "========================================="
echo " Done!"
echo "========================================="
ls -lh "$OUT/master.pdf" "$OUT/master.cod"
