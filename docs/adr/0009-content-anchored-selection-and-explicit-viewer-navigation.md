# ADR 0009: Content-anchored selection and explicit viewer navigation

## Status

Accepted

## Decision

Each viewer instance owns its logical text selection in client-side document
state. Selection identifies original file content, independently of the current
viewport and rendered rows. Native DOM selection is an input adapter or a visual
projection, never the sole source of truth.

Active line, text selection, current search match, and each panel's viewport are
separate concepts. Revealing content is an explicit, one-shot navigation command,
not a persistent consequence of keeping that content selected.

Core provides UI-independent selection types and pure range operations, plus
bounded content extraction. Clients own selection instances, interaction state,
rendering, and clipboard integration. This keeps the model reusable by a future
TUI implementation without adding TUI interactions in this work.

## Context

The shared Web/Desktop viewer renders a viewport-sized page rather than the
entire file. Several issues expose missing boundaries between content identity,
selection, and navigation:

- #138 reports that existing text selections disappear or appear attached to
  screen rows after scrolling. `SelectionContext` currently stores only an
  original line and its source; it cannot represent a text range. Page rendering
  in `content_lines.rs` rebuilds the displayed content without a persistent
  logical text-selection model.
- #133 reports repeated navigation to an unchanged filtered selection. The reveal
  effect in `main_pane.rs` tracks both selection and page size, so resizing can
  reapply navigation without a new selection action.
- #132 reports search navigation losing the current page's origin. Search requests
  have no explicit origin, and both main and filtered reads mutate
  `LogReader::current_view_start`. Search batch merging also sorts matches while
  retaining `current_ordinal`, which can then identify a different match.
- #131 defines active-line copying, single-line fragment selection, and complete
  line selection when dragging across rows. #134 extends that interaction across
  virtual pages using wheel scrolling and drag-edge autoscroll.

These are reported behaviors and verified code mechanisms, not a claim that all
reported interactions have been reproduced or share one root cause.

Copying cannot be reconstructed faithfully from presentation pages alone:
`FileReadOps::read_line` currently limits a line to 10 KiB and applies
`trim_end()`, removing trailing whitespace. Offscreen content may also never have
been rendered during a drag.

This ADR complements [ADR 0002](0002-async-position-aware-search-worker.md), keeping
its asynchronous circular search order and stale-generation rejection. It does
not supersede that decision or the registry policies in
[ADR 0005](0005-registry-file-open-policy.md) and
[ADR 0007](0007-bounded-registry-reader-lifecycle.md).

## Ownership and model

| Concern | Owner and responsibility |
|---|---|
| Selection instance | Client-side state for one viewer/document instance; initially hosted by `LogView` in Web/Desktop. Two viewers of the same file may have different selections. |
| Selection rules | UI-independent types and pure operations in `logmancer-core`: positions, directional ranges, normalization, and expansion to whole lines. |
| Interaction controller | Client-owned transitions for beginning, extending, finishing, clearing, and invalidating selection; adapters translate UI events into those actions. |
| Rendered rows | Project the logical selection onto visible content; do not own its lifetime. |
| Content access and validity | Core resolves ranges against file content and validates their content identity. `LogReader` and `LogRegistry` do not own user text selection. |
| Clipboard | Platform adapter receives extracted text and handles platform permissions and errors. |

The existing selected-line state becomes an explicitly named active-line concept.
It can remain shared between the main and filtered panels. Its source records
provenance, not an instruction to keep scrolling to it. Search-origin activation
must be distinguishable from a direct main-panel action.

The logical text-selection model contains the following information; names and
concrete Rust/API representations remain implementation details:

```text
TextPosition
  original line index, zero-based
  byte offset within line content, at a valid UTF-8 boundary

TextSelection
  content identity
  anchor position
  focus position
  mode: ExactText | WholeLines

Client selection state
  optional TextSelection
  selection revision

Client interaction state
  Idle | Dragging(pointer identity, pointer position, ...)
```

The anchor stays fixed while extending a drag; the focus can move in either
direction. The ordered effective range is derived rather than maintained as a
second mutable range. Exact-text ranges may span multiple lines, allowing #138
to preserve existing behavior before #131 introduces whole-line expansion.

Selection updates are coherent transitions, not unrelated writes of anchor,
focus, and mode. Finishing a drag clears transient interaction state but retains
the selection. A selection revision identifies changes to the selected range;
it is distinct from content validity, search generation, and UI refresh counters.

Model positions are not screen columns. Web adapters convert DOM UTF-16 offsets,
including text split by search decorations, to logical positions. A future TUI
adapter can map terminal cells, tabs, and Unicode display widths to the same
model. No shared type depends on Leptos, DOM nodes, or terminal coordinates.

