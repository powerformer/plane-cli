pub mod api;
pub mod output;

use crate::core::{
    app::{build_version, AppState},
    config::ConfigOverrides,
    skill::{self, SkillInstallOptions, SkillUninstallOptions, SkillUpgradeOptions},
};
use api::ApiMeOptions;
use clap::{ArgAction, ArgMatches, Args, CommandFactory, FromArgMatches, Parser, Subcommand};
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
        help = "Plane server URL or /api/v1 base URL. Overrides config api_base_url and PLANE_API_BASE_URL. Defaults to https://plane.powerformer.net."
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
        long_about = "Call the Plane API using X-API-Key authentication.\n\nAPI commands read api_base_url, api_key, and workspace_slug from --api-base-url/--api-key/--workspace, then plane.toml, then PLANE_API_BASE_URL/PLANE_API_KEY/PLANE_WORKSPACE_SLUG. Start with `plane api me` to verify access; run `plane api --help` for the resource commands."
    )]
    Api(ApiCommand),
    #[command(about = "Install, upgrade, list, and uninstall Plane agent skills.")]
    Skill(SkillCommand),
    #[command(
        about = "Manage cross-project work-item dependencies (label-backed).",
        long_about = "Manage cross-project work-item dependencies, stored as `dep:<KEY>:<SEQ>` labels on the dependent item (Plane's native relations do not cross projects). Subcommands: add, rm, ls, gc."
    )]
    Dep(DepCommand),
    #[command(
        about = "Check for a newer plane and print the upgrade command.",
        long_about = "Check the release channel for a newer plane binary and print the manager command to upgrade. This only reports; it does not download or replace the binary (the manager does that)."
    )]
    Upgrade,
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
    #[command(about = "Manage projects in the workspace.")]
    Project(ApiProjectCommand),
    #[command(name = "work-item", about = "Manage work items in a project.")]
    WorkItem(ApiWorkItemCommand),
    #[command(
        about = "Call an arbitrary /api/v1 path (escape hatch).",
        long_about = "Passthrough to an arbitrary /api/v1-relative path, for endpoints the typed commands do not cover yet. Supports GET, POST, PATCH, PUT, and DELETE."
    )]
    Request(ApiRequestCommand),
    #[command(about = "State CRUD in a project.")]
    State(ApiStateCommand),
    #[command(about = "Label CRUD in a project.")]
    Label(ApiLabelCommand),
    #[command(about = "Cycle CRUD in a project.")]
    Cycle(ApiCycleCommand),
    #[command(about = "Module CRUD in a project.")]
    Module(ApiModuleCommand),
    #[command(about = "Estimate CRUD in a project.")]
    Estimate(ApiEstimateCommand),
    #[command(name = "intake", about = "Intake work item CRUD in a project.")]
    Intake(ApiIntakeCommand),
    #[command(
        about = "Page (document) CRUD in a project.",
        long_about = "Create, read, update, list, and delete Plane pages (documents) in a project.\n\nPage bodies are written as Markdown (converted to HTML) or raw HTML; Plane stores the body as description_html and the collaborative editor hydrates from it on first open."
    )]
    Page(ApiPageCommand),
    #[command(about = "Work item comments (CRUD).")]
    Comment(ApiCommentCommand),
    #[command(about = "Work item links (CRUD).")]
    Link(ApiLinkCommand),
    #[command(about = "Work item relations (list/create).")]
    Relation(ApiRelationCommand),
    #[command(about = "Work item activity (read-only).")]
    Activity(ApiActivityCommand),
    #[command(about = "Project members (CRUD).")]
    Member(ApiMemberCommand),
}

#[derive(Debug, Args)]
struct ApiMeCommand {
    #[arg(long, help = "Print the raw JSON response instead of a short summary.")]
    json: bool,
}

#[derive(Debug, Args)]
struct ApiProjectCommand {
    #[command(subcommand)]
    command: ProjectSubcommand,
}

#[derive(Debug, Subcommand)]
enum ProjectSubcommand {
    #[command(about = "List projects in the workspace.")]
    List(ProjectListCommand),
    #[command(about = "Get a project by id.")]
    Get(ProjectGetCommand),
    #[command(about = "Create a project in the workspace.")]
    Create(ProjectCreateCommand),
    #[command(about = "Update a project by id.")]
    Update(ProjectUpdateCommand),
    #[command(about = "Delete a project by id.")]
    Delete(ProjectDeleteCommand),
    #[command(about = "Archive a project by id.")]
    Archive(ProjectArchiveCommand),
    #[command(about = "Unarchive a project by id.")]
    Unarchive(ProjectUnarchiveCommand),
    #[command(about = "Get a project summary.")]
    Summary(ProjectSummaryCommand),
}

