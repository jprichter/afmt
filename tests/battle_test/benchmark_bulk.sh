#!/usr/bin/env bash

# Compare the legacy sequential process-per-file path with one bulk afmt run.
# Both strategies use the same extensions, exclusions, config, and disposable
# dry-run behavior, so the corpus is not modified during measurement.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
CORPUS_DIR="${1:-}"
RUNS="${AFMT_BENCH_RUNS:-5}"
FORMATTER_BINARY="$PROJECT_DIR/target/release/afmt"
BATTLE_CONFIG="$SCRIPT_DIR/.afmt.toml"

if ! [[ "$RUNS" =~ ^[0-9]+$ ]] || (( 10#$RUNS < 3 )) || (( 10#$RUNS % 2 == 0 )); then
    echo "AFMT_BENCH_RUNS must be an odd integer of at least 3" >&2
    exit 2
fi
if [ -z "$CORPUS_DIR" ] || [ ! -d "$CORPUS_DIR" ]; then
    echo "Usage: $0 CORPUS_DIRECTORY" >&2
    exit 2
fi

CORPUS_DIR="$(cd "$CORPUS_DIR" && pwd)"

echo "Building release binary..."
(cd "$PROJECT_DIR" && cargo build --release --bin afmt)

if ! command -v sort >/dev/null || ! command -v find >/dev/null; then
    echo "benchmark requires find and sort" >&2
    exit 2
fi

mapfile -d '' FILES < <(
    find "$CORPUS_DIR" \
        \( -type d \( -name ".git" -o -name ".sfdx" -o -name "node_modules" -o -name "scripts" \) -prune \) -o \
        -type f \( -name "*.cls" -o -name "*.trigger" -o -name "*.apex" -o -name "*.apexc" \) -print0
)
FILE_COUNT="${#FILES[@]}"
if [ "$FILE_COUNT" -eq 0 ]; then
    echo "No supported Apex files found in $CORPUS_DIR" >&2
    exit 1
fi

now_ns() {
    local value
    value="$(date +%s%N 2>/dev/null || true)"
    if [[ "$value" =~ ^[0-9]+$ ]]; then
        echo "$value"
    elif command -v python3 >/dev/null; then
        python3 -c 'import time; print(time.time_ns())'
    else
        echo "A nanosecond clock or Python 3 is required" >&2
        exit 2
    fi
}

run_baseline() {
    local file
    for file in "${FILES[@]}"; do
        "$FORMATTER_BINARY" --config "$BATTLE_CONFIG" "$file" > /dev/null 2>&1
    done
}

run_bulk() {
    "$FORMATTER_BINARY" --config "$BATTLE_CONFIG" "$CORPUS_DIR" > /dev/null 2>&1
}

measure() {
    local label="$1"
    local start end elapsed
    start="$(now_ns)"
    if [ "$label" = "baseline" ]; then
        run_baseline
    else
        run_bulk
    fi
    end="$(now_ns)"
    elapsed=$((end - start))
    echo "$elapsed"
}

median() {
    printf '%s\n' "$@" | sort -n | awk 'NR == (n + 1) / 2 { print; found = 1 } END { if (!found) print 0 }' n="$#"
}

echo "Corpus: $CORPUS_DIR"
echo "Files: $FILE_COUNT"
echo "Runs: $RUNS (one warm-up plus measured runs)"
echo "Warm-up: baseline"
measure baseline >/dev/null
echo "Warm-up: bulk"
measure bulk >/dev/null

baseline_runs=()
bulk_runs=()
for ((run = 1; run <= RUNS; run++)); do
    baseline_time="$(measure baseline)"
    bulk_time="$(measure bulk)"
    baseline_runs+=("$baseline_time")
    bulk_runs+=("$bulk_time")
    echo "Run $run: baseline=${baseline_time}ns bulk=${bulk_time}ns"
done

baseline_median="$(median "${baseline_runs[@]}")"
bulk_median="$(median "${bulk_runs[@]}")"
speedup="$(awk -v baseline="$baseline_median" -v bulk="$bulk_median" 'BEGIN { if (bulk == 0) print "inf"; else printf "%.2fx", baseline / bulk }')"

echo "Median baseline: ${baseline_median}ns"
echo "Median bulk: ${bulk_median}ns"
echo "Relative speedup: ${speedup}"
