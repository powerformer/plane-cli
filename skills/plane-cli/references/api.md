# Plane API commands (reference)

`plane api` wraps the Plane REST API. `plane api --help` and
`plane api <resource> --help` are the authoritative source for flags; this file
is an overview plus the field values agents most often need.

## Resources and verbs

Most resources share the verbs `list`, `get`, `create`, `update`, `delete`.

- **Workspace-scoped:** `project`
  (`list`/`get`/`create`/`update`/`delete`/`archive`/`unarchive`/`summary`) and
  `member workspace-list`.
- **Project-scoped** (pass `--project <PROJECT>`): `work-item`, `state`,
  `label`, `cycle`, `module`, `estimate`, `intake`, `page`, and `member`.
  `work-item` also has `attach` (see below).
- **Work-item-scoped** (pass `--work-item`, plus `--project` for UUID ids):
  `page` (`list`/`link`/`unlink`), `comment`, `link`, `relation`, and
  `activity` (read-only).

## Human-readable references

Anywhere a command takes a project or work-item UUID, the human-readable form
works too:

- **Project:** the project identifier, e.g. `--project OPEND` or
  `plane api project get OPEND`.
- **Work item:** `<KEY>-<SEQ>`, e.g. `plane api work-item get OPEND-372` or
  `plane api comment list --work-item OPEND-372`. A `<KEY>-<SEQ>` reference
  carries its own project, so `--project` may be omitted.

UUIDs pass through untouched (no extra request); human-readable references cost
one read-only resolution call, so `--dry-run` stays offline only with UUIDs.

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

## Pages (documents)

`plane api page` writes Plane pages (documents). The body is **Markdown**
(converted to HTML) by default, or raw **HTML** for `.html` files or with
`--format html`; Plane stores it as `description_html` and the collaborative
editor hydrates from it on first open. `--access public|private` sets
visibility; `page get --content` prints only the body HTML.

```bash
plane api page create --project <ID> --name "Design Review" --from-file notes.md
plane api page create --project <ID> --name "Spec" --body "## Goals" --access private
plane api page update <PAGE_ID> --project <ID> --from-file notes.md   # replace body
plane api page update <PAGE_ID> --project <ID> --name "Design Review v2"
plane api page get <PAGE_ID> --project <ID> --content
plane api page list --project <ID>
plane api page delete <PAGE_ID> --project <ID>
```

## Work-item page associations

`plane api work-item page` manages the page associations on a work item through:

- `GET  workspaces/{workspace}/projects/{project}/work-items/{work_item}/pages/`
- `POST workspaces/{workspace}/projects/{project}/work-items/{work_item}/pages/`
- `DELETE workspaces/{workspace}/projects/{project}/work-items/{work_item}/pages/{page_id}/`

Use it from the work-item side to inspect and update attached pages:

```bash
plane api work-item page list --project <ID> --work-item <WORK_ITEM_ID>
plane api work-item page link --project <ID> --work-item <WORK_ITEM_ID> <PAGE_ID>
plane api work-item page link --project <ID> --work-item <WORK_ITEM_ID> <PAGE_ID_1> <PAGE_ID_2> --dry-run
plane api work-item page unlink --project <ID> --work-item <WORK_ITEM_ID> <PAGE_ID>
```

`link` accepts one or more page ids and sends one `page_id` request per page;
it supports `--json` / `--dry-run`. List supports `--json`, `--all`, `--fields`,
and `--expand`.

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

## Cross-project dependencies (`plane dep`)

Plane's native relations do not cross projects, so cross-project dependency
edges are stored as `dep:<KEY>:<SEQ>` labels on the *dependent* item (an item
carrying `dep:PLANE:5` is blocked by PLANE-5). `plane dep` is the surface over
them:

```bash
plane dep add --project <ID> --work-item <WI_ID> --on PLANE:5   # target must exist
plane dep rm  --project <ID> --work-item <WI_ID> --on PLANE:5   # detach (label kept)
plane dep ls  --project <ID> [--work-item <WI_ID>]              # list + resolve targets
plane dep gc  --project <ID> [--write]                          # prune orphan dep:* labels
```

`ls` resolves each target via `GET workspaces/<slug>/work-items/<KEY>-<SEQ>/` and
flags dangling ones. `gc` is a dry run unless `--write`.

## Attaching files to a work item

`plane api work-item attach` uploads a local file straight to the server
(no direct object-storage access needed); it resolves the item by identifier
and infers the MIME type from the extension.

```bash
plane api work-item attach --item PLANECLI-8 --file ./build.log
plane api work-item attach --item PLANECLI-8 --file ./diagram.png --type image/png --name arch.png
```

`--item` is `<KEY>-<SEQ>`; pass `--type` to override the inferred content type
(unknown extensions default to `application/octet-stream`, which the server
rejects) and `--name` to override the stored file name.

See [scenarios.md](./scenarios.md) for end-to-end playbooks.
