---
name: plane-cli
description: Use when installing, upgrading, or managing Plane CLI agent skills, or when working in a repository that expects the Plane CLI.
metadata:
  short-description: Manage Plane CLI and Plane agent skills
---

# plane-cli

Use `plane` when the user asks to install, upgrade, or manage Plane CLI agent skills, or when they are working in a repository that expects the Plane CLI.

The CLI is the command truth source. Prefer running `plane --help`, `plane skill --help`, or a subcommand-specific `--help` before assuming behavior.

## Common Commands

- `plane --help`: show the top-level command surface.
- `plane --version`: show the installed binary version.
- `plane skill install`: install the `plane-cli` skill into detected agent skill directories.
- `plane skill install --path <dir>`: install into an explicit final skill directory. The path must end with `plane-cli`.
- `plane skill list`: list skill paths managed by Plane.
- `plane skill upgrade`: upgrade all managed skill installations to the selected release.
- `plane skill uninstall`: uninstall only paths recorded in Plane managed state.

## Managed State

Plane only manages skill paths recorded in the resolved Plane state path, which defaults to `~/.plane/state/skills.json`.
Each installed skill directory also contains `metadata.json`.

Do not delete or overwrite user-created skill directories unless they are recorded as Plane-managed paths and contain Plane-managed metadata.

## Release Behavior

By default, skill install and upgrade resolve artifacts from the Plane release URL.
Use `--channel beta` for beta releases and `--version <version>` for a specific release.