#[derive(Debug, Args)]
struct ProjectListCommand {
    #[arg(long, help = "Follow cursor pages and list every result.")]
    all: bool,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated response fields to include."
    )]
    fields: Option<String>,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated relations to expand."
    )]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct ProjectGetCommand {
    #[arg(value_name = "PROJECT_ID", help = "Project id (UUID).")]
    id: String,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated response fields to include."
    )]
    fields: Option<String>,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated relations to expand."
    )]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct ProjectCreateCommand {
    #[arg(long, help = "Project name (required).")]
    name: String,
    #[arg(long, help = "Project identifier, e.g. PROJ (required).")]
    identifier: String,
    #[arg(long, value_name = "JSON", help = "Extra fields as a JSON object.")]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct ProjectUpdateCommand {
    #[arg(value_name = "PROJECT_ID", help = "Project id (UUID).")]
    id: String,
    #[arg(long, help = "New project name.")]
    name: Option<String>,
    #[arg(long, value_name = "JSON", help = "Fields to change as a JSON object.")]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct ProjectDeleteCommand {
    #[arg(value_name = "PROJECT_ID", help = "Project id (UUID).")]
    id: String,
    #[arg(long, help = "Print the request without sending it.")]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct ProjectArchiveCommand {
    #[arg(value_name = "PROJECT_ID", help = "Project id (UUID).")]
    id: String,
    #[arg(long, help = "Print the request without sending it.")]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct ProjectUnarchiveCommand {
    #[arg(value_name = "PROJECT_ID", help = "Project id (UUID).")]
    id: String,
    #[arg(long, help = "Print the request without sending it.")]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct ProjectSummaryCommand {
    #[arg(value_name = "PROJECT_ID", help = "Project id (UUID).")]
    id: String,
}

#[derive(Debug, Args)]
struct ApiWorkItemCommand {
    #[command(subcommand)]
    command: WorkItemSubcommand,
}

#[derive(Debug, Subcommand)]
enum WorkItemSubcommand {
    #[command(about = "List work items in a project.")]
    List(WorkItemListCommand),
    #[command(about = "Get a work item by id.")]
    Get(WorkItemGetCommand),
    #[command(about = "Create a work item in a project.")]
    Create(WorkItemCreateCommand),
    #[command(about = "Update a work item by id.")]
    Update(WorkItemUpdateCommand),
    #[command(about = "Delete a work item by id.")]
    Delete(WorkItemDeleteCommand),
}

#[derive(Debug, Args)]
struct WorkItemListCommand {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(long, help = "Follow cursor pages and list every result.")]
    all: bool,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated response fields to include."
    )]
    fields: Option<String>,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated relations to expand."
    )]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct WorkItemGetCommand {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(value_name = "WORK_ITEM_ID", help = "Work item id (UUID).")]
    id: String,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated response fields to include."
    )]
    fields: Option<String>,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated relations to expand."
    )]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct WorkItemCreateCommand {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(long, help = "Work item title (required).")]
    name: String,
    #[arg(
        long,
        value_name = "JSON",
        help = "Extra fields as a JSON object, merged under name."
    )]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct WorkItemUpdateCommand {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(value_name = "WORK_ITEM_ID", help = "Work item id (UUID).")]
    id: String,
    #[arg(long, help = "New work item title.")]
    name: Option<String>,
    #[arg(long, value_name = "JSON", help = "Fields to change as a JSON object.")]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct WorkItemDeleteCommand {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(value_name = "WORK_ITEM_ID", help = "Work item id (UUID).")]
    id: String,
    #[arg(long, help = "Print the request without sending it.")]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct ApiRequestCommand {
    #[arg(
        long,
        default_value = "GET",
        help = "HTTP method: GET, POST, PATCH, PUT, or DELETE."
    )]
    method: String,
    #[arg(
        value_name = "PATH",
        help = "/api/v1-relative path, e.g. workspaces/<slug>/projects/."
    )]
    path: String,
    #[arg(
        long,
        value_name = "JSON",
        help = "Request body JSON for POST/PATCH/PUT."
    )]
    data: Option<String>,
}

#[derive(Debug, Args)]
struct ApiStateCommand {
    #[command(subcommand)]
    command: CrudSubcommand,
}

#[derive(Debug, Args)]
struct ApiLabelCommand {
    #[command(subcommand)]
    command: CrudSubcommand,
}

#[derive(Debug, Args)]
struct ApiCycleCommand {
    #[command(subcommand)]
    command: CrudSubcommand,
}

#[derive(Debug, Args)]
struct ApiModuleCommand {
    #[command(subcommand)]
    command: CrudSubcommand,
}

#[derive(Debug, Args)]
struct ApiEstimateCommand {
    #[command(subcommand)]
    command: CrudSubcommand,
}

#[derive(Debug, Args)]
struct ApiIntakeCommand {
    #[command(subcommand)]
    command: CrudSubcommand,
}

#[derive(Debug, Subcommand)]
enum CrudSubcommand {
    #[command(about = "List resources in a project.")]
    List(CrudListArgs),
    #[command(about = "Get a resource by id.")]
    Get(CrudGetArgs),
    #[command(about = "Create a resource in a project.")]
    Create(CrudCreateArgs),
    #[command(about = "Update a resource by id.")]
    Update(CrudUpdateArgs),
    #[command(about = "Delete a resource by id.")]
    Delete(CrudDeleteArgs),
}

