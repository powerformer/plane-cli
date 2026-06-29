#!/usr/bin/env bash
set -euo pipefail

for name in PLANE_RELEASES_PUBLIC_URL RELEASE_CHANNEL RELEASE_VERSION R2_METADATA_URL RUNNER_TEMP; do
  if [ -z "${!name:-}" ]; then
    echo "$name is required" >&2
    exit 1
  fi
done

VERIFY_ATTEMPTS=${PLANE_RELEASE_VERIFY_ATTEMPTS:-36}
VERIFY_INTERVAL_SECONDS=${PLANE_RELEASE_VERIFY_INTERVAL_SECONDS:-10}
VERIFY_CONNECT_TIMEOUT_SECONDS=${PLANE_RELEASE_VERIFY_CONNECT_TIMEOUT_SECONDS:-10}
VERIFY_MAX_TIME_SECONDS=${PLANE_RELEASE_VERIFY_MAX_TIME_SECONDS:-30}

require_positive_int() {
  local name="$1"
  local value="$2"
  case "$value" in
    ''|*[!0-9]*)
      echo "$name must be a positive integer, got: $value" >&2
      exit 1
      ;;
  esac
  if [ "$value" -lt 1 ]; then
    echo "$name must be >= 1, got: $value" >&2
    exit 1
  fi
}

require_positive_int PLANE_RELEASE_VERIFY_ATTEMPTS "$VERIFY_ATTEMPTS"
require_positive_int PLANE_RELEASE_VERIFY_INTERVAL_SECONDS "$VERIFY_INTERVAL_SECONDS"
require_positive_int PLANE_RELEASE_VERIFY_CONNECT_TIMEOUT_SECONDS "$VERIFY_CONNECT_TIMEOUT_SECONDS"
require_positive_int PLANE_RELEASE_VERIFY_MAX_TIME_SECONDS "$VERIFY_MAX_TIME_SECONDS"

curl_with_retry() {
  local mode="$1"
  local url="$2"
  local output="${3:-}"
  local attempt=1
  local status=0

  while [ "$attempt" -le "$VERIFY_ATTEMPTS" ]; do
    echo "verify public URL attempt $attempt/$VERIFY_ATTEMPTS: $url"
    case "$mode" in
      get)
        curl -fsSL \
          --connect-timeout "$VERIFY_CONNECT_TIMEOUT_SECONDS" \
          --max-time "$VERIFY_MAX_TIME_SECONDS" \
          "$url" \
          -o "$output" && return 0
        status=$?
        ;;
      head)
        curl -fsSI \
          --connect-timeout "$VERIFY_CONNECT_TIMEOUT_SECONDS" \
          --max-time "$VERIFY_MAX_TIME_SECONDS" \
          "$url" >/dev/null && return 0
        status=$?
        ;;
      *)
        echo "unknown curl retry mode: $mode" >&2
        exit 1
        ;;
    esac

    if [ "$attempt" -lt "$VERIFY_ATTEMPTS" ]; then
      echo "public URL not ready yet (curl exit $status); retrying in ${VERIFY_INTERVAL_SECONDS}s" >&2
      sleep "$VERIFY_INTERVAL_SECONDS"
    fi
    attempt=$((attempt + 1))
  done

  echo "public URL verification failed after $VERIFY_ATTEMPTS attempts: $url" >&2
  return "$status"
}

metadata="$RUNNER_TEMP/plane-release-metadata.json"
curl_with_retry get "$R2_METADATA_URL?run=${GITHUB_RUN_ID:-local}" "$metadata"

DOWNLOADED_METADATA="$metadata" \
EXPECTED_CHANNEL="$RELEASE_CHANNEL" \
EXPECTED_RELEASE_VERSION="$RELEASE_VERSION" \
EXPECTED_PUBLIC_URL="${PLANE_RELEASES_PUBLIC_URL%/}" \
python3 <<'PY'
import json
import os
from pathlib import Path

metadata = json.loads(Path(os.environ["DOWNLOADED_METADATA"]).read_text(encoding="utf-8"))
if metadata["channel"] != os.environ["EXPECTED_CHANNEL"]:
    raise SystemExit(f"unexpected channel: {metadata['channel']}")
if metadata["releaseVersion"] != os.environ["EXPECTED_RELEASE_VERSION"]:
    raise SystemExit(f"unexpected releaseVersion: {metadata['releaseVersion']}")
expected_public_url = os.environ["EXPECTED_PUBLIC_URL"]
expected_manage_unix = f"{expected_public_url}/manage.sh"
expected_manage_windows = f"{expected_public_url}/manage.ps1"
if metadata["manage"]["unix"] != expected_manage_unix:
    raise SystemExit(f"unexpected unix manager url: {metadata['manage']['unix']}")
if metadata["manage"]["windows"] != expected_manage_windows:
    raise SystemExit(f"unexpected windows manager url: {metadata['manage']['windows']}")
if metadata["channel"] == "beta":
    if metadata.get("betaVersion") != os.environ["EXPECTED_RELEASE_VERSION"]:
        raise SystemExit(f"unexpected betaVersion: {metadata.get('betaVersion')}")
    base_version = metadata.get("baseVersion")
    beta_number = metadata.get("betaNumber")
    if not isinstance(base_version, str) or not base_version:
        raise SystemExit("missing baseVersion")
    if not isinstance(beta_number, int):
        raise SystemExit("missing betaNumber")
    if f"v{base_version}-beta.{beta_number}" != os.environ["EXPECTED_RELEASE_VERSION"]:
        raise SystemExit("beta metadata does not reconstruct expected release version")
for key, item in metadata["artifacts"].items():
    if not item.get("url"):
        raise SystemExit(f"missing artifact url for {key}")
PY

for url in $(python3 - "$metadata" <<'PY'
import json
import sys
from pathlib import Path
metadata = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
for item in metadata["artifacts"].values():
    print(item["url"])
print(metadata["manage"]["unix"])
print(metadata["manage"]["windows"])
PY
); do
  curl_with_retry head "$url"
done
