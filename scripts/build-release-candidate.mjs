import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { ARTIFACT_NAMES, DEPLOYMENT_NAMES, sha256File } from './release-candidate-lib.mjs';

process.on('uncaughtException', (error) => {
  process.stderr.write(`Vellum release candidate: ${error.message}\n`);
  process.exit(1);
});

const root = process.cwd();
const argumentsByName = new Map();
for (const argument of process.argv.slice(2)) {
  const separator = argument.indexOf('=');
  if (separator < 3 || !argument.startsWith('--')) {
    throw new Error(`unsupported argument: ${argument}`);
  }
  argumentsByName.set(argument.slice(2, separator), argument.slice(separator + 1));
}
for (const name of argumentsByName.keys()) {
  if (!['output-dir', 'vellum-revision'].includes(name)) {
    throw new Error(`unsupported argument: --${name}`);
  }
}
if (!argumentsByName.has('output-dir')) {
  throw new Error('--output-dir is required');
}

const run = (command, args, options = {}) => {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? root,
    env: options.env ?? process.env,
    encoding: 'utf8',
  });
  if (result.status !== 0) {
    if (result.stdout) process.stderr.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);
    throw new Error(`${command} failed with status ${result.status ?? 'unknown'}`);
  }
  return (result.stdout ?? '').trim();
};
const resolveCommit = (repository, revision) =>
  run('git', ['-C', repository, 'rev-parse', `${revision}^{commit}`]);
const requireClean = (repository, label) => {
  if (run('git', ['-C', repository, 'status', '--porcelain', '--untracked-files=all']) !== '') {
    throw new Error(`${label} checkout must be clean`);
  }
};
const sha256 = sha256File;

const policy = JSON.parse(fs.readFileSync(path.join(root, 'release/github-source.json'), 'utf8'));
const keystone = path.resolve(root, process.env.KEYSTONE_SOURCE ?? policy.keystone.relativePath);
const vellumRevision = resolveCommit(
  root,
  argumentsByName.get('vellum-revision') ?? 'HEAD',
);
if (vellumRevision !== resolveCommit(root, 'HEAD')) {
  throw new Error('Vellum HEAD does not match the requested revision');
}
if (resolveCommit(keystone, 'HEAD') !== policy.keystone.revision) {
  throw new Error('Keystone HEAD does not match the coordinated source revision');
}
requireClean(root, 'Vellum');
requireClean(keystone, 'Keystone');

const candidate = policy.releaseCandidate ?? {};
const candidatePackages = candidate.packages ?? [];
const installProfiles = policy.sourceInstallProfiles ?? [];
const profiles = candidatePackages.map((binary) => {
  const owners = installProfiles.filter((profile) => {
    const binaries = profile.binaries ?? (profile.binary === undefined ? [] : [profile.binary]);
    return binaries.includes(binary);
  });
  return owners.length === 1 ? { ...owners[0], binary } : undefined;
});
if (
  candidate.platform !== 'macos'
  || candidate.publishing !== 'none'
  || candidate.signing !== 'none'
  || candidate.attestation !== 'none'
  || candidatePackages.join(',') !== 'vellum-daemon,vellum-mcp'
  || profiles.some((profile) => profile === undefined)
) {
  throw new Error('release-candidate scope must be exactly daemon and MCP');
}
if (profiles.some((profile) => profile.platform === 'macos') && process.platform !== 'darwin') {
  throw new Error('the coordinated daemon release candidate requires macOS');
}

const output = path.resolve(root, argumentsByName.get('output-dir'));
if (fs.existsSync(output)) throw new Error('output directory already exists');
fs.mkdirSync(output, { recursive: true, mode: 0o700 });

