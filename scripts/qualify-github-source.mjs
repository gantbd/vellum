import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
import {
  assertContentSafeSmoke,
  isolatedCargoEnvironment,
  requireCanonicalRegularFile,
  validateInstalledBinaryClosure,
} from './cargo-install-qualification-lib.mjs';

process.on('uncaughtException', (error) => {
  process.stderr.write(`Vellum GitHub source qualification: ${error.message}\n`);
  process.exit(1);
});

const args = new Set(process.argv.slice(2));
const unknown = [...args].filter(
  (argument) => !['--verify-remote', '--verify-remote-only'].includes(argument)
    && !argument.startsWith('--vellum-ref='),
);
if (unknown.length !== 0) {
  process.stderr.write(`unknown argument: ${unknown[0]}\n`);
  process.exit(2);
}
const vellumRef =
  [...args].find((argument) => argument.startsWith('--vellum-ref='))?.slice(13) ?? 'HEAD';
const verifyRemoteOnly = args.has('--verify-remote-only');
if (verifyRemoteOnly && args.has('--verify-remote')) {
  process.stderr.write('remote verification modes are mutually exclusive\n');
  process.exit(2);
}
const verifyRemote = args.has('--verify-remote') || verifyRemoteOnly;

const profileBinaries = (profile) => {
  const binaries = profile.binaries ?? (profile.binary === undefined ? [] : [profile.binary]);
  if (
    !Array.isArray(binaries)
    || binaries.length === 0
    || binaries.some((binary) => typeof binary !== 'string' || binary.length === 0)
    || new Set(binaries).size !== binaries.length
  ) {
    throw new Error(`invalid source-install binary closure for ${profile.package ?? 'unknown package'}`);
  }
  return binaries;
};

const root = process.cwd();
const policyPath = path.join(root, 'release/github-source.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const keystoneSource = path.resolve(
  root,
  process.env.KEYSTONE_SOURCE ?? policy.keystone.relativePath,
);

const run = (command, commandArgs, options = {}) => {
  const result = spawnSync(command, commandArgs, {
    cwd: options.cwd ?? root,
    encoding: 'utf8',
    env: options.env ?? process.env,
    stdio: options.capture === false ? 'inherit' : 'pipe',
  });
  if (result.status !== 0) {
    if (result.stdout) process.stderr.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);
    throw new Error(`${command} failed with status ${result.status ?? 'unknown'}`);
  }
  return (result.stdout ?? '').trim();
};

const runExpectingFailure = (command, commandArgs, expectedText, options = {}) => {
  const result = spawnSync(command, commandArgs, {
    cwd: options.cwd ?? root,
    encoding: 'utf8',
    env: options.env ?? process.env,
  });
  if (result.status === 0) throw new Error(`${command} unexpectedly succeeded`);
  const output = `${result.stdout ?? ''}\n${result.stderr ?? ''}`;
  if (!output.includes(expectedText)) {
    throw new Error(`${command} failed without the expected ${expectedText} evidence`);
  }
};

const requireCommit = (repository, revision, label) => {
  const resolved = run('git', ['-C', repository, 'rev-parse', `${revision}^{commit}`]);
  if (!/^[0-9a-f]{40}$/.test(resolved)) throw new Error(`${label} did not resolve to a commit`);
  return resolved;
};

