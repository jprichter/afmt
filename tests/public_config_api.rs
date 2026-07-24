use sf_afmt::formatter::{BraceStyle, Config, Formatter, IndentStyle, JavadocStarColumn};

#[test]
fn public_style_enums_can_configure_the_library() {
    let config = Config {
        brace_style: BraceStyle::Allman,
        indent_style: IndentStyle::Tab,
        javadoc_star_column: JavadocStarColumn::Flush,
        ..Config::default()
    };

    let formatted = Formatter::format_one("class T { }\n", config);

    assert_eq!(formatted, "class T\n{\n}\n");
}