const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-release-candidate-'));
try {
  const rustc = run('rustc', ['-vV']);
  const host = rustc.match(/^host: (.+)$/m)?.[1];
  const rustcRelease = rustc.match(/^release: (.+)$/m)?.[1];
  if (!host || !rustcRelease) throw new Error('rustc did not report a host and release');
  const cargoVersion = run('cargo', ['--version']);

  const buildEnvironment = { ...process.env };
  delete buildEnvironment.RUSTFLAGS;
  delete buildEnvironment.CARGO_BUILD_TARGET;
  buildEnvironment.CARGO_ENCODED_RUSTFLAGS = [
    `--remap-path-prefix=${root}=vellum`,
    `--remap-path-prefix=${keystone}=keystone`,
  ].join('\u001f');
  const target = path.join(temporary, 'target');
  const cargoArguments = [
    'build', '--locked', '--release', '--target', host, '--target-dir', target,
  ];
  for (const packageName of new Set(profiles.map((profile) => profile.package))) {
    cargoArguments.push('--package', packageName);
  }
  for (const name of DEPLOYMENT_NAMES) cargoArguments.push('--package', name);
  run('cargo', cargoArguments, { env: buildEnvironment });

  const artifacts = [];
  for (const profile of profiles) {
    const built = path.join(target, host, 'release', profile.binary);
    if (!fs.statSync(built).isFile()) throw new Error(`${profile.binary} was not built`);
    const staged = path.join(output, profile.binary);
    fs.copyFileSync(built, staged);
    fs.chmodSync(staged, 0o755);
    const help = run(staged, ['--help']);
    if (!help.includes(profile.binary)) {
      throw new Error(`${profile.binary} staged help output is not self-identifying`);
    }
    artifacts.push({
      package: profile.package,
      binary: profile.binary,
      file: profile.binary,
      bytes: fs.statSync(staged).size,
      sha256: sha256(staged),
      ...(profile.binary === 'vellum-mcp'
        ? { role: 'peer connector to an already-running daemon' }
        : {}),
    });
  }
  const deployment = DEPLOYMENT_NAMES.map((name, index) => {
    const source = path.join(target, host, 'release', name);
    const staged = path.join(output, name);
    fs.copyFileSync(source, staged);
    fs.chmodSync(staged, 0o755);
    const help = run(staged, ['--help']);
    if (!help.includes(name)) throw new Error(`${name} staged help output is not self-identifying`);
    return {
      file: name,
      bytes: fs.statSync(staged).size,
      sha256: sha256(staged),
      role: index === 0
        ? 'inert-launchd-plist-renderer'
        : 'explicitly-authorized-launchd-activation-executor',
      runtime: 'native',
    };
  });

  const manifest = {
    schema: 5,
    channel: 'github-release-candidate-artifact',
    claims: [
      'locked-workspace-build',
      'native-help-smoke',
      'sha256-checksums',
      'effective-build-configuration',
      'build-recipe-sha256',
      'inert-launchd-renderer',
      'activation-executor-packaged',
    ],
    nonClaims: [
      'signature', 'attestation', 'github-release', 'crates.io-publication', 'activation',
      'launchd-installation', 'launchd-socket-activation', 'authenticated-health',
    ],
    source: {
      vellum: { repository: policy.vellumRepository, revision: vellumRevision },
      keystone: {
        repository: policy.keystone.repository,
        revision: policy.keystone.revision,
      },
    },
    build: {
      platform: 'macos',
      target: host,
      targetSelection: 'cargo --target',
      profile: 'release',
      locked: true,
      rustc: `rustc ${rustcRelease}`,
      cargo: cargoVersion,
      packages: ['vellum', ...DEPLOYMENT_NAMES],
      features: 'default',
      environment: {
        CARGO_BUILD_TARGET: 'unset',
        RUSTFLAGS: 'unset',
        CARGO_ENCODED_RUSTFLAGS: 'path-remap-only',
      },
    },
    inputs: Object.fromEntries(
      [
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
      ].map((name) => [name, sha256(path.join(root, name))]),
    ),
    artifacts,
    deployment,
  };
  const serialized = `${JSON.stringify(manifest, null, 2)}\n`;
  for (const forbidden of [root, keystone, os.homedir()]) {
    if (forbidden !== '/' && serialized.includes(forbidden)) {
      throw new Error('candidate manifest contains a local filesystem identity');
    }
  }
  const manifestPath = path.join(output, 'manifest.json');
  fs.writeFileSync(manifestPath, serialized, { mode: 0o600 });
  const checksums = [
    ...artifacts.map((artifact) => `${artifact.sha256}  ${artifact.file}`),
    ...deployment.map((artifact) => `${artifact.sha256}  ${artifact.file}`),
    `${sha256(manifestPath)}  manifest.json`,
  ];
  fs.writeFileSync(path.join(output, 'SHA256SUMS'), `${checksums.join('\n')}\n`, {
    mode: 0o600,
  });
  process.stdout.write(
    `Vellum release candidate staged: ${vellumRevision}; ${artifacts.length} binaries; unsigned and unattested\n`,
  );
} catch (error) {
  fs.rmSync(output, { recursive: true, force: true });
  throw error;
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
