# Technical Parts

## Parser

[Tree-sitter apex parser](https://github.com/aheber/tree-sitter-sfapex) is depended.

### Update parser version

The parser is consumed as the `tree-sitter-sfapex` crate, so there is no
vendored grammar to regenerate. Bump the version in `Cargo.toml`:

```toml
tree-sitter-sfapex = "2.4.0"
```

Then run `cargo update -p tree-sitter-sfapex` and the test suite. Grammar
changes are released upstream in
[tree-sitter-sfapex](https://github.com/aheber/tree-sitter-sfapex).

## Test

Afmt is heavily guarded by test scripts in `tests` folder

### Assert testing

`cargo test --test test --  --show-output`

### Battle testing

`tests/battle_test/battle_testing.sh` clones the repos listed in
`tests/battle_test/repos.txt` and formats them with one bulk command.

`./tests/battle_test/battle_testing.sh`
`./tests/battle_test/battle_testing.sh --idempotent`

See [Validation and local benchmarks](../docs/validation-and-benchmarks.md)
for requirements and the local bulk benchmark.

# Extra Info (might outdated)

## 📦 Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) must be installed.

### Steps

1. Clone the repository:
   ```bash
   git clone https://github.com/xixiaofinland/afmt.git
   cd afmt
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

## 🚀 Running the Formatter

### Get help:
```bash
./target/release/afmt --help
```

### Format a file:
```bash
./target/release/afmt path/to/your_apex_file.cls
```

### Run with enabled backtrace:
```bash
RUST_BACKTRACE=1 ./target/release/afmt path/to/your_apex_file.cls
```
<br>

