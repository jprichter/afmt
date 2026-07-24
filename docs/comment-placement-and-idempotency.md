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

Run the focused checks while iterating:

```bash
cargo test --locked --test test idempotency_static
cargo test --locked --test test statics
cargo test --locked --test test comments
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