#[derive(Debug, Args)]
struct CrudListArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(long, help = "Follow cursor pages and list every result.")]
    all: bool,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated response fields to include."
    )]
    fields: Option<String>,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated relations to expand."
    )]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct CrudGetArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(value_name = "ID", help = "Resource id (UUID).")]
    id: String,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated response fields to include."
    )]
    fields: Option<String>,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated relations to expand."
    )]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct CrudCreateArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(long, help = "Resource name (required).")]
    name: String,
    #[arg(
        long,
        value_name = "JSON",
        help = "Extra fields as a JSON object, merged under name."
    )]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct CrudUpdateArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(value_name = "ID", help = "Resource id (UUID).")]
    id: String,
    #[arg(long, help = "New name.")]
    name: Option<String>,
    #[arg(long, value_name = "JSON", help = "Fields to change as a JSON object.")]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct CrudDeleteArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(value_name = "ID", help = "Resource id (UUID).")]
    id: String,
    #[arg(long, help = "Print the request without sending it.")]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct ApiPageCommand {
    #[command(subcommand)]
    command: PageSubcommand,
}

#[derive(Debug, Subcommand)]
enum PageSubcommand {
    #[command(about = "List pages in a project.")]
    List(CrudListArgs),
    #[command(about = "Get a page by id (use --content for the body HTML).")]
    Get(PageGetArgs),
    #[command(about = "Create a page from Markdown or HTML.")]
    Create(PageCreateArgs),
    #[command(about = "Update a page's title, access, or body.")]
    Update(PageUpdateArgs),
    #[command(about = "Delete a page by id.")]
    Delete(CrudDeleteArgs),
}

/// Body-source flags shared by `page create` and `page update`.
#[derive(Debug, Args)]
struct PageBodyArgs {
    #[arg(
        long,
        value_name = "FILE",
        help = "Read the page body from a file. Markdown by default; .html files (or --format html) are sent verbatim."
    )]
    from_file: Option<PathBuf>,
    #[arg(
        long,
        value_name = "TEXT",
        conflicts_with = "from_file",
        help = "Inline page body. Markdown by default; override with --format."
    )]
    body: Option<String>,
    #[arg(
        long,
        value_parser = ["md", "markdown", "html"],
        help = "Body format. Defaults to the --from-file extension, otherwise markdown."
    )]
    format: Option<String>,
}

#[derive(Debug, Args)]
struct PageGetArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(value_name = "ID", help = "Page id (UUID).")]
    id: String,
    #[arg(long, help = "Print only the page body HTML.")]
    content: bool,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated response fields to include."
    )]
    fields: Option<String>,
    #[arg(
        long,
        value_name = "CSV",
        help = "Comma-separated relations to expand."
    )]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct PageCreateArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(long, help = "Page title (required).")]
    name: String,
    #[command(flatten)]
    body: PageBodyArgs,
    #[arg(long, value_parser = ["public", "private"], help = "Page access.")]
    access: Option<String>,
    #[arg(
        long,
        value_name = "JSON",
        help = "Extra fields as a JSON object, merged under name/body."
    )]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct PageUpdateArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(value_name = "ID", help = "Page id (UUID).")]
    id: String,
    #[arg(long, help = "New page title.")]
    name: Option<String>,
    #[command(flatten)]
    body: PageBodyArgs,
    #[arg(long, value_parser = ["public", "private"], help = "Change page access.")]
    access: Option<String>,
    #[arg(long, value_name = "JSON", help = "Fields to change as a JSON object.")]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct ApiCommentCommand {
    #[command(subcommand)]
    command: CommentSubcommand,
}

#[derive(Debug, Subcommand)]
enum CommentSubcommand {
    #[command(about = "List comments on a work item.")]
    List(CommentListArgs),
    #[command(about = "Get a comment by id.")]
    Get(CommentGetArgs),
    #[command(about = "Comment on a work item from Markdown or HTML.")]
    Create(CommentCreateArgs),
    #[command(about = "Update a comment by id from Markdown or HTML.")]
    Update(CommentUpdateArgs),
    #[command(about = "Delete a comment by id.")]
    Delete(CommentDeleteArgs),
}

/// Body-source flags shared by `comment create` and `comment update`.
#[derive(Debug, Args)]
struct CommentBodyArgs {
    #[arg(
        long,
        value_name = "FILE",
        help = "Read the comment body from a file. Markdown by default; .html files (or --format html) are sent verbatim."
    )]
    from_file: Option<PathBuf>,
    #[arg(
        long,
        value_name = "TEXT",
        conflicts_with = "from_file",
        help = "Inline comment body. Markdown by default; override with --format."
    )]
    body: Option<String>,
    #[arg(
        long,
        value_parser = ["md", "markdown", "html"],
        help = "Body format. Defaults to the --from-file extension, otherwise markdown."
    )]
    format: Option<String>,
}

