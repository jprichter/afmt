#!/bin/bash

# -----------------------------------------------------------------------------
# Script to format files in repos with improved git error handling
# -----------------------------------------------------------------------------

# Exit on error, but with proper error handling
set -eo pipefail

# Get the absolute path of the current script's directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_LIST="$SCRIPT_DIR/repos.txt"
TARGET_DIR="$SCRIPT_DIR/repos"
FORMATTER_BINARY="$SCRIPT_DIR/../../target/debug/afmt"
LOG_FILE="$SCRIPT_DIR/format_errors.log"

# Enhanced error handling function
handle_error() {
    local exit_code=$?
    echo "Error occurred in script at line $1, exit code: $exit_code"
    exit $exit_code
}

trap 'handle_error ${LINENO}' ERR

# Create target directory if it doesn't exist
mkdir -p "$TARGET_DIR"

# Check if the formatter binary exists
if [ ! -f "$FORMATTER_BINARY" ]; then
    echo "Formatter binary not found, building it with cargo..."
    (cd "$SCRIPT_DIR/../.." && cargo build) || {
        echo "Cargo build failed, exiting."
        exit 1
    }
fi

# Clear the log file at the start
> "$LOG_FILE"

# Function to check if a repository is accessible
check_repo_availability() {
    local repo_url="$1"
    # Try to do a lightweight ls-remote to check repo accessibility
    if git ls-remote --quiet --exit-code "$repo_url" HEAD &>/dev/null; then
        return 0
    else
        return 1
    fi
}

