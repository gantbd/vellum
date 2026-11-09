# Verification input capture and evidence acceptance

Verification now runs from a captured project copy. Previously, some source
files were read and validated before their hashes were recorded, allowing the
recorded bytes to differ from the bytes used by those checks. The launcher now
copies and hashes each input before the worker imports the local case manifests
and validators. Those validators, graph generation, model copies and solver
invocations all consume the captured tree.

Before publishing a current report, the launcher compares the live source
inventory and bytes with the capture, then checks the current inputs against
the worker's report. A live difference invalidates current-evidence status even
if the isolated worker passed. The report records the retained capture location,
source hashes, worker exit status and both comparisons. Capture directories are
retained in task-specific temporary storage for inspection; they are not added
to the runtime or required by editor consumers.

## Input and output boundaries

TLC processes receive the frozen run's owned work directory as `java.io.tmpdir`
at both launch sites. The pinned jar resolves standard modules by extracting
named files into that directory and scheduling them for deletion on JVM exit.
Using one shared system directory permits interference between concurrent runs;
a missing standard module is a tool failure, not a model counterexample. A failed
historical report stays failed and requires a new complete qualification.

The selected regression checks both launch-site arguments, rejects three lexical
omission/substitution controls, and runs two copies of the existing no-op capacity
witness concurrently with distinct owned directories. Both must resolve their
standard modules inside their own directory and satisfy the normal witness exit/
message checks. Successful probe workspaces are released; failed ones remain for
inspection. This does not establish arbitrary toolchain or host isolation.

Inputs are root Markdown documents and supported regular source files directly
inside `formal`, including Python/Java reference-tool sources and unmodified Unicode
conformance/licensing text. The generated `results.json` and hidden entries are excluded.
Unexpected formal entry types fail rather than silently omit a possible input.
The internal capture manifest is checked before local proof modules are imported.
An ordinary direct invocation of the internal worker is refused.

With `--write`, the graph is generated inside the captured tree and copied back
only after a normal worker exit and a matching live capture. Its final digest
is checked against the report like other inputs. Generation-only runs produce
no new proof report. Diagnostic partial runs remain explicitly noncomplete;
an exit status of zero for a partial run is not a full-suite passing claim.

Tool acceptance requires a normal process exit, the correct success/counterexample/
witness exit class, the expected property text and a successful output validator.
Signals, launch/time-limit errors, null exit status and validator exceptions fail
closed. SMT output must match every ordered condition/result exactly; missing,
duplicate, reordered, unknown or diagnostic output cannot pass.

## Controls and bounded closure

`node formal/evidence-harness.mjs --self-test` runs 49 checks covering acceptance,
fault rejection, exact SMT output, source additions/removals, changed live versus
unchanged captured bytes, manifest tampering, a real frozen child, missing and
malformed reports, failed children, guarded worker invocation, partial evidence
and generation-only behavior, including a schema-only request that must not be
classified as generation-only. A complete run now also requires the explicit
jsonschema 4.25.1 Python verifier for the reference annotation schema artifact
and the pinned ICU4J 79.1 verification jar for Unicode 18 conformance/command
checks. Java capture/tamper and Unicode-tool-only nongeneration controls prevent
those inputs or checks from being silently omitted. Native Tree-sitter reference
checks additionally require VELLUM_TREE_SITTER_PYTHON with the exact environment/
library/query fingerprints in tree-sitter-reference.json. Python source capture/
tamper and parser-tool-only nongeneration controls prevent silent omission.
Installed library/query fingerprints are checked before and after each native
worker, with a deliberate mismatch control. These external libraries are not
frozen into the captured tree or independently certified. No engine dependency follows.
Inconsistent completion metadata, missing selection,
empty/failed check lists and partial runs claiming full success are rejected or
kept explicitly noncomplete. The full verifier records this as one infrastructure
check, separate from the 191 SMT conditions and editor model properties.

The following infrastructure work is implemented and tested within those bounds:

- Verification data and local proof modules come from the captured tree rather
  than being reread from live files after validation.
- Missing/malformed worker evidence and abnormal tool completion cannot publish
  a passing report through the tested acceptance paths.
- Full verification, partial diagnostics and generation retain distinct statuses.

A whole-suite live-edit control on 2026-10-24 also exercised the launcher beyond
the small fixtures. Its captured worker passed all 287 checks, while seven live
source files changed during the run. The launcher exited unsuccessfully and set
both current-evidence comparisons and the outer pass flag to false. The captured
worker used the preceding 35-control harness; the subsequent settled-input run
uses the final 42-control harness. This is an observed freshness control, not an
additional editor theorem or a current passing run.

The diagnostic parent report is retained as `live-control-results.json` under
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-Bwz97I`,
alongside its captured inputs and worker report. Its SHA-256 is
`755a3c30ad94b044f18e1049ba0fb1baf71cf52f7f517a5688e8d21f4ea232aa`.
Temporary retention is not a permanent evidence-archive guarantee.

This closes a concrete freshness/acceptance subset of earlier R-09 assurance work,
not general certificate/induction assurance or the previous full specification
goal. The user's scope includes other closable earlier residuals; that work remains
in scope alongside the pre-specification assurance pass. Existing R-01–R-12 gates
are not waived, and R-13 remains a v1 exclusion rather than future-feature work.

## Assurance limits

The capture is a sequence of file reads, not an atomic filesystem snapshot.
The checks establish equality at sampled boundaries, not that live files never
changed and changed back. Such an ABA change does not alter the isolated bytes
actually verified, but no source-history or concurrent-publication theorem follows.
Later edits can stale a report; consumers must recheck the inventory and hashes.

The captured directory is not a security sandbox or protection from a malicious
trusted host. The bootstrap, Node/OS, external tools and environment remain trusted.
Tool installations and external libraries are not copied or frozen. Existing
version/hash checks remain conditional evidence; toolchain certification, proof
objects and independent kernel replay remain separate gates. The fixtures test
the harness, not native providers, production editor behavior or physical durability.
