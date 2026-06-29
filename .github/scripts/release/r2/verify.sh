#!/usr/bin/env bash
set -euo pipefail

for name in PLANE_RELEASES_S3_AK PLANE_RELEASES_S3_SK PLANE_RELEASES_S3_BUCKET PLANE_RELEASES_S3_URL PLANE_RELEASES_PUBLIC_URL RELEASE_CHANNEL RELEASE_VERSION RUNNER_TEMP; do
  if [ -z "${!name:-}" ]; then
    echo "$name is required" >&2
    exit 1
  fi
done

public_url="${PLANE_RELEASES_PUBLIC_URL%/}"
version_prefix="$RELEASE_CHANNEL/versions/$RELEASE_VERSION"
latest_prefix="$RELEASE_CHANNEL/latest"
latest_metadata_key="$latest_prefix/metadata.json"
version_metadata_key="$version_prefix/metadata.json"
latest_metadata="$RUNNER_TEMP/plane-release-latest-metadata.json"
version_metadata="$RUNNER_TEMP/plane-release-version-metadata.json"
objects_file="$RUNNER_TEMP/plane-release-r2-objects.tsv"

publish_root_manage=0
if [ "$RELEASE_CHANNEL" = "stable" ] || [ "${PLANE_PUBLISH_ROOT_MANAGE:-}" = "1" ]; then
  publish_root_manage=1
fi

s3api() {
  AWS_ACCESS_KEY_ID="$PLANE_RELEASES_S3_AK" \
  AWS_SECRET_ACCESS_KEY="$PLANE_RELEASES_S3_SK" \
  AWS_DEFAULT_REGION=auto \
  AWS_EC2_METADATA_DISABLED=true \
  aws --endpoint-url "${PLANE_RELEASES_S3_URL%/}" s3api "$@" --no-cli-pager
}

echo "verify R2 latest metadata object: s3://$PLANE_RELEASES_S3_BUCKET/$latest_metadata_key"
s3api get-object \
  --bucket "$PLANE_RELEASES_S3_BUCKET" \
  --key "$latest_metadata_key" \
  "$latest_metadata" >/dev/null

echo "verify R2 version metadata object: s3://$PLANE_RELEASES_S3_BUCKET/$version_metadata_key"
s3api get-object \
  --bucket "$PLANE_RELEASES_S3_BUCKET" \
  --key "$version_metadata_key" \
  "$version_metadata" >/dev/null

DOWNLOADED_METADATA="$latest_metadata" \
VERSION_METADATA="$version_metadata" \
OBJECTS_FILE="$objects_file" \
EXPECTED_CHANNEL="$RELEASE_CHANNEL" \
EXPECTED_RELEASE_VERSION="$RELEASE_VERSION" \
EXPECTED_PUBLIC_URL="$public_url" \
EXPECTED_VERSION_PREFIX="$version_prefix" \
EXPECTED_LATEST_PREFIX="$latest_prefix" \
PUBLISH_ROOT_MANAGE="$publish_root_manage" \
python3 <<'PY'
import json
import os
from pathlib import Path

latest = json.loads(Path(os.environ["DOWNLOADED_METADATA"]).read_text(encoding="utf-8"))
version = json.loads(Path(os.environ["VERSION_METADATA"]).read_text(encoding="utf-8"))
if latest != version:
    raise SystemExit("latest metadata and version metadata differ")

metadata = latest
expected_channel = os.environ["EXPECTED_CHANNEL"]
expected_release_version = os.environ["EXPECTED_RELEASE_VERSION"]
expected_public_url = os.environ["EXPECTED_PUBLIC_URL"]
expected_version_prefix = os.environ["EXPECTED_VERSION_PREFIX"]
expected_latest_prefix = os.environ["EXPECTED_LATEST_PREFIX"]

if metadata["channel"] != expected_channel:
    raise SystemExit(f"unexpected channel: {metadata['channel']}")
if metadata["releaseVersion"] != expected_release_version:
    raise SystemExit(f"unexpected releaseVersion: {metadata['releaseVersion']}")

expected_manage_unix = f"{expected_public_url}/manage.sh"
expected_manage_windows = f"{expected_public_url}/manage.ps1"
if metadata["manage"]["unix"] != expected_manage_unix:
    raise SystemExit(f"unexpected unix manager url: {metadata['manage']['unix']}")
