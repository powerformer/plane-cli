# Plane API commands (reference)

`plane api` wraps the Plane REST API. `plane api --help` and
`plane api <resource> --help` are the authoritative source for flags; this file
is an overview plus the field values agents most often need.

## Resources and verbs

Most resources share the verbs `list`, `get`, `create`, `update`, `delete`.

- **Workspace-scoped:** `project`
  (`list`/`get`/`create`/`update`/`delete`/`archive`/`unarchive`/`summary`) and
  `member workspace-list`.
- **Project-scoped** (pass `--project <PROJECT_ID>`): `work-item`, `state`,
  `label`, `cycle`, `module`, `estimate`, `intake`, and `member`.
- **Work-item-scoped** (pass `--project` and `--work-item`): `comment`, `link`,
  `relation`, and `activity` (read-only).

## Conventions

- `--workspace <SLUG>` selects the workspace (or set `workspace_slug` in
  `plane.toml`); `--project` / `--work-item` scope nested resources.
- `--json` prints the raw API response; `--all` follows cursor pages, and with
  `--json` accumulates every page into one JSON array.
- `--fields <CSV>` and `--expand <CSV>` trim or expand the response.
- `create` / `update` take typed flags (such as `--name`) plus `--data '<JSON>'`
  for any other fields; `--dry-run` prints the request instead of sending it.

## Common field values

For `work-item` and the project resources, pass non-typed fields through
`--data`:

- Work item (`work-item create/update --data`): `name`, `state` (state id),
  `priority` (`urgent` | `high` | `medium` | `low` | `none`), `assignees` (list
  of member user ids), `labels` (list of label ids), `parent` (work-item id),
  `start_date` / `target_date` (`YYYY-MM-DD`).
- State (`state create --data`): `group` (`backlog` | `unstarted` | `started` |
  `completed` | `cancelled` | `triage`), `color` (hex).
- Label (`label create --data`): `color` (hex).
- Project member (`member create --data`): `member` (workspace-member user id),
  `role` (`20` Admin | `15` Member | `5` Guest).
- Comment (`comment create --data`): `comment_html`.
- Link (`link create --data`): `url`, `title`.
- Intake (`intake update --data`): `status` (`-2` Pending | `-1` Rejected |
  `0` Snoozed | `1` Accepted | `2` Duplicate).

Resolve user ids with `plane api member workspace-list`; resolve state/label ids
with `plane api state list --project <id>` / `plane api label list --project <id>`.

## Escape hatch

For endpoints the typed commands do not cover, use passthrough (GET, POST,
PATCH, PUT, DELETE):

```bash
plane api request --method PATCH workspaces/<slug>/projects/<id>/ --data '{"name":"New"}'
```

## Print resource links

CLI output is id-centric. When you report a resource back to the user, also
print its full URL so they can open it directly:

- Plane work item:
  `<SERVER_URL>/<workspace>/browse/<PROJECT_IDENTIFIER>-<sequence_id>/` — e.g.
  `https://plane.powerformer.net/acme/browse/WEB-7/`. `<SERVER_URL>` is the Plane
  backend without the `/api/v1` suffix; the project identifier comes from
  `plane api project get <id>`, and `sequence_id` is on the work item.
- Related GitHub issue or pull request: print the full
  `https://github.com/<owner>/<repo>/issues/<n>` or `.../pull/<n>` URL.

Prefer absolute URLs over bare ids in any summary you write back to the user.

See [scenarios.md](./scenarios.md) for end-to-end playbooks.