#[derive(Debug, Args)]
struct CommentListArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(
        long,
        value_name = "WORK_ITEM",
        help = "Work item UUID or identifier (e.g. OPEND-7)."
    )]
    work_item: String,
    #[arg(long, help = "Follow cursor pages and list every result.")]
    all: bool,
    #[arg(long, value_name = "CSV", help = "Response fields to include.")]
    fields: Option<String>,
    #[arg(long, value_name = "CSV", help = "Relations to expand.")]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct CommentGetArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(
        long,
        value_name = "WORK_ITEM",
        help = "Work item UUID or identifier (e.g. OPEND-7)."
    )]
    work_item: String,
    #[arg(value_name = "COMMENT_ID", help = "Comment id (UUID).")]
    id: String,
    #[arg(long, value_name = "CSV", help = "Response fields to include.")]
    fields: Option<String>,
    #[arg(long, value_name = "CSV", help = "Relations to expand.")]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct CommentCreateArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(
        long,
        value_name = "WORK_ITEM",
        help = "Work item UUID or identifier (e.g. OPEND-7)."
    )]
    work_item: String,
    #[command(flatten)]
    body: CommentBodyArgs,
    #[arg(
        long,
        value_name = "JSON",
        help = "Extra fields as a JSON object, merged under the comment body."
    )]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct CommentUpdateArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(
        long,
        value_name = "WORK_ITEM",
        help = "Work item UUID or identifier (e.g. OPEND-7)."
    )]
    work_item: String,
    #[arg(value_name = "COMMENT_ID", help = "Comment id (UUID).")]
    id: String,
    #[command(flatten)]
    body: CommentBodyArgs,
    #[arg(long, value_name = "JSON", help = "Fields to change as a JSON object.")]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct CommentDeleteArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(
        long,
        value_name = "WORK_ITEM",
        help = "Work item UUID or identifier (e.g. OPEND-7)."
    )]
    work_item: String,
    #[arg(value_name = "COMMENT_ID", help = "Comment id (UUID).")]
    id: String,
    #[arg(long, help = "Print the request without sending it.")]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct ApiLinkCommand {
    #[command(subcommand)]
    command: WiSubCommand,
}

#[derive(Debug, Args)]
struct ApiRelationCommand {
    #[command(subcommand)]
    command: WiSubCommand,
}

#[derive(Debug, Subcommand)]
enum WiSubCommand {
    #[command(about = "List sub-resources of a work item.")]
    List(WiSubListArgs),
    #[command(about = "Get a sub-resource by id.")]
    Get(WiSubGetArgs),
    #[command(about = "Create a sub-resource (body via --data).")]
    Create(WiSubCreateArgs),
    #[command(about = "Update a sub-resource by id (body via --data).")]
    Update(WiSubUpdateArgs),
    #[command(about = "Delete a sub-resource by id.")]
    Delete(WiSubDeleteArgs),
}

#[derive(Debug, Args)]
struct WiSubListArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(long, value_name = "WORK_ITEM_ID", help = "Work item id (UUID).")]
    work_item: String,
    #[arg(long, help = "Follow cursor pages and list every result.")]
    all: bool,
    #[arg(long, value_name = "CSV", help = "Response fields to include.")]
    fields: Option<String>,
    #[arg(long, value_name = "CSV", help = "Relations to expand.")]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct WiSubGetArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(long, value_name = "WORK_ITEM_ID", help = "Work item id (UUID).")]
    work_item: String,
    #[arg(value_name = "ID", help = "Sub-resource id (UUID).")]
    id: String,
    #[arg(long, value_name = "CSV", help = "Response fields to include.")]
    fields: Option<String>,
    #[arg(long, value_name = "CSV", help = "Relations to expand.")]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct WiSubCreateArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(long, value_name = "WORK_ITEM_ID", help = "Work item id (UUID).")]
    work_item: String,
    #[arg(long, value_name = "JSON", help = "Request body as a JSON object.")]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct WiSubUpdateArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(long, value_name = "WORK_ITEM_ID", help = "Work item id (UUID).")]
    work_item: String,
    #[arg(value_name = "ID", help = "Sub-resource id (UUID).")]
    id: String,
    #[arg(long, value_name = "JSON", help = "Fields to change as a JSON object.")]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct WiSubDeleteArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(long, value_name = "WORK_ITEM_ID", help = "Work item id (UUID).")]
    work_item: String,
    #[arg(value_name = "ID", help = "Sub-resource id (UUID).")]
    id: String,
    #[arg(long, help = "Print the request without sending it.")]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct ApiActivityCommand {
    #[command(subcommand)]
    command: ActivitySubCommand,
}

#[derive(Debug, Subcommand)]
enum ActivitySubCommand {
    #[command(about = "List a work item's activity.")]
    List(WiSubListArgs),
    #[command(about = "Get an activity entry by id.")]
    Get(WiSubGetArgs),
}

#[derive(Debug, Args)]
struct ApiMemberCommand {
    #[command(subcommand)]
    command: MemberSubCommand,
}

#[derive(Debug, Subcommand)]
enum MemberSubCommand {
    #[command(about = "List members of a project.")]
    List(MemberListArgs),
    #[command(about = "Get a project member by id.")]
    Get(MemberGetArgs),
    #[command(about = "Add a member to a project (body via --data).")]
    Create(MemberCreateArgs),
    #[command(about = "Update a project member by id (body via --data).")]
    Update(MemberUpdateArgs),
    #[command(about = "Remove a project member by id.")]
    Delete(MemberDeleteArgs),
    #[command(name = "workspace-list", about = "List members of the workspace.")]
    WorkspaceList(WorkspaceMemberListArgs),
}

