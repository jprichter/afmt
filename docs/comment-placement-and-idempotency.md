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

## JavaDoc closing lines

JavaDoc blocks (`/** ... */`) normalize a closing line that contains only stars
before the terminator as an empty close. For example, `**/` and `***/` emit the
same normal closing line as `*/`, using the configured star-column style, rather
than creating a phantom content line such as `* *`. Text before the terminator
is still treated as closing-line content and is emitted before a separate close.

## Ignored nodes

A comment is a directive when its delimiters are stripped and its first
whitespace-separated token is `afmt:ignore`. That accepts `// afmt:ignore`,
`//afmt:ignore`, `/* afmt:ignore */`, and a trailing free-text reason such as
`// afmt:ignore column alignment is meaningful`. A near miss like
`// afmt:ignored` is not a directive and formats normally, so a marker that
does not take effect is either honored elsewhere or reported.

A directive applies when it is the last standalone pre-comment for a node. The
node's original source bytes are emitted unchanged, including internal blank
lines. A marker between annotations and a declaration applies to the complete
declaration, rather than to the node the parser happens to hand back next.
That node depends on what the declaration says: with an access modifier the
marker lands inside `modifiers`, in front of the first `modifier`; with
annotations alone `modifiers` closes after the annotation and the marker
becomes a sibling of the type. `ignore_promotion_target` recognizes both and
promotes to the declaration either way.

The marker is printed alongside the source it preserves, so ignoring a node is
idempotent: the marker is still there for the next run to find, and a file
produced by `afmt --write` passes `afmt --check`. `collect_comments` promotes
an annotation-adjacent marker into the enclosing declaration's bucket, which
places it inside the preserved span; `build_with_comments_core` detects that
case with `Comment::is_within` and lets the verbatim bytes carry the marker
instead of printing it twice.

Multiline preserved source forces its surrounding groups to break so width
accounting remains correct. Its interior indentation is raw source and is not
re-anchored to the formatter's current indentation level.

If a recognized marker has no eligible following node, afmt retains the marker
and emits a warning on stderr:

```
Warning: force-app/main/default/classes/Example.cls:12:5: afmt:ignore could not be applied; directive was preserved
```

The location is the marker's own one-based line and column. The prefix is the
path afmt was given, `<stdin>` when the source arrived on stdin, and absent
for a library caller that formatted a string through
`Formatter::try_format_source`; `Formatter::try_format_source_with_origin`
supplies one.

An applied marker never warns; `Comment` tracks that with `ignore_honored`,
which `build_with_comments_core` sets before the marker is printed.

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
inner punctuation, annotation and loop targeting, declarations carrying an
annotation with no access modifier, and recognized marker spellings including a
trailing reason. Because the marker is preserved, every fixture is expected to
round-trip: `ignore_directive_regressions_match_expected_output` asserts each
one matches its `.cls` and is unchanged by a second pass. The `ignore` scenario
(also part of `all`), `ignored_inner_punctuation_is_idempotent`, and
`idempotency_ignore_directive_stable_output` exercise the rest of that
coverage.

The JavaDoc fixture under `tests/comments/` covers star-only closing lines such
as `**/`; `idempotency_comments` verifies that their normalized output is a
fixed point.

Three tests hold the stderr contract:
`honored_ignore_directive_does_not_warn` asserts an applied marker stays
silent, `unhonored_ignore_directive_warns_and_is_preserved` asserts the marker
survives the run that warns about it, and
`unhonored_ignore_directive_warning_locates_each_file` plus
`unhonored_ignore_directive_warning_labels_stdin` pin the `path:line:column`
prefix over a directory of several files and over stdin.

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
