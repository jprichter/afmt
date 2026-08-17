# Comment placement and formatter idempotency

`afmt` treats stable output as a formatter contract: formatting already
formatted output must produce the same output. Comment attachment is part of
that contract, including when a comment sits between a leading token and the
following Apex node.

## Supported placements

- A line comment before a chained method name is emitted on its own line before
  the navigation operator and method name:

  ```apex
  new Builder()
    // explain the chained call
    .build();
  ```

- A block comment authored on the same line as an unbraced `else` remains
  trailing on the `else` keyword:

  ```apex
  else /* explain the fallback */
    fallback();
  ```

- Mixed block and line comments before a chained method preserve their authored
  order and line boundaries.

These placements are attachment behavior, not alternate brace or indentation
styles.

## Ignored nodes

`// afmt:ignore`, `//afmt:ignore`, and `/* afmt:ignore */` are recognized when
they are the last standalone pre-comment for a node. The node's original
source bytes are emitted unchanged, including internal blank lines, while the
marker itself is omitted from the output. A marker between annotations and a
declaration applies to the complete declaration, rather than to its first
modifier.

Multiline preserved source forces its surrounding groups to break so width
accounting remains correct. Its interior indentation is raw source and is not
re-anchored to the formatter's current indentation level.

If a recognized marker has no eligible following node, afmt retains the marker
and emits a warning on stderr. This is an intentional escape hatch from normal
formatting, so a deliberately non-canonical ignored node can be reformatted on
a later invocation: once an honored marker has been consumed, a new formatting
run has no way to identify that node. The idempotency guarantee applies when
the preserved bytes are already stable, as covered by
`idempotency_ignore_directive_stable_output`.

## Implementation boundaries

- `src/utility.rs::collect_comments` attaches same-row comments after an
  `else` token and marks that attachment for inline rendering.
- `src/data_model.rs::MethodInvocationKind::Complex` hoists line pre-comments
  ahead of a chained navigation operator and suppresses their duplicate
  emission from the method name.
- `src/data_model.rs::ValueNode::build_without_pre_comments` preserves the
  method name's remaining post-comments and punctuation when the pre-comments
  are hoisted.

## Regression coverage

The static fixtures under `tests/static/` cover the method-chain and unbraced
`else` placements, including both mixed-comment orders. The focused
`idempotency_static` test formats each fixture twice with the same
`tests/configs/.afmt_static.toml` configuration.

The ignore directive fixtures under `tests/ignore/` cover verbatim preservation
of deliberately non-canonical nodes, internal blank lines, multiline layout,
inner punctuation, annotation and loop targeting, and recognized marker
spellings. The `ignore` scenario, `ignored_inner_punctuation_is_idempotent`,
and `idempotency_ignore_directive_stable_output` tests exercise that coverage.

Run the focused checks while iterating:

```bash
cargo test --locked --test test idempotency_static
cargo test --locked --test test statics
cargo test --locked --test test comments
cargo test --locked --test test ignore
```

Run the complete locked Rust suite before handoff:

```bash
cargo test --locked --all-features
```

The larger battle test uses an external Apex corpus and requires network access;
run it separately when that corpus is available:

```bash
./tests/battle_test/battle_testing.sh --idempotent
```
