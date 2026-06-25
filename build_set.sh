#!/bin/bash
set -e
cd "$(dirname "$0")"

if [ -z "$1" ]; then
    echo "Usage: ./build_set.sh <scryfall_query> [output_name]"
    echo ""
    echo "Examples:"
    echo "  ./build_set.sh 'set:sta lang:ja'          # Mystical Archive JP"
    echo "  ./build_set.sh 'set:wot frame:showcase'    # Wilds of Eldraine showcase"
    echo "  ./build_set.sh 'set:spg'                   # Special Guests"
    echo "  ./build_set.sh 'set:mul'                   # Multiverse Legends"
    echo "  ./build_set.sh 'set:big'                   # The Big Score"
    echo "  ./build_set.sh 'set:otp'                   # Breaking News (Outlaws)"
    echo "  ./build_set.sh 'set:pip frame:showcase'    # Fallout showcase"
    exit 1
fi

QUERY="$1"
NAME="${2:-$(echo "$QUERY" | tr ' :' '_')}"
OUT="output/${NAME}.pdf"
mkdir -p output

echo "Building: $QUERY -> $OUT"
cargo run --release -- print "$QUERY" -o "$OUT"
echo "Done: $OUT"
