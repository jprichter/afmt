#[cfg(test)]
mod tests {
    use sf_afmt::message_helper::red;
    use sf_afmt::{formatter::*, message_helper::yellow};
    use similar::{ChangeTag, TextDiff};
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn invalid_config_is_reported_before_discovery_without_panic_banner() {
        let directory = std::env::temp_dir().join(format!(
            "sf-afmt-invalid-config-cli-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let config_path = directory.join(".afmt.toml");
        let missing_target = directory.join("missing-target");
        fs::write(&config_path, "indent_size = 0\nindent_style = \"tab\"\n").unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_afmt"))
            .arg("--config")
            .arg(&config_path)
            .arg(&missing_target)
            .output()
            .expect("failed to execute afmt");
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(!output.status.success());
        assert!(stderr.contains("Invalid formatter configuration: indent_size must be at least 1"));
        assert!(!stderr.contains("panicked at"));
        assert!(!stderr.contains("thread '"));
        assert!(!stderr.contains("Formatting panicked"));

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn statics() {
        let (total, failed) = run_scenario("tests/static", "static");
        assert_eq!(failed, 0, "{} out of {} tests failed", failed, total);
    }

    #[test]
    fn prettier80() {
        let (total, failed) = run_scenario("tests/prettier80", "prettier80");
        assert_eq!(failed, 0, "{} out of {} tests failed", failed, total);
    }

    #[test]
    fn comments() {
        let (total, failed) = run_scenario("tests/comments", "comments");
        assert_eq!(failed, 0, "{} out of {} tests failed", failed, total);
    }

    #[test]
    fn configurable_style() {
        let (total, failed) = run_scenario("tests/configurable_style", "configurable_style");
        assert_eq!(failed, 0, "{} out of {} tests failed", failed, total);
    }

    #[test]
    fn ignore() {
        let (total, failed) = run_scenario("tests/ignore", "ignore");
        assert_eq!(failed, 0, "{} out of {} tests failed", failed, total);
    }

    #[test]
    fn idempotency_static() {
        run_idempotency("tests/static", "static", "tests/configs/.afmt_static.toml");
    }

    #[test]
    fn idempotency_prettier80() {
        run_idempotency(
            "tests/prettier80",
            "prettier80",
            "tests/configs/.afmt_p80.toml",
        );
    }

    #[test]
    fn idempotency_comments() {
        run_idempotency(
            "tests/comments",
            "comments",
            "tests/configs/.afmt_static.toml",
        );
    }

    #[test]
    fn idempotency_configurable_style() {
        run_idempotency(
            "tests/configurable_style",
            "configurable_style",
            "tests/configs/.afmt_configurable_style.toml",
        );
    }

    #[test]
    fn idempotency_ignore_directive_stable_output() {
        let source = std::fs::read_to_string("tests/ignore/IgnoreDirectiveStable.in")
            .expect("ignore source fixture should be readable");
        let expected = std::fs::read_to_string("tests/ignore/IgnoreDirectiveStable.cls")
            .expect("ignore expected output should be readable");
        let first = Formatter::format_one(&source, Config::default());
        let second = Formatter::format_one(&first, Config::default());

        assert_eq!(first, expected);
        assert_eq!(first, second);
    }

    #[test]
    fn all() {
        let scenarios = [
            ("tests/static", "static"),
            ("tests/prettier80", "prettier80"),
            ("tests/comments", "comments"),
        ];

        let mut total_tests = 0;
        let mut failed_tests = 0;

        println!("Running all test scenarios...");

        for (path, name) in scenarios.iter() {
            let (tests, failures) = run_scenario(path, name);
            total_tests += tests;
            failed_tests += failures;
        }

        println!(
            "\nTest Summary: {}/{} tests passed",
            total_tests - failed_tests,
            total_tests
        );
        if failed_tests > 0 {
            println!(
                "{}",
                red(&format!(
                    "{} out of {} tests failed",
                    failed_tests, total_tests
                ))
            );
        } else {
            println!("{}", yellow("All tests passed!"));
        }
    }

    #[test]
    fn no_expected_output_has_trailing_whitespace() {
        let fixture_dirs = ["tests/static", "tests/prettier80", "tests/comments"];
        let mut offenders = Vec::new();

        for directory in fixture_dirs {
            for entry in std::fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().and_then(|extension| extension.to_str()) != Some("cls") {
                    continue;
                }

                let output = std::fs::read_to_string(&path).unwrap();
                for (line_number, line) in output.lines().enumerate() {
                    if line.ends_with(' ') || line.ends_with('\t') {
                        offenders.push(format!("{}:{}", path.display(), line_number + 1));
                    }
                }
            }
        }

        assert!(
            offenders.is_empty(),
            "trailing whitespace in expected output:\n{}",
            offenders.join("\n")
        );
    }

    fn run_scenario(dir_path: &str, scenario_name: &str) -> (u32, u32) {
        let mut total_tests = 0;
        let mut failed_tests = 0;

        for entry in std::fs::read_dir(dir_path).unwrap() {
            let entry = entry.unwrap();
            let source = entry.path();
            if source.extension().and_then(|ext| ext.to_str()) == Some("in") {
                total_tests += 1;
                if !run_test_file(&source, scenario_name) {
                    failed_tests += 1;
                }
            }
        }

        println!(
            "{} scenario: {}/{} tests passed",
            scenario_name,
            total_tests - failed_tests,
            total_tests
        );
        (total_tests, failed_tests)
    }

    fn run_idempotency(dir_path: &str, scenario_name: &str, config_path: &str) {
        let mut sources = std::fs::read_dir(dir_path)
            .expect("idempotency fixture directory should be readable")
            .map(|entry| entry.expect("fixture entry should be readable").path())
            .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("in"))
            .collect::<Vec<_>>();
        sources.sort();

        for source in sources {
            let run_id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after the Unix epoch")
                .as_nanos();
            let first_path = std::env::temp_dir().join(format!(
                "sf-afmt-idempotency-{}-{}-{}-first.cls",
                std::process::id(),
                scenario_name,
                run_id
            ));
            let second_path = first_path.with_file_name(format!(
                "sf-afmt-idempotency-{}-{}-{}-second.cls",
                std::process::id(),
                scenario_name,
                run_id
            ));

            let first = format_with_afmt(&source, Some(config_path));
            std::fs::write(&first_path, &first).expect("first formatted output should be written");
            let second = format_with_afmt(&first_path, Some(config_path));
            std::fs::write(&second_path, &second)
                .expect("second formatted output should be written");

            let first_bytes = std::fs::read(&first_path).expect("first output should be readable");
            let second_bytes =
                std::fs::read(&second_path).expect("second output should be readable");
            if first_bytes != second_bytes {
                let diff = TextDiff::from_lines(&first, &second)
                    .unified_diff()
                    .header("first pass", "second pass")
                    .to_string();
                panic!(
                    "{} fixture is not idempotent: {}\n{}",
                    scenario_name,
                    source.display(),
                    diff
                );
            }

            std::fs::remove_file(first_path).expect("first temporary output should be removed");
            std::fs::remove_file(second_path).expect("second temporary output should be removed");
        }
    }

    fn run_test_file(source: &Path, scenario_name: &str) -> bool {
        // Wrap the "actual" test in a catch_unwind:
        let result = std::panic::catch_unwind(|| match scenario_name {
            "static" => run_static_test_files(source),
            "prettier80" => run_prettier_test_files(source, "p80"),
            "comments" => run_static_test_files(source),
            "configurable_style" => run_configurable_style_test_files(source),
            "ignore" => run_static_test_files(source),
            _ => panic!("Unknown scenario: {}", scenario_name),
        });

        match result {
            Ok(passed) => passed,
            Err(payload) => {
                eprintln!(
                    "Panic for scenario '{}' with file '{}'\nPanic info: {:?}",
                    scenario_name,
                    source.display(),
                    payload
                );
                false
            }
        }
    }

    fn run_static_test_files(source: &Path) -> bool {
        let expected_file = source.with_extension("cls");
        let output = format_with_afmt(source, Some("tests/configs/.afmt_static.toml"));
        let expected = std::fs::read_to_string(&expected_file).unwrap_or_else(|_| {
            panic!(
                "Failed to read expected .cls file at {}",
                red(&expected_file.to_string_lossy())
            )
        });

        compare("Static:", output, expected, source)
    }

    fn run_configurable_style_test_files(source: &Path) -> bool {
        let expected_file = source.with_extension("cls");
        let output = format_with_afmt(source, Some("tests/configs/.afmt_configurable_style.toml"));
        let expected = std::fs::read_to_string(&expected_file).unwrap_or_else(|_| {
            panic!(
                "Failed to read expected .cls file at {}",
                red(&expected_file.to_string_lossy())
            )
        });

        compare("Configurable style:", output, expected, source)
    }

    fn run_prettier_test_files(source: &Path, config_name: &str) -> bool {
        //let prettier_file = source.with_extension(config_name);
        let prettier_file = source.with_extension("cls");

        if !prettier_file.exists() {
            println!("### {}  file not found, generating...", yellow(config_name),);

            let prettier_output = run_prettier(
                source,
                Some(&format!("tests/configs/.prettierrc_{}.toml", config_name)),
            )
            .expect("Failed to run Prettier");
            save_prettier_output(&prettier_file, &prettier_output);
        }

        let output = format_with_afmt(
            source,
            Some(&format!("tests/configs/.afmt_{}.toml", config_name)),
        );

        let prettier_output = std::fs::read_to_string(&prettier_file)
            .expect("Failed to read the prettier formatted file.");

        compare("Prettier:", output, prettier_output, source)
    }

    fn normalize(content: &str) -> String {
        //println!("{} (Hex):", label);
        let mut normalized = String::new();
        // Git may check out fixtures as CRLF on Windows, while the formatter emits LF.
        let content = content.replace("\r\n", "\n");

        for (i, byte) in content.bytes().enumerate() {
            if i % 16 == 0 && i != 0 {
                normalized.push('\n');
            }
            normalized.push_str(&format!("{:02X} ", byte));
        }

        //println!("{}\n", normalized);
        normalized
    }

    fn compare(against: &str, output: String, expected: String, source: &Path) -> bool {
        let normalized_expected = normalize(&expected);
        let normalized_output = normalize(&output);
        if normalized_output != normalized_expected {
            //if output != expected {
            let source_content =
                std::fs::read_to_string(source).expect("Failed to read the file content.");

            println!("{}", yellow(&format!("\nFailed: {:?}:", source)));
            println!("-------------------------------------\n");
            println!("{}", source_content);
            println!("-------------------------------------\n");
            //print_side_by_side_diff(against, &normalized_output, &normalized_expected);
            print_side_by_side_diff(against, &output, &expected);
            println!("\n-------------------------------------\n");
            println!("{}", yellow(&format!("\nFailed: {:?}:", source)));
            println!("-------------------------------------\n");

            false
        } else {
            true
        }
    }

    fn format_with_afmt(source: &Path, config_path: Option<&str>) -> String {
        let file_path = source
            .to_str()
            .expect("PathBuf to String failed.")
            .to_string();

        let formatter = Formatter::create_from_config(config_path, vec![file_path.clone()])
            .expect("Create formatter failed.");

        let vec = formatter.format();
        vec.into_iter()
            .next()
            .and_then(|result| result.ok())
            .expect("format result failed.")
    }

    fn print_side_by_side_diff(against: &str, output: &str, expected: &str) {
        let diff = TextDiff::from_lines(expected, output);
        println!(
            "\x1b[38;2;255;165;0m{:<60} | {:<60}\x1b[0m",
            against, "Afmt:\n"
        );

        let changes: Vec<_> = diff.iter_all_changes().collect();
        let context_lines = 3;
        let mut i = 0;

        while i < changes.len() {
            if changes[i].tag() != ChangeTag::Equal {
                let mut j = i;
                while j < changes.len() && j - i < 2 * context_lines + 1 {
                    if changes[j].tag() != ChangeTag::Equal {
                        j = j + 2 * context_lines + 1;
                    } else {
                        j += 1;
                    }
                }
                j = j.min(changes.len());

                let start = i.saturating_sub(context_lines);
                let end = j.min(changes.len());

                if start > 0 {
                    println!("...");
                }

                for change in &changes[start..end] {
                    print_change_line(change);
                }

                if end < changes.len() {
                    println!("...");
                }

                // Also print raw diff lines for easy visual comparison
                println!("\n--- Raw diff ---");
                for change in &changes[start..end] {
                    print_raw_diff(change);
                }

                i = j;
            } else {
                i += 1;
            }
        }
    }

    fn print_raw_diff(change: &similar::Change<&str>) {
        match change.tag() {
            ChangeTag::Delete => {
                println!("- {}", change.to_string().replace("\n", "\\n"));
            }
            ChangeTag::Insert => {
                println!("+ {}", change.to_string().replace("\n", "\\n"));
            }
            ChangeTag::Equal => {
                // Skip unchanged lines for raw diff
            }
        }
    }

    fn print_change_line(change: &similar::Change<&str>) {
        let old_line_num = change
            .old_index()
            .map_or("".to_string(), |n| format!("{:>4}", n + 1));
        let new_line_num = change
            .new_index()
            .map_or("".to_string(), |n| format!("{:>4}", n + 1));

        let (left_col, right_col) = match change.tag() {
            ChangeTag::Delete => (
                format!("\x1b[91m- {:<58}\x1b[0m", change.to_string().trim_end()),
                String::from(""),
            ),
            ChangeTag::Insert => (
                String::from(""),
                format!("\x1b[92m+ {:<58}\x1b[0m", change.to_string().trim_end()),
            ),
            ChangeTag::Equal => (
                format!("  {:<58}", change.to_string().trim_end()),
                format!("  {:<58}", change.to_string().trim_end()),
            ),
        };

        println!(
            "{:<4} {:<60} | {:<4} {:<60}",
            old_line_num, left_col, new_line_num, right_col
        );
    }

    fn run_prettier(source: &Path, config_path: Option<&str>) -> Result<String, String> {
        let mut cmd = Command::new("npx");
        cmd.arg("prettier")
            .arg("--plugin=prettier-plugin-apex")
            .arg("--parser=apex")
            .arg(source.to_str().unwrap());

        if let Some(config) = config_path {
            cmd.arg("--config").arg(config);
        }

        let output = cmd.output().expect("Failed to execute Prettier");

        if output.status.success() {
            let formatted_code =
                String::from_utf8(output.stdout).expect("Prettier output is not valid UTF-8");
            Ok(formatted_code)
        } else {
            let error_message = String::from_utf8(output.stderr)
                .unwrap_or_else(|_| "Unknown error while running Prettier".to_string());
            Err(error_message)
        }
    }

    fn save_prettier_output(file_path: &Path, output: &str) {
        let mut file = File::create(file_path).expect("Failed to create .cls file");
        file.write_all(output.as_bytes())
            .expect("Failed to write Prettier output");
    }
}
