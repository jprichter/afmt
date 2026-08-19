use sf_afmt::args::{get_args, Args};
use sf_afmt::config::AfmtConfig;
use sf_afmt::discovery::{discover_targets, DiscoveryConfigError};
use sf_afmt::formatter::{Config, Formatter};
use std::io::{self, Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;
use std::{fs, process};

fn main() {
    let start = Instant::now();

    let args = get_args();
    let result = run(&args, start);

    match result {
        Ok(summary) => process::exit(if summary.has_failures() { 1 } else { 0 }),
        Err(error) => {
            eprintln!("Error: {error}");
            process::exit(1);
        }
    }
}

#[derive(Debug, Default)]
struct RunSummary {
    selected: usize,
    changed: usize,
    written: usize,
    unchanged: usize,
    failed: usize,
    check_failed: usize,
    diagnostics: Vec<String>,
}

impl RunSummary {
    fn has_failures(&self) -> bool {
        self.failed > 0 || self.check_failed > 0
    }

    fn record_error(&mut self, message: String) {
        self.failed += 1;
        eprintln!("Error: {message}");
        self.diagnostics.push(message);
    }

    fn print(&self, elapsed: std::time::Duration) {
        eprintln!(
            "Summary: selected={}, changed={}, written={}, unchanged={}, failed={}, total_elapsed={:?}",
            self.selected,
            self.changed,
            self.written,
            self.unchanged,
            self.failed,
            elapsed
        );
    }
}

fn run(args: &Args, started: Instant) -> Result<RunSummary, String> {
    run_with_writer(args, started, |path, content| fs::write(path, content))
}

/// Stands in for a path when the source arrived on stdin, so diagnostics keep
/// the same `path:line:column` shape they have for files.
const STDIN_ORIGIN: &str = "<stdin>";

fn format_stdin_source(source: &str, config: Config) -> Result<String, String> {
    // Stdin formatting is a synchronous CLI boundary. Keep the library's public
    // formatter free of process-global hook changes while ensuring a caught
    // formatter panic cannot leak Rust's default banner into stderr.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(|| {
        Formatter::try_format_source_with_origin(source, config, Some(STDIN_ORIGIN))
    }));
    std::panic::set_hook(previous_hook);

    match result {
        Ok(result) => result,
        Err(_) => Err("Formatting panicked: unexpected formatter panic".to_string()),
    }
}

fn run_with_writer<F>(args: &Args, started: Instant, write_file: F) -> Result<RunSummary, String>
where
    F: Fn(&std::path::Path, &str) -> std::io::Result<()>,
{
    if args.is_stdin() {
        let app_config = AfmtConfig::load(args.config.as_deref())?;
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|error| format!("Failed to read stdin: {error}"))?;
        let formatted = format_stdin_source(&source, app_config.formatter)?;
        print_source(&formatted);
        return Ok(RunSummary::default());
    }

    let app_config = AfmtConfig::load(args.config.as_deref())?;
    let base = AfmtConfig::base_dir(args.config.as_deref())?;
    let report = discover_targets(&args.paths, &app_config.files, &base)
        .map_err(|error: DiscoveryConfigError| error.to_string())?;
    let formatter = Formatter::new_from_paths(app_config.formatter, report.files.clone());
    let outcomes = formatter.format_with_outcomes();
    let single_output = outcomes.len() == 1;
    let mut summary = RunSummary {
        selected: outcomes.len(),
        ..RunSummary::default()
    };
    let bulk = args.paths.len() > 1
        || outcomes.len() > 1
        || args.paths.first().is_some_and(|path| path.is_dir());

    for error in report.errors {
        summary.record_error(error.to_string());
    }

    for outcome in outcomes {
        let path = outcome.path.clone();
        match outcome.result {
            Ok(formatted) if args.check && formatted.changed => {
                summary.changed += 1;
                summary.check_failed += 1;
                eprintln!("Unformatted: {}", path.display());
            }
            Ok(_formatted) if args.check => {
                summary.unchanged += 1;
            }
            Ok(formatted) if args.write && formatted.changed => {
                summary.changed += 1;
                if let Err(error) = write_file(&path, &formatted.content) {
                    summary.record_error(format!(
                        "{}: failed to write file: {}",
                        path.display(),
                        error
                    ));
                } else {
                    summary.written += 1;
                }
            }
            Ok(_formatted) if args.write => {
                summary.unchanged += 1;
            }
            Ok(formatted) => {
                if formatted.changed {
                    summary.changed += 1;
                } else {
                    summary.unchanged += 1;
                }
                if single_output {
                    print_source(&formatted.content);
                } else {
                    println!("==> {} <==", path.display());
                    print_source(&formatted.content);
                }
            }
            Err(error) => {
                summary.record_error(error.to_string());
            }
        }

        if args.time {
            eprintln!("Timing: {} {:?}", path.display(), outcome.elapsed);
        }
    }

    if args.check {
        eprintln!(
            "Check: {} file(s) would be reformatted",
            summary.check_failed
        );
    }

    let elapsed = started.elapsed();
    if bulk || args.time {
        summary.print(elapsed);
    }
    if args.time {
        eprintln!("Total elapsed: {elapsed:?}");
    }

    Ok(summary)
}

