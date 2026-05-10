---
name: rust-quality-gate
description: "Trigger: commit, PR, pull request, cargo fmt, clippy. Run Logmancer Rust quality gates before commit or PR."
license: Apache-2.0
metadata:
  version: "1.0"
---

## Activation Contract

Use this skill before creating a commit, opening a PR, or preparing changes for review in Logmancer.

## Hard Rules

- Run `cargo fmt` before any commit or PR preparation.
- Run `cargo clippy --workspace -- -D warnings` after formatting.
- Do not create the commit or PR while either command fails.
- If `cargo fmt` changes files, include those formatting changes in the same reviewable work unit.
- Do not run `cargo build`; this project explicitly avoids builds during agent changes unless the user asks.
- Keep Conventional Commit style and never add AI attribution trailers.

## Decision Gates

| Situation | Action |
|-----------|--------|
| `cargo fmt` succeeds with changes | Continue, then include changed files in commit/PR scope |
| `cargo fmt` fails | Stop, report the formatter error, and do not commit or open a PR |
| `cargo clippy --workspace -- -D warnings` fails | Fix warnings if in scope; otherwise stop and report blockers |
| User asks to skip gates | Push back and require explicit confirmation before bypassing |

## Execution Steps

1. Run `cargo fmt` from the repository root.
2. Run `cargo clippy --workspace -- -D warnings` from the repository root.
3. Inspect the resulting working tree before committing or preparing a PR.
4. Proceed only when both commands pass.

## Output Contract

Report:
- `cargo fmt`: pass/fail and whether files changed.
- `cargo clippy --workspace -- -D warnings`: pass/fail.
- Any blockers that prevent commit or PR creation.

## References

- `@AGENTS.md` — Logmancer workspace commands, style, and agent rules.