const requirePinnedKeystoneClosure = (repository, revision, crates, apiClosure) => {
  const absent = [];
  for (const crate of crates) {
    const probe = spawnSync(
      'git',
      ['-C', repository, 'cat-file', '-e', `${revision}:crates/${crate}/Cargo.toml`],
      { encoding: 'utf8' },
    );
    if (probe.status !== 0) absent.push(crate);
  }
  const apiBlockers = [];
  for (const contract of apiClosure ?? []) {
    if (
      typeof contract.package !== 'string'
      || !['blocker', 'qualified'].includes(contract.status)
      || !Array.isArray(contract.requiredSourceFiles)
      || contract.requiredSourceFiles.length === 0
      || !Array.isArray(contract.requiredPublicItems)
      || contract.requiredPublicItems.length === 0
    ) {
      throw new Error('invalid pinned Keystone API-closure contract');
    }
    if (!crates.includes(contract.package)) {
      throw new Error(`API-closure contract names undeclared crate ${contract.package}`);
    }
    const missingFiles = contract.requiredSourceFiles.filter((file) => spawnSync(
      'git', ['-C', repository, 'cat-file', '-e', `${revision}:${file}`], { encoding: 'utf8' },
    ).status !== 0);
    const missingItems = contract.requiredPublicItems.filter((item) => {
      const probe = spawnSync(
        'git', ['-C', repository, 'grep', '-F', '-e', item, revision, '--', `crates/${contract.package}/src`],
        { encoding: 'utf8' },
      );
      return probe.status !== 0;
    });
    if (contract.status === 'blocker') {
      apiBlockers.push(contract.package);
    } else if (missingFiles.length !== 0 || missingItems.length !== 0) {
      throw new Error(
        `pinned Keystone manifest does not close required API for ${contract.package}: `
        + `missing source files [${missingFiles.join(', ')}], public items [${missingItems.join(', ')}]`,
      );
    }
  }
  if (absent.length !== 0 || apiBlockers.length !== 0) {
    const remotes = spawnSync('git', ['-C', repository, 'remote', '-v'], { encoding: 'utf8' });
    const remoteEvidence = remotes.status === 0 && (remotes.stdout ?? '').trim().length !== 0
      ? ''
      : '; the local Keystone checkout also has no configured remote, so pinned-object GitHub retrievability is unverified';
    throw new Error(
      `pinned Keystone revision does not close Vellum: missing required crate(s) [${absent.join(', ')}], `
      + `API-closure blocker(s) [${apiBlockers.join(', ')}]; `
      + `the coordinated GitHub source and cargo-install contract remains blocked${remoteEvidence}`,
    );
  }
};

const vellumRevision = requireCommit(root, vellumRef, 'Vellum revision');
const keystoneRevision = requireCommit(
  keystoneSource,
  policy.keystone.revision,
  'Keystone revision',
);
if (keystoneRevision !== policy.keystone.revision) {
  throw new Error('Keystone revision did not resolve to the manifest commit');
}
requirePinnedKeystoneClosure(
  keystoneSource,
  keystoneRevision,
  policy.keystone.crates,
  policy.keystone.apiClosure,
);