# Clone repositories with better error handling
while IFS= read -r REPO_URL || [ -n "$REPO_URL" ]; do
    # Skip empty lines and comments
    [[ -z "$REPO_URL" || "$REPO_URL" =~ ^# ]] && continue

    REPO_NAME=$(basename -s .git "$REPO_URL")
    REPO_PATH="$TARGET_DIR/$REPO_NAME"

    echo "Checking availability of $REPO_URL..."

    if ! check_repo_availability "$REPO_URL"; then
        echo "Warning: Repository $REPO_URL appears to be inaccessible, skipping..."
        echo "Failed to access repository: $REPO_URL" >> "$LOG_FILE"
        continue
    fi

    echo "Cloning $REPO_URL into $REPO_PATH"

    if [ -d "$REPO_PATH" ]; then
        echo "Directory already exists, removing it..."
        rm -rf "$REPO_PATH"
    fi

    # Clone with retries and proper error handling
    for i in {1..3}; do
        if git clone --depth 1 --single-branch "$REPO_URL" "$REPO_PATH" 2>> "$LOG_FILE"; then
            break
        else
            if [ $i -eq 3 ]; then
                echo "Failed to clone $REPO_URL after 3 attempts"
                continue 2
            fi
            echo "Attempt $i failed, retrying..."
            sleep 2
        fi
    done
done < "$REPO_LIST"

# Clear the log files at the start
> "$LOG_FILE"
# > "$LONG_LINES_LOG_FILE"

# Check for idempotent mode flag
IDEMPOTENT_MODE=false
if [ "$1" == "--idempotent" ]; then
    IDEMPOTENT_MODE=true
    echo "Idempotent testing mode activated."
fi

# Function to format files and log errors with clear info
format_files() {
    local FILE_PATH="$1"

    # echo "Processing file: $FILE_PATH"

    OUTPUT=$($FORMATTER_BINARY --write "$FILE_PATH" 2>&1)
    EXIT_CODE=$?

    if [ $EXIT_CODE -ne 0 ]; then
        if echo "$OUTPUT" | grep -qE "snippet: %%{2,3}"; then
            :  # Skip logging for %% cases as they are from managed package templating
        elif echo "$OUTPUT" | grep -q "%%%"; then
            :  # Same as above
        # elif echo "$OUTPUT" | grep -q "/scripts/"; then
        #     :  # Same as above
        # elif echo "$OUTPUT" | grep -q "Parent node kind: class_body,"; then
        #     :  # Same as above
        else
            {
                echo "========================================"
                echo "Error while formatting file: $FILE_PATH"
                echo "Exit code: $EXIT_CODE"
                echo "----------------------------------------"
                echo "$OUTPUT"
                echo "========================================"
            } >> "$LOG_FILE"
            return 1
        fi
    fi

    # Log long lines
    # PATTERN="^.{$((LINE_LENGTH + 1)),}$"
    # echo "$OUTPUT" | grep -E "$PATTERN" >> "$LONG_LINES_LOG_FILE"
}

idempotent_test() {
    local FILE_PATH="$1"
    TMP1=$(mktemp)
    TMP2=$(mktemp)

    # Format once and save output to TMP1
    $FORMATTER_BINARY "$FILE_PATH" > "$TMP1" 2>/dev/null

    # Format the result of the first formatting and save to TMP2
    $FORMATTER_BINARY "$TMP1" > "$TMP2" 2>/dev/null

    # Capture detailed diff output
    DIFF_OUTPUT=$(diff "$TMP1" "$TMP2")
    if [ -n "$DIFF_OUTPUT" ]; then
        echo "Idempotency test failed for $FILE_PATH" >> "$LOG_FILE"
        echo "Diff details:" >> "$LOG_FILE"
        echo "$DIFF_OUTPUT" >> "$LOG_FILE"
        echo "Difference found in idempotency test for: $FILE_PATH"
    else
        echo "Idempotency test passed for $FILE_PATH"
    fi

    rm -f "$TMP1" "$TMP2"
}

# Detect a line break before a dot when the preceding expression is a pure
# dotted-identifier path. This is a syntactic guard against Apex interpreting
# a type or namespace path as a variable reference. It intentionally
# over-reports genuine variable heads; the formatter's structural rule glues
# those dots too, so zero matches is the expected result.
#
# Known limitation: a head sharing its line with other tokens (for example
# "return controller" followed by a broken dot) is not detected.
name_path_break_check() {
    python3 - "$TARGET_DIR" <<'PY'
import os
import re
import sys

root = sys.argv[1]
path_pattern = re.compile(r"^[ \t]*[A-Za-z_][A-Za-z0-9_]*(\s*\.\s*[A-Za-z_][A-Za-z0-9_]*)*[ \t]*$")
dot_pattern = re.compile(r"^[ \t]*\??\.[A-Za-z_]")
# `this` and `super` are value expressions, not type or namespace references.
# A newline before their member access is legal Apex (the whitespace-before-dot
# compile error applies only to type/namespace paths), so afmt intentionally
# breaks these chains. Only genuine type/namespace splits are violations, so
# chains whose head segment is one of these keywords are not flagged.
breakable_head_keywords = {"this", "super"}
violations = []

for directory, _, filenames in os.walk(root):
    for filename in filenames:
        if not filename.endswith((".cls", ".trigger")):
            continue
        path = os.path.join(directory, filename)
        with open(path, encoding="utf-8") as source_file:
            lines = source_file.readlines()

        for index, line in enumerate(lines):
            if not dot_pattern.match(line) or index == 0:
                continue
            previous = index - 1
            while previous >= 0 and not lines[previous].strip():
                previous -= 1
            if previous < 0 or not path_pattern.match(lines[previous]):
                continue
            head_segment = lines[previous].strip().split(".", 1)[0].strip()
            if head_segment in breakable_head_keywords:
                continue
            violations.append((path, previous + 1))

if violations:
    print(f"Name-path break check failed: {len(violations)} site(s)")
    for path, line_number in violations:
        print(f"  {path}:{line_number}")
    raise SystemExit(1)

print("Name-path break check passed: 0 sites")
PY
}

export -f format_files
export -f idempotent_test
export -f name_path_break_check
export FORMATTER_BINARY
export LOG_FILE
# export LONG_LINES_LOG_FILE
# export LINE_LENGTH

# Record the start time
START_TIME=$(date +%s)

# Find all .cls and .trigger files and process them in parallel
find "$TARGET_DIR" \( -type d \( -name ".sfdx" -o -name "scripts" \) \) -prune -o -type f \( -name "*.cls" -o -name "*.trigger" \) -print0 | \
    parallel -0 -j+0 format_files

# Run idempotent testing if mode is activated
if [ "$IDEMPOTENT_MODE" = true ]; then
    echo "Running idempotency tests..."
    find "$TARGET_DIR" \( -type d \( -name ".sfdx" -o -name "scripts" \) \) -prune -o -type f \( -name "*.cls" -o -name "*.trigger" \) -print0 | \
        parallel -0 -j+0 idempotent_test
fi

name_path_break_check

# find "$TARGET_DIR" -path "$TARGET_DIR/.sfdx" -prune -o -type f \( -name "*.cls" -o -name "*.trigger" \) -print0 | \
#     parallel -0 -j+0 format_files

# Record the end time and calculate the elapsed time
END_TIME=$(date +%s)
ELAPSED_TIME=$((END_TIME - START_TIME))

# Check if any errors were logged
if [ -s "$LOG_FILE" ]; then
    echo "Errors occurred during formatting. Check $LOG_FILE for details."
    echo "Script execution time: $ELAPSED_TIME seconds"
    exit 1
else
    echo "All files processed successfully."
    echo "Script execution time: $ELAPSED_TIME seconds"
    exit 0
fi
