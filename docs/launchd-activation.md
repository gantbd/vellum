# Authorized macOS launchd activation

Status: explicit local executor implemented; no activation is performed by
builds, tests, release workflows, Cargo installation, or candidate
qualification.

`vellum-launchd-activate` is a separate distribution package. It is not one of
the two binaries installed by the `vellum` Cargo-install package. The GitHub
release-candidate payload now checksum-closes the exact native executable and
qualification runs only its `--help` path, recording `launchdEffect:
not-attempted`. An operator must still create an exact private configuration,
retain the qualified candidate and prior candidates, render both exact inert
plists, and create a separate affirmative approval before the executable can
enter its effect path.

The only activating invocation is:

```sh
cargo run --locked --offline -p vellum-launchd-activate -- \
  --execute-authorized-activation \
  --config=/absolute/private/activation.json
```

Omitting the execution switch, using a relative or aliased configuration,
adding an argument, running on a non-macOS host, or changing the file owner or
`0600` mode refuses before `launchctl`.

## Inputs and authority

The closed configuration names:

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
multi-user policy, daemon/MCP coordinated upgrade, semantic-provider lifecycle,
and recovery after an unknown launchctl effect remain separate operator or
future distribution contracts.
