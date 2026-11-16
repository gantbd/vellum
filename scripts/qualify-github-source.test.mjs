import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
import {
  isolatedCargoEnvironment,
  requireCanonicalRegularFile,
  validateInstalledBinaryClosure,
} from './cargo-install-qualification-lib.mjs';

const fixture = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-source-qualifier-test-'));
const vellum = path.join(fixture, 'vellum');
const keystone = path.join(fixture, 'keystone');
const qualifier = path.resolve('scripts/qualify-github-source.mjs');
const vellumRepository = 'https://github.com/gantbd/vellum';
const keystoneRepository = 'https://github.com/gantbd/keystone';

const run = (command, args, cwd, env = process.env) => {
  const result = spawnSync(command, args, { cwd, env, encoding: 'utf8' });
  if (result.status !== 0) {
    process.stderr.write(result.stdout ?? '');
    process.stderr.write(result.stderr ?? '');
    throw new Error(`${command} failed`);
  }
  return (result.stdout ?? '').trim();
};
const write = (root, file, contents) => {
  const target = path.join(root, file);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, contents);
};
const initialize = (repository) => {
  run('git', ['init', '-q'], repository);
  run('git', ['config', 'user.name', 'Vellum qualification'], repository);
  run('git', ['config', 'user.email', 'qualification.invalid'], repository);
};
const commit = (repository) => {
  run('git', ['add', '.'], repository);
  run('git', ['commit', '-q', '-m', 'fixture'], repository);
  return run('git', ['rev-parse', 'HEAD'], repository);
};

