# ADR 0002: Async Position-Aware Search Worker

- Status: Accepted
- Date: 2026-05-17

## Context

Synchronous `apply_search` scanned all indexed lines under write lock, which blocked search start and delayed navigation on large files.

## Decision

1. Search execution uses an event-driven worker via `crossbeam_channel`.
2. `apply_search` initializes search state in `Indexing` phase, then performs a bounded initial wait (up to 500ms) for early worker progress before returning.
3. Worker scans in circular order: `origin..EOF` then `0..origin`.
4. Search state exposes partial vs final totals (`total_matches_final`) and readiness (`is_ready`/phase).
5. Stale generations are rejected on merge.
6. Batch computation happens outside write lock; write lock is used only for bounded merge mutations.
7. `index_filter` adopts the same short-lock batch merge pattern.

## Consequences

- Search start is responsive even on large inputs.
- Clients can poll search status and navigate when first/current match appears, before full completion.
- Additional state complexity is introduced (generation + phase), but correctness under concurrent starts improves.

## Verification

- Unit/integration tests cover async non-blocking start, circular scan completion semantics, and stale generation rejection.
- Quality gates run with `cargo fmt` and `cargo clippy --workspace -- -D warnings`.
