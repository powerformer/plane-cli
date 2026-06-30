#!/usr/bin/env sh
set -eu

# Release smoke for the plane-cli meta-management surface.
#
# This is an executable description of what a terminal user can do with a
# published release: install the manager command entry, run the installed CLI
# (version/help and an optional read-only API call), manage agent skills, set up
# and clear the PATH block, and uninstall cleanly. Release artifacts are read
# from R2/S3 and served from a local HTTP mirror so CI never depends on the
# public Cloudflare edge.

ROOT=$(CDPATH= cd -- "$(dirname "$0")/../../../.." && pwd)
VERSION=${1:-}
CHANNEL=${2:-stable}

[ -n "$VERSION" ] || { printf '%s\n' 'missing release version' >&2; exit 1; }

PATH_MARKER_START="# >>> plane-cli path >>>"

tmpdir=$(mktemp -d)
server_pid=
api_server_pid=

cleanup() {
  if [ -n "$api_server_pid" ]; then
    kill "$api_server_pid" 2>/dev/null || true
    wait "$api_server_pid" 2>/dev/null || true
  fi
  if [ -n "$server_pid" ]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT INT TERM

fail() {
  printf '%s\n' "$*" >&2
  exit 1
}

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

wait_for_api() {
  url="$1"
  key="$2"
  attempts=${SMOKE_MIRROR_ATTEMPTS:-50}
  delay=${SMOKE_MIRROR_DELAY_SECONDS:-0.2}
  i=1
  while [ "$i" -le "$attempts" ]; do
    if curl -fsS -H "X-API-Key: $key" "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep "$delay"
    i=$((i + 1))
  done
  echo "timed out waiting for mock api: $url" >&2
  return 1
}

run_manage() {
  sh "$ROOT/manage.sh" "$@"
}

count_path_markers() {
  total=0
  for file in \
    "$HOME/.zshrc" \
    "$HOME/.bashrc" \
    "$HOME/.bash_profile" \
    "$HOME/.profile" \
    "$HOME/.config/fish/config.fish"; do
    [ -f "$file" ] || continue
    n=$(grep -Fc "$PATH_MARKER_START" "$file" 2>/dev/null || true)
    total=$((total + n))
  done
  printf '%s' "$total"
}

smoke_install() {
  run_manage install --channel "$CHANNEL" --version "$VERSION" --retain=false
  [ -e "$plane" ] || fail "install missing command entry $plane"
  link_target=$(readlink "$plane" || true)
  [ "$link_target" = "$PLANE_INSTALL_ROOT/$VERSION/plane" ] ||
    fail "symlink target $link_target != $PLANE_INSTALL_ROOT/$VERSION/plane"
}

smoke_cli_basics() {
  version_output=$("$plane" --version)
  printf '%s\n' "$version_output" | grep -Fq "$VERSION" ||
    fail "plane --version did not contain $VERSION: $version_output"
  "$plane" help >/dev/null
}

smoke_api_me_mock() {
  # The production Plane API is IP-allowlisted and CI runners are not on it, and
  # we never want a real token in CI. Stand up a local mock of /api/v1/users/me/
  # so `plane api me` is exercised end to end, including the X-API-Key header.
  api_port=$(free_port)
  api_key="smoke-mock-api-key"
  python3 "$ROOT/.github/scripts/release/smoke/mock_api.py" --port "$api_port" --key "$api_key" &
  api_server_pid=$!
  api_base="http://127.0.0.1:$api_port"
  wait_for_api "$api_base/api/v1/users/me/" "$api_key"

  PLANE_API_BASE_URL="$api_base"
  PLANE_API_KEY="$api_key"
  export PLANE_API_BASE_URL PLANE_API_KEY

  api_output=$("$plane" api me)
  printf '%s\n' "$api_output" | grep -Fq "user:" ||
    fail "api me did not report the current user: $api_output"
  printf '%s\n' "$api_output" | grep -Fq "smoke@plane.test" ||
    fail "api me did not render the mock user: $api_output"
  if printf '%s\n' "$api_output" | grep -Fq "$api_key"; then
    fail "api me output leaked the API token"
  fi
  "$plane" api me --json >/dev/null
  echo "api me ok against mock $api_base"

  unset PLANE_API_BASE_URL PLANE_API_KEY
  kill "$api_server_pid" 2>/dev/null || true
  wait "$api_server_pid" 2>/dev/null || true
  api_server_pid=
}

smoke_skill_lifecycle() {
  "$plane" skill install --path "$skill_path" --channel "$CHANNEL" --version "$VERSION"
  "$plane" skill list
  [ -f "$skill_path/SKILL.md" ] || fail "skill install missing $skill_path/SKILL.md"
  [ -f "$skill_path/metadata.json" ] || fail "skill install missing $skill_path/metadata.json"
}

smoke_skill_content_boundary() {
  skill_md="$skill_path/SKILL.md"
  grep -Fq "Version Selection" "$skill_md" ||
    fail "distributed skill is missing the user-facing Version Selection section"
  # The distributed skill is terminal-user-facing only: it must not leak repo
  # operator release workflow, R2, or publishing wording. "published version" is
  # allowed user wording, so reject the operator verbs with word boundaries.
  for pattern in \
    'release behavior' \
    'workflow' \
    '\bR2\b' \
    'operator' \
    '\bpublish\b' \
    'publishing' \
    'runseal'; do
    if grep -inE "$pattern" "$skill_md" >/dev/null 2>&1; then
      fail "distributed skill leaks operator wording matching /$pattern/"
    fi
  done
}

smoke_upgrade() {
  run_manage upgrade --channel "$CHANNEL" --version "$VERSION" --retain=false
  [ -f "$skill_path/SKILL.md" ] || fail "skill upgrade missing $skill_path/SKILL.md"
  list_output=$("$plane" skill list)
  printf '%s\n' "$list_output" | grep -Fq "binary $VERSION" ||
    fail "skill list did not report binary $VERSION: $list_output"
  printf '%s\n' "$list_output" | grep -Fq "skill $VERSION" ||
    fail "skill list did not report skill $VERSION: $list_output"
}

smoke_path_setup_clear() {
  SHELL=/bin/bash
  export SHELL
  run_manage path setup --bin-dir "$PLANE_LOCAL_BIN_DIR"
  run_manage path setup --bin-dir "$PLANE_LOCAL_BIN_DIR"
  count=$(count_path_markers)
  [ "$count" = "1" ] || fail "expected exactly 1 managed PATH marker after setup, found $count"
  run_manage path clear
  count=$(count_path_markers)
  [ "$count" = "0" ] || fail "expected 0 managed PATH markers after clear, found $count"
}

smoke_path_notice() {
  notice="plane does not resolve to"
  # With the local bin dir on PATH and resolving to the managed command, install
  # must stay quiet about PATH setup. The PATH override is scoped to the external
  # sh invocation so it does not leak into the rest of the smoke.
  on_path_output=$(PATH="$PLANE_LOCAL_BIN_DIR:$PATH" sh "$ROOT/manage.sh" install \
    --channel "$CHANNEL" --version "$VERSION" --retain=false 2>&1)
  if printf '%s\n' "$on_path_output" | grep -Fq "$notice"; then
    fail "install printed PATH notice even though $PLANE_LOCAL_BIN_DIR is on PATH"
  fi
  # Without the local bin dir on PATH, install must surface the setup notice.
  off_path_output=$(run_manage install \
    --channel "$CHANNEL" --version "$VERSION" --retain=false 2>&1)
  if ! printf '%s\n' "$off_path_output" | grep -Fq "$notice"; then
    fail "install did not print PATH notice when $PLANE_LOCAL_BIN_DIR is absent from PATH"
  fi
}

smoke_uninstall() {
  "$plane" skill uninstall
  [ ! -e "$skill_path" ] || fail "skill uninstall left $skill_path"
  list_output=$("$plane" skill list)
  printf '%s\n' "$list_output" | grep -Fq "no managed skill installations" ||
    fail "skill list still reports managed installations: $list_output"
  run_manage uninstall --version "$VERSION"
  [ ! -e "$plane" ] || fail "version uninstall left command entry $plane"
  [ ! -e "$PLANE_INSTALL_ROOT/$VERSION" ] ||
    fail "version uninstall left $PLANE_INSTALL_ROOT/$VERSION"
}

smoke_latest() {
  rm -f "$plane"
  PLANE_INSTALL_ROOT="$tmpdir/latest-smoke"
  PLANE_HOME="$PLANE_INSTALL_ROOT"
  export PLANE_INSTALL_ROOT PLANE_HOME
  rm -rf "$PLANE_INSTALL_ROOT"
  run_manage install --channel "$CHANNEL" --install-root "$PLANE_INSTALL_ROOT" --retain=false
  [ -e "$plane" ] || fail "latest install missing command entry $plane"
  "$plane" --version
  "$plane" help >/dev/null
  run_manage uninstall --install-root "$PLANE_INSTALL_ROOT"
  [ ! -e "$PLANE_INSTALL_ROOT" ] || fail "full uninstall left $PLANE_INSTALL_ROOT"
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

# exec so server_pid is the http.server process itself and cleanup's kill reaps it.
(cd "$mirror_root" && exec python3 -m http.server "$port" --bind 127.0.0.1) &
server_pid=$!
wait_for_mirror "$mirror_url/$CHANNEL/versions/$VERSION/metadata.json"

export HOME="$tmpdir/home"
export PLANE_INSTALL_ROOT="$HOME/.local/share/plane"
export PLANE_LOCAL_BIN_DIR="$tmpdir/bin"
export PLANE_HOME="$PLANE_INSTALL_ROOT"
export PLANE_RELEASES_PUBLIC_URL="$mirror_url"
plane="$PLANE_LOCAL_BIN_DIR/plane"
skill_path="$tmpdir/agent/skills/plane-cli"
mkdir -p "$HOME" "$PLANE_INSTALL_ROOT" "$PLANE_LOCAL_BIN_DIR" "$tmpdir/agent/skills"

smoke_install
smoke_cli_basics
smoke_api_me_mock
smoke_skill_lifecycle
smoke_skill_content_boundary
smoke_upgrade
smoke_path_setup_clear
smoke_path_notice
smoke_uninstall

if [ "${SMOKE_LATEST:-}" = "1" ]; then
  smoke_latest
fi
