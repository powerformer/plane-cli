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
- `plane skill install`: install the `plane-cli` skill into detected agent skill directories.
- `plane skill install --path <dir>`: install into an explicit final skill directory. The path must end with `plane-cli`.
- `plane skill list`: list skill paths managed by Plane.
- `plane skill upgrade`: upgrade all managed skill installations to the selected published version.
- `plane skill uninstall`: uninstall only paths recorded in Plane managed state.

## API commands

Beyond `api me`, the CLI wraps the Plane REST API as typed subcommands. Run
`plane api --help`, or `plane api <resource> --help`, for the authoritative list
of resources and flags.

- Workspace-scoped: `project`
  (`list`/`get`/`create`/`update`/`delete`/`archive`/`unarchive`/`summary`) and
  `member workspace-list`.
- Project-scoped (pass `--project <PROJECT_ID>`): `work-item`, `state`, `label`,
  `cycle`, `module`, `estimate`, `intake`, and `member`.
- Work-item-scoped (pass `--project` and `--work-item`): `comment`, `link`,
  `relation`, and `activity` (read-only).

Most resources share the same verbs — `list`, `get`, `create`, `update`,
`delete` — with shared conventions:

- `--workspace <SLUG>` selects the workspace (or set `workspace_slug` in
  `plane.toml`); `--project` and `--work-item` scope nested resources.
- `--json` prints the raw API response; `--all` follows cursor pages, and with
  `--json` accumulates every page into one JSON array.
- `--fields <CSV>` and `--expand <CSV>` trim or expand the response.
- `create`/`update` accept typed flags plus `--data '<JSON>'` for any other
  fields; `--dry-run` prints the request instead of sending it.

For endpoints the typed commands do not cover, use the escape hatch, which
supports GET, POST, PATCH, PUT, and DELETE:

```bash
plane api request --method PATCH workspaces/<slug>/projects/<id>/ --data '{"name":"New"}'
```

### Print resource links

CLI output is id-centric. When you report a resource back to the user, also
print its full URL so they can open it directly instead of re-deriving it from
ids:

- Plane work item:
  `<SERVER_URL>/<workspace>/browse/<PROJECT_IDENTIFIER>-<sequence_id>/` — e.g.
  `https://plane.powerformer.net/acme/browse/PLANE-7/`. `<SERVER_URL>` is the
  Plane backend without the `/api/v1` suffix (default
  `https://plane.powerformer.net`); the project identifier comes from
  `plane api project get <id>`, and `sequence_id` is on the work item.
- Related GitHub issue or pull request: print the full
  `https://github.com/<owner>/<repo>/issues/<n>` or `.../pull/<n>` URL.

Prefer absolute URLs over bare ids in any summary you write back to the user.

## Managed State

Plane only manages skill paths recorded in the resolved Plane state path, which defaults to `~/.plane/state/skills.json`.
Each installed skill directory also contains `metadata.json`.

Do not delete or overwrite user-created skill directories unless they are recorded as Plane-managed paths and contain Plane-managed metadata.

## Version Selection

By default, skill install and upgrade use the stable published version.
Use `--channel beta` to try the beta channel, or `--version <version>` to pin a specific published version.
