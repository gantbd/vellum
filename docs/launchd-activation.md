# Authorized macOS launchd activation

Status: explicit local executor implemented; no activation is performed by
builds, tests, release workflows, Cargo installation, or candidate
qualification.

`vellum-launchd-activate` is a separate distribution package. It is not one of
the two binaries installed by the `vellum` Cargo-install package. The GitHub
release-candidate payload now checksum-closes the exact native executable and
qualification runs only its `--help` path, recording `launchdEffect:
not-attempted`. An operator must still create an exact private configuration,
render the required exact inert plist or plists, and create a separate
affirmative approval before the executable can enter its effect path. Fresh
installation uses the verified install receipt; only upgrade retains distinct
qualified candidate and prior inputs.

The two activating invocations are explicit and use different closed products.
For a fresh label with no positively observed job:

```sh
vellum-launchd-activate --execute-authorized-first-install \
  --config=/absolute/private/first-install.json
```

First installation consumes a `vellum-launchd-first-install` configuration,
the exact canonical `0400` `local-release-candidate-installation` receipt
written by the verified bundle installer, and a separate
`local-first-install-approval` whose requested action is exactly
`install-candidate-as-new-launchagent`. It binds only the installed candidate;
there is no qualification-time rollback prerequisite, fictitious prior
identity, or rollback claim. A positively observed
existing job refuses before effects. Otherwise candidate bootstrap is durably
bracketed, followed by exact program/PID and bounded daemon readiness checks.
A confirmed but unhealthy candidate is booted out through a separately
bracketed effect. Confirmed cleanup yields `installation_refused`; any
unconfirmed bootstrap or cleanup yields `outcome_unknown` and is never retried.

For an upgrade from one exact healthy prior version:

```sh
cargo run --locked --offline -p vellum-launchd-activate -- \
  --execute-authorized-activation \
  --config=/absolute/private/activation.json
```

Omitting the execution switch, mixing first-install and upgrade configuration
channels, using a relative or aliased configuration,
adding an argument, running on a non-macOS host, or changing the file owner or
`0600` mode refuses before `launchctl`.

## Inputs and authority

The closed upgrade configuration names:

- the exact `local-release-candidate-qualification` evidence created by
  `qualify-release-candidate.mjs`;
- the exact canonical `0600` schema-1 activation approval already used by the
  scratch transaction model;
- one empty private `0700` transaction directory;
- the effective-user LaunchAgent label and owner;
- the daemon state, data/control socket, and log paths;
- distinct candidate and prior directories plus their separately rendered
  `0600` plists; and
- positive bounded effect timeout and health-attempt limits.

The approval must affirm
`select-candidate-for-external-activation`, bind one nonzero transaction id,
bind the evidence's distinct candidate/prior manifest digests, and set
`activationAuthorized` to true. Qualification evidence deliberately retains
`activationAuthorized: false`: qualification selects exact eligible inputs but
cannot authorize their use.

The closed first-install configuration instead names the installer receipt,
the installed `bin` directory as its candidate directory, and a separately
rendered exact `0600` plist. The receipt must be canonical, same-owner `0400`,
have schema 2 and channel `local-release-candidate-installation`, retain the
exact closed installer claims and nonclaims, state `activationPerformed:
false`, and close exactly `bin/vellum-daemon`. The
executor requires the receipt's parent to contain only that receipt and the
private `0700` `bin` directory, requires `bin` to contain only that same-owner
`0500` product, and rechecks its receipt byte count and SHA-256
digests. The affirmative first-install approval binds the receipt's manifest
digest. Thus bundle verification and local installation still grant no
launchd authority; the separate approval does.

Both version directories must be canonical same-owner `0700` directories.
The executor hashes each `vellum-daemon` and compares it with the corresponding
qualification evidence. Each plist must exactly equal the inert direct-exec
profile for that exact daemon, label, sockets, and logs: read-only daemon
arguments, `RunAtLoad`, no keepalive, no socket activation, and a `077` umask.
The MCP payload remains part of candidate qualification but is never started by
the activator.

## Transaction and rollback