if metadata["manage"]["windows"] != expected_manage_windows:
    raise SystemExit(f"unexpected windows manager url: {metadata['manage']['windows']}")

r2 = metadata["r2"]
if r2["publicUrl"] != expected_public_url:
    raise SystemExit(f"unexpected r2 publicUrl: {r2['publicUrl']}")
if r2["versionPrefix"] != expected_version_prefix:
    raise SystemExit(f"unexpected r2 versionPrefix: {r2['versionPrefix']}")
if r2["latestPrefix"] != expected_latest_prefix:
    raise SystemExit(f"unexpected r2 latestPrefix: {r2['latestPrefix']}")
if r2["latestMetadataUrl"] != f"{expected_public_url}/{expected_latest_prefix}/metadata.json":
    raise SystemExit(f"unexpected latestMetadataUrl: {r2['latestMetadataUrl']}")
if r2["versionMetadataUrl"] != f"{expected_public_url}/{expected_version_prefix}/metadata.json":
    raise SystemExit(f"unexpected versionMetadataUrl: {r2['versionMetadataUrl']}")

if metadata["channel"] == "beta":
    if metadata.get("betaVersion") != expected_release_version:
        raise SystemExit(f"unexpected betaVersion: {metadata.get('betaVersion')}")
    base_version = metadata.get("baseVersion")
    beta_number = metadata.get("betaNumber")
    if not isinstance(base_version, str) or not base_version:
        raise SystemExit("missing baseVersion")
    if not isinstance(beta_number, int):
        raise SystemExit("missing betaNumber")
    if f"v{base_version}-beta.{beta_number}" != expected_release_version:
        raise SystemExit("beta metadata does not reconstruct expected release version")

objects = []
for key, item in metadata["artifacts"].items():
    if not item.get("url"):
        raise SystemExit(f"missing artifact url for {key}")
    name = item.get("name")
    size = item.get("size")
    content_type = item.get("contentType")
    if not isinstance(name, str) or not name:
        raise SystemExit(f"missing artifact name for {key}")
    if not isinstance(size, int) or size < 1:
        raise SystemExit(f"invalid artifact size for {key}: {size}")
    if not isinstance(content_type, str) or not content_type:
        raise SystemExit(f"missing artifact contentType for {key}")
    expected_url = f"{expected_public_url}/{expected_version_prefix}/{name}"
    if item["url"] != expected_url:
        raise SystemExit(f"unexpected artifact url for {key}: {item['url']}")
    objects.append((f"{expected_version_prefix}/{name}", str(size), content_type))

if os.environ["PUBLISH_ROOT_MANAGE"] == "1":
    objects.append(("manage.sh", "-", "text/x-shellscript; charset=utf-8"))
    objects.append(("manage.ps1", "-", "text/plain; charset=utf-8"))

Path(os.environ["OBJECTS_FILE"]).write_text(
    "".join("\t".join(item) + "\n" for item in objects),
    encoding="utf-8",
)
PY

while IFS="$(printf '\t')" read -r object_key expected_size expected_content_type; do
  [ -n "$object_key" ] || continue
  head_json="$RUNNER_TEMP/plane-release-head.json"
  echo "verify R2 object: s3://$PLANE_RELEASES_S3_BUCKET/$object_key"
  s3api head-object \
    --bucket "$PLANE_RELEASES_S3_BUCKET" \
    --key "$object_key" > "$head_json"

  HEAD_JSON="$head_json" \
  OBJECT_KEY="$object_key" \
  EXPECTED_SIZE="$expected_size" \
  EXPECTED_CONTENT_TYPE="$expected_content_type" \
  python3 <<'PY'
import json
import os
from pathlib import Path

head = json.loads(Path(os.environ["HEAD_JSON"]).read_text(encoding="utf-8"))
key = os.environ["OBJECT_KEY"]
expected_size = os.environ["EXPECTED_SIZE"]
expected_content_type = os.environ["EXPECTED_CONTENT_TYPE"]

if expected_size != "-":
    actual_size = head.get("ContentLength")
    if actual_size != int(expected_size):
        raise SystemExit(f"unexpected size for {key}: {actual_size} != {expected_size}")

actual_content_type = head.get("ContentType")
if expected_content_type and actual_content_type != expected_content_type:
    raise SystemExit(
        f"unexpected content type for {key}: {actual_content_type} != {expected_content_type}"
    )
PY
done < "$objects_file"