#[derive(Debug, Args)]
struct MemberListArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(long, help = "Follow cursor pages and list every result.")]
    all: bool,
    #[arg(long, value_name = "CSV", help = "Response fields to include.")]
    fields: Option<String>,
    #[arg(long, value_name = "CSV", help = "Relations to expand.")]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct MemberGetArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(value_name = "ID", help = "Member id (UUID).")]
    id: String,
    #[arg(long, value_name = "CSV", help = "Response fields to include.")]
    fields: Option<String>,
    #[arg(long, value_name = "CSV", help = "Relations to expand.")]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct MemberCreateArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(long, value_name = "JSON", help = "Request body as a JSON object.")]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct MemberUpdateArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(value_name = "ID", help = "Member id (UUID).")]
    id: String,
    #[arg(long, value_name = "JSON", help = "Fields to change as a JSON object.")]
    data: Option<String>,
    #[arg(long, help = "Print the request body without sending it.")]
    dry_run: bool,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct MemberDeleteArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(value_name = "ID", help = "Member id (UUID).")]
    id: String,
    #[arg(long, help = "Print the request without sending it.")]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct WorkspaceMemberListArgs {
    #[arg(long, help = "Follow cursor pages and list every result.")]
    all: bool,
    #[arg(long, value_name = "CSV", help = "Response fields to include.")]
    fields: Option<String>,
    #[arg(long, value_name = "CSV", help = "Relations to expand.")]
    expand: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct DepCommand {
    #[command(subcommand)]
    command: DepSubcommand,
}

#[derive(Debug, Subcommand)]
enum DepSubcommand {
    #[command(about = "Add a dependency edge (label dep:<KEY>:<SEQ>); target must exist.")]
    Add(DepAddArgs),
    #[command(about = "Remove a dependency edge (detach the label; gc prunes orphans).")]
    Rm(DepRmArgs),
    #[command(about = "List dependency edges and resolve their targets.")]
    Ls(DepLsArgs),
    #[command(about = "Delete orphan dep:* labels (dry run unless --write).")]
    Gc(DepGcArgs),
}

#[derive(Debug, Args)]
struct DepAddArgs {
    #[arg(
        long,
        value_name = "PROJECT_ID",
        help = "Project id (UUID) of the dependent item."
    )]
    project: String,
    #[arg(
        long,
        value_name = "WORK_ITEM_ID",
        help = "Dependent work item id (UUID)."
    )]
    work_item: String,
    #[arg(
        long,
        value_name = "KEY:SEQ",
        help = "Dependency target, e.g. PLANE:5."
    )]
    on: String,
    #[arg(long, help = "Validate and print the plan without writing.")]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct DepRmArgs {
    #[arg(
        long,
        value_name = "PROJECT_ID",
        help = "Project id (UUID) of the dependent item."
    )]
    project: String,
    #[arg(
        long,
        value_name = "WORK_ITEM_ID",
        help = "Dependent work item id (UUID)."
    )]
    work_item: String,
    #[arg(
        long,
        value_name = "KEY:SEQ",
        help = "Dependency target to remove, e.g. PLANE:5."
    )]
    on: String,
}

#[derive(Debug, Args)]
struct DepLsArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(
        long,
        value_name = "WORK_ITEM_ID",
        help = "Limit to one work item; otherwise every item in the project."
    )]
    work_item: Option<String>,
    #[arg(long, help = "Print the raw JSON response.")]
    json: bool,
}

#[derive(Debug, Args)]
struct DepGcArgs {
    #[arg(long, value_name = "PROJECT_ID", help = "Project id (UUID).")]
    project: String,
    #[arg(long, help = "Actually delete orphan dep:* labels (default: dry run).")]
    write: bool,
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
        value_parser = ["stable", "beta"],
        help = "Release channel. Defaults to the channel matching this binary's version (-beta uses beta, otherwise stable)."
    )]
    channel: Option<String>,

    #[arg(
        long,
        value_name = "VERSION",
        help = "Release version to install. Defaults to this binary's own version so the skill matches the CLI."
    )]
    version: Option<String>,

    #[arg(
        long,
        value_name = "URL",
        help = "Release base URL. Overrides config releases_public_url, PLANE_RELEASES_PUBLIC_URL, and the public default."
    )]
    release_url: Option<String>,

    #[arg(
        long,
        default_value_t = true,
        action = ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true",
        help = "Overwrite an existing managed install (default true). Never overwrites unmanaged paths."
    )]
    force: bool,

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
        Some(
            command @ (PlaneCommand::Api(_)
            | PlaneCommand::Skill(_)
            | PlaneCommand::Dep(_)
            | PlaneCommand::Upgrade),
        ) => {
            let overrides = config_overrides_from_matches(&matches);
            let state = match AppState::from_env(overrides) {
                Ok(state) => state,
                Err(error) => return CommandResult::err(1, format!("plane: {error}\n")),
            };
            // `upgrade` already reports the latest version; skip the trailing
            // notice for it so we do not check twice.
            let run_passive = !matches!(command, PlaneCommand::Upgrade);
            let mut result = dispatch(
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
            );
            if run_passive {
                if let Some(notice) = crate::core::update::passive_notice(&state) {
                    result.stderr.push_str(&notice);
                }
            }
            result
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
        Some(PlaneCommand::Dep(command)) => execute_dep(state, command),
        Some(PlaneCommand::Upgrade) => execute_upgrade(state),
    }
}

