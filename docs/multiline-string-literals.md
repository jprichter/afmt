# Multiline string literals

`afmt` supports the Apex Summer '26 triple-quoted multiline string literal
syntax (`'''...'''`). The parser dependency is `tree-sitter-sfapex 3.0.1`,
which provides the `multi_line_string_literal` node and protects content such
as `//` URLs from being parsed as comments.

## Formatting behavior

Multiline string nodes are handled through the existing general-expression and
literal paths. Their source value is emitted as a single text document, so the
triple-quote delimiters, internal line breaks, indentation, `${...}` text, and
other contents are preserved. The formatter does not interpret interpolation
inside the literal.

The text width used for surrounding layout is the number of bytes before the
first newline, rather than the total byte length of the multiline value. This
prevents the literal's later lines from causing unrelated wrapping decisions on
the line where the literal begins. The literal's own interior layout is not
re-indented by the formatter.

Multiline literals are supported in the grammar's general expression
productions. SOQL-specific literal and geolocation paths continue to use the
grammar's `string_literal` node because those productions do not accept
`multi_line_string_literal`.

## Regression coverage

`tests/static/MultilineString.in` and `.cls` cover a multiline assignment, a
method-call argument, interpolation-like text, and an `http://` URL inside the
literal. The static fixture scenario also formats the expected output again to
verify idempotency.

Run the focused static and idempotency checks with:

```bash
cargo test --locked --test test statics
cargo test --locked --test test idempotency_static
```

When changing this behavior, update the fixture pair and run the full locked
checks described in [validation-and-benchmarks.md](validation-and-benchmarks.md).
