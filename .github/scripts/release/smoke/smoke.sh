#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/../../../.." && pwd)
VERSION=${1:-}
CHANNEL=${2:-stable}

[ -n "$VERSION" ] || { printf '%s\n' 'missing release version' >&2; exit 1; }

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT INT TERM

export HOME="$tmpdir/home"
export PLANE_INSTALL_ROOT="$tmpdir/install"
export PLANE_LOCAL_BIN_DIR="$tmpdir/bin"
export PLANE_HOME="$tmpdir/plane-home"
skill_path="$tmpdir/agent/skills/plane-cli"
mkdir -p "$HOME" "$PLANE_INSTALL_ROOT" "$PLANE_LOCAL_BIN_DIR" "$tmpdir/agent/skills"

sh "$ROOT/manage.sh" install --channel "$CHANNEL" --version "$VERSION" --retain=false
"$PLANE_LOCAL_BIN_DIR/plane" --version
"$PLANE_LOCAL_BIN_DIR/plane" help
"$PLANE_LOCAL_BIN_DIR/plane" skill install --path "$skill_path" --channel "$CHANNEL" --version "$VERSION"
test -f "$skill_path/SKILL.md"
sh "$ROOT/manage.sh" upgrade --channel "$CHANNEL" --version "$VERSION" --retain=false
test -f "$skill_path/SKILL.md"
"$PLANE_LOCAL_BIN_DIR/plane" skill uninstall
test ! -e "$skill_path"
sh "$ROOT/manage.sh" uninstall --version "$VERSION"
[ ! -e "$PLANE_INSTALL_ROOT/$VERSION" ] || { printf '%s\n' "version uninstall left $PLANE_INSTALL_ROOT/$VERSION" >&2; exit 1; }

if [ "${SMOKE_LATEST:-}" = "1" ]; then
  rm -f "$PLANE_LOCAL_BIN_DIR/plane"
  rm -rf "$PLANE_INSTALL_ROOT/latest-smoke"
  sh "$ROOT/manage.sh" install --channel "$CHANNEL" --install-root "$PLANE_INSTALL_ROOT/latest-smoke" --retain=false
  "$PLANE_LOCAL_BIN_DIR/plane" --version
  "$PLANE_LOCAL_BIN_DIR/plane" help
  sh "$ROOT/manage.sh" uninstall --install-root "$PLANE_INSTALL_ROOT/latest-smoke"
  [ ! -e "$PLANE_INSTALL_ROOT/latest-smoke" ] || { printf '%s\n' "full uninstall left $PLANE_INSTALL_ROOT/latest-smoke" >&2; exit 1; }
fi
