# Captured verification input retention

The verifier separates durable full-run input snapshots from disposable checker
copies. This is evidence-tool infrastructure, not an engine storage mechanism,
resource profile, capability, UI behavior or execution verb.

## Ownership and publication

Full-run capture explicitly retains its snapshot, even when invoked inside a
checker with disposable-copy settings. External capture remains retained by
default. For a verifier-owned check, the parent creates a private capture scope
and passes its location to the child and descendants. Disposable captures are
created inside that scope, but the owning verifier decides when to release it.

Release follows the complete existing result check: normal process disposition,
expected success/counterexample/witness role, required output and validation
predicate. A valid expected counterexample releases its scope despite a nonzero
child exit. A successful parent also releases copies made by expected failing
descendants. An unexpected process failure or rejected output retains the complete
scope for inspection. A zero exit alone does not authorize reclamation.

Scope release requires the original private ownership object, not a copied object
or a caller-supplied directory. Cleanup uses the recorded creation path and its
device/inode identity; it refuses directories or symlinks already replaced at
validation. A mutated
returned capture path cannot redirect standalone cleanup. Cleanup failure is an
ordinary tool failure and demotes an otherwise valid check; it is not a semantic
counterexample or a passing assurance result.

An explicitly opted-in standalone checker without a parent scope reclaims its
own copies on successful process exit and preserves them on failure. Partial
captures from a handled copy error remain owned and are reclaimed if the checker
ultimately succeeds. This fallback does not substitute for parent ownership of
expected failing descendants.

## Evidence and limits

capture-retention-checks.mjs executes nine standalone lifecycle controls and eight
controls using the actual verifier run function. capture-retention-mutations.mjs
detects nine lifecycle mutations by assertions, with construction/tool failures
kept outside detection. The existing 49 harness freshness/output controls and
63-file mutation-harness review pass. Ten registered acquisition, annotation and
source result checks also pass through the revised run function, with all ten
successful scopes reclaimed. Test-fixture teardown tolerates an already removed
owned path so it cannot mask the assertion that detected premature deletion.

This policy does not collect historical snapshots, impose a disk quota, handle
SIGKILL/power loss, establish atomic filesystem identity or contain hostile native
code. Other tool scratch files and deliberately retained failures/full-run
snapshots have separate lifetimes. Operators still need capacity and retention
management; no numeric product default or physical/global bound follows.
Commands must finish dependent work before reporting final results; scope release
does not terminate descendants or prove the absence of an orphan process.

Verification reports record each check's capture scope and whether it was
released after validation or retained for inspection. These are assurance-tool
artifacts, not runtime MELT/golden/RED/USE populations, editor outcomes or authority.
Storage exhaustion invalidates affected qualification; it cannot be relabeled a
semantic counterexample or inferred pass from unaffected checks. Full frozen
verification and endpoint reconciliation remain required before adoption.
