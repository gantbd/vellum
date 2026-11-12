import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { canonicalJson, sha256File } from './release-candidate-lib.mjs';

const fixture = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-artifact-lifecycle-'));
const repository = path.join(fixture, 'repository');
const candidate = path.join(fixture, 'candidate');
const prior = path.join(fixture, 'prior');
const qualifier = path.resolve('scripts/qualify-release-candidate.mjs');
const rendererTarget = path.join(fixture, 'renderer-target');
const rendererBuild = spawnSync(
  'cargo',
  ['build', '--locked', '--offline', '--package', 'vellum-launchd-render', '--target-dir', rendererTarget],
  { cwd: path.resolve('.'), encoding: 'utf8' },
);
if (rendererBuild.status !== 0) throw new Error(rendererBuild.stderr || 'native renderer build failed');
const rendererSource = fs.readFileSync(path.join(rendererTarget, 'debug', 'vellum-launchd-render'));
const inputs = [
  'Cargo.lock',
  'release/compatibility-baseline.json',
  'release/dependency-policy.json',
  'release/github-source.json',
  'release/source-sbom.manifest.json',
  'distribution/launchd-render/Cargo.toml',
  'distribution/launchd-render/src/lib.rs',
  'distribution/launchd-render/src/main.rs',
  'rust-toolchain.toml',
];

const write = (root, name, contents, mode) => {
  const file = path.join(root, ...name.split('/'));
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, contents, mode === undefined ? undefined : { mode });
};
const runtime = spawnSync('rustc', ['-vV'], { encoding: 'utf8' });
if (runtime.status !== 0) throw new Error('rustc unavailable');
const target = runtime.stdout.match(/^host: (.+)$/m)?.[1];
if (!target) throw new Error('rustc host unavailable');

