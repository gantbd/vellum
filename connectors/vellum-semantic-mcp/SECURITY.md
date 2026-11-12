# Security policy

Report vulnerabilities through the private channel documented by the parent
Vellum repository. Do not include credentials, source text, private paths or
captured protocol transcripts in a public report.

This peer authenticates local service connections with private-file credential
proofs, but local HMAC attribution is not encryption, remote identity or
same-user isolation. Semantic roles are independently attenuated by the
service. Query results are revision-, generation- and witness-validated; rename
is a plan only. Uncertain mutations are never retried under a new identity.

The supported source distribution remains exact, reviewed Git revisions with a
committed lockfile. The package is not published to crates.io.
