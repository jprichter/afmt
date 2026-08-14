# Formatter architecture

`afmt` keeps source-to-source formatting separate from file and batch
adaptation. The split is internal, but the existing public `formatter` module
continues to provide the 1.x API surface.

## Source formatting core

`src/source_formatter.rs` owns the source-only pipeline:

- `Config` data, defaults, and formatter-option validation.
- Apex parsing, comment enrichment, formatting-session construction, and
  pretty-printing.
- Source-formatting error and panic isolation for `try_format_source`.

The core accepts source text and a formatter configuration. It does not read or
write files, construct path-aware errors, measure elapsed time, or invoke
Rayon. Direct source-core tests live beside this module and do not need a
filesystem fixture.

## File and batch adapter

`src/formatter.rs` is the public compatibility facade and file/batch adapter.
It owns:

- `Formatter` construction from source paths and configuration files.
- File reads and `FormattedFile` change detection.
- `FormatFileError` path decoration and `FormatOutcome` timing.
- Rayon-based batch execution while preserving input ordering.

Source-formatting failures are returned by the core first; the adapter adds a
path only when it creates a `FormatFileError`. `formatter::Config` and the
existing `Formatter` methods remain available to 1.x consumers through
re-exports and thin forwarding methods. `Config::from_file` remains an
adapter-side compatibility method because it performs filesystem I/O; TOML
loading for project-wide selection is handled by `src/config.rs`.

## Other boundaries

- `src/config.rs` owns application-level TOML loading and file-selection
  defaults.
- `src/discovery.rs` owns path expansion, glob matching, exclusions,
  de-duplication, and ordering.
- `src/main.rs` owns CLI selection, output, check/write policy, timing display,
  and aggregate status.

Keep future source-formatting changes in the source core when they only need a
buffer and `Config`. Keep filesystem, path, timing, and batch concerns at the
adapter or CLI boundaries.
