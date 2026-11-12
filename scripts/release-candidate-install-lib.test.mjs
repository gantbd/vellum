import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

import { installCandidateBundle } from './release-candidate-install-lib.mjs';
import { buildCandidateBundle } from './release-candidate-bundle-lib.mjs';
import { canonicalJson, sha256File } from './release-candidate-lib.mjs';

const root = fs.realpathSync('.');
const scratch = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-candidate-install-test-')));
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
const privateRoot = (label) => {
  const value = path.join(scratch, label);
  fs.mkdirSync(value, { mode: 0o700 });
  fs.chmodSync(value, 0o700);
  return value;
};
const expectRefusal = (action, pattern) => {
  let failure;
  try {
    action();
  } catch (error) {
    failure = error;
  }
  if (failure === undefined || !pattern.test(failure.message)) {
    throw new Error(`expected refusal ${pattern}, received ${failure?.message ?? 'success'}`);
  }
};
const inputNames = [
  'Cargo.lock',
  'release/compatibility-baseline.json',
  'release/dependency-policy.json',
  'release/github-source.json',
  'release/source-sbom.manifest.json',
  'distribution/launchd-render/Cargo.toml',
  'distribution/launchd-render/src/lib.rs',
  'distribution/launchd-render/src/main.rs',
  'distribution/launchd-activate/Cargo.toml',
  'distribution/launchd-activate/src/lib.rs',
  'distribution/launchd-activate/src/main.rs',
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
    schema: 4,
    channel: 'github-release-candidate-artifact',
    claims: [
      'locked-workspace-build', 'native-help-smoke', 'sha256-checksums',
      'effective-build-configuration', 'inert-launchd-renderer',
      'activation-executor-packaged',
    ],
    nonClaims: [
      'signature', 'attestation', 'github-release', 'crates.io-publication', 'activation',
      'launchd-installation', 'launchd-socket-activation', 'authenticated-health',
    ],
    source: {
      vellum: { repository: 'https://github.com/example/vellum', revision: 'a'.repeat(40) },
      keystone: { repository: 'https://github.com/example/keystone', revision: 'c'.repeat(40) },
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
    source: { vellumRevision: 'a'.repeat(40), keystoneRevision: 'c'.repeat(40) },
    build: {
      target, rustc: 'rustc 1.95.0', cargo: 'cargo 1.95.0',
      profile: 'release', features: 'default',
    },
    manifestSha256: sha256File(path.join(candidate, 'manifest.json')),
    files,
  };
  write(path.join(qualified, 'candidate-byte-equivalence.json'), canonicalJson(evidence), 0o600);
  write(bundle, buildCandidateBundle(qualified, { repositoryRoot: root }), 0o644);

  const installation = privateRoot('installation');
  const installed = run(process.execPath, [
    'scripts/install-release-candidate.mjs',
    `--bundle-file=${bundle}`,
    `--install-root=${installation}`,
  ]);
  if (!installed.stdout.includes('2 products; activation not performed')) {
    throw new Error('installer omitted its exact product/effect disposition');
  }
  if (fs.readdirSync(installation).sort().join(',') !== 'bin,install-receipt.json') {
    throw new Error('installed closure is incompatible');
  }
  if (fs.readdirSync(path.join(installation, 'bin')).sort().join(',') !== 'vellum-daemon,vellum-mcp') {
    throw new Error('installer admitted a non-product payload');
  }
  for (const name of ['vellum-daemon', 'vellum-mcp']) {
    const file = path.join(installation, 'bin', name);
    if ((fs.statSync(file).mode & 0o777) !== 0o500) throw new Error(`${name} mode drifted`);
    if (!run(file, ['--help']).stdout.includes(name)) throw new Error(`${name} install smoke failed`);
  }
  const receiptFile = path.join(installation, 'install-receipt.json');
  if ((fs.statSync(receiptFile).mode & 0o777) !== 0o400) {
    throw new Error('installation receipt mode drifted');
  }
  const receiptText = fs.readFileSync(receiptFile, 'utf8');
  const receipt = JSON.parse(receiptText);
  if (
    receiptText !== canonicalJson(receipt)
    || receipt.activationPerformed !== false
    || receipt.products.map((product) => product.file).join(',')
      !== 'bin/vellum-daemon,bin/vellum-mcp'
    || receiptText.includes(scratch)
  ) {
    throw new Error('installation receipt is noncanonical, path-bearing, or effect-ambiguous');
  }
  const repeated = installCandidateBundle(bundle, installation, { repositoryRoot: root });
  if (repeated.status !== 'already-installed' || repeated.activationPerformed !== false) {
    throw new Error('exact repeated installation was not idempotent');
  }

  expectRefusal(
    () => installCandidateBundle(path.relative(root, bundle), privateRoot('relative-bundle')),
    /bundle must be absolute/,
  );
  expectRefusal(
    () => installCandidateBundle(
      `${scratch}/qualified/../vellum-release-candidate.tar`,
      privateRoot('aliased-bundle-root'),
    ),
    /bundle must be canonical and non-symbolic/,
  );
  const linkedBundle = path.join(scratch, 'bundle-link');
  fs.symlinkSync(bundle, linkedBundle);
  expectRefusal(
    () => installCandidateBundle(linkedBundle, privateRoot('linked-bundle-root')),
    /bundle must be canonical and non-symbolic/,
  );
  const linkedRootTarget = privateRoot('linked-root-target');
  const linkedRoot = path.join(scratch, 'linked-root');
  fs.symlinkSync(linkedRootTarget, linkedRoot);
  expectRefusal(
    () => installCandidateBundle(bundle, linkedRoot),
    /installation root must be canonical and non-symbolic/,
  );
  const wrongMode = privateRoot('wrong-mode');
  fs.chmodSync(wrongMode, 0o755);
  expectRefusal(
    () => installCandidateBundle(bundle, wrongMode),
    /installation root must be a directory with mode 700/,
  );
  const extra = privateRoot('extra');
  write(path.join(extra, 'foreign'), 'foreign', 0o600);
  expectRefusal(
    () => installCandidateBundle(bundle, extra, { repositoryRoot: root }),
    /installation root closure is incomplete or contains extra entries/,
  );

  for (const point of ['after-bin', 'after-vellum-daemon', 'after-vellum-mcp']) {
    const interrupted = privateRoot(`interrupted-${point}`);
    expectRefusal(
      () => installCandidateBundle(bundle, interrupted, { repositoryRoot: root, faultAfter: point }),
      new RegExp(`injected interruption at ${point}`),
    );
    if (fs.existsSync(path.join(interrupted, 'install-receipt.json'))) {
      throw new Error(`${point} interruption fabricated a terminal receipt`);
    }
    expectRefusal(
      () => installCandidateBundle(bundle, interrupted, { repositoryRoot: root }),
      /installation root closure is incomplete or contains extra entries/,
    );
  }

  const substituted = privateRoot('substituted');
  installCandidateBundle(bundle, substituted, { repositoryRoot: root });
  fs.chmodSync(path.join(substituted, 'bin/vellum-daemon'), 0o700);
  fs.appendFileSync(path.join(substituted, 'bin/vellum-daemon'), 'substitution');
  fs.chmodSync(path.join(substituted, 'bin/vellum-daemon'), 0o500);
  expectRefusal(
    () => installCandidateBundle(bundle, substituted, { repositoryRoot: root }),
    /installed vellum-daemon identity is incompatible/,
  );

  process.stdout.write('Vellum verified candidate installation and interruption qualification passed\n');
} finally {
  fs.rmSync(scratch, { recursive: true, force: true });
}
