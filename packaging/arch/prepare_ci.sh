#!/usr/bin/env bash
set -euo pipefail

if (( $# < 2 || $# > 3 )); then
  echo "Usage: $0 CHECKOUT OUTPUT_DIRECTORY [RELEASE_TAG]" >&2
  exit 2
fi

checkout=$(realpath "$1")
output=$(realpath -m "$2")
release_tag=${3:-}
commit=$(git -C "$checkout" rev-parse HEAD)
manifest=$(git -C "$checkout" show "$commit:logmancer-desktop/Cargo.toml")
version=$(sed -nE 's/^version = "([^"]+)"$/\1/p' <<< "$manifest" | head -n 1)

if [[ ! "$version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-beta\.([0-9]+))?$ ]]; then
  echo "Unsupported package version: $version" >&2
  exit 1
fi
major=${BASH_REMATCH[1]}
minor=${BASH_REMATCH[2]}
patch=${BASH_REMATCH[3]}
beta=${BASH_REMATCH[5]}
pkgver="$major.$minor.$patch"
if [[ -n "$beta" ]]; then
  pkgver+="beta$beta"
fi

if [[ -n "$release_tag" ]]; then
  if [[ "$release_tag" != "v$version" ]] ||
     [[ $(git -C "$checkout" rev-parse "$release_tag^{commit}") != "$commit" ]]; then
    echo "Release tag must match the checkout commit and manifest version ($version)." >&2
    exit 1
  fi
else
  if [[ $(git -C "$checkout" rev-parse --is-shallow-repository) == true ]]; then
    echo 'Development versioning requires full Git history (fetch-depth: 0).' >&2
    exit 1
  fi
  count=$(git -C "$checkout" rev-list --count "$commit")
  # Stable-based snapshots sort after that stable and before the next patch.
  if [[ -z "$beta" ]]; then
    pkgver="$major.$minor.$((10#$patch + 1))alpha0"
  fi
  pkgver+=".r$count.g${commit:0:12}"
fi

mkdir -p "$output"
if [[ -e "$output/PKGBUILD" || -e "$output/logmancer-source.tar.gz" ]]; then
  echo "Output directory already contains a recipe or source archive: $output" >&2
  exit 1
fi
git -C "$checkout" archive --format=tar --prefix=logmancer-source/ "$commit" |
  gzip -n > "$output/logmancer-source.tar.gz"
checksum=$(sha256sum "$output/logmancer-source.tar.gz")
checksum=${checksum%% *}
git -C "$checkout" show "$commit:packaging/arch/PKGBUILD" |
  sed -e "s/^pkgver=.*/pkgver=$pkgver/" \
      -e "s/^sha256sums=.*/sha256sums=('$checksum')/" > "$output/PKGBUILD"
printf 'Prepared %s from %s in %s\n' "$pkgver" "$commit" "$output"
