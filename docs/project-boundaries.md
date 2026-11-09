# Vellum project boundaries

Status: accepted migration contract.

Vellum is delivered as four independently releasable projects. Repository
placement must follow dependency direction rather than deployment convenience.

## Projects

| Project | Owns | Must not own |
| --- | --- | --- |
| `vellum` | Closed semantic engine, authoritative source/history state, admission, retained outcomes and bounded instrumentation | UI state, filesystem paths, platform services or connector policy |
| `vellum-sdk` | Stable host API, sessions, capability ports and `vellum-model` | Concrete OS, storage, clipboard or rendering implementations |
| `vellum-adapters` | Qualified interpretations of SDK ports, initially local filesystem, annotation JSON, ICU and macOS clipboard | Semantic authority, application navigation or hidden retry policy |
| `vellum-apps` | TUI and Iced composition roots, bindings, rendering and application lifecycle | Independent source/history authority or reusable adapter policy |

`vellum-editor` is renamed `vellum-model`. The model is UI-independent
interaction state: tabs, cursors, selections, viewport, search, comments,
status and explicit save/close workflows. It is not an application and is not
the authoritative document model.

`workbench` is reserved for a navigable human/agent product that composes
Vellum, Chronos, coordination/chat and orchestration.

## Required dependency direction

```text
vellum-apps ------> vellum-model ------> vellum-sdk ------> vellum-engine
     |                                      ^
     +------------> vellum-adapters --------+
```

Applications are composition roots. They select and inject adapters. The SDK
and model must not select a concrete adapter through a mandatory dependency.
An adapter may depend on SDK port contracts and engine evidence types; neither
the SDK nor engine may depend on the adapter.

## Adapter crates

Use role-bearing names:

- `vellum-adapter-local-fs`
- `vellum-adapter-annotation-json`
- `vellum-adapter-unicode-icu`
- `vellum-adapter-clipboard-macos`

Local filesystem interaction is a supported, shippable adapter. It is not an
engine intrinsic. Agent and harness consumers must be able to substitute an
in-memory, workspace-mediated or remote adapter without simulating paths or OS
effects inside the engine.

## Migration gates

1. **Complete for current adapters:** `vellum-sdk` defines the adapter-neutral synchronous
   resource-effect algebra, annotation-codec algebra, Unicode-provider
   coalgebra, closed failure classes, original write/read completions and
   deferred adapter factories.
   `vellum-adapter-local-fs`, `vellum-adapter-annotation-json` and
   `vellum-adapter-unicode-icu` prove those contracts while retaining their
   qualified native/wire behavior. Future SDK ports must follow the same pattern
   without changing engine semantics.
2. **Complete:** the local session is generic over the SDK
   resource port and accepts its adapter factory at the composition boundary.
   The local-filesystem dependency is test-only conformance wiring. Preserve
   this shape for subsequent ports. Annotation serialization is likewise an
   explicit method-scoped codec interpretation rather than session authority.
3. **Complete:** the shared interaction crate is `vellum-model` and has no
   normal concrete-adapter dependencies. Filesystem and codec implementations
   are injected through SDK-owned erased ports.
4. **Complete:** concrete adapters use role-bearing names and both reference
   applications compose filesystem, annotation and Unicode implementations
   explicitly.
5. Extract SDK, adapters and apps into sibling repositories while preserving
   history. Replace temporary path dependencies with pinned release or source
   identities.
6. Run unit, behavior, conformance, security/privacy and live qualification
   gates in their owning projects. Cross-project tests verify contracts; they
   do not recreate a hidden monolithic workspace.

Physical extraction is not complete while an SDK/model crate imports a
concrete adapter, even if those crates happen to live in separate repositories.
The boundary fitness test admits no normal reverse edges. Any new reverse edge
is a failure.
