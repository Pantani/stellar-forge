#!/usr/bin/env python3
"""Render a Homebrew formula from release archive checksums."""

from __future__ import annotations

import argparse
from pathlib import Path


TARGETS = {
    "aarch64-apple-darwin": ("macos", "arm"),
    "x86_64-apple-darwin": ("macos", "intel"),
    "x86_64-unknown-linux-gnu": ("linux", None),
}


def read_checksum(path: Path) -> str:
    content = path.read_text(encoding="utf-8").strip()
    if not content:
        raise ValueError(f"{path} is empty")
    return content.split()[0]


def render_formula(
    *,
    version: str,
    repo: str,
    binary_name: str,
    checksums_dir: Path,
    license_id: str | None,
) -> str:
    checksums: dict[str, str] = {}
    for target in TARGETS:
        archive = f"{binary_name}-{version}-{target}.tar.gz"
        checksums[target] = read_checksum(checksums_dir / f"{archive}.sha256")

    base_url = f"https://github.com/{repo}/releases/download/v{version}"
    lines = [
        "class StellarForge < Formula",
        '  desc "Rust CLI for manifest-driven Stellar workspaces"',
        f'  homepage "https://github.com/{repo}"',
        f'  version "{version}"',
    ]
    if license_id:
        lines.append(f'  license "{license_id}"')

    lines.extend(
        [
            "",
            "  on_macos do",
            "    on_arm do",
            f'      url "{base_url}/{binary_name}-{version}-aarch64-apple-darwin.tar.gz"',
            f'      sha256 "{checksums["aarch64-apple-darwin"]}"',
            "    end",
            "",
            "    on_intel do",
            f'      url "{base_url}/{binary_name}-{version}-x86_64-apple-darwin.tar.gz"',
            f'      sha256 "{checksums["x86_64-apple-darwin"]}"',
            "    end",
            "  end",
            "",
            "  on_linux do",
            f'    url "{base_url}/{binary_name}-{version}-x86_64-unknown-linux-gnu.tar.gz"',
            f'    sha256 "{checksums["x86_64-unknown-linux-gnu"]}"',
            "  end",
            "",
            "  def install",
            f'    bin.install "{binary_name}"',
            "  end",
            "",
            "  test do",
            f'    assert_match "{binary_name}", shell_output("#{{bin}}/{binary_name} --help")',
            "  end",
            "end",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True)
    parser.add_argument("--repo", default="Pantani/stellar-forge")
    parser.add_argument("--binary-name", default="stellar-forge")
    parser.add_argument("--checksums-dir", type=Path, required=True)
    parser.add_argument("--license", dest="license_id")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    formula = render_formula(
        version=args.version,
        repo=args.repo,
        binary_name=args.binary_name,
        checksums_dir=args.checksums_dir,
        license_id=args.license_id,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(formula, encoding="utf-8")


if __name__ == "__main__":
    main()