try {
  fs.mkdirSync(vellum);
  fs.mkdirSync(keystone);
  initialize(keystone);
  write(
    keystone,
    'Cargo.toml',
    `[workspace]\nmembers = ["crates/keystone-codec", "crates/keystone-observe"]\nresolver = "3"\n\n[workspace.package]\nversion = "0.0.1"\nedition = "2024"\npublish = false\n`,
  );
  for (const name of ['keystone-codec', 'keystone-observe']) {
    write(
      keystone,
      `crates/${name}/Cargo.toml`,
      `[package]\nname = "${name}"\nversion.workspace = true\nedition.workspace = true\npublish.workspace = true\n\n[lib]\npath = "src/lib.rs"\n`,
    );
    write(keystone, `crates/${name}/src/lib.rs`, name === 'keystone-observe'
      ? 'mod histogram; mod saturation; pub use histogram::{FixedDurationHistogram, FixedDurationHistogramSnapshot};\n'
      : 'pub const QUALIFIED: bool = true;\n');
  }
  write(
    keystone,
    'crates/keystone-observe/src/histogram.rs',
    '#[derive(Clone, Copy)] pub struct FixedDurationHistogram<const N: usize>; #[derive(Clone, Copy)] pub struct FixedDurationHistogramSnapshot<const N: usize>;\n',
  );
  write(
    keystone,
    'crates/keystone-observe/src/saturation.rs',
    'pub struct SaturatingCounter;\n',
  );
  const keystoneRevision = commit(keystone);

  initialize(vellum);
  write(
    vellum,
    'Cargo.toml',
    `[workspace]\nmembers = ["applications/vellum-daemon", "distribution/cargo-install"]\nresolver = "3"\n\n[workspace.package]\nversion = "0.1.0"\nedition = "2024"\npublish = false\n`,
  );
  const packages = [
    ['applications/vellum-daemon', 'vellum-daemon', 'keystone-observe'],
  ];
  for (const [directory, name, dependency] of packages) {
    write(
      vellum,
      `${directory}/Cargo.toml`,
      `[package]\nname = "${name}"\nversion.workspace = true\nedition.workspace = true\npublish.workspace = true\n\n[[bin]]\nname = "${name}"\npath = "src/main.rs"\n\n[dependencies]\n${dependency} = { version = "=0.0.1", git = "${keystoneRepository}", rev = "${keystoneRevision}" }\n`,
    );
    write(
      vellum,
      `${directory}/src/main.rs`,
      `${dependency === 'keystone-observe' ? 'use keystone_observe::{FixedDurationHistogram, FixedDurationHistogramSnapshot}; const _: Option<FixedDurationHistogram<1>> = None; const _: Option<FixedDurationHistogramSnapshot<1>> = None; ' : ''}fn main() { let args: Vec<_> = std::env::args().skip(1).collect(); if args == ["--help"] { println!("${name} fixture help"); } else { eprintln!("usage: ${name} --help"); std::process::exit(2); } }\n`,
    );
    write(vellum, `${directory}/src/lib.rs`, `pub const PACKAGE: &str = "${name}";\n`);
  }
  write(
    vellum,
    'distribution/cargo-install/Cargo.toml',
    `[package]\nname = "vellum"\nversion.workspace = true\nedition.workspace = true\npublish.workspace = true\n\n[[bin]]\nname = "vellum-daemon"\npath = "src/bin/vellum-daemon.rs"\n\n[dependencies]\nvellum-daemon = { version = "=0.1.0", path = "../../applications/vellum-daemon" }\n`,
  );
  for (const name of ['vellum-daemon']) {
    const crate = name.replaceAll('-', '_');
    write(
      vellum,
      `distribution/cargo-install/src/bin/${name}.rs`,
      `fn main() { let args: Vec<_> = std::env::args().skip(1).collect(); let _ = ${crate}::PACKAGE; if args == ["--help"] { println!("${name} fixture help"); } else { eprintln!("usage: ${name} --help"); std::process::exit(2); } }\n`,
    );
  }
  const sourcePolicy = {
    schema: 1,
    channel: 'github-coordinated-source',
    installLockContract: 'workspace-build-and-git-install-locked; path-install-local-convenience',
    vellumRepository,
    keystone: {
      repository: keystoneRepository,
      revision: keystoneRevision,
      relativePath: '../keystone',
      crates: ['keystone-observe'],
      apiClosure: [
        {
          package: 'keystone-observe',
          status: 'qualified',
          consumers: ['vellum-daemon'],
          requiredSourceFiles: [
            'crates/keystone-observe/src/histogram.rs',
            'crates/keystone-observe/src/saturation.rs',
          ],
          requiredPublicItems: [
            'FixedDurationHistogram',
            'FixedDurationHistogramSnapshot',
          ],
          reason: 'fixture API contract',
        },
      ],
    },
    sourceInstallProfiles: [
      {
        package: 'vellum',
        path: 'distribution/cargo-install',
        binaries: ['vellum-daemon'],
        platform: 'unix',
        role: 'installation-only composition of daemon',
      },
    ],
    excludedPresentations: ['vellum-tui', 'vellum-gui-macos'],
    requiredArchiveFiles: ['Cargo.toml', 'Cargo.lock', 'release/github-source.json'],
  };
  write(vellum, 'release/github-source.json', `${JSON.stringify(sourcePolicy, null, 2)}\n`);
  const fixtureGitConfiguration = path.join(fixture, 'gitconfig');
  fs.writeFileSync(
    fixtureGitConfiguration,
    `[url "${pathToFileURL(keystone).href}"]\n\tinsteadOf = ${keystoneRepository}\n`,
  );
  const fixtureCargoHome = path.join(fixture, 'fixture-cargo-home');
  fs.mkdirSync(fixtureCargoHome);
  run('cargo', ['generate-lockfile'], vellum, {
    ...process.env,
    CARGO_HOME: fixtureCargoHome,
    GIT_CONFIG_GLOBAL: fixtureGitConfiguration,
    CARGO_NET_GIT_FETCH_WITH_CLI: 'true',
  });
  const vellumRevision = commit(vellum);

  const output = run(
    'node',
    [qualifier, `--vellum-ref=${vellumRevision}`],
    vellum,
    {
      ...process.env,
      KEYSTONE_SOURCE: keystone,
      CARGO_HOME: path.join(fixture, 'ambient-cargo-home-must-not-be-used'),
      CARGO_TARGET_DIR: path.join(fixture, 'ambient-target-must-not-be-used'),
      CARGO_INSTALL_ROOT: path.join(fixture, 'ambient-install-must-not-be-used'),
      CARGO_BUILD_TARGET: 'ambient-target-triple-must-not-be-used',
      RUSTFLAGS: '--cfg qualification_ambient_must_not_be_used',
      RUSTC_WRAPPER: path.join(fixture, 'ambient-wrapper-must-not-be-used'),
    },
  );
  if (!output.includes('1 immutable-Git install smoke binaries from 1 install package(s); workspace release build locked')) {
    throw new Error('qualifier did not report the daemon fixture binary');
  }
  if (!output.includes('Vellum and Keystone GitHub reachability were not verified')) {
    throw new Error('qualifier did not preserve the remote-retrievability nonclaim');
  }

  // Remote qualification must inspect both sovereign source repositories in
  // one pass. A first failure cannot hide a second blocker or leak transport
  // diagnostics into the maintained content-safe result.
  const policyPath = path.join(vellum, 'release/github-source.json');
  const compatiblePolicy = fs.readFileSync(policyPath, 'utf8');
  const reachablePolicy = JSON.parse(compatiblePolicy);
  reachablePolicy.vellumRepository = pathToFileURL(vellum).href;
  reachablePolicy.keystone.repository = pathToFileURL(keystone).href;
  fs.writeFileSync(policyPath, `${JSON.stringify(reachablePolicy, null, 2)}\n`);
  const reachable = run(
    'node', [qualifier, `--vellum-ref=${vellumRevision}`, '--verify-remote-only'],
    vellum, { ...process.env, KEYSTONE_SOURCE: keystone },
  );
  if (
    !reachable.includes('Vellum and Keystone exact remote commits reachable')
    || reachable.includes('workspace release build locked')
  ) {
    throw new Error('remote-only qualification did not stop after exact source reachability');
  }
  const unavailableVellum = path.join(fixture, 'unavailable-vellum.git');
  const unavailableKeystone = path.join(fixture, 'unavailable-keystone.git');
  run('git', ['init', '--bare', '-q', unavailableVellum], fixture);
  run('git', ['init', '--bare', '-q', unavailableKeystone], fixture);
  const unavailablePolicy = JSON.parse(compatiblePolicy);
  unavailablePolicy.vellumRepository = pathToFileURL(unavailableVellum).href;
  unavailablePolicy.keystone.repository = pathToFileURL(unavailableKeystone).href;
  fs.writeFileSync(policyPath, `${JSON.stringify(unavailablePolicy, null, 2)}\n`);
  try {
    const unavailable = spawnSync(
      'node', [qualifier, `--vellum-ref=${vellumRevision}`, '--verify-remote-only'],
      { cwd: vellum, env: { ...process.env, KEYSTONE_SOURCE: keystone }, encoding: 'utf8' },
    );
    const evidence = `${unavailable.stdout ?? ''}\n${unavailable.stderr ?? ''}`;
    if (
      unavailable.status === 0
      || !evidence.includes('GitHub remote did not return pinned commit(s): [vellum, keystone]')
      || evidence.includes('not our ref')
    ) {
      throw new Error('qualifier did not aggregate content-safe remote blockers');
    }
  } finally {
    fs.writeFileSync(policyPath, compatiblePolicy);
  }

  // A manifest at the pin is package-presence evidence, not API closure. Keep
  // both manifests while removing the required public surface and prove the
  // qualifier refuses the resulting pinned revision before any success claim.
  fs.rmSync(path.join(keystone, 'crates/keystone-observe/src/histogram.rs'));
  fs.rmSync(path.join(keystone, 'crates/keystone-observe/src/saturation.rs'));
  write(keystone, 'crates/keystone-observe/src/lib.rs', 'pub const MANIFEST_ONLY: bool = true;\n');
  const manifestOnlyRevision = commit(keystone);
  const manifestOnlyPolicy = JSON.parse(compatiblePolicy);
  manifestOnlyPolicy.keystone.revision = manifestOnlyRevision;
  fs.writeFileSync(policyPath, `${JSON.stringify(manifestOnlyPolicy, null, 2)}\n`);
  try {
    const manifestOnly = spawnSync(
      'node', [qualifier, `--vellum-ref=${vellumRevision}`],
      { cwd: vellum, env: { ...process.env, KEYSTONE_SOURCE: keystone }, encoding: 'utf8' },
    );
    const evidence = `${manifestOnly.stdout ?? ''}\n${manifestOnly.stderr ?? ''}`;
    if (
      manifestOnly.status === 0
      || !evidence.includes('manifest does not close required API for keystone-observe')
    ) {
      throw new Error('qualifier treated pinned manifest presence as API closure');
    }
  } finally {
    fs.writeFileSync(policyPath, compatiblePolicy);
  }

  for (const ambient of [
    'ambient-cargo-home-must-not-be-used',
    'ambient-target-must-not-be-used',
    'ambient-install-must-not-be-used',
  ]) {
    if (fs.existsSync(path.join(fixture, ambient))) {
      throw new Error(`qualifier consumed ambient path ${ambient}`);
    }
  }

  const adversarial = path.join(fixture, 'adversarial');
  fs.mkdirSync(path.join(adversarial, 'bin'), { recursive: true });
  const executable = path.join(adversarial, 'bin', 'vellum-daemon');
  fs.writeFileSync(executable, 'fixture', { mode: 0o555 });
  validateInstalledBinaryClosure(adversarial, ['vellum-daemon']);
  for (const [label, mutate] of [
    ['missing binary', () => fs.renameSync(executable, `${executable}.missing`)],
    ['extra binary', () => fs.writeFileSync(path.join(adversarial, 'bin', 'vellum-tui'), 'fixture', { mode: 0o555 })],
    ['writable binary', () => fs.chmodSync(executable, 0o775)],
  ]) {
    const reset = () => {
      fs.rmSync(path.join(adversarial, 'bin'), { recursive: true, force: true });
      fs.mkdirSync(path.join(adversarial, 'bin'));
      fs.writeFileSync(executable, 'fixture', { mode: 0o555 });
    };
    reset();
    mutate();
    let rejected = false;
    try {
      validateInstalledBinaryClosure(adversarial, ['vellum-daemon']);
    } catch {
      rejected = true;
    }
    if (!rejected) throw new Error(`qualifier accepted ${label}`);
  }
  fs.chmodSync(executable, 0o555);
  fs.rmSync(executable);
  fs.symlinkSync(path.join(vellum, 'Cargo.toml'), executable);
  let rejectedSymlink = false;
  try {
    validateInstalledBinaryClosure(adversarial, ['vellum-daemon']);
  } catch {
    rejectedSymlink = true;
  }
  if (!rejectedSymlink) throw new Error('qualifier accepted a symlink binary');

  let rejectedEscape = false;
  try {
    requireCanonicalRegularFile(adversarial, path.join(vellum, 'Cargo.toml'), 'escape probe');
  } catch {
    rejectedEscape = true;
  }
  if (!rejectedEscape) throw new Error('qualifier accepted a noncanonical payload escape');

  const environmentProbe = path.join(fixture, 'environment-probe');
  fs.mkdirSync(environmentProbe);
  const isolated = isolatedCargoEnvironment(environmentProbe, {
    PATH: process.env.PATH,
    CARGO_HOME: '/ambient/cargo',
    CARGO_TARGET_DIR: '/ambient/target',
    CARGO_INSTALL_ROOT: '/ambient/install',
    CARGO_BUILD_TARGET: 'ambient-target',
    RUSTFLAGS: '--cfg ambient',
    RUSTC_WRAPPER: '/ambient/wrapper',
  });
  if (
    isolated.environment.CARGO_HOME === '/ambient/cargo'
    || isolated.environment.CARGO_TARGET_DIR === '/ambient/target'
    || 'CARGO_INSTALL_ROOT' in isolated.environment
    || 'CARGO_BUILD_TARGET' in isolated.environment
    || 'RUSTFLAGS' in isolated.environment
    || 'RUSTC_WRAPPER' in isolated.environment
  ) throw new Error('isolated Cargo environment retained ambient build authority');
  process.stdout.write('Vellum GitHub source qualifier fixture passed\n');
} finally {
  fs.rmSync(fixture, { recursive: true, force: true });
}