fn print_source(content: &str) {
    print!("{content}");
    if !content.ends_with('\n') {
        println!();
    }
    io::stdout()
        .flush()
        .expect("formatted output should be flushed");
}

#[cfg(test)]
mod tests {
    use super::{run_with_writer, Args};
    use sf_afmt::formatter::{Config, Formatter};
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("sf-afmt-phase3-write-{unique}"));
        fs::create_dir(&directory).expect("temporary directory should be created");
        directory
    }

    #[test]
    fn write_policy_handles_changed_unchanged_and_injected_write_error() {
        let directory = temporary_directory();
        let blocked = directory.join("a-blocked.cls");
        let written = directory.join("b-written.cls");
        let unchanged = directory.join("c-unchanged.cls");
        let source = include_str!("../tests/static/variable_declaration.in");
        fs::write(&blocked, source).expect("blocked source should be written");
        fs::write(&written, source).expect("writable source should be written");

        let formatted =
            Formatter::try_format_one(&written, Config::default()).expect("fixture should format");
        fs::write(&unchanged, &formatted.content).expect("formatted source should be written");
        let unchanged_content = fs::read(&unchanged).expect("unchanged source should be readable");
        let unchanged_time = fs::metadata(&unchanged)
            .expect("unchanged metadata should be readable")
            .modified()
            .expect("unchanged timestamp should be available");
        let blocked_for_writer = blocked.clone();

        let args = Args {
            paths: vec![directory.clone()],
            config: None,
            write: true,
            time: false,
            check: false,
        };
        let summary = run_with_writer(&args, std::time::Instant::now(), |path, content| {
            if path == blocked_for_writer {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "injected failure",
                ))
            } else {
                fs::write(path, content)
            }
        })
        .expect("write run should complete with per-file failures");

        assert!(summary.has_failures());
        assert_eq!(summary.selected, 3);
        assert_eq!(summary.changed, 2);
        assert_eq!(summary.written, 1);
        assert_eq!(summary.unchanged, 1);
        assert_eq!(summary.failed, 1);
        assert!(summary
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("a-blocked.cls")
                && diagnostic.contains("failed to write file")));
        assert_eq!(
            fs::read(&written).expect("written source should be readable"),
            formatted.content.as_bytes()
        );
        assert_eq!(
            fs::read(&unchanged).expect("unchanged source should be readable"),
            unchanged_content
        );
        assert_eq!(
            fs::metadata(&unchanged)
                .expect("unchanged metadata should be readable")
                .modified()
                .expect("unchanged timestamp should be available"),
            unchanged_time
        );

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }
}
