from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time

from lib.utils.cli import CliError, run_checked


CHANNEL_WORKFLOWS = {
    "stable": "release-stable.yml",
    "beta": "release-beta.yml",
}


def usage() -> None:
    print(
        """Usage: runseal :release --channel=stable|beta [options]

Trigger a release workflow.

Options:
  --channel <name>      release channel: stable or beta
  --ref <ref>           git ref passed to the workflow (default: main)
  --version <version>   optional workflow version_override
  --watch              watch the triggered workflow run
  --dry-run            print planned action without triggering a workflow
"""
    )


def output(argv: list[str]) -> str:
    result = run_checked(argv, stdout=subprocess.PIPE)
    return result.stdout.decode("utf-8").strip()


def require_operator_tools() -> None:
    run_checked(["gh", "--version"], stdout=subprocess.DEVNULL)
    run_checked(["gh", "auth", "status"], stdout=subprocess.DEVNULL)


def workflow_run_ids(workflow: str, ref: str) -> set[str]:
    raw = output(
        [
            "gh",
            "run",
            "list",
            "--workflow",
            workflow,
            "--branch",
            ref,
            "--event",
            "workflow_dispatch",
            "--limit",
            "20",
            "--json",
            "databaseId",
        ]
    )
    return {str(run["databaseId"]) for run in json.loads(raw)}


def triggered_run_id(workflow: str, ref: str, previous_ids: set[str]) -> str:
    for _ in range(12):
        raw = output(
            [
                "gh",
                "run",
                "list",
                "--workflow",
                workflow,
                "--branch",
                ref,
                "--event",
                "workflow_dispatch",
                "--limit",
                "10",
                "--json",
                "databaseId",
            ]
        )
        runs = json.loads(raw)
        for run in runs:
            run_id = str(run["databaseId"])
            if run_id not in previous_ids:
                return run_id
        time.sleep(5)
    raise CliError(f"could not find the triggered run for {workflow} on {ref}")


def cmd_default(args: list[str]) -> int:
    parser = argparse.ArgumentParser(prog="runseal :release", add_help=False)
    parser.add_argument("--channel", choices=sorted(CHANNEL_WORKFLOWS))
    parser.add_argument("--ref", default="main")
    parser.add_argument("--version", default="")
    parser.add_argument("--watch", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parsed = parser.parse_args(args)

    if parsed.channel is None:
        raise CliError("--channel is required")
    workflow = CHANNEL_WORKFLOWS[parsed.channel]
    argv = [
        "gh",
        "workflow",
        "run",
        workflow,
        "--ref",
        parsed.ref,
        "-f",
        f"ref={parsed.ref}",
        "-f",
        f"version_override={parsed.version}",
    ]
    if parsed.dry_run:
        print(" ".join(argv))
        return 0
    require_operator_tools()
    previous_run_ids = workflow_run_ids(workflow, parsed.ref) if parsed.watch else set()
    result = run_checked(argv, stdout=subprocess.PIPE)
    trigger_output = result.stdout.decode("utf-8").strip()
    if trigger_output:
        print(trigger_output)
    print(f"triggered {workflow} for ref {parsed.ref}")
    if parsed.watch:
        match = re.search(r"/actions/runs/([0-9]+)", trigger_output)
        run_id = match.group(1) if match else triggered_run_id(workflow, parsed.ref, previous_run_ids)
        run_checked(["gh", "run", "watch", run_id, "--interval", "10"])
    return 0


def main(argv: list[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if not args or args[0] in {"-h", "--help", "help"}:
        usage()
        return 0
    try:
        return cmd_default(args)
    except (CliError, RuntimeError, OSError, subprocess.CalledProcessError) as exc:
        print(f"release: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
