#!/usr/bin/env sh
set -eu

COMMAND=${1:-install}
[ $# -gt 0 ] && shift || true
PATH_COMMAND=
if [ "$COMMAND" = path ]; then
  PATH_COMMAND=${1:-help}
  [ $# -gt 0 ] && shift || true
fi

CHANNEL=${PLANE_CHANNEL:-stable}
VERSION=${PLANE_VERSION:-}
PUBLIC_URL=${PLANE_RELEASES_PUBLIC_URL:-https://releases.plane.powerformer.net}
INSTALL_ROOT=${PLANE_INSTALL_ROOT:-"$HOME/.local/share/plane"}
LOCAL_BIN_DIR=${PLANE_LOCAL_BIN_DIR:-"$HOME/.local/bin"}
RETAIN=${PLANE_RETAIN:-}
PATH_MARKER_START="# >>> plane-cli path >>>"
PATH_MARKER_END="# <<< plane-cli path <<<"

print_help() {
  cat <<'EOF'
plane manager

Usage:
  manage.sh install [--channel stable|beta] [--version vX.Y.Z] [--retain[=true|false]]
  manage.sh upgrade [--channel stable|beta] [--version vX.Y.Z] [--retain[=true|false]]
  manage.sh uninstall [--version vX.Y.Z]
  manage.sh path setup
  manage.sh path clear

Environment:
  PLANE_RELEASES_PUBLIC_URL  # default: https://releases.plane.powerformer.net
  PLANE_CHANNEL
  PLANE_VERSION
  PLANE_INSTALL_ROOT
  PLANE_LOCAL_BIN_DIR
  PLANE_RETAIN
EOF
}

print_path_help() {
  cat <<'EOF'
plane manager path commands

Usage:
  manage.sh path setup [--bin-dir DIR]
  manage.sh path clear

`path setup` appends a managed PATH block to the detected shell rc file.
`path clear` removes only the managed block marked by plane-cli comments.
EOF
}

case "$COMMAND" in
  -h|--help|help)
    print_help
    exit 0
    ;;
esac

while [ $# -gt 0 ]; do
  case "$1" in
    --channel)
      CHANNEL=${2:-}
      [ -n "$CHANNEL" ] || { echo "--channel requires a value" >&2; exit 1; }
      shift 2
      ;;
    --channel=*)
      CHANNEL=${1#--channel=}
      shift
      ;;
    --version)
      VERSION=${2:-}
      [ -n "$VERSION" ] || { echo "--version requires a value" >&2; exit 1; }
      shift 2
      ;;
    --version=*)
      VERSION=${1#--version=}
      shift
      ;;
    --public-url)
      PUBLIC_URL=${2:-}
      [ -n "$PUBLIC_URL" ] || { echo "--public-url requires a value" >&2; exit 1; }
      shift 2
      ;;
    --public-url=*)
      PUBLIC_URL=${1#--public-url=}
      shift
      ;;
    --install-root)
      INSTALL_ROOT=${2:-}
      [ -n "$INSTALL_ROOT" ] || { echo "--install-root requires a value" >&2; exit 1; }
      shift 2
      ;;
    --install-root=*)
      INSTALL_ROOT=${1#--install-root=}
      shift
      ;;
    --bin-dir)
      LOCAL_BIN_DIR=${2:-}
      [ -n "$LOCAL_BIN_DIR" ] || { echo "--bin-dir requires a value" >&2; exit 1; }
      shift 2
      ;;
    --bin-dir=*)
      LOCAL_BIN_DIR=${1#--bin-dir=}
      shift
      ;;
    --retain)
      RETAIN=true
      shift
      ;;
    --retain=*)
      RETAIN=${1#--retain=}
      shift
      ;;
    -h|--help|help)
      if [ "$COMMAND" = path ]; then
        print_path_help
      else
        print_help
      fi
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

need_public_url() {
  PUBLIC_URL=${PUBLIC_URL%/}
}

normalize_bool() {
  case "$1" in
    true|1|yes|y|on) printf '%s' true ;;
    false|0|no|n|off) printf '%s' false ;;
    *) echo "invalid --retain value: $1" >&2; exit 1 ;;
  esac
}

normalize_version() {
  printf 'v%s' "$(printf '%s' "$1" | sed 's/^v//')"
}

path_contains_dir() {
  dir="$1"
  old_ifs=$IFS
  IFS=:
  for entry in $PATH; do
    if [ "$entry" = "$dir" ]; then
      IFS=$old_ifs
      return 0
    fi
  done
  IFS=$old_ifs
  return 1
}

plane_command_resolves_to_local_bin() {
  command_path=$(command -v plane 2>/dev/null || true)
  [ "$command_path" = "$LOCAL_BIN_DIR/plane" ]
}

print_path_notice_if_needed() {
  if path_contains_dir "$LOCAL_BIN_DIR" && plane_command_resolves_to_local_bin; then
    return 0
  fi
  cat >&2 <<EOF
plane: plane does not resolve to $LOCAL_BIN_DIR/plane in this shell.
temporary: export PATH="$LOCAL_BIN_DIR:\$PATH"
persist: sh manage.sh path setup
EOF
}

