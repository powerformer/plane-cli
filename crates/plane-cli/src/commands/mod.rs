pub mod api;
pub mod output;

use crate::core::{
    app::{build_version, AppState},
    config::ConfigOverrides,
    skill::{self, SkillInstallOptions, SkillUninstallOptions, SkillUpgradeOptions},
};
use api::ApiMeOptions;
use clap::{ArgMatches, Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use output::CommandResult;
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
    #[arg(
        long,
        global = true,
        value_name = "FILE",
        help = "Path to plane.toml. Defaults to PLANE_CONFIG or {PLANE_HOME:-~/.plane}/plane.toml."
    )]
    pub config: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        value_name = "DIR",
        help = "Plane home directory. Overrides config home, PLANE_HOME, and the ~/.plane default."
    )]
    pub home: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        value_name = "DIR",
        help = "Directory for Plane managed state. Overrides config state_dir and PLANE_STATE_DIR."
    )]
    pub state_dir: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        value_name = "FILE",
        help = "Path to managed skill state. Overrides config skills_state_path and PLANE_SKILLS_STATE_PATH."
    )]
    pub skills_state_path: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        value_name = "URL",
        help = "Plane server URL or /api/v1 base URL. Overrides config api_base_url and PLANE_API_BASE_URL."
    )]
    pub api_base_url: Option<String>,

    #[arg(
        long,
        global = true,
        value_name = "KEY",
        help = "Plane API token sent as X-API-Key. Prefer plane.toml or PLANE_API_KEY for routine use."
    )]
    pub api_key: Option<String>,

    #[arg(
        long = "workspace",
        global = true,
        value_name = "SLUG",
        help = "Default Plane workspace slug for workspace-scoped API commands. Overrides config workspace_slug and PLANE_WORKSPACE_SLUG."
    )]
    pub workspace_slug: Option<String>,

    #[arg(long, global = true, help = "Show detailed diagnostic logs on stderr.")]
    pub verbose: bool,

    #[command(subcommand)]
    command: Option<PlaneCommand>,
}

#[derive(Debug, Subcommand)]
enum PlaneCommand {
    #[command(about = "Print the installed version.")]
    Version,
    #[command(
        about = "Call the Plane API using X-API-Key authentication.",
        long_about = "Call the Plane API using X-API-Key authentication.\n\nAPI commands read api_base_url, api_key, and workspace_slug from --api-base-url/--api-key/--workspace, then plane.toml, then PLANE_API_BASE_URL/PLANE_API_KEY/PLANE_WORKSPACE_SLUG. The first smoke command is `plane api me`, which reads /api/v1/users/me/."
    )]
    Api(ApiCommand),
    #[command(about = "Install, upgrade, list, and uninstall Plane agent skills.")]
    Skill(SkillCommand),
}

#[derive(Debug, Args)]
struct ApiCommand {
    #[command(subcommand)]
    command: ApiSubcommand,
}

#[derive(Debug, Subcommand)]
enum ApiSubcommand {
    #[command(
        about = "Smoke-test Plane API authentication by reading the current user.",
        long_about = "Smoke-test Plane API authentication by reading /api/v1/users/me/.\n\nConfigure api_base_url and api_key in plane.toml, or pass --api-base-url and --api-key. The token is sent as X-API-Key and is never printed by this command."
    )]
    Me(ApiMeCommand),
}

#[derive(Debug, Args)]
struct ApiMeCommand {
    #[arg(long, help = "Print the raw JSON response instead of a smoke summary.")]
    json: bool,
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
        long_about = "Install the plane-cli skill.\n\nBy default, Plane detects common agent homes for Claude Code, Codex, and OpenCode, creates their skills directories when needed, and installs plane-cli there. Pass --path to install into an explicit final skill directory such as /path/to/skills/plane-cli. Plane will not overwrite unmanaged paths."
    )]
    Install(SkillInstallCommand),
    #[command(
        about = "Upgrade every managed plane-cli skill installation to the selected release.",
        long_about = "Upgrade managed plane-cli skill installations.\n\nUpgrade reads the managed installation registry from the resolved Plane state path, defaulting to ~/.plane/state/skills.json. It only touches those paths. Missing managed paths are recreated; existing paths must still contain Plane-managed metadata."
    )]
    Upgrade(SkillUpgradeCommand),
    #[command(
        about = "Uninstall every managed plane-cli skill installation.",
        long_about = "Uninstall managed plane-cli skill installations.\n\nUninstall only removes paths recorded in the resolved Plane state path, defaulting to ~/.plane/state/skills.json, and each target must still contain Plane-managed metadata.json before it is deleted."
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
        help = "Release base URL. Overrides config releases_public_url, PLANE_RELEASES_PUBLIC_URL, and the public default."
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
        help = "Release base URL. Overrides config releases_public_url, PLANE_RELEASES_PUBLIC_URL, and the public default."
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

