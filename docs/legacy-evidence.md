# Legacy verification evidence

Vellum's pre-public local history contained 21 byte-exact verification input
and result archives under `.verification-evidence.*`. They were removed from
the public history before its first push because they duplicated active source
across 20,199 paths, consumed approximately 478 MiB in a checkout, and retained
machine-local and temporary filesystem paths.

The archives are historical forensic material, not active build inputs. Their
own manifests explicitly limit their claims: they are not signed attestations,
new verifier executions, or independent certification. Active formal models,
checks, fixtures, and release gates remain in the ordinary source tree.

## Private recovery artifacts

The exact pre-rewrite state is retained outside this repository in two
operator-private artifacts:

- `vellum-pre-public-history-2026-11-16.bundle` — complete Git history and refs
  before cleanup; SHA-256
  `974dc87aa9513ff4b9a59834c2fc0ac80eebc06ed6df77166264884280df440a`.
- `vellum-verification-evidence-2026-11-16.tar.gz` — the 21 expanded evidence
  archives; SHA-256
  `417615baae0f4e23993cc523e11466af52c52843bb4a23d680c1f3a9adf741ff`.

The original baseline commit was
`b15ae760da133fe72cccfeed4c10325b92369f45`. That identifier belongs to the
private pre-rewrite history and is deliberately not a public source revision.

## Archive inventory

```text
.verification-evidence.19ByYp
.verification-evidence.3Yr0Kv
.verification-evidence.3kalk5
.verification-evidence.5uR1DG
.verification-evidence.6JQNIW
.verification-evidence.6Kxh1R
.verification-evidence.CekvNe
.verification-evidence.JNcp6W
.verification-evidence.RchZuq
.verification-evidence.SEONBp
.verification-evidence.SnMl5y
.verification-evidence.XyK7Im
.verification-evidence.YovcKv
.verification-evidence.bsEkyY
.verification-evidence.ctXCEL
.verification-evidence.czN3Bl
.verification-evidence.khzZFN
.verification-evidence.nVriKr
.verification-evidence.pEYfnW
.verification-evidence.shfO7F
.verification-evidence.xUnL2d
```

The historical specification-closure ledger directly cited `3Yr0Kv`,
`5uR1DG`, `CekvNe`, `RchZuq`, `SEONBp`, `XyK7Im`, `YovcKv`, `pEYfnW`, and
`shfO7F`. Those citations identify entries in this private inventory; they do
not make the removed snapshots part of the current release or source archive.

Future qualification should retain compact, commit-bound summaries in the
repository and place any complete byte-exact execution archive in separately
governed artifact storage.

## Downstream pin migration

Removing the archives from every public commit changed all descendant commit
identities. Coordinated peer repositories must replace these known Vellum pins:

| Pre-cleanup revision | Rewritten revision | Known consumer |
| --- | --- | --- |
| `f4a53ba664acf2a12b6fadfd7d378eef63307c39` | `7fbb70f88d6cd54f834d301923fb4bf623e7f87d` | `vellum-gui-macos` CI |
| `d7de5f2587b3299983792c95f83fcff39ca54d0c` | `b0538a05401974655ac5d628c2dd28773c64e7c6` | `vellum-mcp` |
| `82b171b0fd11fe7511e489cf96cc0c630ffa958c` | `23d763cd08a8343094a9922eba71aacaa4b0b0db` | `vellum-semantic-mcp` |
| `2edb0d475b44d4e3c635eee899bc8684da4ee3b1` | `76b96d2d5f8f08d6612fb0924e7109edf070de4a` | `vellum-tui` CI and workspace coordination |

These mappings identify equivalent cleaned commits; they do not authorize a
peer release or replace that peer's own source-contract checks.
