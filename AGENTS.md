# AGENTS

## Meta

This file is the root operating brief for the repository. It is organized as a
fixed set of blocks — keep them, in this order:

1. **Purpose** — what `plane-cli` is, and what the binary does and does not own.
2. **Directory conventions** — where each kind of file lives.
3. **Core file index** — the entrypoints and the recursive `AGENTS.md` map.
4. **Maintenance flows** — commands for init, local development, and release.
5. **FAQ** — recurring questions.

Each Rust crate owns a local `crates/<crate>/AGENTS.md`; read the child file
before editing that subtree, and keep it scoped to that subtree (ownership,
directory shape, commands). When you add or remove a subtree or a top-level
entrypoint, update the **Core file index** in the same change.

## Purpose

`plane-cli` is the public Rust command line interface for Plane.

The installable `plane` binary owns command dispatch, help/version output, the
app-state/config substrate, managed skill installation, an update check, and a
write-capable Plane API surface: projects, work items and their sub-resources,
the project resources (state/label/cycle/module/estimate/intake), members, and a
`request` passthrough. The API client stays hand-written and loosely typed until
a 1.x OpenAPI pass.

The binary does not own Plane service orchestration or release infrastructure;
those live in workflow scripts and the deployment repo. Hold that boundary:

- Grow the Plane API surface deliberately; keep the client loosely typed (no
  generated bindings) until a 1.x OpenAPI pass, and add resources through the
  shared CRUD abstractions rather than ad hoc.
- Keep support operations in repo-local operator commands rather than the
  product binary.
- Keep release metadata, artifact packaging, and smoke installation in workflow
  scripts, not in Rust product code.
- Prefer stable help, version, output, and app-state boundaries.

## Directory conventions

- `crates/` — the Rust workspace crates. Each owns a local `AGENTS.md`.
- `.github/workflows/` — CI and release workflows.
- `.github/scripts/` — workflow-only helper scripts; keep workflow-only scripts
  here.
- `cli.sh` / `cli.ps1` — repo-local operator entrypoints. Every subcommand takes
  a leading colon, e.g. `./cli.sh :release`, `./cli.sh :init`.
- `scripts/` — the uv-managed Python support tree behind those wrappers:
  commands in `scripts/cli/<command>.py`, shared helpers in `scripts/lib/`, and
  generated-file templates in `scripts/resources/`.
- `manage.sh` / `manage.ps1` — the public install/uninstall entrypoints. `path
  setup|clear` are the explicit PATH-profile mutation commands; install/upgrade
  may create the user command entry but prompt for PATH setup only when `plane`
  is not already resolvable in the current shell.
- `.local/` — repo-local private operator state. Stays gitignored; never a
  source of truth for product behavior.
- Release and manager downloads use R2 metadata and artifacts as the source of
  truth.

## Core file index

Recursive `AGENTS.md`:

- `crates/plane-cli/AGENTS.md` — the installable `plane` binary: app state,
  command dispatch, config substrate, Plane API client and commands, managed
  skill installation, and output model.

Entrypoints:

| Path | Role |
| --- | --- |
| `manage.sh` / `manage.ps1` | public install / uninstall / PATH setup |
| `cli.sh` / `cli.ps1` | repo-local operator dispatch (`:<command>`) |
| `scripts/cli/init.py` | `./cli.sh :init` — local development initializer |
| `scripts/cli/release.py` | `./cli.sh :release` — trigger a release workflow |
| `scripts/resources/templates/hooks/` | git hook bodies installed by `:init` |
| `.github/workflows/guard.yml` | pre-merge gate (fmt, clippy, test, help) |
| `.github/workflows/release-{beta,stable}.yml` | build, publish, smoke a release |
| `.github/scripts/release/` | release helper scripts (metadata, R2, smoke) |

When you add or remove a top-level entrypoint or a crate subtree, update this
index in the same change.

## Maintenance flows

### Repository initialization

After cloning, or when hooks look stale:

```bash
./cli.sh :init
```

It quick-fails on missing required tools or repository entrypoints, then installs
the git hooks from `scripts/resources/templates/hooks/`. Use `--force` only to
replace existing non-init hooks; it backs them up first. The pre-commit hook runs
fmt, clippy, tests, the CLI help smoke, and shell/Python/PowerShell syntax checks
(PowerShell only when `pwsh` is available); the commit-msg hook validates the
subject shape.

### Local development

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p plane-cli -- help
```

These four are the pre-PR gate; CI reruns them in the `guard` workflow.

- **Testing against a real backend:** keep a gitignored `.local/.plane/plane.toml`
  and pass it with `--config`, e.g. `plane --config .local/.plane/plane.toml api me`.
  Relative paths resolve from the config file's directory, so this keeps managed
  state under `.local/`:

  ```toml
  home = "."
  state_dir = "state"
  skills_state_path = "state/skills.json"

  api_base_url = "https://plane.example.com"
  api_key = "plane-api-token"
  workspace_slug = "workspace-slug"
  ```
- **Branch names:** `<area>/<kebab-case-slug>`, where `<area>` matches the
  touched crate or concern (e.g. `cli/help-surface`, `release/prepare-0.1.0`).
- **Commit subject:** `<area>: <imperative summary>` on one line, ideally <= 72
  characters. The body explains why the change is shaped this way first, then the
  change list; end with `Co-Authored-By:` trailers when pair-coded or
  agent-assisted.
- **Tests:** unit tests live under `crates/plane-cli/tests/unit/<area>.rs`,
  registered in `crates/plane-cli/tests/unit.rs`:

  ```rust
  #[path = "../src/<file>.rs"]
  mod <module>;
  #[path = "unit/<area>.rs"]
  mod <area>_cases;
  ```

  Tests that need writable fixtures use
  `std::env::temp_dir().join(format!("plane-<slug>-{pid}-{seq}"))` and clean up
  with `fs::remove_dir_all` at the end of each case.
- **PR description:** `## Why`, `## What`, `## Tests`, in that order. Add
  `## Compatibility` when an output shape, config field, or exit-code behavior
  moves, and `## Trade-off worth flagging` when the change has a downside
  reviewers should hold in mind.

### Release: beta and stable

Trigger a release through the operator CLI (dry-run first):

```bash
./cli.sh :release --channel=beta --dry-run
./cli.sh :release --channel=beta
./cli.sh :release --channel=stable
```

`:release` dispatches the `release-beta.yml` / `release-stable.yml` workflows,
which build the binaries, publish artifacts and metadata to R2, and run the
install/skill smoke across Linux, macOS, and Windows. `main` is PR-only and gated
by the `guard` workflow; required approvals can stay `0` — the guard matrix is
the merge gate. Create and merge PRs through GitHub directly so organization
review rules stay visible; do not use a repo-local merge helper to bypass or
obscure the approval path.

## FAQ

### What Plane API surface does `plane` cover?

`plane api` covers projects, work items and their sub-resources
(comments/links/relations/activity), the project resources
(state/label/cycle/module/estimate/intake), workspace and project members, and a
`request` passthrough for anything not yet typed. The client is hand-written and
loosely typed; Plane service orchestration and deployment stay out of the binary.

### Where do installer changes go?

Public install/uninstall entrypoints live at the repository root as `manage.sh`
and `manage.ps1`. Release and smoke scripts should reference those root files.

### Where do workflow helper scripts go?

Workflow-only helpers belong under `.github/scripts/`. The repository
initialization entrypoint is `./cli.sh :init` (`scripts/cli/init.py`); additional
local support commands, if added, should use `cli.sh` / `cli.ps1` plus
`scripts/cli/`.
