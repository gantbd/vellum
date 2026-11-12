import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

export const MANIFEST_FILE = 'manifest.json';
export const CHECKSUM_FILE = 'SHA256SUMS';
export const ARTIFACT_NAMES = ['vellum-daemon', 'vellum-mcp'];
export const DEPLOYMENT_NAMES = ['vellum-launchd-render', 'vellum-launchd-activate'];
const SMOKE_TIMEOUT_MILLIS = 30_000;

const DEPLOYMENT_IDENTITIES = [
  {
    file: 'vellum-launchd-render',
    role: 'inert-launchd-plist-renderer',
  },
  {
    file: 'vellum-launchd-activate',
    role: 'explicitly-authorized-launchd-activation-executor',
  },
];

export const sha256Bytes = (bytes) =>
  crypto.createHash('sha256').update(bytes).digest('hex');
export const sha256File = (file) => sha256Bytes(fs.readFileSync(file));
export const canonicalJson = (value) => `${JSON.stringify(value, null, 2)}\n`;

const fail = (message) => {
  throw new Error(`release candidate: ${message}`);
};

const regularFile = (directory, name) => {
  if (!/^[A-Za-z0-9.-]+$/.test(name) || name === '.' || name === '..') {
    fail(`non-canonical candidate path ${JSON.stringify(name)}`);
  }
  const file = path.join(directory, name);
  const stat = fs.lstatSync(file);
  if (stat.isSymbolicLink() || !stat.isFile()) fail(`${name} is not a regular file`);
  return file;
};

const exactKeys = (object, keys, label) => {
  if (object === null || typeof object !== 'object' || Array.isArray(object)) {
    fail(`${label} is not an object`);
  }
  const actual = Object.keys(object).sort().join(',');
  const expected = [...keys].sort().join(',');
  if (actual !== expected) fail(`${label} fields are incompatible`);
};