Changing the opened document clears its selection and cancels its outstanding
interactions and copy requests, even if the UI component itself remains mounted.

## Navigation and search

- Selecting a filtered result updates the active line and emits a main-viewer
  reveal command. Resizing, refreshing, or retaining the highlight does not emit
  that command again. Selecting the same result explicitly may issue a new reveal.
- Viewports are independent per panel. Manual scrolling changes a viewport,
  neither clearing a text selection nor selecting the replacement screen rows.
- Navigation commands carry enough viewer/document and request identity to be
  consumed once. Obsolete responses and pending scroll callbacks must not
  overwrite a newer navigation decision.
- Search submission captures the first line of the main page actually displayed,
  not a filtered-page offset or an unfulfilled scroll target, and passes that
  origin explicitly to core. Core retains ADR 0002's origin-to-EOF, then
  beginning-to-origin scan order.
- A search match is identified by search generation and original position
  `(line, start, end)`. Its ordinal is derived display/navigation metadata.
  Merging batches preserves the selected match and recalculates its ordinal;
  earlier matches do not replace it or trigger another reveal.
- An initial match arriving asynchronously may fulfill a still-pending search
  reveal once. Subsequent manual scrolling or a newer navigation supersedes that
  reveal without cancelling search indexing.

These rules do not redesign the ownership of existing server-side search
sessions. They remove implicit viewport dependencies and unstable match identity.

## Content lifetime and copying

File ID alone is not sufficient content identity: the same route can refer to a
replaced file. Pages and extraction requests need a content-validity contract
that can be checked by core, including after registry eviction and restoration.

An append preserves a selection only when it leaves the selected content
unchanged. Truncation or replacement invalidates the selection with a visible
explanation rather than silently associating it with different text. An append
to an incomplete final line requires validation rather than assuming its whole
line selection is unaffected. If validity cannot be established, extraction
fails visibly rather than returning unverified content.

Copying uses a dedicated range-extraction operation, conceptually:

```text
read_selection(content_identity, effective_range, limits) -> text | error
```

- Extraction reads the selected source range, including offscreen lines, without
  requiring visited pages to be cached. Selecting lines 100 through 350 still
  copies that range when only lines 320 through 360 are visible.
- It does not recenter the range like a presentation page or change viewport,
  active line, search origin, or tail/follow state.
- Exact-text selection preserves its endpoints. Whole-line extraction includes
  complete source lines and their existing terminators; it does not invent a
  final newline when the file has none. Numbers and decorations are excluded.
- Extraction does not apply presentation trimming or per-line truncation.
  Whitespace and included line separators are preserved.
- Client copy precedence in the main viewer is text selection first, then the
  active line when #131 is implemented. Editable fields and selection in other
  controls retain their normal copy behavior.
- Requests have server-enforced byte and work bounds, including for a single
  oversized line. Exceeding a bound returns an explicit error, never a silently
  truncated result. Unbounded clipboard streaming is not required.
- A copy request captures an immutable content identity, effective range, and
  selection revision. Document changes, selection changes, or cancellation make
  it obsolete; scrolling alone does not. Preparation/loading and failures are
  visible. Clipboard delivery happens only after successful complete extraction;
  partial extraction must not overwrite the existing clipboard.
- Core content extraction is shared. Web/Desktop expose it through their adapter;
  a future TUI can call it directly and supply its own clipboard adapter.

Registry-backed extraction uses leased reader access and existing authorization
and restoration error handling. It must not create an alternate path-opening
mechanism. A lease prevents reader eviction during the operation; it does not
freeze external file writes. Extraction must not mix incompatible content
versions or claim validity based solely on a lease.

Clipboard integration must account for browser/WebView user activation and
permissions after asynchronous extraction. If direct delivery is unavailable,
provide an explicit action to copy prepared text rather than assuming delayed
`Ctrl+C` will succeed.

## Dragging across virtual pages

The controller captures the pointer on a stable container. Wheel scrolling and
bounded edge autoscroll update the viewport and selection focus while preserving
the original anchor. Page replacement does not terminate the interaction;
reversing direction can shrink or extend the effective range.

Beginning a selection drag pauses automatic viewport movement from tail/follow.
Finishing the drag does not automatically resume it. At EOF, selection scrolling
must not re-enable follow through the normal wheel-at-end behavior.

Autoscroll respects available file boundaries, bounds outstanding page work, and
stops on release, cancellation, lost capture, document change, or teardown.
Stale page responses cannot reset the anchor or overwrite a newer focus. These
are client interaction responsibilities, not fields in the shared range model.

