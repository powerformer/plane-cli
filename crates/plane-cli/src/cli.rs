use crate::{app::AppState, output::CommandResult};

const HELP: &str = "\
plane

Usage:
  plane help
  plane --help
  plane --version

Commands:
  help        Print this help text.

Options:
  -h, --help     Print this help text.
  -V, --version  Print the installed version.
";

pub fn execute(state: &AppState, args: &[String]) -> CommandResult {
    let _workspace_root = &state.config.workspace_root;

    match args {
        [] => CommandResult::ok(help_text()),
        [arg] if arg == "help" || arg == "-h" || arg == "--help" => CommandResult::ok(help_text()),
        [arg] if arg == "version" || arg == "-V" || arg == "--version" => {
            CommandResult::ok(format!("plane {}\n", state.version))
        }
        [command, ..] => CommandResult::err(
            2,
            format!("plane: unknown command or option: {command}\nRun `plane help` for usage.\n"),
        ),
    }
}

pub fn help_text() -> String {
    HELP.to_string()
}
