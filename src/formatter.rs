use crate::context::CommentMap;
use crate::data_model::*;
use crate::doc::{pretty_print, PrettyConfig};
use crate::doc_builder::DocBuilder;
use crate::message_helper::{red, yellow};
use crate::utility::{
    assert_no_missing_comments, clear_thread_comment_map, clear_thread_source_code,
    collect_comments, enrich, set_thread_comment_map, set_thread_source_code, truncate_snippet,
};
use rayon::prelude::*;
use serde::Deserialize;
use std::{
    fs,
    panic::{catch_unwind, AssertUnwindSafe},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tree_sitter::{Node, Parser, Tree};

pub use crate::doc::{BraceStyle, IndentStyle, JavadocStarColumn};

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

    /// JavaDoc continuation-star placement. Default `offset` preserves output.
    #[serde(default)]
    pub javadoc_star_column: JavadocStarColumn,

    /// Normalize known Apex annotation names to Salesforce's canonical casing.
    /// Default `false` preserves source casing.
    #[serde(default)]
    pub normalize_annotation_casing: bool,
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
            javadoc_star_column: JavadocStarColumn::default(),
            normalize_annotation_casing: false,
        }
    }
}

impl Config {
    pub fn new(max_width: u32) -> Self {
        Self {
            max_width,
            ..Config::default()
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.indent_size == 0 {
            return Err("indent_size must be at least 1".to_string());
        }

        Ok(())
    }

    pub fn from_file(path: &str) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {}", e))?;
        let config: Config =
            toml::from_str(&content).map_err(|e| format!("Failed to parse config file: {}", e))?;
        config
            .validate()
            .map_err(|error| format!("Invalid formatter configuration: {error}"))?;
        Ok(config)
    }

    pub fn max_width(&self) -> u32 {
        self.max_width
    }

    pub fn indent_size(&self) -> u32 {
        self.indent_size
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormattedFile {
    pub content: String,
    pub changed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatFileError {
    pub path: PathBuf,
    pub message: String,
}

impl std::fmt::Display for FormatFileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.path.display(), self.message)
    }
}

#[derive(Clone, Debug)]
pub struct FormatOutcome {
    pub path: PathBuf,
    pub elapsed: Duration,
    pub result: Result<FormattedFile, FormatFileError>,
}

struct ThreadStateGuard;

impl Drop for ThreadStateGuard {
    fn drop(&mut self) {
        clear_thread_source_code();
        clear_thread_comment_map();
    }
}

#[derive(Clone, Debug)]
pub struct Formatter {
    config: Config,
    source_files: Vec<PathBuf>,
    //pub errors: ReportedErrors,
}

impl Formatter {
    pub fn new(config: Config, source_files: Vec<String>) -> Self {
        Self::new_from_paths(
            config,
            source_files.into_iter().map(PathBuf::from).collect(),
        )
    }

    pub fn new_from_paths(config: Config, source_files: Vec<PathBuf>) -> Self {
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
        self.format_with_outcomes()
            .into_iter()
            .map(|outcome| {
                outcome
                    .result
                    .map(|formatted| formatted.content)
                    .map_err(|error| error.to_string())
            })
            .collect()
    }

    pub fn format_with_outcomes(&self) -> Vec<FormatOutcome> {
        let config = &self.config;

        self.source_files
            .par_iter()
            .map(|file| {
                let path = file.clone();
                let started = Instant::now();
                let result = Self::try_format_one(&path, config.clone());

                FormatOutcome {
                    path,
                    elapsed: started.elapsed(),
                    result,
                }
            })
            .collect()
    }

    pub fn try_format_one(path: &Path, config: Config) -> Result<FormattedFile, FormatFileError> {
        if let Err(error) = config.validate() {
            return Err(FormatFileError {
                path: path.to_path_buf(),
                message: format!("Invalid formatter configuration: {error}"),
            });
        }

        let source_code = fs::read_to_string(path).map_err(|error| FormatFileError {
            path: path.to_path_buf(),
            message: format!(
                "Failed to read file: {}",
                yellow(error.to_string().as_str())
            ),
        })?;

        let formatted = match catch_unwind(AssertUnwindSafe(|| {
            Self::try_format_source(&source_code, config)
        })) {
            Ok(Ok(formatted)) => formatted,
            Ok(Err(message)) => {
                return Err(FormatFileError {
                    path: path.to_path_buf(),
                    message,
                });
            }
            Err(panic_payload) => {
                return Err(FormatFileError {
                    path: path.to_path_buf(),
                    message: format!("Formatting panicked: {}", panic_message(panic_payload)),
                });
            }
        };

        Ok(FormattedFile {
            changed: source_code != formatted,
            content: formatted,
        })
    }