Before mutation, the executor requires the current launchd job to report a
nonzero PID and the exact prior daemon path, then requires the daemon status
command to report version 2, live/running, healthy, and ready. It performs the
following bounded sequence:

1. boot out the exact prior label;
2. bootstrap the candidate plist;
3. observe the exact candidate program and nonzero PID;
4. obtain bounded healthy/ready daemon status;
5. if candidate identity or health fails, boot it out, bootstrap the exact
   prior plist, and requalify the prior identity and status.

Every `bootout` and `bootstrap` has one create-exclusive, synchronized,
content-safe journal record before invocation and another synchronized record
after its disposition. The records contain transaction/manifest identities,
sequence, phase, effect class and closed disposition, but no paths, source, log
contents or process output. If invocation may have occurred without a matching
confirmation—timeout, output overflow, wait failure, or nonzero launchctl
completion—the result is `outcome_unknown`. The executor performs no automatic
retry or rollback from that state. Known candidate refusal or failed health may
enter the separately bracketed rollback path; rollback uncertainty remains
unknown rather than false restoration.

First-install approval, receipt and journal schemas are distinct from these
upgrade products. Its journal admits only prepared, candidate-load,
candidate-unload and terminal candidate-health/cleanup transitions. It can
neither unload nor restore a prior job.

The terminal receipt is one of `candidate_active`, `prior_restored`,
`activation_refused`, `rollback_refused`, or `outcome_unknown`. Candidate-active
and prior-restored receipts additionally state exact program/PID and
running/healthy/ready status. They are operational evidence for this one local
transaction, not a signature, attestation, remote principal decision, future
availability promise, or durable audit service.

## Effect isolation and qualification

The transaction engine depends only on `LaunchdActivationPortV1`. Deterministic
tests inject process identity, daemon status, and confirmed/refused/unknown
launchd outcomes; they never execute `launchctl`. `MacOsLaunchdPortV1` is the
only real command adapter. It uses fixed `/bin/launchctl` paths and executes
only the already-qualified daemon's `--status` command. `make
check-launchd-activation` runs the tests and warnings-denied Clippy without
performing activation.

Codesigning, notarization, plist installation location, log retention,
multi-user policy, cross-repository daemon/MCP coordination, semantic-provider lifecycle,
and recovery after an unknown launchctl effect remain separate operator or
future distribution contracts.

## Offline journal reconciliation

An interrupted or `outcome_unknown` transaction can be classified without
invoking `launchctl`, the daemon status command, or any retry path:

```sh
vellum-launchd-activate --reconcile-journal \
  --config=/absolute/private/activation.json
```

Upgrade reconciliation revalidates the same exact approval, qualification evidence,
candidate/prior daemon hashes, plists and manifest identities. It then requires
the transaction directory to contain only the private `0700` journal, and every
entry to be a canonical same-owner `0400` file in one contiguous sequence. The
reader validates the complete activation state-machine prefix; transplanted
identities, missing interior entries, reordered phases, impossible effects,
changed dispositions, unknown fields and noncanonical JSON refuse.

The content-safe report distinguishes a durable candidate-active, prior-restored,
activation-refused or rollback-refused terminal from prepared-only, pending,
explicitly unknown and confirmed-effect-without-durable-terminal prefixes. It
always sets `automaticRetryAuthorized` to false and `launchdEffect` to
`not_attempted`. Nonterminal states require either manual reauthorization or an
operator inspection of the exact job/program/PID and daemon status. The report
does not infer current process state from the journal and cannot resume,
rollback, load or unload a job.

The corresponding read-only first-install reconciliation is:

```sh
vellum-launchd-activate --reconcile-first-install-journal \
  --config=/absolute/private/first-install.json
```

It revalidates the canonical installation receipt, installed daemon-only
closure, candidate-only approval, and exact plist, then validates the separate
first-install journal grammar and reports durable
candidate-active or installation-refused terminals separately from prepared,
pending, unknown and confirmed-without-terminal cuts. It likewise sets
`automaticRetryAuthorized` to false and performs no launchd or daemon effect.
