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

The adversarial test refuses candidate-root reuse, payload drift, file-closure
drift and hard-link substitution:

```sh
node scripts/release-reproducibility-lib.test.mjs
```
