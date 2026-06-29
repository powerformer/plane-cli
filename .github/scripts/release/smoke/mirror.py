#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path


def fail(message: str) -> None:
    print(f"[release-smoke] {message}", file=sys.stderr)
    raise SystemExit(1)


def require_env(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        fail(f"{name} is required")
    return value


def s3_get(key: str, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    env = {
        **os.environ,
        "AWS_ACCESS_KEY_ID": require_env("PLANE_RELEASES_S3_AK"),
        "AWS_SECRET_ACCESS_KEY": require_env("PLANE_RELEASES_S3_SK"),
        "AWS_DEFAULT_REGION": "auto",
        "AWS_EC2_METADATA_DISABLED": "true",
    }
    endpoint = require_env("PLANE_RELEASES_S3_URL").rstrip("/")
    bucket = require_env("PLANE_RELEASES_S3_BUCKET")
    print(f"[release-smoke] read R2 object: s3://{bucket}/{key}")
    result = subprocess.run(
        [
            "aws",
            "--endpoint-url",
            endpoint,
            "s3api",
            "get-object",
            "--bucket",
            bucket,
            "--key",
            key,
            str(destination),
            "--no-cli-pager",
        ],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        fail(f"failed to read R2 object {key}: {result.stderr.strip()}")


def load_metadata(path: Path) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        fail(f"{path} is invalid JSON: {error}")
    if not isinstance(value, dict):
        fail(f"{path} must contain a JSON object")
    return value


def rewrite_metadata(
    metadata: dict[str, object],
    *,
    channel: str,
    version: str,
    mirror_url: str,
) -> dict[str, object]:
    version_prefix = f"{channel}/versions/{version}"
    latest_prefix = f"{channel}/latest"
    rewritten = json.loads(json.dumps(metadata))

    if rewritten.get("releaseVersion") != version:
        fail(
            "metadata releaseVersion mismatch: "
            f"{rewritten.get('releaseVersion')} != {version}"
        )

    r2 = rewritten.get("r2")
    if isinstance(r2, dict):
        r2["publicUrl"] = mirror_url
        r2["latestMetadataUrl"] = f"{mirror_url}/{latest_prefix}/metadata.json"
        r2["versionMetadataUrl"] = f"{mirror_url}/{version_prefix}/metadata.json"
        r2["versionPrefix"] = version_prefix
        r2["latestPrefix"] = latest_prefix

    manage = rewritten.get("manage")
    if isinstance(manage, dict):
        manage["unix"] = f"{mirror_url}/manage.sh"
        manage["windows"] = f"{mirror_url}/manage.ps1"

    artifacts = rewritten.get("artifacts")
    if not isinstance(artifacts, dict):
        fail("metadata artifacts must be an object")
    for key, artifact in artifacts.items():
        if not isinstance(artifact, dict):
            fail(f"metadata artifact {key} must be an object")
        name = artifact.get("name")
        if not isinstance(name, str) or not name:
            fail(f"metadata artifact {key} is missing name")
        artifact["url"] = f"{mirror_url}/{version_prefix}/{name}"

    return rewritten


def write_json(path: Path, value: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Create a local release smoke mirror from R2 objects."
    )
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--channel", required=True, choices=["stable", "beta"])
    parser.add_argument("--version", required=True)
    parser.add_argument("--platform", required=True)
    parser.add_argument("--mirror-url", required=True)
    args = parser.parse_args()

    version_prefix = f"{args.channel}/versions/{args.version}"
    latest_prefix = f"{args.channel}/latest"
    version_metadata_key = f"{version_prefix}/metadata.json"
    latest_metadata_key = f"{latest_prefix}/metadata.json"

    raw_dir = args.root / ".r2"
    version_metadata_raw = raw_dir / "version-metadata.json"
    latest_metadata_raw = raw_dir / "latest-metadata.json"
    s3_get(version_metadata_key, version_metadata_raw)
    s3_get(latest_metadata_key, latest_metadata_raw)

    version_metadata = load_metadata(version_metadata_raw)
    latest_metadata = load_metadata(latest_metadata_raw)
    if latest_metadata.get("releaseVersion") != args.version:
        fail(
            "latest metadata does not point at smoke version: "
            f"{latest_metadata.get('releaseVersion')} != {args.version}"
        )

    for name in [args.platform, "plane-cli.tar.gz"]:
        s3_get(f"{version_prefix}/{name}", args.root / version_prefix / name)

    rewritten = rewrite_metadata(
        version_metadata,
        channel=args.channel,
        version=args.version,
        mirror_url=args.mirror_url.rstrip("/"),
    )
    write_json(args.root / version_metadata_key, rewritten)
    write_json(args.root / latest_metadata_key, rewritten)
    print(f"[release-smoke] local mirror root: {args.root}")
    print(f"[release-smoke] local mirror url: {args.mirror_url.rstrip('/')}")


if __name__ == "__main__":
    main()