fn execute_dep(state: &AppState, command: DepCommand) -> CommandResult {
    let result = match command.command {
        DepSubcommand::Add(args) => api::dep::add(
            state,
            api::dep::AddOptions {
                project: args.project,
                work_item: args.work_item,
                on: args.on,
                dry_run: args.dry_run,
            },
        ),
        DepSubcommand::Rm(args) => api::dep::rm(
            state,
            api::dep::RmOptions {
                project: args.project,
                work_item: args.work_item,
                on: args.on,
            },
        ),
        DepSubcommand::Ls(args) => api::dep::ls(
            state,
            api::dep::LsOptions {
                project: args.project,
                work_item: args.work_item,
                json: args.json,
            },
        ),
        DepSubcommand::Gc(args) => api::dep::gc(
            state,
            api::dep::GcOptions {
                project: args.project,
                write: args.write,
            },
        ),
    };
    match result {
        Ok(stdout) => CommandResult::ok(stdout),
        Err(error) => CommandResult::err(1, format!("plane: {error}\n")),
    }
}

fn execute_upgrade(state: &AppState) -> CommandResult {
    match crate::core::update::run_check(state) {
        Ok(stdout) => CommandResult::ok(stdout),
        Err(error) => CommandResult::err(1, format!("plane: {error}\n")),
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
        ApiSubcommand::Me(command) => api::me::me(state, ApiMeOptions { json: command.json }),
        ApiSubcommand::Project(command) => match command.command {
            ProjectSubcommand::List(args) => api::project::list(
                state,
                api::project::ListOptions {
                    all: args.all,
                    fields: args.fields,
                    expand: args.expand,
                    json: args.json,
                },
            ),
            ProjectSubcommand::Get(args) => api::project::get(
                state,
                &args.id,
                api::project::GetOptions {
                    fields: args.fields,
                    expand: args.expand,
                    json: args.json,
                },
            ),
            ProjectSubcommand::Create(args) => api::project::create(
                state,
                api::project::CreateOptions {
                    name: args.name,
                    identifier: args.identifier,
                    data: args.data,
                    dry_run: args.dry_run,
                    json: args.json,
                },
            ),
            ProjectSubcommand::Update(args) => api::project::update(
                state,
                api::project::UpdateOptions {
                    id: args.id,
                    name: args.name,
                    data: args.data,
                    dry_run: args.dry_run,
                    json: args.json,
                },
            ),
            ProjectSubcommand::Delete(args) => api::project::delete(state, &args.id, args.dry_run),
            ProjectSubcommand::Archive(args) => {
                api::project::archive(state, &args.id, args.dry_run)
            }
            ProjectSubcommand::Unarchive(args) => {
                api::project::unarchive(state, &args.id, args.dry_run)
            }
            ProjectSubcommand::Summary(args) => api::project::summary(state, &args.id),
        },
        ApiSubcommand::WorkItem(command) => match command.command {
            WorkItemSubcommand::List(args) => api::work_item::list(
                state,
                api::work_item::ListOptions {
                    project: args.project,
                    all: args.all,
                    fields: args.fields,
                    expand: args.expand,
                    json: args.json,
                },
            ),
            WorkItemSubcommand::Get(args) => api::work_item::get(
                state,
                api::work_item::GetOptions {
                    project: args.project,
                    id: args.id,
                    fields: args.fields,
                    expand: args.expand,
                    json: args.json,
                },
            ),
            WorkItemSubcommand::Create(args) => api::work_item::create(
                state,
                api::work_item::CreateOptions {
                    project: args.project,
                    name: args.name,
                    data: args.data,
                    dry_run: args.dry_run,
                    json: args.json,
                },
            ),
            WorkItemSubcommand::Update(args) => api::work_item::update(
                state,
                api::work_item::UpdateOptions {
                    project: args.project,
                    id: args.id,
                    name: args.name,
                    data: args.data,
                    dry_run: args.dry_run,
                    json: args.json,
                },
            ),
            WorkItemSubcommand::Delete(args) => api::work_item::delete(
                state,
                api::work_item::DeleteOptions {
                    project: args.project,
                    id: args.id,
                    dry_run: args.dry_run,
                },
            ),
        },
        ApiSubcommand::Request(command) => api::request::run(
            state,
            api::request::RequestOptions {
                method: command.method,
                path: command.path,
                data: command.data,
            },
        ),
        ApiSubcommand::State(command) => execute_crud(state, "states", command.command),
        ApiSubcommand::Label(command) => execute_crud(state, "labels", command.command),
        ApiSubcommand::Cycle(command) => execute_crud(state, "cycles", command.command),
        ApiSubcommand::Module(command) => execute_crud(state, "modules", command.command),
        ApiSubcommand::Estimate(command) => execute_crud(state, "estimates", command.command),
        ApiSubcommand::Intake(command) => execute_crud(state, "intake-issues", command.command),
        ApiSubcommand::Page(command) => execute_page(state, command.command),
        ApiSubcommand::Comment(command) => execute_comment(state, command.command),
        ApiSubcommand::Link(command) => execute_wi_sub(state, "links", command.command),
        ApiSubcommand::Relation(command) => execute_wi_sub(state, "relations", command.command),
        ApiSubcommand::Activity(command) => execute_activity(state, command.command),
        ApiSubcommand::Member(command) => execute_member(state, command.command),
    };
    match result {
        Ok(stdout) => CommandResult::ok(stdout),
        Err(error) => CommandResult::err(1, format!("plane: {error}\n")),
    }
}

