use crate::{
    app::AppState,
    output::CommandResult,
    skill::{self, SkillInstallOptions, SkillUninstallOptions, SkillUpgradeOptions},
};
use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "plane",
    version,
    about = "Plane command line interface.",
    long_about = "Plane command line interface.\n\nEvery command is designed to explain its own defaults and managed-state boundaries. Agent skills are a cold-start compatibility layer; this CLI help remains the command truth source.",
    arg_required_else_help = false
)]
pub struct PlaneCli {
    #[arg(long, global = true, help = "Show detailed diagnostic logs on stderr.")]
    pub verbose: bool,

    #[command(subcommand)]
    command: Option<PlaneCommand>,
}

#[derive(Debug, Subcommand)]
enum PlaneCommand {
    #[command(about = "Print the installed version.")]
    Version,
    #[command(about = "Install, upgrade, list, and uninstall Plane agent skills.")]
    Skill(SkillCommand),
}

#[derive(Debug, Args)]
struct SkillCommand {
    #[command(subcommand)]
    command: SkillSubcommand,
}

#[derive(Debug, Subcommand)]
enum SkillSubcommand {
    #[command(
        about = "Install the plane-cli skill into detected agent skill directories or an explicit final path.",
        long_about = "Install the plane-cli skill.\n\nBy default, Plane detects common agent skill directories for Claude Code, Codex, and OpenCode, and installs only into existing parent directories. Pass --path to install into an explicit final skill directory such as /path/to/skills/plane-cli. Plane will not overwrite unmanaged paths."
    )]
    Install(SkillInstallCommand),
    #[command(
        about = "Upgrade every managed plane-cli skill installation to the selected release.",
        long_about = "Upgrade managed plane-cli skill installations.\n\nUpgrade reads the managed installation registry from $PLANE_HOME/state/skills.json and only touches those paths. Missing managed paths are recreated; existing paths must still contain Plane-managed metadata."
    )]
    Upgrade(SkillUpgradeCommand),
    #[command(
        about = "Uninstall every managed plane-cli skill installation.",
        long_about = "Uninstall managed plane-cli skill installations.\n\nUninstall only removes paths recorded in $PLANE_HOME/state/skills.json, and each target must still contain Plane-managed metadata.json before it is deleted."
    )]
    Uninstall(SkillUninstallCommand),
    #[command(about = "List managed plane-cli skill installations.")]
    List,
}

#[derive(Debug, Args)]
struct SkillInstallCommand {
    #[arg(
        long,
        value_name = "DIR",
        help = "Install to this final skill directory, which must end with plane-cli."
    )]
    path: Option<PathBuf>,

    #[arg(
        long,
        default_value = "stable",
        value_parser = ["stable", "beta"],
        help = "Release channel used to resolve the skill artifact."
    )]
    channel: String,

    #[arg(
        long,
        value_name = "VERSION",
        help = "Release version to install, for example v0.1.0-beta.2. Defaults to the channel latest metadata."
    )]
    version: Option<String>,

    #[arg(
        long,
        value_name = "URL",
        help = "Release base URL. Defaults to PLANE_RELEASES_PUBLIC_URL or https://releases.plane.powerformer.net."
    )]
    release_url: Option<String>,

    #[arg(long, help = "Show what would change without writing files.")]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct SkillUpgradeCommand {
    #[arg(
        long,
        value_parser = ["stable", "beta"],
        help = "Release channel used to resolve the skill artifact. Defaults to the channel recorded in managed state."
    )]
    channel: Option<String>,

    #[arg(
        long,
        value_name = "VERSION",
        help = "Release version to upgrade to. Defaults to the selected channel latest metadata."
    )]
    version: Option<String>,

    #[arg(
        long,
        value_name = "URL",
        help = "Release base URL. Defaults to PLANE_RELEASES_PUBLIC_URL or https://releases.plane.powerformer.net."
    )]
    release_url: Option<String>,

    #[arg(long, help = "Show what would change without writing files.")]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct SkillUninstallCommand {
    #[arg(long, help = "Show what would be removed without deleting files.")]
    dry_run: bool,
}

pub fn execute(state: &AppState, args: &[String]) -> CommandResult {
    let argv = std::iter::once("plane".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    let command = PlaneCli::command().version(state.version);
    let matches = match command.clone().try_get_matches_from(argv) {
        Ok(matches) => matches,
        Err(error) => {
            let status = if error.use_stderr() { 2 } else { 0 };
            let rendered = error.render().to_string();
            return if status == 0 {
                CommandResult::ok(rendered)
            } else {
                CommandResult::err(status, rendered)
            };
        }
    };
    let parsed = match PlaneCli::from_arg_matches(&matches) {
        Ok(parsed) => parsed,
        Err(error) => return CommandResult::err(2, error.render().to_string()),
    };

    match parsed.command {
        None => CommandResult::ok(help_text(state.version)),
        Some(PlaneCommand::Version) => CommandResult::ok(format!("plane {}\n", state.version)),
        Some(PlaneCommand::Skill(command)) => execute_skill(state, command),
    }
}

fn execute_skill(state: &AppState, command: SkillCommand) -> CommandResult {
    let result = match command.command {
        SkillSubcommand::Install(command) => skill::install(
            state,
            SkillInstallOptions {
                path: command.path,
                release_url: command.release_url,
                channel: command.channel,
                version: command.version,
                dry_run: command.dry_run,
            },
        ),
        SkillSubcommand::Upgrade(command) => skill::upgrade(
            state,
            SkillUpgradeOptions {
                release_url: command.release_url,
                channel: command.channel,
                version: command.version,
                dry_run: command.dry_run,
            },
        ),
        SkillSubcommand::Uninstall(command) => skill::uninstall(
            state,
            SkillUninstallOptions {
                dry_run: command.dry_run,
            },
        ),
        SkillSubcommand::List => skill::list(state),
    };
    match result {
        Ok(stdout) => CommandResult::ok(stdout),
        Err(error) => CommandResult::err(1, format!("plane: {error}\n")),
    }
}

pub fn help_text(version: &'static str) -> String {
    let mut command = PlaneCli::command().version(version);
    let mut output = Vec::new();
    command.write_long_help(&mut output).expect("write help");
    String::from_utf8(output).expect("help is utf8")
}
