#!/bin/bash
set -e
cd "$(dirname "$0")"

mkdir -p output

cargo run --release -- pokemon \
    "set:base1" "set:base2" "set:base3" "set:base5" \
    "set:gym1" "set:gym2" \
    "set:neo1" "set:neo2" "set:neo3" "set:neo4" \
    -o output/pokemon_classics.pdf
