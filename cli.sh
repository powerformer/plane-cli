#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
COMMAND=${1:-help}
[ $# -gt 0 ] && shift || true

usage() {
  cat <<'EOF'
plane repo operator

Usage:
  ./cli.sh land [options]
  ./cli.sh release --channel=stable|beta [options]

Commands:
  land       Create or update the GitHub PR for the current branch.
  release    Trigger a release workflow.
EOF
}

case "$COMMAND" in
  help|-h|--help)
    usage
    exit 0
    ;;
  land|release)
    ;;
  *)
    echo "unknown command: $COMMAND" >&2
    usage >&2
    exit 2
    ;;
esac

if ! command -v uv >/dev/null 2>&1; then
  echo "missing dependency: uv" >&2
  exit 1
fi

PYTHONPATH="$ROOT/scripts${PYTHONPATH+:$PYTHONPATH}"
export PYTHONPATH
exec uv run --project "$ROOT/scripts" python -m "cli.$COMMAND" "$@"
