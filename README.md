# plane-cli

Public Rust command line interface for Plane.

`plane` includes self-describing help and a managed agent skill installer for
Claude Code, Codex, OpenCode, and explicit custom paths.

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

Upgrade the binary and any already-managed skill installations:

```bash
curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh -s -- upgrade
```

## Usage

```bash
plane help
plane --help
plane --version
plane skill --help
plane skill install --help
plane skill install --channel beta
plane skill install --path /path/to/skills/plane-cli --channel beta
plane skill list
plane skill upgrade --channel beta
plane skill uninstall
```

`plane` resolves configuration at startup. The config path is `--config`, then
`PLANE_CONFIG`, then `{PLANE_HOME:-~/.plane}/plane.toml`. Runtime paths use
`arg > config file > env > default`; managed skill state defaults to
`~/.plane/state/skills.json`. `plane skill uninstall` only removes paths recorded
there and confirmed by the installed skill's `metadata.json`.

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
