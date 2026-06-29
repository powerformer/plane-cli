#!/usr/bin/env bash
set -euo pipefail

for name in GITHUB_STEP_SUMMARY RELEASE_CHANNEL RELEASE_VERSION R2_METADATA_URL R2_VERSION_METADATA_URL R2_VERSION_PREFIX PLANE_RELEASES_PUBLIC_URL; do
  if [ -z "${!name:-}" ]; then
    echo "$name is required" >&2
    exit 1
  fi
done

{
  echo "## Plane ${RELEASE_CHANNEL} release"
  echo ""
  echo "| Field | Value |"
  echo "| --- | --- |"
  echo "| Channel | \`${RELEASE_CHANNEL}\` |"
  echo "| Version | \`${RELEASE_VERSION}\` |"
  if [ -n "${BASE_VERSION:-}" ]; then
    echo "| Base version | \`${BASE_VERSION}\` |"
  fi
  if [ -n "${BETA_NUMBER:-}" ]; then
    echo "| Beta number | \`${BETA_NUMBER}\` |"
  fi
  if [ -n "${STATE_SOURCE:-}" ]; then
    echo "| State source | \`${STATE_SOURCE}\` |"
  fi
  echo "| R2 prefix | \`${R2_VERSION_PREFIX}\` |"
  echo ""
  echo "### Links"
  echo ""
  echo "- Unix manager: ${PLANE_RELEASES_PUBLIC_URL%/}/manage.sh"
  echo "- Windows manager: ${PLANE_RELEASES_PUBLIC_URL%/}/manage.ps1"
  echo "- Latest metadata: ${R2_METADATA_URL}"
  echo "- Version metadata: ${R2_VERSION_METADATA_URL}"
} >> "$GITHUB_STEP_SUMMARY"
