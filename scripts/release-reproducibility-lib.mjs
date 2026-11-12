import fs from 'node:fs';
import path from 'node:path';

import { canonicalJson, sha256File } from './release-candidate-lib.mjs';

const fail = (message) => {
  throw new Error(`release reproducibility: ${message}`);
};

const regularFiles = (root) => fs.readdirSync(root, { withFileTypes: true })
  .map((entry) => {
    if (!entry.isFile() || entry.isSymbolicLink()) {
      fail(`candidate contains a non-regular entry: ${entry.name}`);
    }
    return entry.name;
  })
  .sort();

export function compareCandidateProducts(left, right) {
  if (left.root === right.root) fail('candidate roots must be distinct');
  if (left.manifestSha256 !== right.manifestSha256) {
    fail('candidate manifests are not byte-identical');
  }
  if (canonicalJson(left.manifest) !== canonicalJson(right.manifest)) {
    fail('candidate manifests do not have the same canonical product');
  }

  const leftFiles = regularFiles(left.root);
  const rightFiles = regularFiles(right.root);
  if (leftFiles.join(',') !== rightFiles.join(',')) {
    fail('candidate file closures differ');
  }

  const files = [];
  for (const name of leftFiles) {
    const leftFile = path.join(left.root, name);
    const rightFile = path.join(right.root, name);
    const leftStat = fs.statSync(leftFile);
    const rightStat = fs.statSync(rightFile);
    if (leftStat.dev === rightStat.dev && leftStat.ino === rightStat.ino) {
      fail(`candidate file is aliased rather than independently materialized: ${name}`);
    }
    const leftDigest = sha256File(leftFile);
    const rightDigest = sha256File(rightFile);
    if (leftStat.size !== rightStat.size || leftDigest !== rightDigest) {
      fail(`candidate file differs: ${name}`);
    }
    files.push({ file: name, bytes: leftStat.size, sha256: leftDigest });
  }

  return {
    schema: 1,
    channel: 'local-release-candidate-byte-equivalence',
    claims: ['distinct-materialization', 'exact-file-closure', 'byte-for-byte-equivalence'],
    nonClaims: [
      'independent-build-execution',
      'reproducible-build',
      'signature',
      'attestation',
      'github-release',
      'activation',
    ],
    source: {
      vellumRevision: left.manifest.source.vellum.revision,
      keystoneRevision: left.manifest.source.keystone.revision,
    },
    build: {
      target: left.manifest.build.target,
      rustc: left.manifest.build.rustc,
      cargo: left.manifest.build.cargo,
      profile: left.manifest.build.profile,
      features: left.manifest.build.features,
    },
    manifestSha256: left.manifestSha256,
    files,
  };
}