    pub fn format_one(source_code: &str, config: Config) -> String {
        Self::try_format_source(source_code, config).unwrap_or_else(|message| panic!("{}", message))
    }

    fn try_format_source(source_code: &str, config: Config) -> Result<String, String> {
        config
            .validate()
            .map_err(|error| format!("Invalid formatter configuration: {error}"))?;

        let ast_tree = Self::try_parse(source_code)?;
        clear_thread_source_code();
        clear_thread_comment_map();
        let _thread_state_guard = ThreadStateGuard;

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
            config.javadoc_star_column,
            config.normalize_annotation_casing,
        );
        let b = DocBuilder::new(c);
        let doc_ref = root.build(&b);

        let result = pretty_print(doc_ref, config.max_width, c);

        // debugging tool: use this to print named node value + comments in bucket
        // print_comment_map(&ast_tree);

        assert_no_missing_comments();

        Ok(result)
    }

    pub fn parse(source_code: &str) -> Tree {
        Self::try_parse(source_code).unwrap_or_else(|message| panic!("{}", message))
    }

    fn try_parse(source_code: &str) -> Result<Tree, String> {
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
                let mut diagnostic = format!(
                    "Error in node kind: {}, at byte range: {}-{}, snippet: {}",
                    yellow(error_node.kind()),
                    error_node.start_byte(),
                    error_node.end_byte(),
                    error_snippet,
                );
                if let Some(p) = error_node.parent() {
                    let parent_snippet =
                        truncate_snippet(&source_code[p.start_byte()..p.end_byte()]);
                    diagnostic.push_str(&format!(
                        "\nParent node kind: {}, at byte range: {}-{}, snippet: {}",
                        yellow(p.kind()),
                        p.start_byte(),
                        p.end_byte(),
                        parent_snippet,
                    ));
                }
                return Err(format!(
                    "{}\n{}",
                    diagnostic,
                    red("Parser encounters an error node in the tree.")
                ));
            }
        }

        Ok(ast_tree)
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

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Formatter};
    use crate::doc::{BraceStyle, IndentStyle, JavadocStarColumn};
    use rayon::prelude::*;
    #[cfg(unix)]
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temporary_directory() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("sf-afmt-phase1-{unique}"));
        fs::create_dir(&directory).expect("temporary directory should be created");
        directory
    }

    #[test]
    fn path_aware_results_keep_input_order_and_content_associations() {
        let directory = temporary_directory();
        let source = include_str!("../tests/static/variable_declaration.in");
        let first_path = directory.join("first.cls");
        let second_path = directory.join("second.cls");
        fs::write(&first_path, source.replace("class A", "class First"))
            .expect("first source should be written");
        fs::write(&second_path, source.replace("class A", "class Second"))
            .expect("second source should be written");

        let formatter = Formatter::new(
            Config::default(),
            vec![
                first_path.to_string_lossy().into_owned(),
                second_path.to_string_lossy().into_owned(),
            ],
        );
        let outcomes = formatter.format_with_outcomes();

        assert_eq!(outcomes.len(), 2);
        assert_eq!(outcomes[0].path, first_path);
        assert_eq!(outcomes[1].path, second_path);
        assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()));
        assert!(outcomes.iter().all(|outcome| {
            outcome
                .result
                .as_ref()
                .expect("outcome should succeed")
                .changed
        }));
        assert!(outcomes[0]
            .result
            .as_ref()
            .expect("first outcome should succeed")
            .content
            .contains("class First"));
        assert!(outcomes[1]
            .result
            .as_ref()
            .expect("second outcome should succeed")
            .content
            .contains("class Second"));

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn compatibility_format_returns_results_in_input_order() {
        let directory = temporary_directory();
        let source = include_str!("../tests/static/variable_declaration.in");
        let first_path = directory.join("first.cls");
        let second_path = directory.join("second.cls");
        fs::write(&first_path, source.replace("class A", "class First"))
            .expect("first source should be written");
        fs::write(&second_path, source.replace("class A", "class Second"))
            .expect("second source should be written");

        let formatter = Formatter::new(
            Config::default(),
            vec![
                first_path.to_string_lossy().into_owned(),
                second_path.to_string_lossy().into_owned(),
            ],
        );
        let results = crate::format(formatter);

        assert_eq!(results.len(), 2);
        assert!(results[0]
            .as_ref()
            .expect("first result should succeed")
            .contains("class First"));
        assert!(results[1]
            .as_ref()
            .expect("second result should succeed")
            .contains("class Second"));

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn formatted_results_report_unchanged_content() {
        let directory = temporary_directory();
        let source = include_str!("../tests/static/variable_declaration.in");
        let path = directory.join("formatted.cls");
        fs::write(&path, source).expect("source should be written");

        let first = Formatter::try_format_one(&path, Config::default())
            .expect("first formatting should succeed");
        fs::write(&path, &first.content).expect("formatted source should be written");
        let second = Formatter::try_format_one(&path, Config::default())
            .expect("second formatting should succeed");

        assert!(first.changed);
        assert!(!second.changed);
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn parse_errors_are_path_aware_and_do_not_stop_other_files() {
        let directory = temporary_directory();
        let invalid_path = directory.join("invalid.cls");
        let valid_path = directory.join("valid.cls");
        fs::write(&invalid_path, "class Broken {").expect("invalid source should be written");
        fs::write(
            &valid_path,
            include_str!("../tests/static/variable_declaration.in"),
        )
        .expect("valid source should be written");

        let outcomes = Formatter::new(
            Config::default(),
            vec![
                invalid_path.to_string_lossy().into_owned(),
                valid_path.to_string_lossy().into_owned(),
            ],
        )
        .format_with_outcomes();

        let error = outcomes[0]
            .result
            .as_ref()
            .expect_err("invalid source should fail");
        assert_eq!(error.path, invalid_path);
        assert!(error.message.contains("byte range"));
        assert!(error.message.contains("Parser encounters an error node"));
        assert!(outcomes[1].result.is_ok());

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn invalid_configuration_is_path_aware_without_panic_rewrite() {
        let directory = temporary_directory();
        let path = directory.join("panic.cls");
        fs::write(
            &path,
            include_str!("../tests/static/variable_declaration.in"),
        )
        .expect("source should be written");

        let error = Formatter::try_format_one(
            &path,
            Config {
                max_width: 80,
                indent_size: 0,
                ..Config::default()
            },
        )
        .expect_err("invalid formatter configuration should return an error");

        assert_eq!(error.path, path);
        assert!(error
            .message
            .contains("Invalid formatter configuration: indent_size must be at least 1"));
        assert!(!error.message.contains("Formatting panicked"));
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn config_validation_accepts_lower_bound_and_zero_width() {
        let config = Config {
            max_width: 0,
            indent_size: 1,
            ..Config::default()
        };

        assert_eq!(config.validate(), Ok(()));
    }

    #[test]
    fn config_validation_rejects_zero_indent() {
        assert_eq!(
            Config {
                indent_size: 0,
                ..Config::default()
            }
            .validate(),
            Err("indent_size must be at least 1".to_string())
        );
    }

    #[test]
    fn config_file_rejects_zero_indent_as_configuration_error() {
        let directory = temporary_directory();
        let path = directory.join(".afmt.toml");
        fs::write(&path, "indent_size = 0\n").unwrap();

        let error = Config::from_file(path.to_str().unwrap()).unwrap_err();

        assert!(error.contains("Invalid formatter configuration"));
        assert!(error.contains("indent_size must be at least 1"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn reused_workers_release_formatting_state() {
        let directory = temporary_directory();
        let path = directory.join("reused.cls");
        fs::write(
            &path,
            include_str!("../tests/static/variable_declaration.in"),
        )
        .expect("source should be written");

        let cleanup_checks = (0..32)
            .into_par_iter()
            .map(|_| {
                let result = Formatter::try_format_one(&path, Config::default());
                (result.is_ok(), crate::utility::thread_state_is_empty())
            })
            .collect::<Vec<_>>();

        assert!(cleanup_checks
            .iter()
            .all(|(formatted, cleaned)| *formatted && *cleaned));
        assert!(crate::utility::thread_state_is_empty());
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn path_native_constructor_preserves_non_utf8_paths() {
        let directory = temporary_directory();
        let path = directory.join(OsString::from_vec(vec![
            b'n', b'o', b'n', b'-', 0xff, b'.', b'c', b'l', b's',
        ]));
        fs::write(
            &path,
            include_str!("../tests/static/variable_declaration.in"),
        )
        .expect("source should be written");

        let outcome = Formatter::new_from_paths(Config::default(), vec![path.clone()])
            .format_with_outcomes()
            .pop()
            .expect("one outcome should be returned");

        assert_eq!(outcome.path, path);
        assert!(outcome.result.is_ok());
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn path_aware_results_include_read_errors() {
        let path = std::env::temp_dir().join("sf-afmt-phase1-file-that-does-not-exist.cls");
        let outcome = Formatter::new(Config::default(), vec![path.to_string_lossy().into_owned()])
            .format_with_outcomes()
            .pop()
            .expect("one outcome should be returned");

        let error = outcome.result.expect_err("missing input should fail");
        assert_eq!(error.path, path);
        assert!(error.message.contains("Failed to read file"));
    }

    #[test]
    fn style_keys_default_when_omitted() {
        let config: Config = toml::from_str("max_width = 80\n").unwrap();

        assert_eq!(config.brace_style, BraceStyle::KAndR);
        assert_eq!(config.indent_style, IndentStyle::Space);
        assert_eq!(config.javadoc_star_column, JavadocStarColumn::Offset);
        assert!(!config.wrap_single_statements);
        assert!(!config.normalize_annotation_casing);
    }

    #[test]
    fn style_keys_parse_from_snake_case() {
        let config: Config = toml::from_str(
            "brace_style = \"allman\"\nindent_style = \"tab\"\nwrap_single_statements = true\njavadoc_star_column = \"flush\"\nnormalize_annotation_casing = true\n",
        )
        .unwrap();

        assert_eq!(config.brace_style, BraceStyle::Allman);
        assert_eq!(config.indent_style, IndentStyle::Tab);
        assert_eq!(config.javadoc_star_column, JavadocStarColumn::Flush);
        assert!(config.wrap_single_statements);
        assert!(config.normalize_annotation_casing);
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
    fn invalid_javadoc_star_column_is_an_error() {
        let result: Result<Config, _> = toml::from_str("javadoc_star_column = \"aligned\"\n");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_annotation_casing_value_is_an_error() {
        let result: Result<Config, _> = toml::from_str("normalize_annotation_casing = \"true\"\n");
        assert!(result.is_err());
    }

    #[test]
    fn flush_javadoc_stars_are_indented_and_idempotent() {
        let source = "class T {\n  /**\n   * @param value the input\n   *\n   * @return the result\n   */\n  Integer m(Integer value) { /* keep */ return value; }\n}\n";
        let config = Config {
            brace_style: BraceStyle::Allman,
            indent_style: IndentStyle::Tab,
            javadoc_star_column: JavadocStarColumn::Flush,
            ..Config::default()
        };

        let first = std::thread::spawn({
            let source = source.to_owned();
            let config = config.clone();
            move || Formatter::format_one(&source, config)
        })
        .join()
        .unwrap();

        assert!(
            first.contains("\t/**\n\t* @param value the input\n\t*\n\t* @return the result\n\t*/")
        );
        assert!(first.contains("/* keep */"));
        let second = std::thread::spawn({
            let source = first.clone();
            move || Formatter::format_one(&source, config)
        })
        .join()
        .unwrap();
        assert_eq!(second, first);
    }

    #[test]
    fn annotation_casing_normalizes_known_names_and_preserves_unknown_names() {
        let source = "@iStEsT(SeeAllData=true)\n@MyCustomAnno\nclass T {\n  // Keep this adjacent comment.\n  @aUrAeNaBlEd\n  static void run() {}\n}\n";
        let config = Config {
            normalize_annotation_casing: true,
            ..Config::default()
        };

        let output = Formatter::format_one(source, config);

        assert!(output.contains("@IsTest(SeeAllData=true)"));
        assert!(output.contains("@MyCustomAnno"));
        assert!(output.contains("// Keep this adjacent comment."));
        assert!(output.contains("@AuraEnabled"));
    }

    #[test]
    fn annotation_casing_covers_every_known_name() {
        let cases = [
            ("iStEsT", "IsTest"),
            ("tEsTsEtUp", "TestSetup"),
            ("tEsTvIsIbLe", "TestVisible"),
            ("aUrAeNaBlEd", "AuraEnabled"),
            ("fUtUrE", "Future"),
            ("iNvOcAbLeMeThOd", "InvocableMethod"),
            ("iNvOcAbLeVaRiAbLe", "InvocableVariable"),
            ("hTtPgEt", "HttpGet"),
            ("hTtPpOsT", "HttpPost"),
            ("hTtPpUt", "HttpPut"),
            ("hTtPpAtCh", "HttpPatch"),
            ("hTtPdElEtE", "HttpDelete"),
            ("rEsTrEsOuRcE", "RestResource"),
            ("rEaDoNlY", "ReadOnly"),
            ("rEmOtEaCtIoN", "RemoteAction"),
            ("dEpReCaTeD", "Deprecated"),
            ("sUpPrEsSwArNiNgS", "SuppressWarnings"),
            ("nAmEsPaCeAcCeSsIbLe", "NamespaceAccessible"),
            ("jSoNaCcEsS", "JsonAccess"),
        ];
        let mut source = cases
            .iter()
            .map(|(input, _)| format!("@{}\n", input))
            .collect::<String>();
        source.push_str("@mYCustomAnno\nclass T {}\n");
        let config = Config {
            normalize_annotation_casing: true,
            ..Config::default()
        };

        let output = Formatter::format_one(&source, config);

        for (_, expected) in cases {
            assert!(
                output.lines().any(|line| line == format!("@{}", expected)),
                "missing canonical annotation @{expected} in output:\n{output}"
            );
        }
        assert!(output.lines().any(|line| line == "@mYCustomAnno"));
    }

    #[test]
    fn annotation_name_comments_survive_casing() {
        let source = "@/* Keep this name comment. */iStEsT\nclass T {}\n";
        let config = Config {
            normalize_annotation_casing: true,
            ..Config::default()
        };

        let output = Formatter::format_one(source, config);

        assert!(
            output.contains("@/* Keep this name comment. */ IsTest"),
            "unexpected output:\n{output}"
        );
    }

    #[test]
    fn annotation_casing_is_idempotent() {
        let source = "@ISTEST\nclass T {}\n";
        let config = Config {
            normalize_annotation_casing: true,
            ..Config::default()
        };

        let once_config = config.clone();
        let once = std::thread::spawn(move || Formatter::format_one(source, once_config))
            .join()
            .unwrap();
        let once_for_second_pass = once.clone();
        let twice =
            std::thread::spawn(move || Formatter::format_one(&once_for_second_pass, config))
                .join()
                .unwrap();

        assert_eq!(once, twice);
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
    fn indent_size_one_is_idempotent_for_spaces_and_tabs() {
        let source = "class T {\n  void m() {\n    if (a) x();\n  }\n}\n";
        let space_config = Config {
            indent_size: 1,
            ..Config::default()
        };
        let tab_config = Config {
            indent_size: 1,
            indent_style: IndentStyle::Tab,
            ..Config::default()
        };

        let space_once = Formatter::format_one(source, space_config.clone());
        let tab_once = Formatter::format_one(source, tab_config.clone());

        assert_eq!(
            space_once,
            "class T {\n void m() {\n  if (a)\n   x();\n }\n}\n"
        );
        assert_eq!(
            tab_once,
            "class T {\n\tvoid m() {\n\t\tif (a)\n\t\t\tx();\n\t}\n}\n"
        );
        assert_eq!(Formatter::format_one(&space_once, space_config), space_once);
        assert_eq!(Formatter::format_one(&tab_once, tab_config), tab_once);
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
