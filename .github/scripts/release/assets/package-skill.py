#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import shutil
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path


SKILL_NAME = "plane-cli"


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    release_version = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("RELEASE_VERSION", "")
    release_root = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(os.environ.get("RELEASE_ROOT", ""))
    if not release_version:
        fail("missing release version")
    if not release_root:
        fail("missing release root")

    repo_root = Path(__file__).resolve().parents[4]
    source = repo_root / "skills" / SKILL_NAME
    if not (source / "SKILL.md").is_file():
        fail(f"missing skill source: {source / 'SKILL.md'}")

    release_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="plane-skill-") as tmp:
        tmp_root = Path(tmp)
        skill_root = tmp_root / SKILL_NAME
        shutil.copytree(source, skill_root)
        metadata = {
            "schemaVersion": 1,
            "name": SKILL_NAME,
            "skillVersion": release_version,
            "package": {
                "root": SKILL_NAME,
                "format": ["tar.gz", "zip"],
            },
        }
        (skill_root / "metadata.json").write_text(
            json.dumps(metadata, indent=2) + "\n",
            encoding="utf-8",
        )

        tar_path = release_root / f"{SKILL_NAME}.tar.gz"
        with tarfile.open(tar_path, "w:gz") as tar:
            tar.add(skill_root, arcname=SKILL_NAME)

        zip_path = release_root / f"{SKILL_NAME}.zip"
        with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for file_path in sorted(skill_root.rglob("*")):
                if file_path.is_file():
                    archive.write(file_path, file_path.relative_to(tmp_root).as_posix())

    print(tar_path)
    print(zip_path)


if __name__ == "__main__":
    main()