shell_name() {
  basename "${SHELL:-}" 2>/dev/null || printf '%s' ""
}

detected_shell_rc() {
  shell=$(shell_name)
  case "$shell" in
    zsh) printf '%s\n' "$HOME/.zshrc" ;;
    bash)
      case "$(uname -s)" in
        Darwin) printf '%s\n' "$HOME/.bash_profile" ;;
        *) printf '%s\n' "$HOME/.bashrc" ;;
      esac
      ;;
    fish) printf '%s\n' "$HOME/.config/fish/config.fish" ;;
    *) return 1 ;;
  esac
}

path_line_for_shell() {
  shell=$(shell_name)
  if [ "$shell" = fish ]; then
    if [ "$LOCAL_BIN_DIR" = "$HOME/.local/bin" ]; then
      printf '%s\n' 'fish_add_path "$HOME/.local/bin"'
    else
      printf 'fish_add_path "%s"\n' "$LOCAL_BIN_DIR"
    fi
    return
  fi
  if [ "$LOCAL_BIN_DIR" = "$HOME/.local/bin" ]; then
    printf '%s\n' 'export PATH="$HOME/.local/bin:$PATH"'
  else
    printf 'export PATH="%s:$PATH"\n' "$LOCAL_BIN_DIR"
  fi
}

remove_path_block_file() {
  file="$1"
  [ -f "$file" ] || return 1
  if ! grep -F "$PATH_MARKER_START" "$file" >/dev/null 2>&1; then
    return 1
  fi
  if ! grep -F "$PATH_MARKER_END" "$file" >/dev/null 2>&1; then
    return 1
  fi
  tmp="${file}.plane-path.$$"
  awk -v start="$PATH_MARKER_START" -v end="$PATH_MARKER_END" '
    index($0, start) { skip = 1; changed = 1; next }
    index($0, end) { skip = 0; next }
    !skip { print }
    END { if (!changed) exit 2 }
  ' "$file" > "$tmp" || {
    status=$?
    rm -f "$tmp"
    return "$status"
  }
  mv "$tmp" "$file"
  return 0
}

path_block_exists() {
  for file in \
    "$HOME/.zshrc" \
    "$HOME/.bashrc" \
    "$HOME/.bash_profile" \
    "$HOME/.profile" \
    "$HOME/.config/fish/config.fish"; do
    [ -f "$file" ] || continue
    if grep -F "$PATH_MARKER_START" "$file" >/dev/null 2>&1 &&
      grep -F "$PATH_MARKER_END" "$file" >/dev/null 2>&1; then
      return 0
    fi
  done
  return 1
}

setup_path() {
  case "$PATH_COMMAND" in
    setup) ;;
    -h|--help|help|"")
      print_path_help
      return 0
      ;;
    *)
      echo "unknown path command: $PATH_COMMAND" >&2
      return 1
      ;;
  esac

  rc=$(detected_shell_rc) || {
    cat >&2 <<EOF
plane: could not detect a supported shell rc file from SHELL=${SHELL:-}.
manual: export PATH="$LOCAL_BIN_DIR:\$PATH"
EOF
    return 1
  }
  rc_dir=$(dirname "$rc")
  mkdir -p "$rc_dir"
  [ -f "$rc" ] || : > "$rc"
  if remove_path_block_file "$rc"; then
    action=updated
  else
    action=configured
  fi
  {
    printf '\n%s\n' "$PATH_MARKER_START"
    path_line_for_shell
    printf '%s\n' "$PATH_MARKER_END"
  } >> "$rc"
  printf 'plane: %s PATH in %s\n' "$action" "$rc"
}

clear_path() {
  case "$PATH_COMMAND" in
    clear) ;;
    -h|--help|help|"")
      print_path_help
      return 0
      ;;
    *)
      echo "unknown path command: $PATH_COMMAND" >&2
      return 1
      ;;
  esac

  removed=false
  for file in \
    "$HOME/.zshrc" \
    "$HOME/.bashrc" \
    "$HOME/.bash_profile" \
    "$HOME/.profile" \
    "$HOME/.config/fish/config.fish"; do
    if remove_path_block_file "$file"; then
      printf 'plane: removed PATH block from %s\n' "$file"
      removed=true
    fi
  done
  if [ "$removed" = false ]; then
    printf 'plane: no managed PATH block found\n'
  fi
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

latest_version() {
  metadata="$1"
  sed -n 's/.*"releaseVersion"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$metadata" | head -n 1
}