## Delivery order and issue dependencies

Tracking epic: [#139](https://github.com/ignsabbag/logmancer/issues/139).

Recommended integration order is #133, #132, #138, #131, then #134. The first three
can proceed independently under the contracts in this ADR; this ordering is not
a chain of technical blockers.

| Issue | Deliverable | Hard dependency within this work |
|---|---|---|
| #133 | One-shot filter-origin reveal and independent subsequent main scrolling. | None. |
| #132 | Explicit search origin and stable match identity across batch merges. | None. |
| #138 | Content-anchored text-selection foundation, visible projection, and offscreen copy of an established selection. | None. |
| #131 | Active-line fallback copy and whole-line multi-row selection using that foundation. | #138. |
| #134 | Wheel-during-drag and edge autoscroll across pages, reusing selection and extraction. | #131; #138 is transitive. |

The copy contract is a shared foundation, not a separate implementation per
issue. #138 must not depend on implementing #131's new interaction semantics.
Related issues reference the ADR; native GitHub blockers represent only the hard
dependencies above. Acceptance of this ADR does not itself approve implementation
of the child issues. Preserve compatibility with selected-text occurrence search
tracked in #73 without implementing it as part of this work.

## Alternatives considered

- **Native DOM selection as the owner:** cannot represent unrendered content or
  survive page replacement reliably.
- **Selection stored in a shared reader:** couples independent viewers and
  anonymous clients to one user's interaction, contrary to ADR 0007's cache model.
- **Cache every visited page for copy:** can miss skipped content, retains
  presentation truncation, and grows with the drag instead of an explicit bound.
- **Continuously derive navigation from selected state:** repeats reveals on
  unrelated reactive updates and prevents independent scrolling.
- **Implement everything in #134:** makes a new interaction feature responsible
  for repairing existing bugs and encourages duplicate selection/copy paths.

## Consequences

- Selection survives virtualization, while navigation remains under user control.
- Pure model rules and source extraction are reusable without sharing client
  state or imposing a Web-specific representation on TUI.
- Content validity, asynchronous request identity, and bounded extraction become
  explicit contracts rather than accidental properties of rendered pages.
- Existing component contexts, API adapters, and affected core callers need
  focused migrations; changes can land in independently verifiable issues.
- Clipboard support remains platform-dependent and requires runtime validation,
  not just model tests.

## Details to settle during implementation design

- Content identity and validation strategy, including replacement, reader
  restoration, external writes, and incomplete final lines. Size/mtime alone
  must not be presented as proof that selected bytes are unchanged.
- Concrete extraction byte/work limits and how loading, cancellation, and errors
  are presented. These must be specified before shipping the extraction path.
- Endpoint boundary conventions and UI behavior when a multi-line drag returns
  to its anchor line, without duplicating derived range state.
- Mapping displayed text to source positions for clipped lines or invalid UTF-8,
  so copied content and visual feedback do not silently disagree.

## Out of scope

- Implementing TUI selection, terminal mouse handling, or its clipboard transport.
- Advanced text-selection behavior in the filtered panel.
- Selected-text occurrence search implementation (#71/#73).
- Multi-client search-session isolation, persisted selections, or new tab support.
- Unbounded copy/export, arbitrary external-file mutation support, or a general
  editor/undo framework.

## Verification checklist

- [ ] Pure range tests cover direction reversal, normalization, fragment versus
  whole-line extraction boundaries, and Unicode-safe positions.
- [ ] Single-line fragments and multi-line selections remain attached to original
  content when scrolling away and back, including copy while offscreen.
- [ ] Empty lines, tabs, trailing whitespace, CRLF/LF, missing final newline,
  decorated text, and oversized lines have explicit regression coverage.
- [ ] Filter selection reveals once; manual scrolling, refresh, and resize do not
  repeat that reveal, while a new explicit selection still navigates.
- [ ] Search tests cover explicit main-page origin despite filtered reads,
  circular wrap, stable identity/ordinal after merges, and superseded late reveals.
- [ ] Extraction preserves source content without mutating navigation state;
  stale content, failed reads, and resource limits produce no partial copy.
- [ ] Append validity, truncation/replacement invalidation, and reader restoration
  cannot silently redirect a selection to different content.
- [ ] Wheel/edge scrolling preserves the anchor, respects boundaries, stops on
  every termination path, and does not resume follow or accumulate unbounded work.
- [ ] Out-of-order responses, pending scroll callbacks, document switches, and
  obsolete copy requests cannot overwrite newer client decisions.
- [ ] Browser and Desktop checks cover pointer capture, DOM offset conversion,
  editable-control copy, clipboard permissions, and asynchronous delivery fallback.