const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-github-source-'));
try {
  if (verifyRemote) {
    const unavailable = [];
    for (const [name, repository, revision] of [
      ['vellum', policy.vellumRepository, vellumRevision],
      ['keystone', policy.keystone.repository, policy.keystone.revision],
    ]) {
      const remoteProbe = path.join(temporary, `${name}-remote.git`);
      run('git', ['init', '--bare', remoteProbe]);
      const fetch = spawnSync(
        'git', ['-C', remoteProbe, 'fetch', '--depth=1', repository, revision],
        { cwd: root, encoding: 'utf8', env: process.env },
      );
      if (fetch.status !== 0) {
        unavailable.push(name);
        continue;
      }
      const fetched = run('git', ['-C', remoteProbe, 'rev-parse', 'FETCH_HEAD']);
      if (fetched !== revision) {
        unavailable.push(name);
      }
    }
    if (unavailable.length !== 0) {
      throw new Error(
        `GitHub remote did not return pinned commit(s): [${unavailable.join(', ')}]`,
      );
    }
  }

  qualification: {
    if (verifyRemoteOnly) {
      process.stdout.write(
        `Vellum and Keystone exact remote commits reachable: vellum ${vellumRevision}, keystone ${keystoneRevision}\n`,
      );
      break qualification;
    }
  const bundle = path.join(temporary, 'bundle');
  fs.mkdirSync(bundle);
  const vellumArchive = path.join(temporary, 'vellum.tar');
  run('git', [
    '-C', root, 'archive', '--format=tar', '--prefix=vellum/',
    `--output=${vellumArchive}`, vellumRevision,
  ]);
  run('tar', ['-xf', vellumArchive, '-C', bundle]);

  const vellum = path.join(bundle, 'vellum');
  for (const required of policy.requiredArchiveFiles) {
    const candidate = path.join(vellum, required);
    if (!fs.existsSync(candidate)) {
      throw new Error(`Vellum archive is missing required file ${required}`);
    }
    requireCanonicalRegularFile(vellum, candidate, `required archive file ${required}`);
  }

  const metadata = JSON.parse(
    run('cargo', ['metadata', '--offline', '--locked', '--no-deps', '--format-version', '1'], {
      cwd: vellum,
    }),
  );
  if (!metadata.packages.every((pkg) => Array.isArray(pkg.publish) && pkg.publish.length === 0)) {
    throw new Error('an archived Vellum workspace package permits registry publication');
  }
  for (const presentation of policy.excludedPresentations) {
    if (metadata.packages.some((pkg) => pkg.name === presentation)) {
      throw new Error(`${presentation} must remain outside the Vellum source release unit`);
    }
  }

  const expectedKeystone = new Set(policy.keystone.crates);
  const observedKeystone = new Set();
  const expectedKeystoneSource = `git+${policy.keystone.repository}?rev=${keystoneRevision}`;
  for (const pkg of metadata.packages) {
    for (const dependency of pkg.dependencies) {
      if (!dependency.name.startsWith('keystone-')) continue;
      observedKeystone.add(dependency.name);
      if (dependency.path != null || dependency.source !== expectedKeystoneSource) {
        throw new Error(`${pkg.name} does not pin ${dependency.name} to the qualified Git revision`);
      }
    }
  }
  if (
    observedKeystone.size !== expectedKeystone.size
    || [...observedKeystone].some((name) => !expectedKeystone.has(name))
  ) {
    throw new Error('the archived Keystone dependency set drifted from the source manifest');
  }

  const installRoot = path.join(temporary, 'install');
  fs.mkdirSync(installRoot, { mode: 0o700 });
  const { environment: cargoEnvironment, cargoHome, target } = isolatedCargoEnvironment(temporary);
  const gitConfiguration = path.join(temporary, 'gitconfig');
  fs.writeFileSync(
    gitConfiguration,
    `[url "${pathToFileURL(root).href}"]\n\tinsteadOf = ${policy.vellumRepository}\n`
      + `[url "${pathToFileURL(keystoneSource).href}"]\n\tinsteadOf = ${policy.keystone.repository}\n`,
    { mode: 0o600 },
  );
  cargoEnvironment.GIT_CONFIG_GLOBAL = gitConfiguration;
  cargoEnvironment.CARGO_NET_GIT_FETCH_WITH_CLI = 'true';
  const fetchEnvironment = { ...cargoEnvironment };
  delete fetchEnvironment.CARGO_NET_OFFLINE;
  run('cargo', ['fetch', '--locked'], { cwd: vellum, env: fetchEnvironment });
  const gitSeed = path.join(temporary, 'git-seed');
  fs.mkdirSync(path.join(gitSeed, 'src'), { recursive: true });
  fs.writeFileSync(
    path.join(gitSeed, 'Cargo.toml'),
    `[package]\nname = "vellum-git-seed"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n`
      + `[dependencies]\nvellum-daemon = { git = ${JSON.stringify(policy.vellumRepository)}, rev = ${JSON.stringify(vellumRevision)} }\n`,
  );
  fs.writeFileSync(path.join(gitSeed, 'src/lib.rs'), 'pub const PRESENT: bool = true;\n');
  run('cargo', ['fetch'], { cwd: gitSeed, env: fetchEnvironment });
  const controlProfile = policy.sourceInstallProfiles[0];
  const controlManifest = path.join(vellum, controlProfile.path, 'Cargo.toml');
  const controlDependency = path.join(temporary, 'qualification-lock-control');
  fs.mkdirSync(path.join(controlDependency, 'src'), { recursive: true });
  fs.writeFileSync(
    path.join(controlDependency, 'Cargo.toml'),
    '[package]\nname = "qualification-lock-control"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n',
  );
  fs.writeFileSync(path.join(controlDependency, 'src/lib.rs'), 'pub const PRESENT: bool = true;\n');
  const originalManifest = fs.readFileSync(controlManifest, 'utf8');
  const driftedManifest = originalManifest.replace(
    /^\[dependencies\]\s*$/m,
    `[dependencies]\nqualification-lock-control = { path = ${JSON.stringify(controlDependency)} }`,
  );
  if (driftedManifest === originalManifest) {
    throw new Error('could not construct the workspace-lock drift control');
  }
  fs.writeFileSync(controlManifest, driftedManifest);
  try {
    runExpectingFailure(
      'cargo',
      [
        'build', '--offline', '--locked', '--package', controlProfile.package,
        '--target-dir', path.join(temporary, 'lock-control-target'),
      ],
      'lock file',
      { cwd: vellum, env: cargoEnvironment },
    );
  } finally {
    fs.writeFileSync(controlManifest, originalManifest);
  }

  const lockedBuild = ['build', '--offline', '--locked', '--release', '--target-dir', target];
  for (const profile of policy.sourceInstallProfiles) {
    lockedBuild.push('--package', profile.package);
  }
  run('cargo', lockedBuild, { cwd: vellum, capture: false, env: cargoEnvironment });

  const installed = [];
  for (const profile of policy.sourceInstallProfiles) {
    if (profile.platform === 'macos' && process.platform !== 'darwin') {
      throw new Error(`${profile.package} source-install qualification requires macOS`);
    }
    run(
      'cargo',
      [
        'install', '--offline', '--locked', '--git', policy.vellumRepository,
        '--rev', vellumRevision, profile.package,
        '--root', installRoot, '--target-dir', target,
      ],
      { cwd: temporary, capture: false, env: cargoEnvironment },
    );
    const binaries = profileBinaries(profile);
    installed.push(...binaries);
    for (const binaryName of binaries) {
      const binary = path.join(installRoot, 'bin', binaryName);
      const help = run(binary, ['--help'], { cwd: temporary, env: cargoEnvironment });
      if (!help.includes(binaryName)) {
        throw new Error(`${binaryName} installed help output is not self-identifying`);
      }
      fs.chmodSync(binary, 0o555);
    }
    validateInstalledBinaryClosure(installRoot, installed);
    for (const binaryName of binaries) {
      const binary = path.join(installRoot, 'bin', binaryName);
      const startup = spawnSync(binary, [], {
        cwd: temporary,
        encoding: 'utf8',
        env: cargoEnvironment,
      });
      assertContentSafeSmoke(startup, binaryName, [
        temporary,
        fs.realpathSync(temporary),
        vellum,
        fs.realpathSync(vellum),
        cargoHome,
        target,
      ]);
    }
  }

  const installedBinaryCount = policy.sourceInstallProfiles
    .map(profileBinaries)
    .reduce((count, binaries) => count + binaries.length, 0);

  process.stdout.write(
    `Vellum coordinated GitHub source qualified: vellum ${vellumRevision}, keystone ${keystoneRevision}, ${installedBinaryCount} immutable-Git install smoke binaries from ${policy.sourceInstallProfiles.length} install package(s); workspace release build locked\n`,
  );
  if (!verifyRemote) {
    process.stdout.write(
      'Vellum and Keystone GitHub reachability were not verified; rerun with --verify-remote before claiming a remotely reproducible bundle\n',
    );
  }
  }
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
