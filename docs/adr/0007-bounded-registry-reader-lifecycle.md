# ADR 0007: Bounded registry reader lifecycle

## Status

Accepted

## Decision

`LogRegistry` owns the complete lifecycle of its readers. Every reader
operation acquires an internal lease, and removal waits for active leases to
finish before shutting down the reader workers and releasing its resources.

For long-running SSR services, the registry acts as a shared anonymous cache.
On each file-open attempt, it applies a lazy retention policy based on an idle
TTL and a soft budget for in-process index memory. It never evicts a reader
with an active lease.

## Context

The registry currently retains each opened `LogReader` for the process
lifetime. A reader owns a memory map, line-offset index, filter state, search
state, and reload, filter, and search workers. There is no removal operation
or resource bound.

The web service can be used by multiple anonymous clients. Browser tabs cannot
safely own server readers: closing one tab must not release a reader another
client may still use. Browser lifecycle notifications are also best effort.

ADR 0006 preserves stable IDs and canonical paths for recent persistent files.
That history can restore an evicted reader when a subsequent operation uses its
ID, provided the path still exists and passes the registry file-open policy
defined by ADR 0005.

## Decisions

| Area | Decision |
|---|---|
| Lifecycle owner | `LogRegistry` owns reader acquisition, recency, removal, and restoration. API handlers do not manage lifecycle counters. |
| Leased access | Every reader operation acquires a lease before use and releases it on every completion path, including errors. A closing reader rejects new leases. |
| Resource release | Removal waits for active leases, signals all reader workers to stop, joins them, then releases the reader and its memory map and in-memory state. |
| Cache ownership | SSR readers are shared anonymous server-cache entries, not resources owned by browser tabs or clients. There is no public close endpoint in this phase. |
| Retention trigger | Retention runs only when opening a file. No periodic reaper is introduced. Entries can remain beyond their TTL when no later file is opened. |
| Idle retention | Entries older than the configured idle TTL are eligible for removal only when they have no active lease. |
| Memory budget | One configurable, soft index-memory budget governs estimated line-offset, filter, and search-index memory. The mapped-file size is observed separately and is not part of the budget. |
| Eviction policy | When over budget, remove least-recently-used idle readers until an internal target below the configured budget is reached. If every candidate is active, opening succeeds and the budget may remain exceeded until a later opening. |
| Restoration | An evicted persistent reader is restored silently on its next operation when its historical path is still available and authorized. Unavailable or unauthorized paths return an actionable error. |

## Consequences

- A long-running service can reclaim reader resources without relying on
  browser-tab shutdown.
- Active operations cannot be interrupted by TTL or LRU eviction.
- The configured budget is not a hard memory guarantee: it deliberately favors
  successful file opening over evicting active readers or rejecting admission.
- Persisted recent routes continue to work after a reader is evicted or a
  process restarts, subject to ADR 0005 authorization.
- Index-memory metrics and mapped-file-size metrics must remain distinct so
  operators understand what the retention budget controls.

## Out of scope

- Browser-upload cleanup or persistent upload storage.
- A periodic retention process.
- Hard admission rejection or eviction of active readers.
- User authentication, client ownership, or per-user resource quotas.
- Web or Desktop multi-file workspace UI.

## Related work

- #95 centralizes leased reader access.
- #96 implements deterministic inactive-reader release.
- #94 implements lazy idle retention during file opening.

## Verification checklist

- [ ] Reader operations cannot run after their reader begins closing.
- [ ] Leases are released after successful and failed operations.
- [ ] Removal waits for active operations and joins every reader worker.
- [ ] Idle readers are selected by TTL and LRU only when a file is opened.
- [ ] Active readers are never evicted.
- [ ] Index memory, mapped-file size, and eviction outcomes are observable.
- [ ] A valid evicted persistent route restores transparently.
- [ ] Missing or unauthorized persisted routes return an actionable error.
