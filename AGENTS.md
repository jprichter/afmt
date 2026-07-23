# Agent guidance

`afmt` is a Rust CLI and library for formatting Salesforce Apex. Keep this
file as a navigation aid for coding agents; put durable behavior details in
the focused project documentation below.

- User-facing bulk formatting, configuration, discovery, and CLI semantics:
  [docs/project-wide-formatting.md](docs/project-wide-formatting.md)
- Test, battle-corpus, and local benchmark workflow:
  [docs/validation-and-benchmarks.md](docs/validation-and-benchmarks.md)
- Basic installation and usage examples: [README.md](README.md)

When changing project-wide formatting behavior, update the focused guide and
its relevant tests together. Keep `CLAUDE.md` unchanged.
