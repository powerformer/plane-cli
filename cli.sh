#!/usr/bin/env sh
set -eu

commands="release init"

usage() {
  echo "usage: ./cli.sh :<command> [args...]" >&2
  echo "commands: $commands" >&2
}

if [ "$#" -lt 1 ]; then
  usage
  exit 1
fi

case "$1" in
  :*) name="${1#:}" ;;
  *)
    usage
    exit 1
    ;;
esac

found=0
for command in $commands; do
  if [ "$command" = "$name" ]; then
    found=1
    break
  fi
done

if [ "$found" -ne 1 ]; then
  echo "unknown command: :$name" >&2
  usage
  exit 1
fi

if ! command -v uv >/dev/null 2>&1; then
  echo "missing dependency: uv" >&2
  exit 1
fi

module=$(printf '%s' "$name" | tr '-' '_')
root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
shift
cd "$root"
PYTHONPATH="$root/scripts${PYTHONPATH+:$PYTHONPATH}"
export PYTHONPATH
exec uv run --project "$root/scripts" python -m "cli.$module" "$@"
