---
name: plane-cli
description: Use when installing, configuring, or upgrading the Plane CLI, calling the Plane API (projects, work items, pages, comments, members, intake), or managing plane-cli agent skills.
metadata:
  short-description: Bootstrap and manage Plane CLI and agent skills
---

# plane-cli

Use `plane` to install, configure, and upgrade the Plane CLI, to call the Plane
API (projects, work items, and their sub-resources), and to manage plane-cli
agent skills — or whenever a repository expects the Plane CLI.

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

### 3. Get an API token

The CLI defaults its backend to `https://plane.powerformer.net`, so the one
thing you must supply is a personal API token:

1. Open the Plane backend (default `https://plane.powerformer.net`) and sign in.
2. Click your avatar in the top-right, then go to **Settings**.
3. Open **API Tokens** (personal access tokens), create one, and copy it — the
   token is shown only once.

### 4. Configure the CLI

Write the token to `$PLANE_HOME/plane.toml` (default `~/.plane/plane.toml`).
Replace `<PLANE_API_TOKEN>` with the token you just copied:

```toml
api_key = "<PLANE_API_TOKEN>"
```

Only set `api_base_url` if your backend differs from the default (the CLI
appends `/api/v1`):

```toml
api_base_url = "<PLANE_BASE_URL>"
api_key = "<PLANE_API_TOKEN>"
```

One-off, via environment for a single command:

```bash
PLANE_API_KEY=<PLANE_API_TOKEN> plane api me
```

### 5. Verify the API loop

```bash
plane api me
```

Success prints a short summary of the authenticated user. The token is sent as
`X-API-Key` and is never printed. Whether the request reaches the backend
depends on your own network access (for example IP allowlisting) and token
validity, which are outside the CLI.

## Common Commands

- `plane --help`: show the top-level command surface.
- `plane --version`: show the installed binary version.
- `plane upgrade`: check the release channel for a newer `plane` and print the command to upgrade (it reports only; the manager performs the upgrade).
- `plane dep add|rm|ls|gc`: manage cross-project work-item dependency edges, stored as `dep:<KEY>:<SEQ>` labels; see [references/api.md](./references/api.md).
- `plane api work-item attach --item <KEY-SEQ> --file <path>`: upload a file attachment to a work item (server-proxied; the CLI never touches object storage); see [references/api.md](./references/api.md).
- `plane skill install`: install the `plane-cli` skill into detected agent skill directories.
- `plane skill install --path <dir>`: install into an explicit final skill directory. The path must end with `plane-cli`.
- `plane skill list`: list skill paths managed by Plane.
- `plane skill upgrade`: upgrade all managed skill installations to the selected published version.
- `plane skill uninstall`: uninstall only paths recorded in Plane managed state.

## API commands

Beyond `api me`, the CLI wraps the Plane REST API as typed subcommands —
`project`, `work-item`, `state`/`label`/`cycle`/`module`/`estimate`/`intake`,
`page` (documents), `comment`/`link`/`relation`/`activity`, and `member`. Most
share the verbs `list`/`get`/`create`/`update`/`delete`, scoped by `--workspace`,
`--project`, and `--work-item`, with `--json`, `--all`, `--fields`/`--expand`,
`--data`, and `--dry-run`. A `request` passthrough covers anything not yet typed.

When you report a resource, also print its full URL — the Plane work-item browse
link or the related GitHub issue/PR — so the user can jump straight to it.

- Full resource/verb tables, field values, page (document) authoring, work-item
  page associations, and the
  escape hatch: [references/api.md](./references/api.md).
- End-to-end scenarios (stand up a project, drive a work item, triage intake):
  [references/scenarios.md](./references/scenarios.md).
- `plane api --help` and `plane api <resource> --help` stay the truth source.

## Managed State

Plane only manages skill paths recorded in the resolved Plane state path, which defaults to `~/.plane/state/skills.json`.
Each installed skill directory also contains `metadata.json`.

Do not delete or overwrite user-created skill directories unless they are recorded as Plane-managed paths and contain Plane-managed metadata.

## Version Selection

By default, skill install and upgrade use the stable published version.
Use `--channel beta` to try the beta channel, or `--version <version>` to pin a specific published version.
