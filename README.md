# plane-cli

[![guard](https://github.com/powerformer/plane-cli/actions/workflows/guard.yml/badge.svg)](https://github.com/powerformer/plane-cli/actions/workflows/guard.yml)
[![release](https://img.shields.io/github/v/tag/powerformer/plane-cli?sort=semver&label=release)](https://github.com/powerformer/plane-cli/tags)
![platforms](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-blue)
[![license](https://img.shields.io/badge/license-MIT-blue)](./LICENSE)

Public Rust command line interface for Plane. Manage projects, work items, and
pages from your terminal — and install an agent skill so Claude Code, Codex, and
OpenCode can drive Plane too.

## Install

Unix / macOS:

```bash
curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh
```

Windows PowerShell:

```powershell
irm https://releases.plane.powerformer.net/manage.ps1 | pwsh
```

This installs the latest stable `plane` to `~/.local/bin/plane` (Unix) or
`%USERPROFILE%\.local\bin\plane.cmd` (Windows). If that directory is on your
`PATH`, you're ready; otherwise see [More install options](#more-install-options).

## Quick start

1. Get an API token: sign in to your Plane backend (default
   `https://plane.powerformer.net`), then **avatar → Settings → API Tokens →**
   create one and copy it.

2. Save it to `~/.plane/plane.toml`. Set `api_base_url` only if your backend is
   not the default; `workspace_slug` is the default workspace for the
   `project`/`work-item`/`page` commands:

   ```toml
   api_key = "<PLANE_API_TOKEN>"
   workspace_slug = "<your-workspace-slug>"
   ```

3. Verify access and explore:

   ```bash
   plane api me
   plane api project list
   plane api work-item create --project <PROJECT_ID> --name "Fix login" --data '{"priority":"high"}'
   plane api page create --project <PROJECT_ID> --name "Design Review" --from-file notes.md
   ```

`plane --help` and `plane api <resource> --help` are the source of truth for
every command and flag.

## Agent skill

`plane` ships a managed skill so coding agents discover the CLI cold:

```bash
plane skill install
```

It installs into detected Claude Code, Codex, and OpenCode homes (or an explicit
`--path <dir>/plane-cli`). Manage installs with `plane skill list`,
`plane skill upgrade`, and `plane skill uninstall`.

## Configuration

`plane` resolves configuration at startup: the config path is `--config`, then
`PLANE_CONFIG`, then `{PLANE_HOME:-~/.plane}/plane.toml`. Individual runtime
values use `arg > config file > env > default`; managed skill state defaults to
`~/.plane/state/skills.json`.

## Upgrade

```bash
curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh -s -- upgrade
```

`plane upgrade` checks the release channel for a newer binary and prints the
command to run (it never replaces the binary itself — the manager does that).

## More install options

<details>
<summary>Beta channel, pinning, uninstall, and PATH setup</summary>

Install the latest beta, or pin an exact version:

```bash
curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh -s -- install --channel beta
curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh -s -- install --version v0.1.0 --channel stable
```

Uninstall:

```bash
curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh -s -- uninstall
```

The manager only creates the user command entry; it does not edit your `PATH`
unless you ask. To persist PATH setup explicitly:

```bash
curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh -s -- path setup
curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh -s -- path clear
```

PowerShell:

```powershell
$manager = Join-Path $env:TEMP "plane-manage.ps1"
iwr https://releases.plane.powerformer.net/manage.ps1 -OutFile $manager
pwsh -File $manager path setup
pwsh -File $manager path clear
```

`path clear` removes only the fixed `plane-cli path` marker block written by
`path setup`; it does not touch user-authored `PATH` lines.

</details>

## Development

Building, testing, hooks, releases, and the repository layout are documented in
[AGENTS.md](./AGENTS.md), which is also the entry brief for AI coding agents.

## License

[MIT](./LICENSE).