const makeCandidate = (directory, revision, marker) => {
  fs.mkdirSync(directory);
  for (const name of ['vellum-daemon', 'vellum-mcp']) {
    write(directory, name, `#!/bin/sh\necho "${name} ${marker}"\n`, 0o755);
  }
  const artifacts = ['vellum-daemon', 'vellum-mcp'].map((name, index) => {
    const file = path.join(directory, name);
    return {
      package: name,
      binary: name,
      file: name,
      bytes: fs.statSync(file).size,
      sha256: sha256File(file),
      ...(index === 1 ? { role: 'peer connector to an already-running daemon' } : {}),
    };
  });
  write(directory, 'vellum-launchd-render', rendererSource, 0o755);
  const deploymentFile = path.join(directory, 'vellum-launchd-render');
  const deployment = [{
    file: 'vellum-launchd-render', bytes: fs.statSync(deploymentFile).size,
    sha256: sha256File(deploymentFile), role: 'inert-launchd-plist-renderer', runtime: 'native',
  }];
  const manifest = {
    schema: 3,
    channel: 'github-release-candidate-artifact',
    claims: [
      'locked-workspace-build',
      'native-help-smoke',
      'sha256-checksums',
      'effective-build-configuration',
      'inert-launchd-renderer',
    ],
    nonClaims: [
      'signature', 'attestation', 'github-release', 'crates.io-publication', 'activation',
      'launchd-installation', 'launchd-socket-activation', 'authenticated-health',
    ],
    source: {
      vellum: { repository: 'https://github.com/example/vellum', revision },
      keystone: { repository: 'https://github.com/example/keystone', revision: 'c'.repeat(40) },
    },
    build: {
      platform: 'macos',
      target,
      targetSelection: 'cargo --target',
      profile: 'release',
      locked: true,
      rustc: 'rustc 1.95.0',
      cargo: 'cargo 1.95.0',
      packages: ['vellum-daemon', 'vellum-mcp', 'vellum-launchd-render'],
      features: 'default',
      environment: {
        CARGO_BUILD_TARGET: 'unset',
        RUSTFLAGS: 'unset',
        CARGO_ENCODED_RUSTFLAGS: 'path-remap-only',
      },
    },
    inputs: Object.fromEntries(inputs.map((name) => [name, sha256File(path.join(repository, name))])),
    artifacts,
    deployment,
  };
  write(directory, 'manifest.json', canonicalJson(manifest));
  write(
    directory,
    'SHA256SUMS',
    `${[...artifacts, ...deployment].map(({ sha256, file }) => `${sha256}  ${file}`).join('\n')}\n${sha256File(path.join(directory, 'manifest.json'))}  manifest.json\n`,
  );
};
const runQualification = (candidateDirectory, priorDirectory, evidence) => spawnSync(
  process.execPath,
  [
    qualifier,
    `--candidate-dir=${candidateDirectory}`,
    `--prior-candidate-dir=${priorDirectory}`,
    `--evidence-file=${evidence}`,
    `--repository-root=${repository}`,
  ],
  { encoding: 'utf8' },
);
const expectRefusal = (label, mutate) => {
  const mutant = path.join(fixture, `mutant-${label}`);
  fs.cpSync(candidate, mutant, { recursive: true });
  mutate(mutant);
  const evidence = path.join(fixture, `${label}.json`);
  const result = runQualification(mutant, prior, evidence);
  if (result.status === 0 || fs.existsSync(evidence)) {
    throw new Error(`${label} mutant was not refused before evidence creation`);
  }
};
const expectPriorRefusal = (label, mutate) => {
  const mutant = path.join(fixture, `mutant-prior-${label}`);
  fs.cpSync(prior, mutant, { recursive: true });
  mutate(mutant);
  const evidence = path.join(fixture, `prior-${label}.json`);
  const result = runQualification(candidate, mutant, evidence);
  if (result.status === 0 || fs.existsSync(evidence)) {
    throw new Error(`prior ${label} mutant was not refused before evidence creation`);
  }
};
const refreshCandidateIdentity = (root) => {
  const manifestFile = path.join(root, 'manifest.json');
  const manifest = JSON.parse(fs.readFileSync(manifestFile, 'utf8'));
  const deploymentFile = path.join(root, 'vellum-launchd-render');
  manifest.deployment[0].bytes = fs.statSync(deploymentFile).size;
  manifest.deployment[0].sha256 = sha256File(deploymentFile);
  fs.writeFileSync(manifestFile, canonicalJson(manifest));
  fs.writeFileSync(
    path.join(root, 'SHA256SUMS'),
    `${[...manifest.artifacts, ...manifest.deployment].map(({ sha256, file }) => `${sha256}  ${file}`).join('\n')}\n${sha256File(manifestFile)}  manifest.json\n`,
  );
};

