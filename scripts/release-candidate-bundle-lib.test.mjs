import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

import {
  buildCandidateBundle,
  verifyCandidateBundle,
} from './release-candidate-bundle-lib.mjs';
import { canonicalJson, sha256File } from './release-candidate-lib.mjs';

const root = fs.realpathSync('.');
const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-candidate-bundle-test-'));
const qualified = path.join(scratch, 'qualified');
const candidate = path.join(qualified, 'candidate');
const bundle = path.join(scratch, 'vellum-release-candidate.tar');

const run = (command, args, options = {}) => {
  const result = spawnSync(command, args, { cwd: options.cwd ?? root, encoding: 'utf8' });
  if (result.status !== 0) throw new Error(result.stderr || `${command} failed`);
  return result;
};
const write = (file, contents, mode) => {
  fs.writeFileSync(file, contents, { mode });
  fs.chmodSync(file, mode);
};
const inputNames = [
  'Cargo.lock',
  'release/compatibility-baseline.json',
  'release/dependency-policy.json',
  'release/portable-core.json',
  'release/github-source.json',
  'release/source-sbom.manifest.json',
  'distribution/launchd-render/Cargo.toml',
  'distribution/launchd-render/src/lib.rs',
  'distribution/launchd-render/src/main.rs',
  'distribution/launchd-activate/Cargo.toml',
  'distribution/launchd-activate/src/lib.rs',
  'distribution/launchd-activate/src/main.rs',
  'scripts/build-release-candidate.mjs',
  'scripts/release-candidate-lib.mjs',
  'scripts/qualify-clean-release-candidate.mjs',
  'scripts/compare-release-candidates.mjs',
  'scripts/release-reproducibility-lib.mjs',
  '.github/workflows/release-candidate.yml',
  '.github/workflows/portable-core.yml',
  'rust-toolchain.toml',
];

const targetDirectory = path.join(scratch, 'target');
run('cargo', [
  'build', '--locked', '--offline', '--package', 'vellum-launchd-render',
  '--package', 'vellum-launchd-activate', '--target-dir', targetDirectory,
]);
const runtime = run('rustc', ['-vV']).stdout;
const target = runtime.match(/^host: (.+)$/m)?.[1];
if (!target) throw new Error('rustc host target is absent');
const policy = JSON.parse(fs.readFileSync(path.join(root, 'release/github-source.json'), 'utf8'));
const vellumRevision = run('git', ['rev-parse', 'HEAD^{commit}']).stdout.trim();
const vellumTree = run('git', ['rev-parse', `${vellumRevision}^{tree}`]).stdout.trim();
const keystoneRoot = fs.realpathSync(path.resolve(root, process.env.KEYSTONE_SOURCE ?? policy.keystone.relativePath));
const keystoneTree = run(
  'git', ['-C', keystoneRoot, 'rev-parse', `${policy.keystone.revision}^{tree}`],
).stdout.trim();

const recomputeHeaderChecksum = (bytes, offset) => {
  bytes.fill(0x20, offset + 148, offset + 156);
  let sum = 0;
  for (let cursor = offset; cursor < offset + 512; cursor += 1) sum += bytes[cursor];
  bytes.write(sum.toString(8).padStart(6, '0'), offset + 148, 6, 'ascii');
  bytes[offset + 154] = 0;
  bytes[offset + 155] = 0x20;
};
const entryOffsets = (bytes) => {
  const offsets = [];
  let cursor = 0;
  while (!bytes.subarray(cursor, cursor + 512).every((byte) => byte === 0)) {
    offsets.push(cursor);
    const raw = bytes.subarray(cursor + 124, cursor + 136).toString('ascii').replaceAll('\0', '').trim();
    const size = Number.parseInt(raw, 8);
    cursor += 512 + Math.ceil(size / 512) * 512;
  }
  return { offsets, terminator: cursor };
};
const writeMutant = (label, bytes) => {
  const file = path.join(scratch, `${label}.tar`);
  write(file, bytes, 0o644);
  let refused = false;
  try {
    verifyCandidateBundle(file, { repositoryRoot: root, smoke: false });
  } catch {
    refused = true;
  }
  if (!refused) throw new Error(`${label} bundle mutant was accepted`);
};