#[allow(dead_code)]
pub fn execute(state: &AppState, args: &[String]) -> CommandResult {
    let matches = match parse_matches(state.version, args) {
        Ok(matches) => matches,
        Err(result) => return result,
    };
    let parsed = match PlaneCli::from_arg_matches(&matches) {
        Ok(parsed) => parsed,
        Err(error) => return CommandResult::err(2, error.render().to_string()),
    };

    dispatch(state, parsed)
}

pub fn execute_from_env(args: &[String]) -> CommandResult {
    let version = build_version();
    let matches = match parse_matches(version, args) {
        Ok(matches) => matches,
        Err(result) => return result,
    };
    let parsed = match PlaneCli::from_arg_matches(&matches) {
        Ok(parsed) => parsed,
        Err(error) => return CommandResult::err(2, error.render().to_string()),
    };

    match parsed.command {
        None => CommandResult::ok(help_text(version)),
        Some(PlaneCommand::Version) => CommandResult::ok(format!("plane {version}\n")),
        Some(command @ (PlaneCommand::Api(_) | PlaneCommand::Skill(_))) => {
            let overrides = config_overrides_from_matches(&matches);
            let state = match AppState::from_env(overrides) {
                Ok(state) => state,
                Err(error) => return CommandResult::err(1, format!("plane: {error}\n")),
            };
            dispatch(
                &state,
                PlaneCli {
                    command: Some(command),
                    config: None,
                    home: None,
                    state_dir: None,
                    skills_state_path: None,
                    api_base_url: None,
                    api_key: None,
                    workspace_slug: None,
                    verbose: parsed.verbose,
                },
            )
        }
    }
}

fn parse_matches(version: &'static str, args: &[String]) -> Result<ArgMatches, CommandResult> {
    let argv = std::iter::once("plane".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    let command = PlaneCli::command().version(version);
    let matches = match command.clone().try_get_matches_from(argv) {
        Ok(matches) => matches,
        Err(error) => {
            let status = if error.use_stderr() { 2 } else { 0 };
            let rendered = error.render().to_string();
            return if status == 0 {
                Err(CommandResult::ok(rendered))
            } else {
                Err(CommandResult::err(status, rendered))
            };
        }
    };
    Ok(matches)
}

fn dispatch(state: &AppState, parsed: PlaneCli) -> CommandResult {
    match parsed.command {
        None => CommandResult::ok(help_text(state.version)),
        Some(PlaneCommand::Version) => CommandResult::ok(format!("plane {}\n", state.version)),
        Some(PlaneCommand::Api(command)) => execute_api(state, command),
        Some(PlaneCommand::Skill(command)) => execute_skill(state, command),
    }
}

fn config_overrides_from_matches(matches: &ArgMatches) -> ConfigOverrides {
    ConfigOverrides {
        config_path: matches.get_one::<PathBuf>("config").cloned(),
        plane_home: matches.get_one::<PathBuf>("home").cloned(),
        state_dir: matches.get_one::<PathBuf>("state_dir").cloned(),
        skills_state_path: matches.get_one::<PathBuf>("skills_state_path").cloned(),
        api_base_url: matches.get_one::<String>("api_base_url").cloned(),
        api_key: matches.get_one::<String>("api_key").cloned(),
        workspace_slug: matches.get_one::<String>("workspace_slug").cloned(),
    }
}

fn execute_api(state: &AppState, command: ApiCommand) -> CommandResult {
    let result = match command.command {
        ApiSubcommand::Me(command) => api::me(state, ApiMeOptions { json: command.json }),
    };
    match result {
        Ok(stdout) => CommandResult::ok(stdout),
        Err(error) => CommandResult::err(1, format!("plane: {error}\n")),
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
