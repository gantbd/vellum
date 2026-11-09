# Vellum project boundaries

Status: physically extracted and dependency-closed on 2026-11-09.

Vellum is delivered as four independently releasable projects. Repository
placement follows dependency direction rather than deployment convenience.

| Project | Owns | Must not own |
| --- | --- | --- |
| `vellum` | Closed semantic engine, authoritative source/history state, admission, retained outcomes and bounded instrumentation | UI state, filesystem paths, platform services or connector policy |
| `vellum-sdk` | Stable host API, sessions, capability ports and `vellum-model` | Concrete OS, storage, clipboard or rendering implementations |
| `vellum-adapters` | Qualified interpretations of SDK ports | Semantic authority, application navigation or hidden retry policy |
| `vellum-apps` | TUI and Iced composition roots, bindings, rendering and lifecycle | Independent source/history authority or reusable adapter policy |

```text
vellum-apps ------> vellum-model ------> vellum-sdk ------> vellum-engine
     |                                      ^
     +------------> vellum-adapters --------+
```

Applications are composition roots and inject adapters. The SDK/model cannot
select a concrete adapter through a shipped dependency. An adapter may depend
on SDK ports and engine evidence types; neither the SDK nor engine may depend
on an adapter.

The history-preserving extractions were established from source commit
`920dbfc` and committed as:

- `vellum-sdk` — `8f8d032`
- `vellum-adapters` — `de59f30`
- `vellum-apps` — `e63df9c`

Each sibling has an executable boundary gate. The apps project also owns the
installability policy and finite release blocker ledger. This repository
contains exactly `vellum-engine`; its gate rejects additional workspace members
or any upward Vellum dependency.

`vellum-model` is UI-independent interaction state, not authoritative document
state. Local filesystem behavior is a replaceable adapter, not an engine
intrinsic. `workbench` remains the prospective navigable human/agent product
that composes Vellum, Chronos, chat, and orchestration.
