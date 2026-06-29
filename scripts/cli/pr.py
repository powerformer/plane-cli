from __future__ import annotations

import argparse
import json
import subprocess
import sys

from lib.utils.cli import CliError, run_checked


def usage() -> None:
    print(
        """Usage: runseal :pr [options]

Create or update the GitHub PR for the current branch.

Options:
  --base <branch>       PR base branch (default: main)
  --title <title>       title when creating a new PR
  --body-file <path>    body file when creating a new PR
  --ready              mark the PR ready for review
  --auto-merge         enable squash auto-merge and branch deletion
  --checks             print PR checks after create/update
  --watch-checks       watch PR checks after create/update
  --no-push            do not push the current branch first
  --dry-run            print planned actions without changing remote state
"""
    )


def output(argv: list[str]) -> str:
    result = run_checked(argv, stdout=subprocess.PIPE)
    return result.stdout.decode("utf-8").strip()


def run_report(argv: list[str]) -> None:
    result = subprocess.run(argv, check=False)
    if result.returncode not in {0, 8}:
        raise subprocess.CalledProcessError(result.returncode, argv)


def current_branch() -> str:
    branch = output(["git", "branch", "--show-current"])
    if not branch:
        raise CliError("not on a branch")
    return branch


def require_operator_tools() -> None:
    run_checked(["git", "--version"], stdout=subprocess.DEVNULL)
    run_checked(["gh", "--version"], stdout=subprocess.DEVNULL)
    run_checked(["gh", "auth", "status"], stdout=subprocess.DEVNULL)


def find_pr(branch: str) -> dict[str, object] | None:
    raw = output(
        [
            "gh",
            "pr",
            "list",
            "--head",
            branch,
            "--json",
            "number,title,state,url,isDraft",
        ]
    )
    items = json.loads(raw)
    if not items:
        return None
    return items[0]


def create_pr(branch: str, base: str, title: str | None, body_file: str | None) -> dict[str, object]:
    argv = [
        "gh",
        "pr",
        "create",
        "--draft",
        "--base",
        base,
        "--head",
        branch,
    ]
    if title:
        argv.extend(["--title", title])
    else:
        argv.append("--fill")
    if body_file:
        argv.extend(["--body-file", body_file])
    elif title:
        argv.append("--fill")
    run_checked(argv)
    found = find_pr(branch)
    if found is None:
        raise CliError(f"created PR for {branch}, but could not find it afterward")
    return found


def cmd_default(args: list[str]) -> int:
    parser = argparse.ArgumentParser(prog="runseal :pr", add_help=False)
    parser.add_argument("--base", default="main")
    parser.add_argument("--title")
    parser.add_argument("--body-file")
    parser.add_argument("--ready", action="store_true")
    parser.add_argument("--auto-merge", action="store_true")
    parser.add_argument("--checks", action="store_true")
    parser.add_argument("--watch-checks", action="store_true")
    parser.add_argument("--no-push", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parsed = parser.parse_args(args)

    require_operator_tools()
    branch = current_branch()
    if branch in {parsed.base, "main", "master"}:
        raise CliError(f"refusing to open a PR from base branch: {branch}")

    if parsed.dry_run:
        print(f"branch: {branch}")
        print(f"base: {parsed.base}")
        print(f"push: {not parsed.no_push}")
        print("pr: create if missing, otherwise reuse existing")
        print(f"ready: {parsed.ready}")
        print(f"auto_merge: {parsed.auto_merge}")
        print(f"checks: {parsed.checks or parsed.watch_checks}")
        return 0

    if not parsed.no_push:
        run_checked(["git", "push", "-u", "origin", branch])

    pr = find_pr(branch)
    if pr is None:
        pr = create_pr(branch, parsed.base, parsed.title, parsed.body_file)
        print(f"created PR #{pr['number']}: {pr['url']}", flush=True)
    else:
        print(f"found PR #{pr['number']}: {pr['url']}", flush=True)

    number = str(pr["number"])
    if parsed.ready:
        run_checked(["gh", "pr", "ready", number])
        print(f"marked PR #{number} ready")
    if parsed.auto_merge:
        run_checked(["gh", "pr", "merge", number, "--auto", "--squash", "--delete-branch"])
        print(f"enabled auto-merge for PR #{number}")
    if parsed.watch_checks:
        run_checked(["gh", "pr", "checks", number, "--watch", "--interval", "10"])
    elif parsed.checks:
        run_report(["gh", "pr", "checks", number])
    return 0


def main(argv: list[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if not args or args[0] in {"-h", "--help", "help"}:
        usage()
        return 0
    try:
        return cmd_default(args)
    except (CliError, RuntimeError, OSError, subprocess.CalledProcessError) as exc:
        print(f"pr: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
