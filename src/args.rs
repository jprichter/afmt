use clap::{Arg as ClapArg, Command};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Args {
    pub paths: Vec<PathBuf>,
    pub config: Option<PathBuf>,
    pub write: bool,
    pub time: bool,
    pub check: bool,
}

pub fn get_args() -> Args {
    let version = env!("CARGO_PKG_VERSION"); // read from Cargo.toml in compiling time

    let matches = command(version).get_matches();

    Args {
        paths: matches
            .get_many::<PathBuf>("path")
            .expect("At least one path is required")
            .cloned()
            .collect(),
        config: matches.get_one::<PathBuf>("config").cloned(),
        write: matches.get_flag("write"),
        time: matches.get_flag("time"),
        check: matches.get_flag("check"),
    }
}

fn command(version: &'static str) -> Command {
    Command::new("afmt")
        .version(version)
        .about(format!("Apex format tool (afmt): {}", version))
        .arg_required_else_help(true)
        .arg(
            ClapArg::new("path")
                .value_name("PATH")
                .help("A file or directory to format")
                .required(true)
                .index(1)
                .num_args(1..)
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            ClapArg::new("config")
                .short('c')
                .long("config")
                .value_name("CONFIG")
                .help("Path to the .afmt.toml configuration file")
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            ClapArg::new("write")
                .short('w')
                .long("write")
                .help("Write the formatted result back to the file")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            ClapArg::new("time")
                .short('t')
                .long("time")
                .help("Display execution time after formatting")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            ClapArg::new("check")
                .short('C')
                .long("check")
                .help("Verify if the file is already formatted; exit with non-zero status if not")
                .conflicts_with("write")
                .action(clap::ArgAction::SetTrue),
        )
        .after_help(
            "EXAMPLES:\n\
             \n\
             # Dry run: print the result without overwriting the file\n\
             afmt ./file.cls\n\
             \n\
             # Format and write changes back to the file\n\
             afmt --write src/file.cls\n\
             \n\
             # Format a directory recursively\n\
             afmt --write force-app\n\
             \n\
             # Check mixed files and directories\n\
             afmt --check force-app packages/shared.cls\n\
             \n\
             # Use a specific config file\n\
             afmt --config .afmt.toml --time --write .\n\
             \n\
             # Display execution time after formatting\n\
             afmt --time ./file.cls\n\
             \n\
             # Verify if the file is already formatted\n\
             afmt --check ./file.cls\n\
            ",
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_one_or_more_path_values() {
        let matches = command("test")
            .try_get_matches_from(["afmt", "one.cls", "nested", "two.apex"])
            .unwrap();
        let paths = matches
            .get_many::<PathBuf>("path")
            .unwrap()
            .cloned()
            .collect::<Vec<_>>();

        assert_eq!(
            paths,
            vec![
                PathBuf::from("one.cls"),
                PathBuf::from("nested"),
                PathBuf::from("two.apex")
            ]
        );
    }
}
