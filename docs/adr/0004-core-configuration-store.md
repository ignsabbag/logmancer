# ADR 0004: Core configuration store

## Status

Accepted

## Decision

Core owns persisted configuration files through `ConfigStore`.

Web and desktop resolve only the platform-specific configuration directory.
`ConfigStore` receives that directory and owns configuration document names,
native filesystem operations, atomic writes, locks, backups, and recovery.

`LogRegistry` receives a `ConfigStore` and creates its internal
`VisualRulesManager` from the visual-rules store supplied by that configuration
store. The registry exposes semantic visual-rules operations without exposing
the manager itself.

## Context

Visual-rules persistence currently receives a path assembled separately by web
and desktop. This distributes configuration-file naming and native persistence
concerns across platform layers.

`LogRegistry` currently receives a pre-built `VisualRulesManager`, making its
lifecycle external and requiring runtimes to carry registry and manager as
parallel dependencies.

Future persisted recent-file history will use another JSON document in the
same configuration directory. It has different domain behavior from visual
rules, but should use the same core-owned persistence infrastructure.

## Decisions

| Area | Decision |
|---|---|
| Configuration directory | Web and desktop resolve the platform-specific directory. |
| Configuration files | `ConfigStore` owns file names and native persistence infrastructure. |
| File names | Configuration document names are private core details, including `visual-rules.json` and the future `recent-files.json`. |
| Visual rules | `LogRegistry` creates its `VisualRulesManager` from `ConfigStore`. |
| Registry API | The registry exposes semantic visual-rules operations rather than its internal manager. |
| Document semantics | Each manager owns validation, revisions, limits, and lifecycle behavior for its document. |

## Consequences

- Web and desktop no longer assemble individual configuration file paths.
- One registry owns the one visual-rules manager used by all of its readers.
- Visual rules and recent-file history can share persistence infrastructure
  without sharing domain semantics.
- Tests can inject document-specific stores without depending on platform
  configuration directories.

## Out of scope

- Persisted recent-file history and stable route IDs.
- Persistent upload storage and cleanup.
- File-open authorization policies.

## Verification checklist

- [ ] Web and desktop resolve configuration directories without naming
  individual configuration files.
- [ ] Registry readers and visual-rules operations use one internal manager.
- [ ] Configuration document names are defined only in core.
- [ ] Visual-rules persistence retains its current atomic-write, conflict, and
  recovery behavior.
