# Arch checkout packages and build workflows

Build and Release share `package-linux.yml`, `package-windows.yml`, and
`package-arch.yml`. Existing Linux/Windows artifact names remain unchanged;
Arch uses `logmancer-package-linux-arch`.

## Development and branch builds

Automatic Build runs after a successful push Test on `main` and packages the
exact tested SHA. To test another branch, run Build manually from Actions,
select the branch and platform (`all`, `linux`, `windows`, or `arch`). Manual
Build runs the existing tests and clippy before packaging that same SHA.
Manual artifacts are not removed by automatic main-artifact cleanup.

Release packages the triggering tag's commit and publishes portable ZIPs,
DEB, NSIS, and `.pkg.tar.zst` assets only after all packaging jobs succeed.

## Arch source selection

The Ubuntu runner hosts an `archlinux:base-devel` container. The container
installs Arch build dependencies and Rust 1.95.0 with the WASM target;
`makepkg` runs as an unprivileged builder. This is not a `pkgctl` clean chroot.
The rolling Arch image/dependencies are not pinned to a historical snapshot.

`packaging/arch/prepare_ci.sh` uses `git archive` to package tracked files from
HEAD, excluding local changes and untracked files. It writes the archive and
a temporary `PKGBUILD` with its SHA-256 checksum and package version. The
committed recipe is a checkout-based template, not an AUR submission recipe.
Its build/package functions are the single compilation/installation recipe.
The recipe installs pinned cargo-leptos 0.3.10 and builds the web assets and
all workspace executables. The auxiliary tool uses two build jobs and no LTO
or debug symbols to limit memory usage; application profiles remain unchanged.
Tool and Cargo dependency downloads require network.

To prepare a recipe manually, from a committed checkout with full history:

```bash
bash packaging/arch/prepare_ci.sh "$PWD" /path/to/empty-recipe-directory
# For a release, supply the matching tag as the third argument:
bash packaging/arch/prepare_ci.sh "$PWD" /path/to/another-empty-directory v0.5.0-beta.2
```

In Arch, install `base-devel`, `rustup`, `pkgconf`, and the recipe's runtime
dependencies. As a non-root user, install/default Rust 1.95.0 with
`wasm32-unknown-unknown`, then run `makepkg` in the prepared directory.

## Versions

- Release `v0.5.0-beta.2` becomes `0.5.0beta2-1`.
- Development on that beta becomes `0.5.0beta2.r<commit-count>.g<12-char-SHA>-1`.
- Development on stable `0.5.0` becomes
  `0.5.1alpha0.r<commit-count>.g<12-char-SHA>-1`, between that stable and its
  next patch release.
- Release tags must match both the manifest version and HEAD.
- Development requires full history. Commit counts order snapshots along a
  branch; unrelated branches are not promised a global chronological order.

Only stable and `-beta.N` manifest versions are supported. `pkgrel` is 1.

## Installation

```bash
sudo pacman -U ./logmancer-*.pkg.tar.zst
sudo pacman -R logmancer
```

Real executables and assets live under `/usr/lib/logmancer`. Public wrappers
expose all four commands through `/usr/bin`. A compatibility link at
`/usr/lib/logmancer-desktop` points to the suite for Tauri resource resolution.
Desktop integration, icons, and the MIT license are installed under `/usr/share`.
Configuration remains in the applications' per-user directories.

## Deferred validation

These workflows check build success and require an artifact to exist before
upload; they do not claim functional validation of the installed package.
Issue #112 tracks multiplatform content, installation, execution, and removal
checks. Issue #127 tracks the public-tag AUR recipe, `.SRCINFO`, `namcap`, and
`pkgctl build` in a clean chroot. Neither AUR publication nor prebuilt AUR
recipes are part of this implementation.
