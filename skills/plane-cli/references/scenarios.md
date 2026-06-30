# Scenarios (end-to-end)

Concrete, multi-step Plane tasks built from the commands in
[api.md](./api.md). Replace `<...>` placeholders with real ids; capture ids from
each step's output (add `--json` to parse them programmatically). Set
`workspace_slug` in `plane.toml` or pass `--workspace <slug>` on each call.

## Scenario A — Stand up a new project

1. Create the project and note its id:

   ```bash
   plane api project create --name "Acme Web" --identifier WEB --json
   ```

2. Seed a couple of workflow states and a label (project id from step 1):

   ```bash
   plane api state create --project <project-id> --name "In Review" \
     --data '{"group":"started","color":"#f59e0b"}'
   plane api label create --project <project-id> --name "bug" \
     --data '{"color":"#ef4444"}'
   ```

3. Add a teammate. Find their user id first, then add them as a Member:

   ```bash
   plane api member workspace-list --json          # pick the member's user id
   plane api member create --project <project-id> \
     --data '{"member":"<user-id>","role":15}'
   ```

## Scenario B — Drive a work item from open to linked

1. Find a target state id, then create the work item:

   ```bash
   plane api state list --project <project-id>
   plane api work-item create --project <project-id> \
     --name "Fix login redirect" --data '{"priority":"high"}' --json
   ```

2. Assign it, move it into a state, and tag it (ids from step 1 and
   `member workspace-list` / `label list`):

   ```bash
   plane api work-item update --project <project-id> <work-item-id> \
     --data '{"state":"<state-id>","assignees":["<user-id>"],"labels":["<label-id>"]}'
   ```

3. Comment, then link the GitHub PR that implements it:

   ```bash
   plane api comment create --project <project-id> --work-item <work-item-id> \
     --data '{"comment_html":"<p>Picking this up.</p>"}'
   plane api link create --project <project-id> --work-item <work-item-id> \
     --data '{"url":"https://github.com/acme/web/pull/42","title":"PR #42"}'
   ```

4. Review history, then report back with clickable links:

   ```bash
   plane api activity list --project <project-id> --work-item <work-item-id>
   ```

   Work item: `<SERVER_URL>/<workspace>/browse/WEB-<sequence_id>/`
   PR: `https://github.com/acme/web/pull/42`

## Scenario C — Triage an intake item

1. List pending intake items and inspect one:

   ```bash
   plane api intake list --project <project-id>
   plane api intake get --project <project-id> <intake-id>
   ```

2. Accept it (or reject `-1`, snooze `0`, mark duplicate `2`):

   ```bash
   plane api intake update --project <project-id> <intake-id> \
     --data '{"status":1}'
   ```

3. Once accepted it becomes a normal work item — continue with Scenario B
   (assign, state, comment, link).
