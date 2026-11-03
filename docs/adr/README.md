# Architecture Decision Records

Read this index at the start of every task. Before proposing or changing behavior
covered by a record, read that ADR in full and follow its relevant relationships.
Do not load every ADR by default.

This index is a routing guide, not a second source of architectural decisions.
The individual ADRs contain the decision details and verification requirements.

## Index

| ADR | Status | Read when working on |
|---|---|---|
| [0001: Root-scoped server file browser](0001-server-file-browser-root-scoped.md) | Accepted | Server file browsing, allowed roots, path validation, symlinks, and safe errors. |
| [0002: Async position-aware search worker](0002-async-position-aware-search-worker.md) | Accepted | Asynchronous search, partial results, search generations, worker concurrency, and lock boundaries. |
| [0003: Core visual rules as line style intent](0003-core-visual-rules-as-line-style.md) | Accepted | Visual rules, whole-line styling, and separation from inline search highlighting. |
| [0004: Core configuration store](0004-core-configuration-store.md) | Accepted | Configuration persistence, `ConfigStore`, and ownership boundaries between core and platform layers. |
| [0005: Registry-level file-open policy](0005-registry-file-open-policy.md) | Accepted | File opening, canonicalization, and authorization through `LogRegistry`, including restoration flows. |
| [0006: Persisted recent files](0006-persisted-recent-files.md) | Accepted | Recent-file history, stable route IDs, persistent versus ephemeral openings, and reader restoration. |
| [0007: Bounded registry reader lifecycle](0007-bounded-registry-reader-lifecycle.md) | Accepted | Reader leases, worker shutdown, eviction, shared SSR cache ownership, and resource limits. |
| [0008: Suite distribution and platform packaging](0008-suite-distribution-and-platform-packaging.md) | Accepted | Launcher behavior, platform selection, distribution, installers, and installed asset resolution. |
| [0009: Content-anchored selection and explicit viewer navigation](0009-content-anchored-selection-and-explicit-viewer-navigation.md) | Accepted | Text selection, explicit navigation, viewport ownership, content extraction, and clipboard integration. |

## Workflow

1. **Locate:** Use the index to identify relevant ADRs, then read their full text
   and any related records needed for the task. If the index is insufficient,
   search this directory rather than assuming no decision applies.
2. **Check:** Compare the proposed change with the accepted decisions. Accepted
   ADRs are current constraints; proposed ADRs are discussion context, not approval
   to implement them. Superseded ADRs explain history rather than current policy.
3. **Propose:** Implementing an existing decision does not require another ADR.
   For a new architectural decision or a conflict with an accepted one, draft a
   new numbered ADR with status `Proposed` and reference the related records.
   Agree on the new decision before implementing conflicting behavior.
4. **Review:** Acceptance requires explicit approval; agents must not approve
   their own proposals. Once approved, update the ADR status and this index.
5. **Maintain:** Add new ADRs and update statuses and relationships in this index
   in the same change as the records. Keep entries short and focused on when to
   read the full document, rather than duplicating its decisions.

When a new ADR supersedes an accepted decision, preserve the old decision text.
Update only its status and relationship metadata to mark it `Superseded` and link
to the replacement with `Superseded by`. Link back from the new ADR with
`Supersedes`, and keep both records discoverable in this index.

## Writing an ADR

- Create an ADR for cross-cutting decisions, responsibility boundaries, shared
  contracts, or trade-offs that are difficult to reverse—not routine implementation
  changes. Keep one coherent decision per record; separate independent decisions.
- Start from [the template](template.md), using the next available number and a
  `NNNN-short-descriptive-title.md` filename. Check for numbering collisions before
  integration. New records start as `Proposed`; `Accepted` means explicitly
  approved, not necessarily implemented.
- Keep `Status`, `Context`, `Decision`, `Consequences`, and `Verification`.
  Explain the problem, constraints, chosen approach, rationale, and trade-offs.
  Verification should describe observable criteria or tests, not an implementation
  task list.
- Add `Alternatives considered`, `Out of scope`, `Related decisions`, or
  `Decision details` only when useful. Include relationship metadata when a record
  supersedes another. Do not reformat historical ADRs just to match the template.
- **Be concise but technically complete.** Include all technical details needed
  to understand, implement, and verify the decision, using as few words as possible.
  Avoid repetition, filler, and implementation details that do not affect the
  architectural decision. Prefer precise statements and tables when they improve
  clarity. Do not sacrifice correctness or necessary context for brevity; there is
  no fixed word limit.
- Remove drafting comments before review. Follow the workflow above for approval,
  relationships, and index updates.
