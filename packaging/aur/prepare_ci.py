#!/usr/bin/env python3
"""Stage a temporary PKGBUILD and an archive of an exact, committed Git tree."""

import argparse
import hashlib
from pathlib import Path
import re
import shutil
import subprocess
import tomllib


def arch_version(version: str) -> str:
    """Keep beta < stable using pacman's alpha suffix ordering."""
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-beta\.\d+)?", version):
        raise ValueError(f"Unsupported release version: {version}")
    return version.replace("-beta.", "beta")


def git(repo: Path, *args: str) -> str:
    return subprocess.check_output(["git", "-C", str(repo), *args], text=True).strip()


def stage(repo: Path, destination: Path, ref: str, release: bool) -> str:
    commit = git(repo, "rev-parse", "--verify", f"{ref}^{{commit}}")
    manifest = tomllib.loads(git(repo, "show", f"{commit}:logmancer-desktop/Cargo.toml"))
    version = manifest["package"]["version"]
    pkgver = arch_version(version)
    if release:
        if ref != f"v{version}":
            raise ValueError(f"Release tag {ref!r} does not match crate version v{version}")
        if git(repo, "rev-parse", "--verify", f"refs/tags/{ref}^{{commit}}") != commit:
            raise ValueError("Release source does not match the tag")
    else:
        # Date orders snapshots even when a version is unchanged. Snapshots of
        # a beta remain below the corresponding stable release.
        timestamp = git(repo, "show", "-s", "--format=%ct", commit)
        pkgver += f".r{timestamp}.g{commit[:12]}"

    destination.mkdir(parents=True, exist_ok=False)
    recipe_dir = repo / "packaging" / "aur"
    shutil.copy2(recipe_dir / "logmancer.desktop", destination)
    archive = destination / "logmancer-source.tar.gz"
    subprocess.run(
        ["git", "-C", str(repo), "archive", "--format=tar.gz", "--prefix=logmancer-source/",
         "-o", str(archive), commit], check=True,
    )
    with archive.open("rb") as source:
        checksum = hashlib.file_digest(source, "sha256").hexdigest()
    recipe = (recipe_dir / "PKGBUILD").read_text()
    replacements = {
        r"^pkgver=.*$": f"pkgver={pkgver}",
        r"^_srcname=.*$": "_srcname=logmancer-source",
        r"^source=.*$": "source=('logmancer-source.tar.gz' 'logmancer.desktop')",
        r"^sha256sums=\('[0-9a-f]+'": f"sha256sums=('{checksum}'",
    }
    for pattern, replacement in replacements.items():
        recipe, count = re.subn(pattern, replacement, recipe, flags=re.MULTILINE)
        if count != 1:
            raise ValueError(f"Expected one recipe field matching {pattern!r}")
    (destination / "PKGBUILD").write_text(recipe)
    (destination / "SOURCE_COMMIT").write_text(commit + "\n")
    return pkgver


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--ref", default="HEAD")
    parser.add_argument("--release", action="store_true")
    args = parser.parse_args()
    print(stage(args.repo.resolve(), args.destination.resolve(), args.ref, args.release))


if __name__ == "__main__":
    main()
