# Battle tests

`battle_testing.sh` clones the disposable repositories listed in
`repos.txt`, runs one bulk directory formatting command, and redirects
formatted stdout. The selection is configured by `.afmt.toml`; it preserves
the legacy `.sfdx` and `scripts` pruning in addition to afmt's standard
repository exclusions.

If the bulk command reports a failure, the script captures its stderr and runs
the legacy per-file classifier. That fallback recognizes tolerated managed
package template markers such as `%%` while retaining detailed diagnostics for
unexpected files. A successful bulk command does not start per-file processes.

For disposable clones, `--idempotent` runs bulk `--write` followed by bulk
`--check`. If either bulk command fails, the existing per-file idempotency
diagnostic runs to identify the affected files.

## Local benchmark

The benchmark builds the release binary once, then compares a sequential
process-per-file dry run with one bulk dry run over the same corpus and
exclusions. It performs one warm-up and five measured runs by default, and
reports each run, file count, median wall time, and relative speedup. The
measured run count must be odd and at least three so the reported median is
unambiguous; set `AFMT_BENCH_RUNS` to another odd count when needed.

Required tools: Bash 4+, Cargo/Rust, `find`, `sort`, `awk`, and either GNU
`date` with nanosecond output or Python 3. Example:

```bash
AFMT_BENCH_RUNS=7 ./tests/battle_test/benchmark_bulk.sh /path/to/local/apex-repo
```

The benchmark performs dry runs only; the corpus is not modified. Build,
clone, and download time is excluded from measured runs.
