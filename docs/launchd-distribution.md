# macOS launchd distribution profile

Status: inert LaunchAgent renderer and adversarial qualification implemented;
installation and activation remain unauthorized.

The GitHub release-candidate payload includes `vellum-launchd-render`, a
self-contained Rust plist renderer. It has no Node or scripting-runtime
dependency. It never writes a plist, changes a mode, installs a binary, or
invokes `launchctl`. It validates one exact deployment record and emits XML to
standard output. Review and installation are separate operator actions outside
Vellum's current authority.

The record names the label, daemon binary, effective owner UID, private state
directory, data socket, control socket, persistent lock, workspace identity
metadata, and stdout/stderr logs. All paths are normalized and absolute. The
state directory must be a real same-owner `0700` directory; the binary must be
a real executable owned by root or that owner and not group/other writable.
The state directory, daemon binary, and configuration file must equal their
canonical real paths, so aliases through a symbolic-link ancestor refuse even
when the final component is not itself a link. Existing private files must be real same-owner files with no group/other
permissions. Socket paths must be absent for this new-activation check. The
fixed lock and metadata names match the daemon's current private persistence
contract. Symlinks, writable inputs, aliases, escapes, unresolved placeholders,
unknown fields, malformed JSON, and owner mismatch fail closed.

Run the renderer as the intended LaunchAgent user:

```sh
vellum-launchd-render --config=/absolute/path/deployment.json > candidate.plist
plutil -lint candidate.plist
```

The emitted `ProgramArguments` runs the daemon directly with only `--socket`
and `--control`; therefore its authority ceiling is read-only. It uses
`RunAtLoad`, disables `KeepAlive`, and sends logs to the two explicit private
paths under a `077` umask. It contains no `Sockets` dictionary: this is not launchd socket
activation. It contains no health endpoint or authentication claim.

Release-candidate qualification executes the exact packaged renderer twice:
once with the selected candidate daemon and once with the distinct selected
rollback daemon. Each run uses a fresh same-owner `0700` scratch state and a
create-exclusive `0600` deployment record, must reproduce the exact inert
profile, and must pass `/usr/bin/plutil -lint`. The retained evidence contains
only the closed invocation class, exit code and lint/profile dispositions; it
contains no deployment path, plist, log path or configuration contents. A
renderer whose checksum is internally consistent but whose behavior emits a
different or malformed profile is refused before evidence publication.

The local non-activating activation-transaction model stages each candidate and prior
daemon/MCP/plist triple as one immutable digest-named version and atomically
replaces a content-safe desired-deployment pointer. A synchronized journal
distinguishes prepared, candidate, rollback, terminal, refused, and unknown
cuts, including crashes between pointer and journal publication. This closes a
local desired-state and recovery algebra only. Launchd does not consume that
pointer, so pointer replacement is not relabeled as job activation, process
replacement, or rollback. The scratch qualifier executes neither staged binary
and never invokes `launchctl`. The separate
[authorized activation executor](launchd-activation.md) uses the same exact
qualification and approval identities but adds durable pre/post launchctl
brackets, exact running-program/PID checks, bounded daemon status, and
exact-prior rollback after known candidate health failure. It is opt-in source
tooling, not a Cargo-install or candidate-payload binary; its tests inject all
effects and never invoke launchctl.

For an authorized first installation, create and verify the private directory,
render and inspect the plist, place the exact qualified candidate and plist,
then activate through the separately reviewed executor. Its status gate invokes
the exact selected daemon's `--status` command and requires the version-2
live/running/healthy/ready product; a startup line alone is not readiness.

For an authorized upgrade, retain a distinct qualified rollback candidate,
request shutdown over the current control socket (or let launchd send
`SIGTERM`), wait for `vellum-daemon stopped` and successful process exit, verify
both socket paths are absent, atomically replace the binary and plist, and only
then start the new process. Do not delete the persistent lock or identity
metadata. If readiness or the read-only smoke fails, stop the candidate,
restore the selected prior binary and plist atomically, and repeat the same
checks. A crash may leave exact recorded residue for the daemon's conservative
recovery; conflicting evidence is an operator-visible failed start, never a
reason for an `rm` preflight.

launchd may escalate termination beyond `SIGTERM`; Vellum cannot promise that
an external supervisor will wait forever for a continuously progressing peer.
Deployment-specific throttling, log retention, resource limits, plist
destination, codesigning and notarization remain outside this profile. The
executor fixes the effective user's `gui/<uid>` domain and one exact label;
other domains require a new reviewed policy.
