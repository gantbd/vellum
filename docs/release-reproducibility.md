# Release-candidate byte equivalence

Vellum's GitHub candidate workflow builds the same exact revision twice into
distinct candidate directories and compares the closed products with
`scripts/compare-release-candidates.mjs`. Both candidates must first satisfy the
ordinary release-candidate validator: exact file closure, canonical manifest,
current policy-input hashes, target compatibility and artifact checksums.

The comparison then requires:

- distinct candidate roots;
- a byte-identical canonical manifest;
- the same exact regular-file set;
- separate filesystem materialization for every file (hard-link aliases are
  refused); and
- equal byte counts and SHA-256 digests for every file.

The emitted create-exclusive evidence is content-safe. It records the exact
Vellum and Keystone revisions, effective toolchain/target selection, manifest
digest and per-file size/digest products without local paths, usernames or
temporary-directory identities.

Run the comparator directly after producing two candidates:

```sh
node scripts/compare-release-candidates.mjs \
  --candidate-a=/private/path/candidate-a \
  --candidate-b=/private/path/candidate-b \
  --evidence-file=/private/path/equivalence.json
```

This evidence proves byte-for-byte equality of two separately materialized
candidate products. The comparator cannot prove that an external caller
actually performed two independent compilations, so its standalone record
explicitly does **not** claim an independently executed or reproducible build.
The coordinated workflow supplies the stronger construction fact by invoking
the clean candidate builder twice, but that remains CI evidence rather than a
universal cross-host reproducibility claim. The record is not a signature,
attestation, GitHub Release, activation authorization or crates.io publication.

The clean-source composition additionally writes create-exclusive canonical
`source-provenance.json`. It binds the selected Vellum and Keystone commit and
tree objects to the candidate-manifest and byte-equivalence digests. The bundle
parser requires that exact relationship before smoke or installation. This is
unsigned local object evidence, not a GitHub reachability result, signature,
attestation, independent-build claim, or cross-host reproducibility claim.

The maintained clean-source gate creates detached local clones of the exact
selected Vellum commit and coordinated Keystone commit, builds twice with Cargo
offline, and retains one candidate plus the equivalence evidence:

```sh
node scripts/qualify-clean-release-candidate.mjs \
  --vellum-ref="$(git rev-parse HEAD)" \
  --output-dir=../vellum-qualified-candidate
```

Uncommitted files are never copied into the build. The gate uses only local Git
objects and a local exact-URL rewrite; it neither checks remote reachability nor
publishes, signs, attests, installs or activates the result. Registry sources
must already exist in the selected Cargo cache because network access is
disabled. The synthetic end-to-end test proves exact-ref selection, dirty-tree
exclusion, the two-build comparison, content-safe evidence and non-destructive
refusal of an existing output directory.

The adversarial test refuses candidate-root reuse, payload drift, file-closure
drift and hard-link substitution:

```sh
node scripts/release-reproducibility-lib.test.mjs
```
