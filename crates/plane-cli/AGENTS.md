# AGENTS

`crates/plane-cli/` owns the installable `plane` binary, app state, command
dispatch, config substrate, output model, and CLI-facing behavior.

## Directory Rules

- `src/main.rs` wires state creation, command dispatch, output printing, and
  exit codes.
- `src/app.rs` owns `AppState` and build-version resolution.
- `src/cli.rs` owns command parsing and help text. Keep CLI behavior stable and
  update tests when output or accepted arguments move.
- `src/config/` owns the typed configuration substrate used by app state.
- `src/output.rs` owns command result modeling and stdout/stderr emission.
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
