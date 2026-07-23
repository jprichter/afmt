#!/usr/bin/env bash

# Lightweight validation for benchmark argument handling. This intentionally
# stops before the release build so it can run in every test environment.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TEMP_DIR"' EXIT

if AFMT_BENCH_RUNS=4 "$SCRIPT_DIR/benchmark_bulk.sh" "$TEMP_DIR" > /dev/null 2> "$TEMP_DIR/error.log"; then
    echo "benchmark accepted an even run count" >&2
    exit 1
fi

grep -q "AFMT_BENCH_RUNS must be an odd integer of at least 3" "$TEMP_DIR/error.log"
echo "benchmark run-count validation passed"
