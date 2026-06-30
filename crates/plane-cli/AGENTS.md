# AGENTS

`crates/plane-cli/` owns the installable `plane` binary: app state, command
dispatch, the config substrate, the Plane API client and commands, managed skill
installation, and CLI-facing behavior. The crate is split into a `core/`
substrate and a `commands/` surface.

## Directory Rules

- `src/main.rs` wires logging, command dispatch, output printing, and exit codes.
- `src/core/` owns the substrate:
  - `app.rs` owns `AppState` and build-version resolution.
  - `config/` owns the typed configuration substrate used by app state.
  - `request.rs` owns the Plane `/api/v1` client; `error.rs` its error types.
  - `model/` holds the loose serde models for API responses.
  - `skill.rs` owns managed skill install/upgrade/uninstall.
  - `update.rs` owns the best-effort "newer release" check (`plane upgrade` and
    the passive notice).
  - `logger.rs` owns tracing setup.
- `src/commands/` owns the command surface:
  - `mod.rs` owns clap parsing, dispatch, and help text. Keep CLI behavior stable
    and update tests when output or accepted arguments move.
  - `output.rs` owns `CommandResult` modeling and stdout/stderr emission.
  - `api/` owns the Plane API subcommands (project, work-item, generic CRUD,
    sub-resources, members, passthrough).
- `tests/unit/` contains CLI unit coverage. Register each new unit test module
  in `tests/unit.rs`.

Do not add release, repository orchestration, R2 publishing, or runtime service
management here.

## Common Commands

```bash
cargo test --locked -p plane-cli --test unit
cargo run --locked -p plane-cli -- help
cargo run --locked -p plane-cli -- --version
```

## Standard Workflow

- Keep command output small and stable.
- Add tests for every accepted command or output shape change.
- Use `AppState` for process-level context that future commands need.
- Keep support operations in `scripts/cli/` behind `cli.sh` / `cli.ps1`.
