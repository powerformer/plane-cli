---
name: plane-cli
description: Use when installing, bootstrapping, upgrading, or managing the Plane CLI and its agent skills, or when working in a repository that expects the Plane CLI.
metadata:
  short-description: Bootstrap and manage Plane CLI and agent skills
---

# plane-cli

Use `plane` when the user asks to install, upgrade, or manage Plane CLI agent skills, or when they are working in a repository that expects the Plane CLI.

The CLI is the command truth source. Prefer running `plane --help`, `plane skill --help`, or a subcommand-specific `--help` before assuming behavior.

## Bootstrap

Go from nothing to a working `plane` on a new machine.

### 1. Install

Unix or macOS:

```bash
curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh -s -- install --channel beta
```

Windows PowerShell:

```powershell
$m = Join-Path $env:TEMP "plane-manage.ps1"
iwr https://releases.plane.powerformer.net/manage.ps1 -OutFile $m
pwsh -File $m install --channel beta
```

The manager installs a command entry at `~/.local/bin/plane` (Unix) or
`%USERPROFILE%\.local\bin\plane.cmd` (Windows). If that directory is not on
`PATH`, call the full path, or run the manager with `path setup` and reopen the
shell.

### 2. Verify the help loop

```bash
plane --version
plane help
```

### 3. Configure API access

Point the CLI at your own Plane backend and a personal API token you generate in
your Plane account. Replace `<PLANE_BASE_URL>` with your Plane server URL (the
CLI appends `/api/v1`) and `<PLANE_API_TOKEN>` with your token.

Persistent, in `~/.plane/plane.toml`:

```toml
api_base_url = "<PLANE_BASE_URL>"
api_key = "<PLANE_API_TOKEN>"
```

One-off, via environment for a single command:

```bash
PLANE_API_BASE_URL=<PLANE_BASE_URL> PLANE_API_KEY=<PLANE_API_TOKEN> plane api me
```

### 4. Verify the API loop

```bash
plane api me
```

Success prints a short summary of the authenticated user. The token is sent as
`X-API-Key` and is never printed. Whether the request reaches your backend
depends on your own network access (for example IP allowlisting) and token
validity, which are outside the CLI.

## Common Commands

- `plane --help`: show the top-level command surface.
- `plane --version`: show the installed binary version.
- `plane skill install`: install the `plane-cli` skill into detected agent skill directories.
- `plane skill install --path <dir>`: install into an explicit final skill directory. The path must end with `plane-cli`.
- `plane skill list`: list skill paths managed by Plane.
- `plane skill upgrade`: upgrade all managed skill installations to the selected published version.
- `plane skill uninstall`: uninstall only paths recorded in Plane managed state.

## Managed State

Plane only manages skill paths recorded in the resolved Plane state path, which defaults to `~/.plane/state/skills.json`.
Each installed skill directory also contains `metadata.json`.

Do not delete or overwrite user-created skill directories unless they are recorded as Plane-managed paths and contain Plane-managed metadata.

## Version Selection

By default, skill install and upgrade use the stable published version.
Use `--channel beta` to try the beta channel, or `--version <version>` to pin a specific published version.