fn execute_crud(
    state: &AppState,
    segment: &str,
    command: CrudSubcommand,
) -> Result<String, String> {
    match command {
        CrudSubcommand::List(args) => api::crud::list(
            state,
            &args.project,
            segment,
            api::crud::ListOptions {
                all: args.all,
                fields: args.fields,
                expand: args.expand,
                json: args.json,
            },
        ),
        CrudSubcommand::Get(args) => api::crud::get(
            state,
            &args.project,
            segment,
            &args.id,
            api::crud::GetOptions {
                fields: args.fields,
                expand: args.expand,
                json: args.json,
            },
        ),
        CrudSubcommand::Create(args) => api::crud::create(
            state,
            &args.project,
            segment,
            api::crud::CreateOptions {
                name: args.name,
                data: args.data,
                dry_run: args.dry_run,
                json: args.json,
            },
        ),
        CrudSubcommand::Update(args) => api::crud::update(
            state,
            &args.project,
            segment,
            &args.id,
            api::crud::UpdateOptions {
                name: args.name,
                data: args.data,
                dry_run: args.dry_run,
                json: args.json,
            },
        ),
        CrudSubcommand::Delete(args) => {
            api::crud::delete(state, &args.project, segment, &args.id, args.dry_run)
        }
    }
}

fn parse_page_access(value: &Option<String>) -> Result<Option<api::page::Access>, String> {
    match value.as_deref() {
        None => Ok(None),
        Some("public") => Ok(Some(api::page::Access::Public)),
        Some("private") => Ok(Some(api::page::Access::Private)),
        Some(other) => Err(format!(
            "invalid access '{other}'; expected 'public' or 'private'"
        )),
    }
}

fn execute_page(state: &AppState, command: PageSubcommand) -> Result<String, String> {
    match command {
        PageSubcommand::List(args) => api::crud::list(
            state,
            &args.project,
            "pages",
            api::crud::ListOptions {
                all: args.all,
                fields: args.fields,
                expand: args.expand,
                json: args.json,
            },
        ),
        PageSubcommand::Get(args) => api::page::get(
            state,
            &args.project,
            &args.id,
            api::page::GetOptions {
                content: args.content,
                fields: args.fields,
                expand: args.expand,
                json: args.json,
            },
        ),
        PageSubcommand::Create(args) => api::page::create(
            state,
            &args.project,
            api::page::CreateOptions {
                name: args.name,
                body: api::page::BodyArgs {
                    from_file: args.body.from_file.as_deref(),
                    body: args.body.body.as_deref(),
                    format: args.body.format.as_deref(),
                },
                access: parse_page_access(&args.access)?,
                data: args.data,
                dry_run: args.dry_run,
                json: args.json,
            },
        ),
        PageSubcommand::Update(args) => api::page::update(
            state,
            &args.project,
            &args.id,
            api::page::UpdateOptions {
                name: args.name,
                body: api::page::BodyArgs {
                    from_file: args.body.from_file.as_deref(),
                    body: args.body.body.as_deref(),
                    format: args.body.format.as_deref(),
                },
                access: parse_page_access(&args.access)?,
                data: args.data,
                dry_run: args.dry_run,
                json: args.json,
            },
        ),
        PageSubcommand::Delete(args) => {
            api::crud::delete(state, &args.project, "pages", &args.id, args.dry_run)
        }
    }
}

fn wi_sub_collection(
    state: &AppState,
    project: &str,
    work_item: &str,
    segment: &str,
) -> Result<String, String> {
    let workspace = api::require_workspace(state)?;
    Ok(format!(
        "workspaces/{workspace}/projects/{project}/work-items/{work_item}/{segment}/"
    ))
}

fn execute_wi_sub(
    state: &AppState,
    segment: &str,
    command: WiSubCommand,
) -> Result<String, String> {
    match command {
        WiSubCommand::List(a) => {
            let collection = wi_sub_collection(state, &a.project, &a.work_item, segment)?;
            api::generic::list(
                state,
                &collection,
                api::generic::ListOptions {
                    all: a.all,
                    fields: a.fields,
                    expand: a.expand,
                    json: a.json,
                },
            )
        }
        WiSubCommand::Get(a) => {
            let collection = wi_sub_collection(state, &a.project, &a.work_item, segment)?;
            api::generic::get(
                state,
                &collection,
                &a.id,
                api::generic::GetOptions {
                    fields: a.fields,
                    expand: a.expand,
                    json: a.json,
                },
            )
        }
        WiSubCommand::Create(a) => {
            let collection = wi_sub_collection(state, &a.project, &a.work_item, segment)?;
            api::generic::create(
                state,
                &collection,
                api::generic::WriteOptions {
                    data: a.data,
                    dry_run: a.dry_run,
                    json: a.json,
                },
            )
        }
        WiSubCommand::Update(a) => {
            let collection = wi_sub_collection(state, &a.project, &a.work_item, segment)?;
            api::generic::update(
                state,
                &collection,
                &a.id,
                api::generic::WriteOptions {
                    data: a.data,
                    dry_run: a.dry_run,
                    json: a.json,
                },
            )
        }
        WiSubCommand::Delete(a) => {
            let collection = wi_sub_collection(state, &a.project, &a.work_item, segment)?;
            api::generic::delete(state, &collection, &a.id, a.dry_run)
        }
    }
}

