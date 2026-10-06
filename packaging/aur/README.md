# Arch Linux suite package

The `PKGBUILD` builds the complete suite from the public `v0.5.0-beta.2`
source archive, verified with SHA-256. It does not repackage a DEB or download
Logmancer binaries. Only x86_64 is currently supported.

## Manual build

On an up-to-date Arch system, install `base-devel`, `rustup`, `namcap`,
`devtools`, `python`, and `desktop-file-utils`. Copy `PKGBUILD`, `.SRCINFO`, and
`logmancer.desktop` into a writable build directory, then run as a normal user:

```sh
makepkg --syncdeps --log
python /path/to/logmancer/packaging/aur/check_namcap.py PKGBUILD logmancer-*.pkg.tar.zst
sudo pacman -U logmancer-*.pkg.tar.zst
python /path/to/logmancer/packaging/aur/smoke_test.py
```

For a clean-chroot build, run in that build directory:

```sh
pkgctl build --repo extra --arch x86_64 --clean --inspect never
```

`--repo extra` selects Arch's dependency environment; it does **not** publish
the package to an official repository. The host needs working mounts/namespaces,
sudo access, and enough disk space for the chroot, Rust tools, and build outputs.
Do not run `makepkg` as root.

The recipe installs the source's pinned Rust toolchain and WASM target into
build-local `RUSTUP_HOME`, and builds pinned `cargo-leptos` 0.3.10 into a local
tools directory. It does not change the user's Rust defaults. Network access is
needed for Rust distributions, crates, and cargo-leptos's auxiliary tools
(including Dart Sass). Logmancer Cargo dependencies use the committed lockfile;
this is a source-built package, not a claim of fully offline or byte-for-byte
reproducible builds.

## Installed layout

- `/usr/lib/logmancer/`: launcher, Desktop, Web, TUI, and generated `site/`.
- `/usr/bin/logmancer{,-desktop,-web,-tui}`: symlinks to the real executables.
  Linux executable resolution follows these links, preserving sibling lookup.
- `/usr/lib/Logmancer`: compatibility link to the suite for Tauri's Linux
  resource lookup, which uses `productName` from its configuration.
- `/usr/share/applications/logmancer.desktop`: starts Desktop directly.
- `/usr/share/icons/hicolor/{32x32,128x128,256x256}/apps/logmancer.png`.
- `/usr/share/licenses/logmancer/LICENSE`.

No Web service, automatic startup, or default file association is installed.
Web remains manually started and loopback-only by default. Removing the package
does not remove per-user application data or logs.

## Development and release workflows

`arch-package.yml` is reused by Build, Release, and Desktop diagnostics build.
The manual diagnostics workflow uses the exact dispatch SHA and uploads
`logmancer-diagnostics-package-linux-arch`, keeping the same Arch validations
without adding Rust tests or clippy.
It stages the exact checked-out
Git commit using `git archive`, excluding untracked files and local changes.
`prepare_ci.py` creates a **new** build directory containing a temporary recipe,
source archive, matching checksum, and `SOURCE_COMMIT`. It refuses to overwrite
an existing directory. Compilation and installation functions are unchanged.

```sh
# Snapshot of the committed tree (not local edits):
python packaging/aur/prepare_ci.py /tmp/opencode/logmancer-arch-snapshot --ref HEAD

# Release tag must match the committed desktop crate version:
python packaging/aur/prepare_ci.py /tmp/opencode/logmancer-arch-release \
  --ref v0.5.0-beta.2 --release
```

Arch versions normalize `0.5.0-beta.2` to `0.5.0beta2`. Development versions add
`.r<commit timestamp>.g<12-character SHA>`, identifying the exact source and
ordering ordinary mainline snapshots chronologically. A beta and its snapshots
sort below the corresponding stable version. Commit timestamps are not a
universal ordering across rebases, cherry-picks, or unrelated branches; these
artifacts are intended for mainline development, not a rolling package repo.

CI uses a privileged Arch container to run `pkgctl build` in a fresh chroot,
checks namcap errors, installs the result, runs headless smoke tests as an
unprivileged user, and removes the package. Namcap warnings remain visible and
must be reviewed; errors fail the job. Build uploads `.pkg.tar.zst` and source
provenance as artifacts. Release additionally publishes `.pkg.tar.zst` alongside
the existing installers and portable archives. Publication waits for Arch
validation as well as the other platforms.

The committed AUR recipe remains independent of CI. When moving it to another
released tag, update `pkgver`, `_tag`, `_srcname`, and the archive checksum, then
regenerate `.SRCINFO` with `makepkg --printsrcinfo > .SRCINFO`. Never replace
checksums with `SKIP`. AUR publication, `logmancer-bin`, and automated AUR updates
are not implemented here.

## Validation checklist (#113 / #112)

Run recipe tests without compiling Rust:

```sh
python -m unittest discover -s packaging/aur/tests -v
```

The tests cover source selection, checksums, version ordering, release-tag
validation, isolation from local edits, and staged package paths/permissions.
The Test workflow also runs these recipe checks on pull requests, without
building Rust or using a privileged container.
Installed smoke tests cover direct Web startup, launcher Web startup, SSR and
CSS/JS/WASM assets from an unrelated working directory, loopback listeners,
direct and launcher TUI startup, and headless default selection.

Before closing the issues, record the successful chroot build, namcap output,
installation and removal results, **and manually validate in an Arch graphical
session**:

- Start Desktop from the application menu, `logmancer-desktop`,
  `logmancer desktop`, and `logmancer <file>`.
- Verify the SSR UI hydrates, opening a log works, and icons appear.
- Start Web/TUI directly and through the launcher; verify an explicit
  `LEPTOS_SITE_ROOT` override remains authoritative.
- Remove the package with `sudo pacman -R logmancer`; verify public commands,
  resources, desktop entry, and icons are gone, while user data remains.

CI headless checks do not constitute graphical Desktop validation. The existing
Windows/Debian validation under #112 does not need to be repeated merely to
prepare this Arch recipe.
