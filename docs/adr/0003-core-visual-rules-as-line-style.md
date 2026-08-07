# ADR 0003: Core Visual Rules as Line Style Intent

- Status: Accepted
- Date: 2026-07-15

## Context

Visual rules need a shared core contract before web, TUI, desktop, or persisted configuration can render user-defined highlights consistently.

Logmancer Web already has span-based search decorations (`LineDecoration { start, end, kind }`) for inline match highlighting. Reusing that model for visual rules would couple core behavior to web rendering concerns and blur two different responsibilities: base line styling and inline overlays.

## Decision

Core visual rules produce whole-line style intent through `PageLine::style` and `LineStyleIntent`.

Visual rules do not produce span-based decorations. Existing web search decorations remain an inline overlay concern.

| Area | Decision |
|---|---|
| Scope | Visual rules are core-owned and UI-neutral. |
| Output | Matching rules attach optional whole-line foreground/background style intent to `PageLine`. |
| Priority | Array order defines priority; the first matching rule wins. |
| Matching | Rules support text and regex matchers with per-rule case sensitivity. |
| Invalid rules | Invalid rules are skipped/disabled safely; reads must not crash. |
| Non-interference | Visual rules do not alter visibility, filtering, search, pagination, tail behavior, or navigation. |
| Rendering | UI layers decide how to render `LineStyleIntent`; web span decorations remain separate. |
| Persistence | Saving/loading visual rules is out of scope for this decision. |

## Consequences

- Core can expose visual-rule results without depending on CSS, Ratatui, Leptos, or web-specific decoration kinds.
- Web can later combine visual rules as a base line style with search highlights as inline overlays.
- Future persistence work can store UI-neutral rule definitions instead of frontend-specific render metadata.
- Consumers must treat `PageLine::style` as optional metadata; undecorated reads remain valid.

## Verification

- Core tests cover text and regex matching, per-rule case sensitivity, array-order priority, invalid regex skipping, and reader non-interference.
- `cargo test -p logmancer-core` verifies core behavior.
- `cargo clippy --workspace -- -D warnings` verifies the workspace remains lint-clean before PR.
