---
name: prepare-release
description: "Trigger: release, beta, prerelease, stable release, version bump, release tag. Prepare and publish Logmancer releases safely."
license: Apache-2.0
metadata:
  version: "1.0"
---

## Activation Contract

Use this skill when preparing a Logmancer beta or stable release. Read
`skills/rust-quality-gate/SKILL.md` before preparing the release commit or PR.

## Hard Rules

- Confirm the version, source branch (normally `main`), and scope: local preparation or publication. Local-only requests do not authorize commits, pushes, tags, or GitHub releases.
- Use SemVer: `X.Y.Z-beta.N` for betas and `X.Y.Z` for stable releases. Tags use the `v` prefix; package versions do not.
- Never overwrite, move, or delete an existing release tag. Investigate an existing tag or release before proceeding.
- Preserve unrelated local changes and stage only explicitly reviewed release files.
- Respect branch protections: prepare a PR and wait for required checks and merge; do not bypass protections with administrative privileges or a direct push.
- Do not publish a tag until validation succeeds and the release commit is on the confirmed source branch.
- Use the existing release workflow rather than manually building or uploading packages unless the user requests otherwise.
- Do not change CI, packaging, dependencies, or architecture as part of a version bump without separate authorization.

## Changelog Policy

| Release type | Required decision |
|-------------|-------------------|
| Beta | Keep all pending changes under `[Unreleased]`, including changes shipped in earlier betas. Do not create beta sections or move the comparison link away from the last stable release. Leave `CHANGELOG.md` unchanged if it already follows this policy and contains the relevant changes; explain this in the commit/PR summary. |
| Stable | Move pending entries into `[X.Y.Z] - YYYY-MM-DD` using the actual release date. Leave an empty `[Unreleased]` section and update comparison links from the previous stable release to the new stable release, and from the new stable release to `HEAD`. |

GitHub release notes may describe beta-specific changes without restructuring
`CHANGELOG.md`. Inspect commits since the preceding beta or stable tag to establish
the release scope.

## Execution Steps

### 1. Inspect and confirm

- Read `AGENTS.md`, the quality gate skill, and `.github/workflows/release.yml`.
- Inspect `git status --short --branch`, fetch remote refs and tags, and compare the source branch with its remote. Do not silently merge divergent branches.
- Inspect existing tags, GitHub releases, changes since the preceding release, and CI results for the exact source commit.
- Verify GitHub authentication if publication is requested. Use configured Git credentials or the authenticated GitHub CLI; never print credentials.
- Stop if the requested version already exists, the source is ambiguous, or unrelated tracked changes would enter the release.

### 2. Prepare versions and notes

Update these files consistently, without upgrading dependencies:

- `logmancer-core/Cargo.toml`
- `logmancer-tui/Cargo.toml`
- `logmancer-web/Cargo.toml`
- `logmancer-desktop/Cargo.toml`
- `logmancer-launcher/Cargo.toml`
- The five workspace package entries in `Cargo.lock`
- `logmancer-desktop/tauri.conf.json`
- The current development version in `README.md`

Search tracked files for additional references to the old version, distinguishing
historical release documentation from current version metadata. Apply the changelog
policy above and inspect the final diff.

### 3. Validate

- Run `cargo fmt`, then `cargo clippy --workspace -- -D warnings` as required by the quality gate.
- Run `cargo test --workspace` for release validation, including Desktop and SSR coverage. Report ignored tests and any missing prerequisites; do not call partial validation a full pass.
- Run `git diff --check` and verify version consistency and the absence of unrelated changes.
- If a tool is missing from `PATH`, locate the installed toolchain before treating it as unavailable. Do not install or upgrade tools without authorization.
- Stop on failed checks. For local-only requests, leave the reviewed changes uncommitted and report their status.

### 4. Commit and merge when authorized

- Stage only reviewed files and use a Conventional Commit such as `chore(release): prepare vX.Y.Z-beta.N` or `chore(release): prepare vX.Y.Z`.
- Include the explicit changelog decision in the commit/PR summary.
- Follow the repository's PR and branch-protection rules. Wait for required checks and merge before tagging.
- Refresh the source branch and verify that the merged commit contains the intended versions. A squash merge may have a different commit ID than the preparation commit.

### 5. Tag and monitor when publication is authorized

- Create an annotated tag on the verified release commit: `git tag -a vVERSION COMMIT -m "Logmancer vVERSION"`.
- Push only the intended tag: `git push origin vVERSION`.
- Find the Release workflow run for that exact tag and commit, then monitor it with `gh run watch RUN_ID --exit-status`.
- The current workflow packages Linux and Windows portable ZIPs, a Linux DEB, an Arch `.pkg.tar.zst`, and a Windows NSIS installer inside a setup ZIP (no standalone EXE). Public filenames use `logmancer-VERSION-PLATFORM-x86_64`, with `-portable` or `-setup` for ZIPs. It marks `vX.Y.Z-beta.N` releases as prereleases.
- After the workflow succeeds, verify the GitHub release's tag, prerelease flag, notes, and all expected assets. Stable releases must not be marked as prereleases.
- If packaging or publishing fails, report the failing job and logs. Do not retag, overwrite assets, rerun publication, or introduce fixes without authorization.
- Clearly distinguish a pushed tag or running workflow from a successfully published release.

## Output Contract

Report the version and source commit, changed files, changelog decision, validation
results, and any blockers. If publication was requested, include the tag, workflow
URL/status, and verified release URL/assets when available. State explicitly when
changes remain local or publication is still in progress.