export function validateCandidate(directory, options = {}) {
  const root = fs.realpathSync(directory);
  const entries = fs.readdirSync(root, { withFileTypes: true });
  const expectedFiles = [...ARTIFACT_NAMES, ...DEPLOYMENT_NAMES, MANIFEST_FILE, CHECKSUM_FILE].sort();
  const actualFiles = entries.map((entry) => entry.name).sort();
  if (actualFiles.join(',') !== expectedFiles.join(',')) {
    fail('file closure differs from daemon, MCP, manifest, and checksums');
  }
  for (const entry of entries) regularFile(root, entry.name);

  const manifestFile = regularFile(root, MANIFEST_FILE);
  const manifestText = fs.readFileSync(manifestFile, 'utf8');
  let manifest;
  try {
    manifest = JSON.parse(manifestText);
  } catch (error) {
    fail(`manifest JSON is invalid: ${error.message}`);
  }
  if (manifestText !== canonicalJson(manifest)) fail('manifest JSON is not canonical');
  exactKeys(
    manifest,
    ['schema', 'channel', 'claims', 'nonClaims', 'source', 'build', 'inputs', 'artifacts', 'deployment'],
    'manifest',
  );
  if (manifest.schema !== 4 || manifest.channel !== 'github-release-candidate-artifact') {
    fail('manifest schema or channel is incompatible');
  }
  if (
    manifest.claims.join(',')
      !== 'locked-workspace-build,native-help-smoke,sha256-checksums,effective-build-configuration,inert-launchd-renderer,activation-executor-packaged'
    || manifest.nonClaims.join(',')
      !== 'signature,attestation,github-release,crates.io-publication,activation,launchd-installation,launchd-socket-activation,authenticated-health'
  ) {
    fail('manifest claims or nonclaims drifted');
  }
  exactKeys(manifest.source, ['vellum', 'keystone'], 'source');
  for (const name of ['vellum', 'keystone']) {
    exactKeys(manifest.source[name], ['repository', 'revision'], `source.${name}`);
    if (!/^https:\/\/github\.com\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(manifest.source[name].repository)) {
      fail(`${name} repository is not a canonical GitHub repository`);
    }
    if (!/^[0-9a-f]{40}$/.test(manifest.source[name].revision)) {
      fail(`${name} revision is not a full commit identity`);
    }
  }
  exactKeys(
    manifest.build,
    ['platform', 'target', 'targetSelection', 'profile', 'locked', 'rustc', 'cargo', 'packages', 'features', 'environment'],
    'build',
  );
  if (
    manifest.build.platform !== 'macos'
    || manifest.build.profile !== 'release'
    || manifest.build.locked !== true
    || manifest.build.targetSelection !== 'cargo --target'
    || manifest.build.packages.join(',') !== ['vellum', ...DEPLOYMENT_NAMES].join(',')
    || manifest.build.features !== 'default'
    || JSON.stringify(manifest.build.environment) !== JSON.stringify({
      CARGO_BUILD_TARGET: 'unset',
      RUSTFLAGS: 'unset',
      CARGO_ENCODED_RUSTFLAGS: 'path-remap-only',
    })
  ) {
    fail('effective build configuration is incompatible');
  }
  if (!/^[A-Za-z0-9_.-]+-apple-darwin$/.test(manifest.build.target)) {
    fail('build target is not macOS');
  }
  if (!/^rustc \d+\.\d+\.\d+/.test(manifest.build.rustc)) fail('rustc identity is invalid');
  if (!/^cargo \d+\.\d+\.\d+/.test(manifest.build.cargo)) fail('Cargo identity is invalid');

  const requiredInputs = [
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
  if (Object.keys(manifest.inputs).sort().join(',') !== requiredInputs.sort().join(',')) {
    fail('captured build-input closure is incompatible');
  }
  for (const [name, digest] of Object.entries(manifest.inputs)) {
    if (!/^[0-9a-f]{64}$/.test(digest)) fail(`input digest is invalid for ${name}`);
  }

  if (!Array.isArray(manifest.artifacts) || manifest.artifacts.length !== ARTIFACT_NAMES.length) {
    fail('artifact closure is incompatible');
  }
  const artifactFiles = [];
  for (const [index, artifact] of manifest.artifacts.entries()) {
    const required = index === 1
      ? ['package', 'binary', 'file', 'bytes', 'sha256', 'role']
      : ['package', 'binary', 'file', 'bytes', 'sha256'];
    exactKeys(artifact, required, `artifacts[${index}]`);
    const expectedName = ARTIFACT_NAMES[index];
    if (artifact.package !== 'vellum' || artifact.binary !== expectedName || artifact.file !== expectedName) {
      fail('artifact ordering or identity is incompatible');
    }
    if (index === 1 && artifact.role !== 'peer connector to an already-running daemon') {
      fail('MCP artifact role is incompatible');
    }
    if (!Number.isSafeInteger(artifact.bytes) || artifact.bytes <= 0) fail(`${expectedName} byte count is invalid`);
    const file = regularFile(root, artifact.file);
    if ((fs.statSync(file).mode & 0o111) === 0) fail(`${expectedName} is not executable`);
    if (fs.statSync(file).size !== artifact.bytes || sha256File(file) !== artifact.sha256) {
      fail(`${expectedName} content does not match its immutable manifest identity`);
    }
    artifactFiles.push(file);
  }

  if (!Array.isArray(manifest.deployment) || manifest.deployment.length !== DEPLOYMENT_IDENTITIES.length) {
    fail('deployment artifact closure is incompatible');
  }
  const deploymentFiles = manifest.deployment.map((deployment, index) => {
    const expected = DEPLOYMENT_IDENTITIES[index];
    exactKeys(deployment, ['file', 'bytes', 'sha256', 'role', 'runtime'], `deployment[${index}]`);
    if (
      deployment.file !== expected.file
      || deployment.role !== expected.role
      || deployment.runtime !== 'native'
      || !Number.isSafeInteger(deployment.bytes)
      || deployment.bytes <= 0
    ) fail('launchd deployment artifact identity is incompatible');
    const deploymentFile = regularFile(root, deployment.file);
    if ((fs.statSync(deploymentFile).mode & 0o111) === 0) {
      fail(`${deployment.file} is not executable`);
    }
    if (
      fs.statSync(deploymentFile).size !== deployment.bytes
      || sha256File(deploymentFile) !== deployment.sha256
    ) {
      fail(`${deployment.file} does not match its immutable manifest identity`);
    }
    return deploymentFile;
  });

  const checksumText = fs.readFileSync(regularFile(root, CHECKSUM_FILE), 'utf8');
  const checksumLines = [
    ...manifest.artifacts.map((artifact) => `${artifact.sha256}  ${artifact.file}`),
    ...manifest.deployment.map((artifact) => `${artifact.sha256}  ${artifact.file}`),
    `${sha256File(manifestFile)}  ${MANIFEST_FILE}`,
  ];
  if (checksumText !== `${checksumLines.join('\n')}\n`) {
    fail('checksum file is non-canonical or does not close over the manifest');
  }

  if (options.repositoryRoot !== undefined) {
    for (const [name, digest] of Object.entries(manifest.inputs)) {
      const current = path.join(options.repositoryRoot, ...name.split('/'));
      if (sha256File(current) !== digest) fail(`candidate is incompatible with current ${name}`);
    }
  }
  if (options.runtimeTarget !== undefined && manifest.build.target !== options.runtimeTarget) {
    fail(`candidate target ${manifest.build.target} is incompatible with runtime ${options.runtimeTarget}`);
  }
  return {
    root,
    manifest,
    manifestSha256: sha256File(manifestFile),
    artifactFiles,
    deploymentFiles,
  };
}

export function smokeCandidate(candidate) {
  return candidate.manifest.artifacts.map((artifact, index) => {
    const result = spawnSync(candidate.artifactFiles[index], ['--help'], {
      encoding: 'utf8',
      env: { PATH: process.env.PATH ?? '' },
      timeout: SMOKE_TIMEOUT_MILLIS,
    });
    if (result.status !== 0 || result.signal !== null) {
      fail(`${artifact.binary} help smoke failed`);
    }
    const stdout = result.stdout ?? '';
    const stderr = result.stderr ?? '';
    if (!stdout.includes(artifact.binary) && !stderr.includes(artifact.binary)) {
      fail(`${artifact.binary} help smoke is not self-identifying`);
    }
    return {
      artifact: artifact.binary,
      arguments: ['--help'],
      exitCode: result.status,
      stdoutBytes: Buffer.byteLength(stdout),
      stdoutSha256: sha256Bytes(stdout),
      stderrBytes: Buffer.byteLength(stderr),
      stderrSha256: sha256Bytes(stderr),
    };
  });
}

const escapeXml = (value) => value
  .replaceAll('&', '&amp;')
  .replaceAll('<', '&lt;')
  .replaceAll('>', '&gt;')
  .replaceAll('"', '&quot;')
  .replaceAll("'", '&apos;');

const expectedLaunchdProfile = (configuration) => {
  const args = [configuration.binary, '--socket', configuration.dataSocket, '--control', configuration.controlSocket];
  const array = args.map((argument) => `      <string>${escapeXml(argument)}</string>`).join('\n');
  return `<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0">\n  <dict>\n    <key>Label</key>\n    <string>${escapeXml(configuration.label)}</string>\n    <key>ProgramArguments</key>\n    <array>\n${array}\n    </array>\n    <key>RunAtLoad</key>\n    <true/>\n    <key>KeepAlive</key>\n    <false/>\n    <key>ProcessType</key>\n    <string>Background</string>\n    <key>Umask</key>\n    <integer>63</integer>\n    <key>StandardOutPath</key>\n    <string>${escapeXml(configuration.stdoutLog)}</string>\n    <key>StandardErrorPath</key>\n    <string>${escapeXml(configuration.stderrLog)}</string>\n  </dict>\n</plist>\n`;
};

export function smokeLaunchdDeployment(candidate) {
  const scratch = fs.realpathSync.native(fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-launchd-candidate-')));
  try {
    fs.chmodSync(scratch, 0o700);
    const stateDirectory = path.join(scratch, 'state');
    fs.mkdirSync(stateDirectory, { mode: 0o700 });
    fs.chmodSync(stateDirectory, 0o700);
    const configuration = {
      schema: 1,
      label: 'com.example.vellum.qualification',
      binary: candidate.artifactFiles[0],
      stateDirectory,
      dataSocket: path.join(stateDirectory, 'vellum.sock'),
      controlSocket: path.join(stateDirectory, 'vellum-control.sock'),
      lockFile: path.join(stateDirectory, '.vellum-daemon.lock'),
      metadataFile: path.join(stateDirectory, 'workspace-identity-v1'),
      stdoutLog: path.join(stateDirectory, 'stdout.log'),
      stderrLog: path.join(stateDirectory, 'stderr.log'),
      ownerUid: process.geteuid(),
    };
    const configFile = path.join(scratch, 'deployment.json');
    fs.writeFileSync(configFile, canonicalJson(configuration), { flag: 'wx', mode: 0o600 });
    fs.chmodSync(configFile, 0o600);
    const result = spawnSync(
      candidate.deploymentFiles[0],
      [`--config=${configFile}`],
      {
        encoding: 'utf8',
        env: { PATH: process.env.PATH ?? '' },
        timeout: SMOKE_TIMEOUT_MILLIS,
        maxBuffer: 128 * 1024,
      },
    );
    if (result.status !== 0 || result.signal !== null || result.error !== undefined) {
      fail('launchd renderer smoke failed');
    }
    const plist = result.stdout ?? '';
    if (plist !== expectedLaunchdProfile(configuration)) {
      fail('launchd renderer output is not the exact inert profile');
    }
    if ((result.stderr ?? '') !== '') fail('launchd renderer emitted unexpected diagnostics');
    const plistFile = path.join(scratch, 'candidate.plist');
    fs.writeFileSync(plistFile, plist, { flag: 'wx', mode: 0o600 });
    const lint = spawnSync('/usr/bin/plutil', ['-lint', plistFile], {
      encoding: 'utf8',
      timeout: SMOKE_TIMEOUT_MILLIS,
    });
    if (lint.status !== 0 || lint.signal !== null || lint.error !== undefined) {
      fail('launchd renderer output failed plist lint');
    }
    return {
      artifact: DEPLOYMENT_NAMES[0],
      runtime: 'native',
      invocation: 'private-absolute-deployment-record',
      exitCode: result.status,
      exactInertProfile: true,
      plistLint: 'passed',
    };
  } finally {
    fs.rmSync(scratch, { recursive: true, force: true });
  }
}

export function smokeLaunchdActivationExecutor(candidate) {
  const result = spawnSync(candidate.deploymentFiles[1], ['--help'], {
    encoding: 'utf8',
    env: { PATH: process.env.PATH ?? '' },
    timeout: SMOKE_TIMEOUT_MILLIS,
    maxBuffer: 128 * 1024,
  });
  const stdout = result.stdout ?? '';
  const stderr = result.stderr ?? '';
  if (
    result.status !== 0
    || result.signal !== null
    || result.error !== undefined
    || !stdout.includes('vellum-launchd-activate')
    || !stdout.includes('--execute-authorized-first-install')
    || !stdout.includes('--execute-authorized-activation')
    || !stdout.includes('--reconcile-first-install-journal')
    || stderr !== ''
  ) {
    fail('launchd activation executor help smoke failed');
  }
  return {
    artifact: DEPLOYMENT_NAMES[1],
    runtime: 'native',
    invocation: 'help-only',
    exitCode: result.status,
    explicitAuthorizationRequired: true,
    launchdEffect: 'not-attempted',
  };
}
