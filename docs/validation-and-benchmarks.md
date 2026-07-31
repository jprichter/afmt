# Validation and local benchmarks

Use the repository's Rust checks for normal changes. The focused tests cover
configuration, discovery, CLI behavior, formatter outcomes, and fixture
idempotency; the broader command runs the complete locked test set.

```bash
cargo test --locked --all-features
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
```

The battle test is an external-corpus gate. It clones the repositories listed
in `tests/battle_test/repos.txt` into the disposable
`tests/battle_test/repos` directory, runs the bulk command, and retains a
per-file diagnostic fallback for tolerated managed-package template failures.
The bulk command writes the formatted output into the disposable clones before
the structural checks inspect them.
Use `--idempotent` to run bulk `--write` followed by bulk `--check`:

```bash
./tests/battle_test/battle_testing.sh --idempotent
```

The battle corpus is not part of the repository and requires network access,
Git, GNU Parallel, and a Unix-like shell. Do not treat an unavailable corpus
as a product-code failure.

## Local bulk benchmark

`tests/battle_test/benchmark_bulk.sh` builds `target/release/afmt` once and
compares the legacy sequential process-per-file dry run with one bulk dry run
over the same local corpus, config, extensions, and exclusions. It performs a
warm-up for each mode, then five measured runs by default. The corpus is not
modified.

```bash
AFMT_BENCH_RUNS=5 \
  ./tests/battle_test/benchmark_bulk.sh /path/to/local/apex-repo
```

`AFMT_BENCH_RUNS` must be an odd integer of at least three so the reported
median is unambiguous. The benchmark reports the selected file count, every
measured run, both medians, and relative speedup. Build, clone, and download
time is outside the measured runs. Required tools are Bash 4+, Cargo/Rust,
`find`, `sort`, `awk`, and either nanosecond-capable `date` or Python 3.

The lightweight `tests/battle_test/benchmark_bulk_test.sh` checks run-count
validation without building the release binary.
