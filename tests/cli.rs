use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_directory() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("sf-afmt-phase3-{unique}"));
    fs::create_dir(&directory).expect("temporary directory should be created");
    directory
}

fn run_cli(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_afmt"))
        .current_dir(directory)
        .args(args)
        .output()
        .expect("afmt should run")
}

fn run_cli_with_stdin(directory: &Path, args: &[&str], source: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_afmt"))
        .current_dir(directory)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("afmt should run");
    child
        .stdin
        .take()
        .expect("stdin should be available")
        .write_all(source.as_bytes())
        .expect("source should be written to stdin");
    child.wait_with_output().expect("afmt should finish")
}

fn write_fixture(path: &Path) {
    fs::write(path, include_str!("static/variable_declaration.in"))
        .expect("fixture should be written");
}

#[test]
fn single_file_dry_run_keeps_stdout_as_plain_formatted_source() {
    let directory = temporary_directory();
    let path = directory.join("Account.cls");
    write_fixture(&path);

    let output = run_cli(&directory, &[path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.starts_with("class A {"));
    assert!(!stdout.contains("==>"));
    assert!(output.stderr.is_empty());

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn stdin_formats_source_without_diagnostics_and_is_idempotent() {
    let directory = temporary_directory();
    let source = include_str!("static/variable_declaration.in");
    let expected = include_str!("static/variable_declaration.cls");

    let output = run_cli_with_stdin(&directory, &["-"], source);
    assert!(output.status.success());
    assert_eq!(output.stdout, expected.as_bytes());
    assert!(output.stderr.is_empty());

    let second = run_cli_with_stdin(&directory, &["-"], expected);
    assert!(second.status.success());
    assert_eq!(second.stdout, output.stdout);
    assert!(second.stderr.is_empty());

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn stdin_honors_explicit_config() {
    let directory = temporary_directory();
    let config = directory.join("custom.toml");
    fs::write(&config, "indent_size = 4\n").expect("config should be written");

    let output = run_cli_with_stdin(
        &directory,
        &["-c", config.to_str().unwrap(), "-"],
        "class A{Integer value;}\n",
    );
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("    Integer value;"));
    assert!(output.stderr.is_empty());

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn empty_stdin_preserves_the_printer_trailing_newline_contract() {
    let directory = temporary_directory();

    let output = run_cli_with_stdin(&directory, &["-"], "");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"\n");
    assert!(output.stderr.is_empty());

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn stdin_ignores_time_without_polluting_formatted_stdout_or_stderr() {
    let directory = temporary_directory();
    let source = include_str!("static/variable_declaration.in");
    let expected = include_str!("static/variable_declaration.cls");

    let output = run_cli_with_stdin(&directory, &["--time", "-"], source);
    assert!(output.status.success());
    assert_eq!(output.stdout, expected.as_bytes());
    assert!(output.stderr.is_empty());

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn stdin_format_error_has_no_partial_stdout_or_panic_banner() {
    let directory = temporary_directory();

    let output = run_cli_with_stdin(&directory, &["-"], "class Broken {");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("Error:"));
    assert!(!stderr.contains("panicked at"));
    assert!(!stderr.contains("thread '"));

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn stdin_formatter_panic_is_a_normal_error_without_a_panic_banner() {
    let directory = temporary_directory();
    let source = include_str!("fixtures/demo-campaign.apex");

    let output = run_cli_with_stdin(&directory, &["-"], source);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert_eq!(stderr.matches("Error:").count(), 1, "stderr: {stderr}");
    assert!(stderr.contains("Formatting panicked:"));
    assert!(!stderr.contains("panicked at"));
    assert!(!stderr.contains("thread '"));

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn stdin_rejects_other_paths_write_and_check() {
    let directory = temporary_directory();

    for args in [["-", "other.cls"], ["--write", "-"], ["--check", "-"]] {
        let output = run_cli(&directory, &args);
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        assert!(output.stdout.is_empty(), "args: {args:?}");
        let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
        assert!(stderr.contains("stdin path '-'"), "stderr: {stderr}");
    }

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn one_file_directory_dry_run_keeps_stdout_plain_and_reports_bulk_summary() {
    let directory = temporary_directory();
    let path = directory.join("Account.cls");
    write_fixture(&path);
    let original = fs::read(&path).expect("source should be readable");

    let directory_arg = directory.to_str().unwrap();
    let output = run_cli(&directory, &[directory_arg]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stdout.starts_with("class A {"));
    assert!(!stdout.contains("==>"));
    assert!(stderr.contains("Summary: selected=1"));
    assert_eq!(
        fs::read(&path).expect("source should remain readable"),
        original
    );

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn multi_file_dry_run_is_delimited_and_sorted() {
    let directory = temporary_directory();
    let first = directory.join("z.cls");
    let second = directory.join("a.cls");
    write_fixture(&first);
    write_fixture(&second);

    let directory_arg = directory.to_str().unwrap();
    let output = run_cli(&directory, &[directory_arg]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let first_delimiter = format!("==> {} <==", second.display());
    let second_delimiter = format!("==> {} <==", first.display());
    assert_eq!(stdout.matches("==> ").count(), 2);
    assert!(stdout.contains(&first_delimiter));
    assert!(stdout.contains(&second_delimiter));
    assert!(stdout.find(&first_delimiter) < stdout.find(&second_delimiter));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Summary: selected=2"));

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn write_continues_after_a_format_failure_and_reports_summary() {
    let directory = temporary_directory();
    let invalid = directory.join("a-invalid.cls");
    let valid = directory.join("b-valid.cls");
    fs::write(&invalid, "class Broken {").expect("invalid fixture should be written");
    write_fixture(&valid);
    let original_valid = fs::read_to_string(&valid).expect("valid source should be readable");

    let output = run_cli(&directory, &["--write", directory.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("a-invalid.cls"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Summary: selected=2"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("written=1"));
    assert_ne!(
        fs::read_to_string(&valid).expect("valid source should remain readable"),
        original_valid
    );

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn check_evaluates_every_file_and_reports_changed_count() {
    let directory = temporary_directory();
    let changed = directory.join("a-changed.cls");
    let unchanged = directory.join("b-unchanged.cls");
    write_fixture(&changed);
    write_fixture(&unchanged);
    let format_unchanged = run_cli(&directory, &["--write", unchanged.to_str().unwrap()]);
    assert!(format_unchanged.status.success());

    let output = run_cli(&directory, &["--check", directory.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("Unformatted:"));
    assert!(stderr.contains("a-changed.cls"));
    assert!(
        stderr.contains("Check: 1 file(s) would be reformatted"),
        "unexpected check diagnostics: {stderr}"
    );
    assert!(stderr.contains("Summary: selected=2, changed=1, written=0, unchanged=1, failed=0"));

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn check_lists_all_changed_paths_and_accepts_a_fully_formatted_file() {
    let directory = temporary_directory();
    let first_changed = directory.join("a-changed.cls");
    let second_changed = directory.join("b-changed.cls");
    let unchanged = directory.join("c-unchanged.cls");
    write_fixture(&first_changed);
    write_fixture(&second_changed);
    write_fixture(&unchanged);

    let unchanged_format = run_cli(&directory, &["--write", unchanged.to_str().unwrap()]);
    assert!(unchanged_format.status.success());
    let unchanged_bytes = fs::read(&unchanged).expect("unchanged source should be readable");
    let changed_bytes = fs::read(&first_changed).expect("changed source should be readable");

    let output = run_cli(&directory, &["--check", directory.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("a-changed.cls"));
    assert!(stderr.contains("b-changed.cls"));
    assert!(!stderr.contains("c-unchanged.cls"));
    assert!(stderr.contains("Check: 2 file(s) would be reformatted"));
    assert!(stderr.contains("Summary: selected=3, changed=2, written=0, unchanged=1, failed=0"));
    assert_eq!(
        fs::read(&unchanged).expect("unchanged source should be readable"),
        unchanged_bytes
    );
    assert_eq!(
        fs::read(&first_changed).expect("changed source should be readable"),
        changed_bytes
    );

    let formatted_check = run_cli(&directory, &["--check", unchanged.to_str().unwrap()]);
    assert!(formatted_check.status.success());
    let formatted_stderr =
        String::from_utf8(formatted_check.stderr).expect("stderr should be UTF-8");
    assert!(formatted_stderr.contains("Check: 0 file(s) would be reformatted"));

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn check_parse_error_and_no_match_are_nonzero_without_writes() {
    let directory = temporary_directory();
    let invalid = directory.join("a-invalid.cls");
    let valid = directory.join("b-valid.cls");
    fs::write(&invalid, "class Broken {").expect("invalid source should be written");
    write_fixture(&valid);
    let valid_write = run_cli(&directory, &["--write", valid.to_str().unwrap()]);
    assert!(valid_write.status.success());
    let valid_bytes = fs::read(&valid).expect("valid source should be readable");

    let parse_output = run_cli(&directory, &["--check", directory.to_str().unwrap()]);
    assert!(!parse_output.status.success());
    let parse_stderr = String::from_utf8(parse_output.stderr).expect("stderr should be UTF-8");
    assert!(parse_stderr.contains("a-invalid.cls"));
    assert!(
        parse_stderr.contains("Summary: selected=2, changed=0, written=0, unchanged=1, failed=1")
    );
    assert_eq!(
        fs::read(&valid).expect("valid source should remain readable"),
        valid_bytes
    );

    let empty = directory.join("empty");
    fs::create_dir(&empty).expect("empty directory should be created");
    fs::write(empty.join("notes.txt"), "not Apex").expect("unrelated file should be written");
    let no_match_output = run_cli(&directory, &["--check", empty.to_str().unwrap()]);
    assert!(!no_match_output.status.success());
    assert!(String::from_utf8_lossy(&no_match_output.stderr)
        .contains("No eligible Apex files were found"));

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn bulk_write_then_check_is_idempotent_and_preserves_files() {
    let directory = temporary_directory();
    let class_file = directory.join("Account.cls");
    let trigger_file = directory.join("Account.trigger");
    write_fixture(&class_file);
    write_fixture(&trigger_file);

    let write_output = run_cli(&directory, &["--write", directory.to_str().unwrap()]);
    assert!(write_output.status.success());
    let class_bytes = fs::read(&class_file).expect("class output should be readable");
    let trigger_bytes = fs::read(&trigger_file).expect("trigger output should be readable");
    let class_time = fs::metadata(&class_file)
        .expect("class metadata should be readable")
        .modified()
        .expect("class timestamp should be available");
    let trigger_time = fs::metadata(&trigger_file)
        .expect("trigger metadata should be readable")
        .modified()
        .expect("trigger timestamp should be available");

    let check_output = run_cli(&directory, &["--check", directory.to_str().unwrap()]);
    assert!(check_output.status.success());
    assert!(String::from_utf8_lossy(&check_output.stderr)
        .contains("Summary: selected=2, changed=0, written=0, unchanged=2, failed=0"));
    assert_eq!(
        fs::read(&class_file).expect("class output should remain readable"),
        class_bytes
    );
    assert_eq!(
        fs::read(&trigger_file).expect("trigger output should remain readable"),
        trigger_bytes
    );
    assert_eq!(
        fs::metadata(&class_file)
            .expect("class metadata should remain readable")
            .modified()
            .expect("class timestamp should remain available"),
        class_time
    );
    assert_eq!(
        fs::metadata(&trigger_file)
            .expect("trigger metadata should remain readable")
            .modified()
            .expect("trigger timestamp should remain available"),
        trigger_time
    );

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[test]
fn time_reports_per_file_and_total_on_stderr() {
    let directory = temporary_directory();
    let first = directory.join("a.cls");
    let second = directory.join("b.cls");
    write_fixture(&first);
    write_fixture(&second);

    let output = run_cli(&directory, &["--time", directory.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(!stdout.contains("Timing:"));
    assert!(stderr.contains(&format!("Timing: {}", first.display())));
    assert!(stderr.contains(&format!("Timing: {}", second.display())));
    assert!(stderr.contains("Total elapsed:"));

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}
