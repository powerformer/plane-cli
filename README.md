# plane-cli

Public Rust command line interface for Plane.

The first release is intentionally small. `plane` currently provides stable
help and version output while the repository, release, and installation
machinery are put in place.

## Install

Unix:

```bash
curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh
```

Windows PowerShell:

```powershell
irm https://releases.plane.powerformer.net/manage.ps1 | pwsh
```

Install the latest beta:

```bash
curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh -s -- install --channel beta
```

Pin a version:

```bash
curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh -s -- install --version v0.1.0-beta.1 --channel beta
```

Uninstall:

```bash
curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh -s -- uninstall
```

## Usage

```bash
plane help
plane --help
plane --version
```

## Development

```bash
python3 scripts/init.py
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p plane-cli -- help
```

Repo-local operator commands use `cli.sh` / `cli.ps1` wrappers rather than the
installable `plane` binary:

```bash
./cli.sh release --channel=beta [options]
./cli.sh release --channel=stable [options]
```

Create and merge PRs through GitHub directly so organization review rules stay
visible. For source-change shape, branch names, commit/PR conventions, and release
workflow notes, see [AGENTS.md](./AGENTS.md).
