use crate::context::CommentMap;
use crate::data_model::*;
use crate::doc::{pretty_print, BraceStyle, IndentStyle, PrettyConfig};
use crate::doc_builder::DocBuilder;
use crate::message_helper::{red, yellow};
use crate::utility::{
    assert_no_missing_comments, collect_comments, enrich, set_thread_comment_map,
    set_thread_source_code, truncate_snippet,
};
use serde::Deserialize;
use std::sync::mpsc;
use std::thread;
use std::{fs, path::Path};
use tree_sitter::{Node, Parser, Tree};

#[allow(unused_imports)]
use crate::utility::print_comment_map;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_max_width")]
    pub max_width: u32,

    #[serde(default = "default_indent_size")]
    pub indent_size: u32,

    /// Opening-brace placement. Default `k_and_r` preserves current output.
    #[serde(default)]
    pub brace_style: BraceStyle,

    /// Wrap single-statement `if`/`else`/loop bodies in braces. Default `false`.
    #[serde(default)]
    pub wrap_single_statements: bool,

    /// Indentation character. Default `space`.
    #[serde(default)]
    pub indent_style: IndentStyle,
}

fn default_max_width() -> u32 {
    80
}

fn default_indent_size() -> u32 {
    2
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_width: default_max_width(),
            indent_size: default_indent_size(),
            brace_style: BraceStyle::default(),
            wrap_single_statements: false,
            indent_style: IndentStyle::default(),
        }
    }
}

impl Config {
    pub fn new(max_width: u32) -> Self {
        Self {
            max_width,
            indent_size: 2,
            brace_style: BraceStyle::default(),
            wrap_single_statements: false,
            indent_style: IndentStyle::default(),
        }
    }

    pub fn from_file(path: &str) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {}", e))?;
        let config: Config =
            toml::from_str(&content).map_err(|e| format!("Failed to parse config file: {}", e))?;
        Ok(config)
    }

    pub fn max_width(&self) -> u32 {
        self.max_width
    }

    pub fn indent_size(&self) -> u32 {
        self.indent_size
    }
}

#[derive(Clone, Debug)]
pub struct Formatter {
    config: Config,
    source_files: Vec<String>,
    //pub errors: ReportedErrors,
}

