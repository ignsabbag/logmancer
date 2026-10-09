#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
temporary=$(mktemp -d "${TMPDIR:-/tmp}/logmancer-arch-tests.XXXXXXXX")
trap 'rm -rf -- "$temporary"' EXIT
repository="$temporary/checkout with spaces"
git init -q "$repository"
git -C "$repository" config user.name 'Packaging test'
git -C "$repository" config user.email 'packaging@example.invalid'
mkdir -p "$repository/packaging/arch" "$repository/logmancer-desktop"
cp "$root/packaging/arch/PKGBUILD" "$repository/packaging/arch/PKGBUILD"
printf '[package]\nversion = "0.5.0-beta.2"\n' > "$repository/logmancer-desktop/Cargo.toml"
git -C "$repository" add .
git -C "$repository" commit -qm 'Beta fixture'
git -C "$repository" tag v0.5.0-beta.2
sha=$(git -C "$repository" rev-parse HEAD)

# Untracked and modified files must not become part of the sources.
printf 'untracked\n' > "$repository/local-only.txt"
printf 'dirty\n' >> "$repository/logmancer-desktop/Cargo.toml"
bash "$root/packaging/arch/prepare_ci.sh" "$repository" "$temporary/development"
grep -Fx "pkgver=0.5.0beta2.r1.g${sha:0:12}" "$temporary/development/PKGBUILD"
if tar -tzf "$temporary/development/logmancer-source.tar.gz" | grep -q local-only; then
  echo 'Untracked file leaked into sources' >&2
  exit 1
fi
if tar -xOzf "$temporary/development/logmancer-source.tar.gz" logmancer-source/logmancer-desktop/Cargo.toml | grep -q dirty; then
  echo 'Uncommitted change leaked into sources' >&2
  exit 1
fi

# Sources are deterministic; the recipe's checksum matches the archive.
bash "$root/packaging/arch/prepare_ci.sh" "$repository" "$temporary/repeated"
cmp "$temporary/development/logmancer-source.tar.gz" "$temporary/repeated/logmancer-source.tar.gz"
checksum=$(sha256sum "$temporary/development/logmancer-source.tar.gz")
grep -Fx "sha256sums=('${checksum%% *}')" "$temporary/development/PKGBUILD"
cmp "$root/packaging/arch/PKGBUILD" "$repository/packaging/arch/PKGBUILD"

bash "$root/packaging/arch/prepare_ci.sh" "$repository" "$temporary/release" v0.5.0-beta.2
grep -Fx 'pkgver=0.5.0beta2' "$temporary/release/PKGBUILD"
if bash "$root/packaging/arch/prepare_ci.sh" "$repository" "$temporary/wrong-tag" v0.5.0; then
  echo 'Accepted a mismatched tag' >&2
  exit 1
fi
if bash "$root/packaging/arch/prepare_ci.sh" "$repository" "$temporary/release" v0.5.0-beta.2; then
  echo 'Overwrote an existing recipe' >&2
  exit 1
fi

printf '[package]\nversion = "0.5.0"\n' > "$repository/logmancer-desktop/Cargo.toml"
git -C "$repository" add logmancer-desktop/Cargo.toml
git -C "$repository" commit -qm 'Stable fixture'
git -C "$repository" tag v0.5.0
sha=$(git -C "$repository" rev-parse HEAD)
bash "$root/packaging/arch/prepare_ci.sh" "$repository" "$temporary/stable-development"
grep -Fx "pkgver=0.5.1alpha0.r2.g${sha:0:12}" "$temporary/stable-development/PKGBUILD"
bash "$root/packaging/arch/prepare_ci.sh" "$repository" "$temporary/stable-release" v0.5.0
grep -Fx 'pkgver=0.5.0' "$temporary/stable-release/PKGBUILD"
if bash "$root/packaging/arch/prepare_ci.sh" "$repository" "$temporary/stale-tag" v0.5.0-beta.2; then
  echo 'Accepted a tag from another commit' >&2
  exit 1
fi

git clone -q --depth 1 "file://$repository" "$temporary/shallow"
if bash "$root/packaging/arch/prepare_ci.sh" "$temporary/shallow" "$temporary/shallow-recipe"; then
  echo 'Accepted shallow development history' >&2
  exit 1
fi
printf '[package]\nversion = "0.5.0-rc.1"\n' > "$repository/logmancer-desktop/Cargo.toml"
git -C "$repository" add logmancer-desktop/Cargo.toml
git -C "$repository" commit -qm 'Unsupported version fixture'
if bash "$root/packaging/arch/prepare_ci.sh" "$repository" "$temporary/unsupported"; then
  echo 'Accepted an unsupported version' >&2
  exit 1
fi
echo 'Arch source preparation tests passed.'

# Available on Arch, optional on other hosts running the source tests.
if command -v vercmp >/dev/null; then
  for pair in \
    '0.5.0beta2 0.5.0beta2.r1.gaaaaaaaaaaaa' \
    '0.5.0beta2.r1.gaaaaaaaaaaaa 0.5.0beta2.r2.gbbbbbbbbbbbb' \
    '0.5.0beta2.r2.gbbbbbbbbbbbb 0.5.0beta3' \
    '0.5.0beta3 0.5.0' \
    '0.5.0 0.5.1alpha0.r1.gaaaaaaaaaaaa' \
    '0.5.1alpha0.r1.gaaaaaaaaaaaa 0.5.1beta1' \
    '0.5.1beta1 0.5.1'; do
    read -r earlier later <<< "$pair"
    if [[ $(vercmp "$earlier" "$later") != -1 ]]; then
      echo "Incorrect Arch version ordering: $earlier >= $later" >&2
      exit 1
    fi
  done
  echo 'Arch version ordering tests passed.'
fi