try {
  fs.mkdirSync(candidate, { recursive: true, mode: 0o700 });
  fs.chmodSync(candidate, 0o700);
  for (const name of ['vellum-daemon', 'vellum-mcp']) {
    write(path.join(candidate, name), `#!/bin/sh\necho '${name} --help'\n`, 0o755);
  }
  for (const name of ['vellum-launchd-render', 'vellum-launchd-activate']) {
    fs.copyFileSync(path.join(targetDirectory, 'debug', name), path.join(candidate, name));
    fs.chmodSync(path.join(candidate, name), 0o755);
  }
  const artifacts = ['vellum-daemon', 'vellum-mcp'].map((name, index) => ({
    package: 'vellum', binary: name, file: name,
    bytes: fs.statSync(path.join(candidate, name)).size,
    sha256: sha256File(path.join(candidate, name)),
    ...(index === 1 ? { role: 'peer connector to an already-running daemon' } : {}),
  }));
  const deployment = ['vellum-launchd-render', 'vellum-launchd-activate'].map((name, index) => ({
    file: name,
    bytes: fs.statSync(path.join(candidate, name)).size,
    sha256: sha256File(path.join(candidate, name)),
    role: index === 0
      ? 'inert-launchd-plist-renderer'
      : 'explicitly-authorized-launchd-activation-executor',
    runtime: 'native',
  }));
  const manifest = {
    schema: 5,
    channel: 'github-release-candidate-artifact',
    claims: [
      'locked-workspace-build', 'native-help-smoke', 'sha256-checksums',
      'effective-build-configuration', 'build-recipe-sha256', 'inert-launchd-renderer',
      'activation-executor-packaged',
    ],
    nonClaims: [
      'signature', 'attestation', 'github-release', 'crates.io-publication', 'activation',
      'launchd-installation', 'launchd-socket-activation', 'authenticated-health',
    ],
    source: {
      vellum: { repository: policy.vellumRepository, revision: vellumRevision },
      keystone: { repository: policy.keystone.repository, revision: policy.keystone.revision },
    },
    build: {
      platform: 'macos', target, targetSelection: 'cargo --target', profile: 'release',
      locked: true, rustc: 'rustc 1.95.0', cargo: 'cargo 1.95.0',
      packages: ['vellum', 'vellum-launchd-render', 'vellum-launchd-activate'],
      features: 'default',
      environment: {
        CARGO_BUILD_TARGET: 'unset', RUSTFLAGS: 'unset',
        CARGO_ENCODED_RUSTFLAGS: 'path-remap-only',
      },
    },
    inputs: Object.fromEntries(inputNames.map((name) => [name, sha256File(path.join(root, name))])),
    artifacts,
    deployment,
  };
  write(path.join(candidate, 'manifest.json'), canonicalJson(manifest), 0o600);
  write(
    path.join(candidate, 'SHA256SUMS'),
    `${[...artifacts, ...deployment].map(({ sha256, file }) => `${sha256}  ${file}`).join('\n')}\n${sha256File(path.join(candidate, 'manifest.json'))}  manifest.json\n`,
    0o600,
  );
  const files = fs.readdirSync(candidate).sort().map((name) => ({
    file: name,
    bytes: fs.statSync(path.join(candidate, name)).size,
    sha256: sha256File(path.join(candidate, name)),
  }));
  const evidence = {
    schema: 1,
    channel: 'local-release-candidate-byte-equivalence',
    claims: ['distinct-materialization', 'exact-file-closure', 'byte-for-byte-equivalence'],
    nonClaims: [
      'independent-build-execution', 'reproducible-build', 'signature', 'attestation',
      'github-release', 'activation',
    ],
    source: { vellumRevision, keystoneRevision: policy.keystone.revision },
    build: { target, rustc: 'rustc 1.95.0', cargo: 'cargo 1.95.0', profile: 'release', features: 'default' },
    manifestSha256: sha256File(path.join(candidate, 'manifest.json')),
    files,
  };
  write(path.join(qualified, 'candidate-byte-equivalence.json'), canonicalJson(evidence), 0o600);
  const provenance = {
    schema: 1,
    channel: 'local-release-candidate-source-provenance',
    claims: [
      'exact-local-git-objects', 'clean-detached-checkouts',
      'candidate-manifest-bound', 'byte-equivalence-bound',
    ],
    nonClaims: [
      'remote-reachability', 'signature', 'attestation', 'github-release',
      'independent-build-execution', 'cross-host-reproducible-build',
    ],
    source: {
      vellum: { ...manifest.source.vellum, tree: vellumTree },
      keystone: { ...manifest.source.keystone, tree: keystoneTree },
    },
    subject: {
      manifestSha256: evidence.manifestSha256,
      byteEquivalenceSha256: sha256File(path.join(qualified, 'candidate-byte-equivalence.json')),
    },
  };
  write(path.join(qualified, 'source-provenance.json'), canonicalJson(provenance), 0o600);

  const first = buildCandidateBundle(qualified, { repositoryRoot: root });
  const second = buildCandidateBundle(qualified, { repositoryRoot: root });
  if (!first.equals(second)) throw new Error('candidate bundle bytes are not deterministic');
  const provenanceFile = path.join(qualified, 'source-provenance.json');
  const provenanceText = fs.readFileSync(provenanceFile, 'utf8');
  const substituted = JSON.parse(provenanceText);
  substituted.source.vellum.tree = 'e'.repeat(40);
  write(provenanceFile, canonicalJson(substituted), 0o600);
  let sourceSubstitutionRefused = false;
  try {
    buildCandidateBundle(qualified, { repositoryRoot: root });
  } catch {
    sourceSubstitutionRefused = true;
  } finally {
    write(provenanceFile, provenanceText, 0o600);
  }
  if (!sourceSubstitutionRefused) {
    throw new Error('candidate bundle accepted substituted source-tree provenance');
  }

  const packaged = run(process.execPath, [
    'scripts/package-release-candidate.mjs',
    `--qualified-dir=${qualified}`,
    `--bundle-file=${bundle}`,
    `--repository-root=${root}`,
  ]);
  if (!packaged.stdout.includes('upload transport may flatten only the outer mode')) {
    throw new Error('packager omitted its transport claim');
  }
  fs.chmodSync(bundle, 0o644);
  const verified = run(process.execPath, [
    'scripts/verify-release-candidate-bundle.mjs',
    `--bundle-file=${bundle}`,
    `--repository-root=${root}`,
  ]);
  if (!verified.stdout.includes('native smoke passed')) {
    throw new Error('download verifier omitted its smoke evidence');
  }

  const original = fs.readFileSync(bundle);
  const layout = entryOffsets(original);

  const traversal = Buffer.from(original);
  traversal.fill(0, layout.offsets[0], layout.offsets[0] + 100);
  traversal.write('../evil/', layout.offsets[0], 'ascii');
  recomputeHeaderChecksum(traversal, layout.offsets[0]);
  writeMutant('traversal', traversal);

  const link = Buffer.from(original);
  link.write('2', layout.offsets[1] + 156, 1, 'ascii');
  recomputeHeaderChecksum(link, layout.offsets[1]);
  writeMutant('link', link);

  const modeLoss = Buffer.from(original);
  modeLoss.write('0000644\0', layout.offsets[1] + 100, 8, 'ascii');
  recomputeHeaderChecksum(modeLoss, layout.offsets[1]);
  writeMutant('mode-loss', modeLoss);

  const corrupt = Buffer.from(original);
  corrupt[layout.offsets[1] + 512] ^= 0x01;
  writeMutant('corrupt', corrupt);

  const finalEntry = original.subarray(layout.offsets.at(-1), layout.terminator);
  writeMutant('extra-entry', Buffer.concat([
    original.subarray(0, layout.terminator),
    finalEntry,
    Buffer.alloc(1024),
  ]));

  process.stdout.write('Vellum candidate bundle round-trip and adversarial qualification passed\n');
} finally {
  fs.rmSync(scratch, { recursive: true, force: true });
}
