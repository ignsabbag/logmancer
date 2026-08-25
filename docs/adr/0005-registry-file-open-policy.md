# ADR 0005: Registry-level file-open policy

## Status

Accepted

## Decision

`LogRegistry` accepts an optional `FileOpenPolicy` and validates every file
open through that policy before creating a `LogReader`.

The policy receives an input path and returns the authorized, canonical path
that the reader consumes. A rejected path prevents reader creation.

Registries without a policy preserve the current unrestricted native-file
behavior.

## Context

SSR server-file browsing already constrains paths to
`LOGMANCER_SERVER_FILE_ROOT`. That validation currently lives in individual
HTTP handlers.

Future persisted recent-file history must restore paths without bypassing the
active authorization boundary. Applying authorization at the registry is the
only way to ensure every caller of `open_file` follows the same rule.

This ADR complements ADR 0001. ADR 0001 defines the SSR server-browser
capability; this ADR defines the registry-level enforcement point used by that
capability and future restoration flows.

## Decisions

| Area | Decision |
|---|---|
| Enforcement point | `LogRegistry` applies the policy before every reader is created. |
| Policy contract | The policy validates and canonicalizes a path, returning the path that may be opened. |
| Policy ownership | Core defines the policy contract; platform layers construct policies from deployment-specific authorization rules. |
| SSR authorization | SSR supplies roots authorized by the deployment, including the configured server root. |
| Current uploads | SSR also allows direct children of the system temporary directory whose names start with `logmancer-upload-`, preserving the current upload flow. |
| Future uploads | The temporary upload exception will be replaced with a persistent Logmancer-managed upload directory when uploads are implemented. |
| HTTP behavior | Handlers retain request validation, text-readability checks, and HTTP error mapping. |

## Consequences

- Server-browser openings, future history restoration, and other registry
  callers cannot bypass an installed policy.
- The core does not depend on `LOGMANCER_SERVER_FILE_ROOT`, HTTP, or SSR.
- Desktop and TUI can use the registry without a policy.
- Existing SSR root validation remains useful for safe request-specific errors,
  while the registry provides the final authorization boundary.

## Out of scope

- Persisted recent-file history and stable route IDs.
- Persistent upload storage and cleanup.
- Reader close or eviction APIs.
- User authentication and authorization.

## Verification checklist

- [ ] A registry with a policy opens only the canonical path returned by that
  policy.
- [ ] A rejected policy prevents reader creation.
- [ ] A registry without a policy continues to open native files as before.
- [ ] SSR server-root restrictions remain enforced.
- [ ] Future path restoration uses the registry and cannot bypass an installed
  policy.
