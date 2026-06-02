#!/usr/bin/env python3
"""Smoke test for the Homebrew formula renderer."""

from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / ".github" / "scripts" / "render-homebrew-formula.py"


def main() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        version = "0.1.0"
        targets = [
            "aarch64-apple-darwin",
            "x86_64-apple-darwin",
            "x86_64-unknown-linux-gnu",
        ]
        for index, target in enumerate(targets):
            checksum = f"{index + 1:064x}"
            archive = f"stellar-forge-{version}-{target}.tar.gz"
            (tmp_path / f"{archive}.sha256").write_text(
                f"{checksum}  {archive}\n",
                encoding="utf-8",
            )

        output = tmp_path / "stellar-forge.rb"
        subprocess.run(
            [
                "python3",
                str(SCRIPT),
                "--version",
                version,
                "--checksums-dir",
                str(tmp_path),
                "--license",
                "MIT",
                "--output",
                str(output),
            ],
            check=True,
        )

        formula = output.read_text(encoding="utf-8")
        assert "class StellarForge < Formula" in formula
        assert 'license "MIT"' in formula
        for index in range(len(targets)):
            checksum = f"{index + 1:064x}"
            assert f'sha256 "{checksum}"' in formula
        assert "aarch64-apple-darwin.tar.gz" in formula
        assert "x86_64-apple-darwin.tar.gz" in formula
        assert "x86_64-unknown-linux-gnu.tar.gz" in formula
        assert 'bin.install "stellar-forge"' in formula


if __name__ == "__main__":
    main()