try {
  fs.mkdirSync(repository);
  for (const [index, name] of inputs.entries()) {
    write(repository, name, `qualified input ${index}\n`);
  }
  makeCandidate(candidate, 'a'.repeat(40), 'candidate');
  makeCandidate(prior, 'b'.repeat(40), 'prior');

  const evidence = path.join(fixture, 'evidence.json');
  const qualified = runQualification(candidate, prior, evidence);
  if (qualified.status !== 0) throw new Error(qualified.stderr || 'valid qualification failed');
  const report = JSON.parse(fs.readFileSync(evidence, 'utf8'));
  if (report.state !== 'ready-for-manual-activation-review' || report.activationAuthorized !== false) {
    throw new Error('qualification blurred readiness and activation');
  }
  if (report.rollback.selectedPriorVellumRevision !== 'b'.repeat(40)) {
    throw new Error('qualification omitted the selected prior version');
  }
  for (const product of [report.candidate.launchdSmoke, report.rollback.launchdSmoke]) {
    if (
      product?.artifact !== 'vellum-launchd-render'
      || product.runtime !== 'native'
      || product.invocation !== 'private-absolute-deployment-record'
      || product.exitCode !== 0
      || product.exactInertProfile !== true
      || product.plistLint !== 'passed'
    ) {
      throw new Error('qualification omitted content-safe packaged launchd smoke evidence');
    }
    if (JSON.stringify(product).includes(fixture)) {
      throw new Error('launchd smoke evidence disclosed private fixture paths');
    }
  }

  expectRefusal('corrupt-artifact', (root) => fs.appendFileSync(path.join(root, 'vellum-daemon'), 'corruption'));
  expectRefusal('unknown-file', (root) => write(root, 'surprise', 'not closed\n'));
  expectRefusal('incompatible-schema', (root) => {
    const file = path.join(root, 'manifest.json');
    const manifest = JSON.parse(fs.readFileSync(file, 'utf8'));
    manifest.schema = 999;
    fs.writeFileSync(file, canonicalJson(manifest));
    const lines = fs.readFileSync(path.join(root, 'SHA256SUMS'), 'utf8').trim().split('\n');
    lines[3] = `${sha256File(file)}  manifest.json`;
    fs.writeFileSync(path.join(root, 'SHA256SUMS'), `${lines.join('\n')}\n`);
  });
  expectRefusal('symlink-artifact', (root) => {
    fs.unlinkSync(path.join(root, 'vellum-daemon'));
    fs.symlinkSync(path.join(candidate, 'vellum-daemon'), path.join(root, 'vellum-daemon'));
  });
  expectRefusal('corrupt-launchd-renderer', (root) => {
    fs.appendFileSync(path.join(root, 'vellum-launchd-render'), 'corruption');
  });
  expectRefusal('symlink-launchd-renderer', (root) => {
    fs.unlinkSync(path.join(root, 'vellum-launchd-render'));
    fs.symlinkSync(path.join(candidate, 'vellum-launchd-render'), path.join(root, 'vellum-launchd-render'));
  });
  expectRefusal('invalid-launchd-renderer-output', (root) => {
    fs.writeFileSync(
      path.join(root, 'vellum-launchd-render'),
      '#!/bin/sh\nprintf "not a plist\\n"\n',
      { mode: 0o755 },
    );
    refreshCandidateIdentity(root);
  });
  expectPriorRefusal('invalid-launchd-renderer-output', (root) => {
    fs.writeFileSync(
      path.join(root, 'vellum-launchd-render'),
      '#!/bin/sh\nprintf "not a plist\\n"\n',
      { mode: 0o755 },
    );
    refreshCandidateIdentity(root);
  });
  expectRefusal('failed-smoke', (root) => {
    const binary = path.join(root, 'vellum-daemon');
    fs.writeFileSync(binary, '#!/bin/sh\nexit 7\n', { mode: 0o755 });
    const manifestFile = path.join(root, 'manifest.json');
    const manifest = JSON.parse(fs.readFileSync(manifestFile, 'utf8'));
    manifest.artifacts[0].bytes = fs.statSync(binary).size;
    manifest.artifacts[0].sha256 = sha256File(binary);
    fs.writeFileSync(manifestFile, canonicalJson(manifest));
    fs.writeFileSync(
      path.join(root, 'SHA256SUMS'),
      `${[...manifest.artifacts, ...manifest.deployment].map(({ sha256, file }) => `${sha256}  ${file}`).join('\n')}\n${sha256File(manifestFile)}  manifest.json\n`,
    );
  });

  const sameEvidence = path.join(fixture, 'same-version.json');
  const same = runQualification(candidate, candidate, sameEvidence);
  if (same.status === 0 || fs.existsSync(sameEvidence)) {
    throw new Error('candidate was accepted as its own rollback version');
  }

  const mismatchEvidence = path.join(fixture, 'policy-mismatch.json');
  fs.appendFileSync(path.join(repository, 'Cargo.lock'), 'drift\n');
  const mismatch = runQualification(candidate, prior, mismatchEvidence);
  if (mismatch.status === 0 || fs.existsSync(mismatchEvidence)) {
    throw new Error('current policy incompatibility was not refused');
  }

  process.stdout.write('Vellum artifact-lifecycle qualification mutants passed\n');
} finally {
  fs.rmSync(fixture, { recursive: true, force: true });
}
