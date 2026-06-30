# AGENTS

`plane-cli` is the public Rust command line interface for Plane.

The current product surface is intentionally minimal: the installable `plane`
binary owns command dispatch, help/version output, the app-state/config
substrate, managed skill installation, and small read-only Plane API smoke
checks. It does not yet own Plane product workflows, service orchestration, or
project mutations.

## Directory Rules

- `crates/` contains the Rust workspace crates. Each core crate owns its local
  `AGENTS.md`; read the child file before editing that subtree.
- `.github/workflows/` contains CI and release workflows.
- `.github/scripts/` contains workflow-only helper scripts. Keep workflow-only
  scripts there.
- `cli.sh` and `cli.ps1` are the repo-local operator entrypoints for support
  tasks that do not belong in the installable `plane` binary. Every subcommand
  is invoked with a leading colon, e.g. `./cli.sh :release` and `./cli.sh :init`.
- `scripts/` contains the repo-local uv-managed Python support command tree
  used by the repo-local operator wrappers.
- `.local/` is repo-local private operator state. It must stay gitignored and
  must not become a source of truth for product behavior.
- `./cli.sh :init` (`scripts/cli/init.py`) is the idempotent post-clone
  initializer. It quick-fails on missing required tools or repository
  entrypoints, installs local git hooks from
  `scripts/resources/templates/hooks/`, and exits cleanly only when the checkout
  is ready for development.
- `manage.sh` and `manage.ps1` are the public install/uninstall entrypoints at
  the repository root.
- `manage.sh path setup|clear` and `manage.ps1 path setup|clear` are the
  explicit PATH profile mutation commands. Install and upgrade may create the
  user command entry, but must only prompt for PATH setup when `plane` is not
  already directly resolvable in the current shell.
- Release and manager downloads use R2 metadata and artifacts as the source of
  truth.

### Recursive AGENTS Index

- `crates/plane-cli/AGENTS.md`: installable `plane` binary, app state, command
  dispatch, config substrate, output model, and CLI-facing behavior.

When adding or removing a core subtree, update this index in the same change.
Child `AGENTS.md` files should stay local: ownership, directory shape,
commands, workflow notes, and FAQ for that subtree.

### Project Boundaries

- Keep the CLI capability small until real Plane workflows are designed.
- Keep support operations in repo-local operator commands rather than the
  product binary.
- Keep release metadata, artifact packaging, and smoke installation in workflow
  scripts, not in Rust product code.
- Prefer stable help, version, output, and app-state boundaries over early
  feature breadth.

## Common Commands

```bash
./cli.sh :init
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p plane-cli -- help
./cli.sh :release --channel=beta --dry-run
```

`./cli.sh :init` is the default post-clone command. Use `--force` only when
intentionally replacing existing non-init hooks; it backs them up first.

## Standard Workflow

### Initialize

After cloning or when hooks look stale, run:

```bash
./cli.sh :init
```

Hook bodies live in `scripts/resources/templates/hooks/`. The pre-commit hook
runs fmt, clippy, tests, the CLI help smoke, and shell/Python/PowerShell syntax
checks (PowerShell only when `pwsh` is available). The commit-msg hook validates
the commit subject shape.

### Branch Names

Use `<area>/<kebab-case-slug>`, where `<area>` matches the touched crate or
concern. Examples:

- `cli/help-surface`
- `release/prepare-0.1.0`
- `scripts/release-bootstrap`

### Commit Messages

Subject: `<area>: <imperative summary>` on one line, ideally <= 72 characters.
The body explains why the change is shaped this way first, then the change
list. End with any `Co-Authored-By:` trailers when pair-coded or
agent-assisted.

### Tests

Unit tests for `plane-cli` live under `crates/plane-cli/tests/unit/<area>.rs`
and are registered in `crates/plane-cli/tests/unit.rs`:

```rust
#[path = "../src/<file>.rs"]
mod <module>;
#[path = "unit/<area>.rs"]
mod <area>_cases;
```

Tests that need writable fixtures should use
`std::env::temp_dir().join(format!("plane-<slug>-{pid}-{seq}"))` and clean up
with `fs::remove_dir_all` at the end of each case.

### Pre-PR Checks

Every PR must pass these commands before review:

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p plane-cli -- help
```

CI reruns them in the `guard` workflow.

### PR Descriptions

Use these top-level sections, in order:

```markdown
## Why
<what is broken or missing today>

## What
<concrete change list; reference filenames and modules>

## Tests
<commands run and results>
```

Add `## Compatibility` when an output shape, config field, or exit-code
behavior moves. Add `## Trade-off worth flagging` when the change has a
downside that reviewers should hold in mind.

### Merging

`main` is PR-only once repository protection is enabled and is protected by the
`guard` workflow. Required approvals can stay `0`; the guard matrix is the
merge gate.

Create and merge PRs through GitHub directly so organization review rules stay
visible. Do not depend on a repo-local merge helper to bypass or obscure the
approval path.

## FAQ

### Does `plane` Implement Plane Workflows Yet?

Not beyond read-only API smoke checks. The current CLI is a releaseable shell
with help/version output, managed skill installation, and internal structure for
future commands.

### Where Do Installer Changes Go?

Public install/uninstall entrypoints live at the repository root as
`manage.sh` and `manage.ps1`. Release and smoke scripts should reference those
root files.

### Where Do Workflow Helper Scripts Go?

Workflow-only helpers belong under `.github/scripts/`. The repository
initialization entrypoint is `./cli.sh :init` (`scripts/cli/init.py`); additional
local support commands, if added, should use `cli.sh` / `cli.ps1` plus
`scripts/cli/`.
