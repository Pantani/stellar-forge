#!/usr/bin/env python3
"""Smoke tests for Cargo license metadata release preflight helper."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / ".github" / "scripts" / "resolve-cargo-license-metadata.py"


def resolve(package: dict[str, str]) -> subprocess.CompletedProcess[str]:
    metadata = {"packages": [package]}
    with tempfile.TemporaryDirectory() as tmp:
        metadata_path = Path(tmp) / "metadata.json"
        metadata_path.write_text(json.dumps(metadata), encoding="utf-8")
        return subprocess.run(  # noqa: S603
            [sys.executable, str(SCRIPT), "--metadata-json", str(metadata_path)],
            text=True,
            capture_output=True,
        )


def main() -> None:
    result = resolve({"license": "MIT"})
    assert result.returncode == 0
    assert result.stdout.splitlines() == ["MIT", "", "true"]

    result = resolve({"license_file": "LICENSE"})
    assert result.returncode == 0
    assert result.stdout.splitlines() == ["", "LICENSE", "true"]

    result = resolve({})
    assert result.returncode != 0


if __name__ == "__main__":
    main()
