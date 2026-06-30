#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/../../../.." && pwd)
VERSION=${1:-}
CHANNEL=${2:-stable}

[ -n "$VERSION" ] || { printf '%s\n' 'missing release version' >&2; exit 1; }

tmpdir=$(mktemp -d)
server_pid=

cleanup() {
  if [ -n "$server_pid" ]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT INT TERM

platform_archive() {
  os=$(uname -s)
  arch=$(uname -m)
  case "$os:$arch" in
    Linux:x86_64|Linux:amd64) echo "plane-x86_64-unknown-linux-gnu.tar.gz" ;;
    Darwin:arm64|Darwin:aarch64) echo "plane-aarch64-apple-darwin.tar.gz" ;;
    Darwin:x86_64|Darwin:amd64) echo "plane-x86_64-apple-darwin.tar.gz" ;;
    *) echo "unsupported platform: $os $arch" >&2; exit 1 ;;
  esac
}

free_port() {
  python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()'
}

wait_for_mirror() {
  url="$1"
  attempts=${SMOKE_MIRROR_ATTEMPTS:-50}
  delay=${SMOKE_MIRROR_DELAY_SECONDS:-0.2}
  i=1
  while [ "$i" -le "$attempts" ]; do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep "$delay"
    i=$((i + 1))
  done
  echo "timed out waiting for smoke mirror: $url" >&2
  return 1
}

mirror_root="$tmpdir/release-mirror"
port=$(free_port)
mirror_url="http://127.0.0.1:$port"
python3 "$ROOT/.github/scripts/release/smoke/mirror.py" \
  --root "$mirror_root" \
  --channel "$CHANNEL" \
  --version "$VERSION" \
  --platform "$(platform_archive)" \
  --mirror-url "$mirror_url"

(cd "$mirror_root" && python3 -m http.server "$port" --bind 127.0.0.1) &
server_pid=$!
wait_for_mirror "$mirror_url/$CHANNEL/versions/$VERSION/metadata.json"

export HOME="$tmpdir/home"
export PLANE_INSTALL_ROOT="$HOME/.local/share/plane"
export PLANE_LOCAL_BIN_DIR="$tmpdir/bin"
export PLANE_HOME="$PLANE_INSTALL_ROOT"
export PLANE_RELEASES_PUBLIC_URL="$mirror_url"
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
  export PLANE_INSTALL_ROOT="$tmpdir/latest-smoke"
  export PLANE_HOME="$PLANE_INSTALL_ROOT"
  rm -rf "$PLANE_INSTALL_ROOT"
  sh "$ROOT/manage.sh" install --channel "$CHANNEL" --install-root "$PLANE_INSTALL_ROOT" --retain=false
  "$PLANE_LOCAL_BIN_DIR/plane" --version
  "$PLANE_LOCAL_BIN_DIR/plane" help
  sh "$ROOT/manage.sh" uninstall --install-root "$PLANE_INSTALL_ROOT"
  [ ! -e "$PLANE_INSTALL_ROOT" ] || { printf '%s\n' "full uninstall left $PLANE_INSTALL_ROOT" >&2; exit 1; }
fi
