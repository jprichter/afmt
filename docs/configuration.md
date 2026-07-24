# Formatter configuration

`afmt` reads configuration only when a file is explicitly supplied with
`--config` or `-c`. It does not discover `.afmt.toml` implicitly. Omitted
options retain the formatter defaults.

```toml
max_width = 80
indent_size = 2

# Optional style controls
brace_style = "k_and_r"            # or "allman"
wrap_single_statements = false      # or true
indent_style = "space"              # or "tab"
javadoc_star_column = "offset"      # or "flush"
```

`brace_style = "allman"` puts opening braces on their own lines. The default
`"k_and_r"` keeps them on the construct's header line.

`wrap_single_statements = true` adds braces around otherwise bare `if`,
`else`, and loop bodies. The default is `false`.

`indent_style` selects spaces or tabs for emitted indentation. `indent_size`
controls the logical indentation width; with tabs, it is the number of columns
represented by each indentation level.

`javadoc_star_column` controls only the leading star on continuation lines in
Javadoc (`/** ... */`) comments:

- `"offset"` (default) emits ` * content`, preserving existing output.
- `"flush"` emits `* content`, aligning the star with the comment indentation.

In both modes, afmt normalizes the separator after the star to one space.
Blank Javadoc lines and the closing line follow the same star-column choice.
Non-Javadoc block comments and line comments are unaffected by this option.