impl Formatter {
    pub fn new(config: Config, source_files: Vec<String>) -> Self {
        Self {
            config,
            source_files,
            //errors: ReportedErrors::default(),
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn create_from_config(
        config_path: Option<&str>,
        source_files: Vec<String>,
    ) -> Result<Formatter, String> {
        let config = match config_path {
            Some(path) => Config::from_file(path)
                .map_err(|e| format!("{}: {}", yellow(&e.to_string()), path))?,
            None => Config::default(),
        };
        Ok(Formatter::new(config, source_files))
    }

    pub fn format(&self) -> Vec<Result<String, String>> {
        let (tx, rx) = mpsc::channel();
        let config = self.config.clone();

        for file in &self.source_files {
            let tx = tx.clone();
            let config = config.clone();
            let file = file.clone();

            thread::spawn(move || {
                let result = std::panic::catch_unwind(|| {
                    let source_code = fs::read_to_string(Path::new(&file))
                        .map_err(|e| {
                            format!(
                                "Failed to read file: {} {}",
                                red(&file),
                                yellow(e.to_string().as_str())
                            )
                        })
                        .unwrap();

                    Formatter::format_one(&source_code, config)
                });
                match result {
                    Ok(result) => {
                        tx.send(Ok(result)).expect("failed to send result in tx");
                    }
                    Err(_) => tx
                        .send(Err("Thread panicked".to_string()))
                        .expect("failed to send error in tx"),
                }
            });
        }

        drop(tx);

        rx.into_iter().collect()
    }

    pub fn format_one(source_code: &str, config: Config) -> String {
        let ast_tree = Formatter::parse(source_code);
        set_thread_source_code(source_code.to_string()); // important to set thread level source code now;

        let mut cursor = ast_tree.walk();
        let mut comment_map = CommentMap::new();
        collect_comments(&mut cursor, &mut comment_map);
        set_thread_comment_map(comment_map); // important to set thread level comment map;

        // traverse the tree to build enriched data
        let root: Root = enrich(&ast_tree);

        // traverse enriched data and create pretty print combinators
        let c = PrettyConfig::new(
            config.indent_size,
            config.brace_style,
            config.wrap_single_statements,
            config.indent_style,
        );
        let b = DocBuilder::new(c);
        let doc_ref = root.build(&b);

        let result = pretty_print(doc_ref, config.max_width, c);

        // debugging tool: use this to print named node value + comments in bucket
        // print_comment_map(&ast_tree);

        assert_no_missing_comments();

        result
    }

    pub fn parse(source_code: &str) -> Tree {
        let mut parser = Parser::new();
        let language_fn = tree_sitter_sfapex::apex::LANGUAGE;
        parser
            .set_language(&language_fn.into())
            .expect("Error loading Apex parser");

        let ast_tree = parser.parse(source_code, None).unwrap();
        let root_node = &ast_tree.root_node();

        if root_node.has_error() {
            if let Some(error_node) = Self::find_last_error_node(root_node) {
                let error_snippet =
                    truncate_snippet(&source_code[error_node.start_byte()..error_node.end_byte()]);
                println!(
                    "Error in node kind: {}, at byte range: {}-{}, snippet: {}",
                    yellow(error_node.kind()),
                    error_node.start_byte(),
                    error_node.end_byte(),
                    error_snippet,
                );
                if let Some(p) = error_node.parent() {
                    let parent_snippet =
                        truncate_snippet(&source_code[p.start_byte()..p.end_byte()]);
                    println!(
                        "Parent node kind: {}, at byte range: {}-{}, snippet: {}",
                        yellow(p.kind()),
                        p.start_byte(),
                        p.end_byte(),
                        parent_snippet,
                    );
                }
            }
            panic!("{}", red("Parser encounters an error node in the tree."));
        }

        ast_tree
    }

    fn find_last_error_node<'tree>(node: &Node<'tree>) -> Option<Node<'tree>> {
        if !node.has_error() {
            return None; // If the current node has no error, return None
        }

        let mut last_error_node = Some(*node);

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.has_error() {
                    last_error_node = Self::find_last_error_node(&child);
                }
            }
        }

        last_error_node // Return the last (deepest) error node
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Formatter};
    use crate::doc::{BraceStyle, IndentStyle};

    #[test]
    fn style_keys_default_when_omitted() {
        let config: Config = toml::from_str("max_width = 80\n").unwrap();

        assert_eq!(config.brace_style, BraceStyle::KAndR);
        assert_eq!(config.indent_style, IndentStyle::Space);
        assert!(!config.wrap_single_statements);
    }

    #[test]
    fn style_keys_parse_from_snake_case() {
        let config: Config = toml::from_str(
            "brace_style = \"allman\"\nindent_style = \"tab\"\nwrap_single_statements = true\n",
        )
        .unwrap();

        assert_eq!(config.brace_style, BraceStyle::Allman);
        assert_eq!(config.indent_style, IndentStyle::Tab);
        assert!(config.wrap_single_statements);
    }

    #[test]
    fn invalid_brace_style_is_an_error() {
        let result: Result<Config, _> = toml::from_str("brace_style = \"stroustrup\"\n");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_indent_style_is_an_error() {
        let result: Result<Config, _> = toml::from_str("indent_style = \"spaces\"\n");
        assert!(result.is_err());
    }

    #[test]
    fn allman_moves_container_and_control_braces_to_their_own_line() {
        let source = "class T {\n  void m() {\n    if (a) { x(); }\n  }\n}\n";
        let config = Config {
            brace_style: BraceStyle::Allman,
            ..Config::default()
        };

        let out = Formatter::format_one(source, config);

        assert_eq!(
            out,
            "class T\n{\n  void m()\n  {\n    if (a)\n    {\n      x();\n    }\n  }\n}\n"
        );
    }

    #[test]
    fn wrap_single_statements_adds_braces_to_bare_clause_bodies() {
        let source = "class T {\n  void m() {\n    if (a) x(); else y();\n    for (Account acc : accts) z();\n  }\n}\n";
        let config = Config {
            wrap_single_statements: true,
            ..Config::default()
        };

        let out = Formatter::format_one(source, config);

        // Bare `if`/`else` and loop bodies each gain a brace block; else-if stays
        // inline (none here). K&R placement is preserved (default brace_style).
        assert!(out.contains("if (a) {\n      x();\n    } else {\n      y();\n    }"));
        assert!(out.contains("for (Account acc : accts) {\n      z();\n    }"));
    }

    #[test]
    fn tab_indentation_is_independent_of_brace_style_and_wrapping() {
        let source = "class T {\n  void m() {\n    if (a) x();\n  }\n}\n";
        let config = Config {
            indent_style: IndentStyle::Tab,
            ..Config::default()
        };

        let out = Formatter::format_one(source, config);

        assert_eq!(
            out,
            "class T {\n\tvoid m() {\n\t\tif (a)\n\t\t\tx();\n\t}\n}\n"
        );
    }

    #[test]
    fn allman_applies_to_properties_and_accessor_bodies() {
        let source = "public class Me {\n  public integer prop {\n    get {\n      return prop;\n    }\n    set {\n      prop = value;\n    }\n  }\n}\n";
        let config = Config {
            brace_style: BraceStyle::Allman,
            ..Config::default()
        };

        let out = Formatter::format_one(source, config);

        assert!(out.contains("public integer prop\n  {"));
        assert!(out.contains("get\n    {"));
        assert!(out.contains("set\n    {"));
    }

    #[test]
    fn wrapped_single_statements_include_empty_loop_bodies() {
        let source = "class T {\n  void m() {\n    for (Integer i = 0; i < 1; i++);\n    for (Account acc : accts);\n    while (true);\n  }\n}\n";
        let config = Config {
            wrap_single_statements: true,
            ..Config::default()
        };

        let out = Formatter::format_one(source, config);

        assert_eq!(out.matches("{\n    }").count(), 3);
    }

    #[test]
    fn wrapped_empty_while_preserves_terminator_comments() {
        let source = "class T {\n  void m() {\n    while (true) /* while empty */ ;\n  }\n}\n";
        let config = Config {
            wrap_single_statements: true,
            ..Config::default()
        };

        let out = Formatter::format_one(source, config);

        assert!(out.contains("/* while empty */"));
        assert!(out.contains("while (true) /* while empty */"));
    }

    #[test]
    fn allman_places_catch_and_finally_on_new_lines() {
        let source = "class T {\n  void m() {\n    try {\n      work();\n    } catch (Exception e) {\n      recover();\n    } finally {\n      finish();\n    }\n  }\n}\n";
        let config = Config {
            brace_style: BraceStyle::Allman,
            ..Config::default()
        };

        let out = Formatter::format_one(source, config);

        assert!(out.contains("    }\n    catch (Exception e)\n    {"));
        assert!(out.contains("    }\n    finally\n    {"));
    }

    #[test]
    fn default_config_keeps_k_and_r_and_bare_bodies() {
        let source = "class T {\n  void m() {\n    if (a) x();\n  }\n}\n";

        let out = Formatter::format_one(source, Config::default());

        assert_eq!(
            out,
            "class T {\n  void m() {\n    if (a)\n      x();\n  }\n}\n"
        );
    }
}
