# Project-wide formatting

This guide documents the as-built behavior of the bulk `afmt` CLI. The
formatter still accepts a single file, but positional inputs are now one or
more files or directories.

## Inputs and discovery

- A directory is traversed recursively without following directory symlinks.
- Eligible files use the `.cls`, `.trigger`, `.apex`, or `.apexc` extensions.
- Explicit file inputs bypass the include globs, but exclusions still apply.
- Overlapping inputs are de-duplicated.
- Results are processed in deterministic normalized path order.
- All paths remain OS-native for file operations; glob matching uses portable
  `/` separators.
- A missing, unsupported, or otherwise invalid input is reported before
  formatting. An input set with no eligible files is an error.

The default exclusions are `.git`, `.sfdx`, and `node_modules`. Directory
symlinks are not followed during discovery. `afmt` does not read `.gitignore`.

## Configuration

Configuration is opt-in: pass `--config` (or `-c`) to load a TOML file. No
configuration file is discovered implicitly. Existing flat formatter keys
remain valid:

```toml
max_width = 80
indent_size = 4
brace_style = "k_and_r"            # or "allman"
wrap_single_statements = false      # add braces to bare clause bodies
indent_style = "space"              # or "tab"

[files]
include = ["**/*.cls", "**/*.trigger", "**/*.apex", "**/*.apexc"]
exclude = ["**/.git/**", "**/.sfdx/**", "**/node_modules/**"]
```

The `[files].include` and `[files].exclude` arrays replace their respective
defaults when supplied; they do not merge with them. Exclusions always win
over inclusions. Invalid glob syntax is a configuration error and prevents
formatting.

The style keys are independent and default to afmt's existing K&R braces,
brace-less single-statement clauses, and space indentation. With
`indent_style = "tab"`, `indent_size` controls the number of columns per indent
level. Line wrapping measures each emitted tab as one logical column, so visual
tab stops may differ from `max_width` calculations.

Allman formatting places property and accessor body braces on their own lines.
Compact auto-properties without accessor bodies keep their `{ get; set; }`
contents on one line.

With `brace_style = "allman"`, opening braces also move to their own lines for
classes, interfaces, enums, methods, constructors, triggers, static
initializers, control-flow blocks, `try`/`catch`/`finally`, `switch` blocks and
rules, and `System.runAs` blocks. `brace_style = "k_and_r"` preserves the
default same-line placement. With `wrap_single_statements = true`, bare
single statements in `if`/`else`, classic and enhanced `for`, `while`, and
`do` clauses receive brace blocks; existing blocks are not double-wrapped and
`else if` chains remain inline. Empty clause statements become empty blocks
so the result remains idempotent.

Patterns are matched against paths relative to the config file's parent when
`--config` is supplied, or relative to the process working directory when no
config is supplied. Patterns should use `/` separators so the same config is
portable across Windows, macOS, and Linux.

## Output and exit behavior

```text
afmt [OPTIONS] <PATH>...
```

- Without `--write` or `--check`, formatted source is written to stdout.
- A single-file invocation preserves the existing source-only output.
- A multi-file or directory invocation separates each source block with a
  deterministic `==> path <==` delimiter.
- `--write` writes changed files in place and leaves unchanged files alone.
  Bulk writes are best effort: failures are reported with their paths while
  other selected files continue.
- `--check` never writes and exits nonzero if any selected file would change,
  if discovery/formatting fails, or if no eligible files are selected. It
  conflicts with `--write`.
- `--time` writes per-file and total timing diagnostics to stderr. It does not
  alter formatted stdout.
- Bulk operations report selected, changed, written, unchanged, failed, and
  elapsed counts on stderr.

Exit status `0` means the requested operation completed successfully. Exit
status `1` indicates an application error, a changed file in check mode, or a
partial write failure.

## Implementation boundaries

`src/config.rs` owns application-level TOML loading and file-selection
defaults. `src/discovery.rs` owns path expansion, glob matching, exclusions,
de-duplication, and ordering. `src/formatter.rs` owns formatting and
path-aware per-file outcomes. `src/main.rs` owns CLI output, check/write
policy, timing, and aggregate status.
