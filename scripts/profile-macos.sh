#!/bin/sh
set -eu

binary=$1
version=$2
api=$3
input=$4
output=$5
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
mkdir -p "$output"
"$binary" "$version" "$api" "$input" 8 > "$output/run.json" &
profile_pid=$!
trap 'kill "$profile_pid" 2>/dev/null || true' EXIT HUP INT TERM
sample "$profile_pid" 4 1 -fullPaths -file "$output/stacks.txt"
wait "$profile_pid"
trap - EXIT HUP INT TERM
cargo run --quiet --manifest-path "$repo_root/Cargo.toml" --example collapse_sample -- "$output/stacks.txt" > "$output/stacks.folded"
inferno-flamegraph --deterministic --colors rust --title "$api: $(basename "$input")" \
    --subtitle "macOS sample; 4 seconds at 1 ms; relative stack samples" "$output/stacks.folded" > "$output/flamegraph.svg"
