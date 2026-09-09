# ADR 0001: Begin as a modular monolith

- Status: Accepted
- Date: 2026-09-09

## Context

Reel needs to coordinate several external systems, but its first milestone is a
single movie playback and acquisition slice. Splitting domain, application,
transport, and integration code into workspace crates now would create public
interfaces before the behavior and vocabulary are stable.

## Decision

Keep backend behavior in feature-oriented modules within the `reel-api` crate.
Keep HTTP handlers thin and place product behavior in deep modules. Extract a
workspace crate only when demonstrated reuse, isolation, or build constraints
justify a real seam.

## Consequences

- The first vertical slice remains local and inexpensive to refactor.
- Module interfaces can emerge from working behavior.
- Compile-time enforcement between architectural areas is deferred.
- A future extraction must preserve the domain language rather than mirror
  external system endpoints.