old_versions() {
  current="$1"
  [ -d "$INSTALL_ROOT" ] || return 0
  for path in "$INSTALL_ROOT"/*; do
    [ -d "$path" ] || continue
    name=$(basename "$path")
    [ "$name" != "$current" ] || continue
    case "$name" in
      v[0-9]*) ;;
      *) continue ;;
    esac
    printf '%s\n' "$name"
  done
}

retain_old_versions() {
  old="$1"
  if [ -z "$old" ]; then
    printf '%s' true
    return
  fi
  if [ -n "$RETAIN" ]; then
    normalize_bool "$RETAIN"
    return
  fi
  if [ -t 0 ]; then
    printf 'plane: remove previously installed versions after install? [y/N] ' >&2
    IFS= read -r answer || answer=
    case "$answer" in
      y|Y|yes|YES|Yes) printf '%s' false ;;
      *) printf '%s' true ;;
    esac
    return
  fi
  echo "plane: preserving previous versions; pass --retain=false to prune after install" >&2
  printf '%s' true
}

install_plane() {
  need_public_url
  tmpdir=$(mktemp -d)
  trap 'rm -rf "$tmpdir"' EXIT INT TERM

  if [ -z "$VERSION" ]; then
    curl -fsSL "$PUBLIC_URL/$CHANNEL/latest/metadata.json" -o "$tmpdir/metadata.json"
    VERSION=$(latest_version "$tmpdir/metadata.json")
    [ -n "$VERSION" ] || { echo "failed to resolve latest plane version" >&2; exit 1; }
  fi
  VERSION=$(normalize_version "$VERSION")

  old=$(old_versions "$VERSION")
  retain=$(retain_old_versions "$old")

  archive=$(platform_archive)
  archive_url="$PUBLIC_URL/$CHANNEL/versions/$VERSION/$archive"
  curl -fsSL "$archive_url" -o "$tmpdir/$archive"
  rm -rf "$INSTALL_ROOT/$VERSION"
  mkdir -p "$INSTALL_ROOT/$VERSION" "$LOCAL_BIN_DIR"
  tar -xzf "$tmpdir/$archive" -C "$INSTALL_ROOT/$VERSION"
  chmod +x "$INSTALL_ROOT/$VERSION/plane"

  link="$LOCAL_BIN_DIR/plane"
  rm -f "$link"
  ln -s "$INSTALL_ROOT/$VERSION/plane" "$link"
  "$link" --version
  print_path_notice_if_needed

  if [ "$retain" = false ]; then
    printf '%s\n' "$old" | while IFS= read -r old_version; do
      [ -n "$old_version" ] || continue
      rm -rf "$INSTALL_ROOT/$old_version"
      printf 'removed old plane %s from %s\n' "$old_version" "$INSTALL_ROOT"
    done
  fi

  printf 'installed plane to %s\n' "$link"
}

remove_plane_link_if_managed() {
  bin_path="$LOCAL_BIN_DIR/plane"
  [ -L "$bin_path" ] || return 0
  link_target=$(readlink "$bin_path" || true)
  case "$link_target" in
    "$INSTALL_ROOT"/*)
      rm -f "$bin_path"
      printf 'removed %s\n' "$bin_path"
      ;;
  esac
}

remove_empty_dir() {
  dir="$1"
  if [ -d "$dir" ]; then
    rmdir "$dir" 2>/dev/null || true
  fi
}

uninstall_plane() {
  bin_path="$LOCAL_BIN_DIR/plane"
  if [ -n "$VERSION" ]; then
    VERSION=$(normalize_version "$VERSION")
    target="$INSTALL_ROOT/$VERSION/plane"
    if [ -L "$bin_path" ]; then
      link_target=$(readlink "$bin_path" || true)
      if [ "$link_target" = "$target" ]; then
        rm -f "$bin_path"
        printf 'removed %s\n' "$bin_path"
      fi
    fi
    rm -rf "$INSTALL_ROOT/$VERSION"
    remove_empty_dir "$INSTALL_ROOT"
    printf 'removed plane %s from %s\n' "$VERSION" "$INSTALL_ROOT"
    if path_block_exists; then
      printf 'plane: run `sh manage.sh path clear` to remove the managed PATH block\n'
    fi
    return
  fi

  remove_plane_link_if_managed
  rm -rf "$INSTALL_ROOT"
  remove_empty_dir "$LOCAL_BIN_DIR"
  printf 'removed plane from %s and %s\n' "$INSTALL_ROOT" "$bin_path"
  if path_block_exists; then
    printf 'plane: run `sh manage.sh path clear` to remove the managed PATH block\n'
  fi
}

upgrade_plane() {
  install_plane
  "$LOCAL_BIN_DIR/plane" skill upgrade \
    --channel "$CHANNEL" \
    --version "$VERSION" \
    --release-url "$PUBLIC_URL"
}

case "$COMMAND" in
  install) install_plane ;;
  upgrade) upgrade_plane ;;
  uninstall) uninstall_plane ;;
  path)
    case "$PATH_COMMAND" in
      setup) setup_path ;;
      clear) clear_path ;;
      -h|--help|help|"") print_path_help ;;
      *)
        echo "unknown path command: $PATH_COMMAND" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "unknown command: $COMMAND" >&2
    exit 1
    ;;
esac