fn execute_comment(state: &AppState, command: CommentSubcommand) -> Result<String, String> {
    match command {
        CommentSubcommand::List(a) => api::comment::list(
            state,
            api::comment::ListOptions {
                project: &a.project,
                work_item: &a.work_item,
                all: a.all,
                fields: a.fields.clone(),
                expand: a.expand.clone(),
                json: a.json,
            },
        ),
        CommentSubcommand::Get(a) => api::comment::get(
            state,
            api::comment::GetOptions {
                project: &a.project,
                work_item: &a.work_item,
                id: &a.id,
                fields: a.fields.clone(),
                expand: a.expand.clone(),
                json: a.json,
            },
        ),
        CommentSubcommand::Create(a) => api::comment::create(
            state,
            api::comment::CreateOptions {
                project: &a.project,
                work_item: &a.work_item,
                body: api::page::BodyArgs {
                    from_file: a.body.from_file.as_deref(),
                    body: a.body.body.as_deref(),
                    format: a.body.format.as_deref(),
                },
                data: a.data.clone(),
                dry_run: a.dry_run,
                json: a.json,
            },
        ),
        CommentSubcommand::Update(a) => api::comment::update(
            state,
            api::comment::UpdateOptions {
                project: &a.project,
                work_item: &a.work_item,
                id: &a.id,
                body: api::page::BodyArgs {
                    from_file: a.body.from_file.as_deref(),
                    body: a.body.body.as_deref(),
                    format: a.body.format.as_deref(),
                },
                data: a.data.clone(),
                dry_run: a.dry_run,
                json: a.json,
            },
        ),
        CommentSubcommand::Delete(a) => api::comment::delete(
            state,
            api::comment::DeleteOptions {
                project: &a.project,
                work_item: &a.work_item,
                id: &a.id,
                dry_run: a.dry_run,
            },
        ),
    }
}

fn execute_activity(state: &AppState, command: ActivitySubCommand) -> Result<String, String> {
    match command {
        ActivitySubCommand::List(a) => {
            let collection = wi_sub_collection(state, &a.project, &a.work_item, "activities")?;
            api::generic::list(
                state,
                &collection,
                api::generic::ListOptions {
                    all: a.all,
                    fields: a.fields,
                    expand: a.expand,
                    json: a.json,
                },
            )
        }
        ActivitySubCommand::Get(a) => {
            let collection = wi_sub_collection(state, &a.project, &a.work_item, "activities")?;
            api::generic::get(
                state,
                &collection,
                &a.id,
                api::generic::GetOptions {
                    fields: a.fields,
                    expand: a.expand,
                    json: a.json,
                },
            )
        }
    }
}

fn member_collection(state: &AppState, project: &str) -> Result<String, String> {
    let workspace = api::require_workspace(state)?;
    Ok(format!(
        "workspaces/{workspace}/projects/{project}/members/"
    ))
}

fn execute_member(state: &AppState, command: MemberSubCommand) -> Result<String, String> {
    match command {
        MemberSubCommand::List(a) => {
            let collection = member_collection(state, &a.project)?;
            api::generic::list(
                state,
                &collection,
                api::generic::ListOptions {
                    all: a.all,
                    fields: a.fields,
                    expand: a.expand,
                    json: a.json,
                },
            )
        }
        MemberSubCommand::Get(a) => {
            let collection = member_collection(state, &a.project)?;
            api::generic::get(
                state,
                &collection,
                &a.id,
                api::generic::GetOptions {
                    fields: a.fields,
                    expand: a.expand,
                    json: a.json,
                },
            )
        }
        MemberSubCommand::Create(a) => {
            let collection = member_collection(state, &a.project)?;
            api::generic::create(
                state,
                &collection,
                api::generic::WriteOptions {
                    data: a.data,
                    dry_run: a.dry_run,
                    json: a.json,
                },
            )
        }
        MemberSubCommand::Update(a) => {
            let collection = member_collection(state, &a.project)?;
            api::generic::update(
                state,
                &collection,
                &a.id,
                api::generic::WriteOptions {
                    data: a.data,
                    dry_run: a.dry_run,
                    json: a.json,
                },
            )
        }
        MemberSubCommand::Delete(a) => {
            let collection = member_collection(state, &a.project)?;
            api::generic::delete(state, &collection, &a.id, a.dry_run)
        }
        MemberSubCommand::WorkspaceList(a) => {
            let workspace = api::require_workspace(state)?;
            let collection = format!("workspaces/{workspace}/members/");
            api::generic::list(
                state,
                &collection,
                api::generic::ListOptions {
                    all: a.all,
                    fields: a.fields,
                    expand: a.expand,
                    json: a.json,
                },
            )
        }
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
                force: command.force,
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
