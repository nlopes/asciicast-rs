#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
after_revision=${1:-psymwmru}
run_directory=${2:-"$repo_root/target/comparison"}
filter=${3:-}
case "$run_directory" in /*) ;; *) run_directory="$repo_root/$run_directory" ;; esac
if test -e "$run_directory"; then
    printf '%s\n' "Choose a new result directory; existing results are preserved." >&2
    exit 1
fi
mkdir -p "$run_directory"
cd "$repo_root"
rustc --version --verbose > "$run_directory/toolchain.txt"
uname -a > "$run_directory/system.txt"

for arm in before after; do
    revision="$after_revision"
    if test "$arm" = before; then revision="($after_revision)-"; fi
    snapshot="$run_directory/$arm"
    mkdir -p "$snapshot"
    jj --ignore-working-copy log --no-graph -r "$revision" -T 'commit_id ++ "\n"' > "$snapshot/revision.txt"
    jj --ignore-working-copy file list -r "$revision" src Cargo.toml Cargo.lock README.md |
    while IFS= read -r file; do
        mkdir -p "$snapshot/$(dirname "$file")"
        jj --ignore-working-copy file show -r "$revision" "$file" > "$snapshot/$file"
    done
    # Both versions use the current test tools and their original library dependencies.
    awk '/^\[dev-dependencies\]/ {exit} {print}' "$snapshot/Cargo.toml" > "$snapshot/manifest.tmp"
    awk '/^\[dev-dependencies\]/ {copy=1} copy {print}' "$repo_root/Cargo.toml" >> "$snapshot/manifest.tmp"
    mv "$snapshot/manifest.tmp" "$snapshot/Cargo.toml"
    cp -R "$repo_root/benches" "$repo_root/examples" "$snapshot/"
    mkdir -p "$snapshot/tests"
    cp "$repo_root/tests/properties.rs" "$snapshot/tests/"
    cp "$repo_root/tests/proptest-regressions.txt" "$snapshot/tests/"
    cargo test --manifest-path "$snapshot/Cargo.toml" --target-dir "$run_directory/build" --all-features --test properties
    for bench in parser allocations; do
        cargo bench --manifest-path "$snapshot/Cargo.toml" --target-dir "$run_directory/build" --all-features --bench "$bench" --no-run --message-format=json > "$snapshot/$bench-build.jsonl"
        executable=$(jq -r --arg name "$bench" 'select(.reason == "compiler-artifact" and .target.name == $name and .executable != null) | .executable' "$snapshot/$bench-build.jsonl")
        cp "$executable" "$run_directory/$arm-$bench"
    done
    cargo build --manifest-path "$snapshot/Cargo.toml" --target-dir "$run_directory/build" --profile bench --all-features --example profile --message-format=json > "$snapshot/profile-build.jsonl"
    executable=$(jq -r 'select(.reason == "compiler-artifact" and .target.name == "profile" and .executable != null) | .executable' "$snapshot/profile-build.jsonl")
    cp "$executable" "$run_directory/$arm-profile"
    rg --files "$snapshot/src" | while IFS= read -r file; do shasum -a 256 "$file"; done > "$snapshot/sources.sha256"
    shasum -a 256 "$snapshot/Cargo.toml" "$snapshot/Cargo.lock" >> "$snapshot/sources.sha256"
done
shasum -a 256 "$run_directory"/before-* "$run_directory"/after-* "$repo_root"/benches/*.rs "$repo_root"/benches/support/mod.rs "$repo_root"/examples/profile.rs "$repo_root"/tests/properties.rs "$repo_root"/tests/proptest-regressions.txt "$repo_root"/scripts/compare.sh > "$run_directory/hashes.txt"

# Compilation finishes before any timing or allocation measurement starts.
ASCIICAST_BENCH_OUTPUT="$run_directory/timing" "$run_directory/before-parser" "$filter" --bench --save-baseline before
ASCIICAST_BENCH_OUTPUT="$run_directory/timing" "$run_directory/after-parser" "$filter" --bench --baseline before
test -f "$run_directory/timing/report/index.html"
ASCIICAST_MEMORY_OUTPUT="$run_directory/memory" "$run_directory/before-allocations" --save-baseline before
ASCIICAST_MEMORY_OUTPUT="$run_directory/memory" "$run_directory/after-allocations" --save-baseline after --baseline before
printf 'Timing report: %s\nAllocation plots: %s\n' "$run_directory/timing/report/index.html" "$run_directory/memory"
