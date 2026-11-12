import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { validateCandidate } from './release-candidate-lib.mjs';

const fixture = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-candidate-test-'));
const vellum = path.join(fixture, 'vellum');
const keystone = path.join(fixture, 'keystone');
const output = path.join(fixture, 'candidate');
const builder = path.resolve('scripts/build-release-candidate.mjs');

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
  fs.mkdirSync(repository);
  run('git', ['init', '-q'], repository);
  run('git', ['config', 'user.name', 'Vellum qualification'], repository);
  run('git', ['config', 'user.email', 'qualification.invalid'], repository);
};
const commit = (repository) => {
  run('git', ['add', '.'], repository);
  run('git', ['commit', '-q', '-m', 'fixture'], repository);
  return run('git', ['rev-parse', 'HEAD'], repository);
};
const sha256 = (file) => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');

try {
  initialize(keystone);
  write(
    keystone,
    'Cargo.toml',
    '[workspace]\nmembers = ["crates/keystone-codec", "crates/keystone-observe"]\nresolver = "3"\n',
  );
  for (const name of ['keystone-codec', 'keystone-observe']) {
    write(
      keystone,
      `crates/${name}/Cargo.toml`,
      `[package]\nname = "${name}"\nversion = "0.0.1"\nedition = "2024"\npublish = false\n[lib]\npath = "src/lib.rs"\n`,
    );
    write(keystone, `crates/${name}/src/lib.rs`, 'pub const QUALIFIED: bool = true;\n');
  }
  const keystoneRevision = commit(keystone);

  initialize(vellum);
  write(
    vellum,
    'Cargo.toml',
    '[workspace]\nmembers = ["applications/vellum-daemon", "connectors/vellum-mcp", "distribution/launchd-render"]\nresolver = "3"\n',
  );
  const profiles = [
    ['applications/vellum-daemon', 'vellum-daemon', 'keystone-observe'],
    ['connectors/vellum-mcp', 'vellum-mcp', 'keystone-codec'],
  ];
  for (const [directory, name, dependency] of profiles) {
    write(
      vellum,
      `${directory}/Cargo.toml`,
      `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\npublish = false\n[[bin]]\nname = "${name}"\npath = "src/main.rs"\n[dependencies]\n${dependency} = { path = "../../../keystone/crates/${dependency}" }\n`,
    );
    write(
      vellum,
      `${directory}/src/main.rs`,
      `fn main() { println!("${name} --help"); }\n`,
    );
  }
  write(
    vellum,
    'distribution/launchd-render/Cargo.toml',
    '[package]\nname = "vellum-launchd-render"\nversion = "0.1.0"\nedition = "2024"\npublish = false\n[[bin]]\nname = "vellum-launchd-render"\npath = "src/main.rs"\n',
  );
  write(vellum, 'distribution/launchd-render/src/lib.rs', 'pub const INERT: bool = true;\n');
  write(
    vellum,
    'distribution/launchd-render/src/main.rs',
    'fn main() { println!("vellum-launchd-render --help"); }\n',
  );
  write(
    vellum,
    'release/github-source.json',
    `${JSON.stringify({
      schema: 1,
      channel: 'github-coordinated-source',
      vellumRepository: 'https://github.com/example/vellum',
      keystone: {
        repository: 'https://github.com/example/keystone',
        revision: keystoneRevision,
        relativePath: '../keystone',
        crates: ['keystone-codec', 'keystone-observe'],
      },
      sourceInstallProfiles: [
        {
          package: 'vellum-daemon',
          path: 'applications/vellum-daemon',
          binary: 'vellum-daemon',
          platform: 'macos',
        },
        {
          package: 'vellum-mcp',
          path: 'connectors/vellum-mcp',
          binary: 'vellum-mcp',
          platform: 'unix',
          role: 'peer connector to an already-running daemon',
        },
      ],
      releaseCandidate: {
        workflow: '.github/workflows/release-candidate.yml',
        platform: 'macos',
        packages: ['vellum-daemon', 'vellum-mcp'],
        publishing: 'none',
        signing: 'none',
        attestation: 'none',
      },
    }, null, 2)}\n`,
  );
  for (const [index, name] of [
    'release/compatibility-baseline.json',
    'release/dependency-policy.json',
    'release/source-sbom.manifest.json',
  ].entries()) {
    write(vellum, name, `qualified build input ${index}\n`);
  }
  write(vellum, 'rust-toolchain.toml', '[toolchain]\nchannel = "1.95.0"\n');
  run('cargo', ['generate-lockfile', '--offline'], vellum);
  const vellumRevision = commit(vellum);

  const report = run(
    'node',
    [builder, `--vellum-revision=${vellumRevision}`, `--output-dir=${output}`],
    vellum,
    { ...process.env, KEYSTONE_SOURCE: keystone },
  );
  if (!report.includes('2 binaries; unsigned and unattested')) {
    throw new Error('builder did not preserve its signature and attestation nonclaims');
  }
  const manifestText = fs.readFileSync(path.join(output, 'manifest.json'), 'utf8');
  if (manifestText.includes(fixture)) throw new Error('manifest disclosed the fixture path');
  const manifest = JSON.parse(manifestText);
  if (
    manifest.schema !== 3
    || manifest.claims.at(-1) !== 'inert-launchd-renderer'
    || !manifest.nonClaims.includes('activation')
  ) {
    throw new Error('builder omitted the lifecycle schema, configuration claim, or activation nonclaim');
  }
  if (manifest.source.vellum.revision !== vellumRevision) {
    throw new Error('manifest did not record the exact Vellum revision');
  }
  validateCandidate(output, { repositoryRoot: vellum, runtimeTarget: manifest.build.target });
  if (manifest.source.keystone.revision !== keystoneRevision) {
    throw new Error('manifest did not record the exact Keystone revision');
  }
  if (manifest.artifacts.map((artifact) => artifact.binary).join(',') !== 'vellum-daemon,vellum-mcp') {
    throw new Error('candidate artifact scope drifted');
  }
  if (manifest.deployment[0].file !== 'vellum-launchd-render' || manifest.deployment[0].role !== 'inert-launchd-plist-renderer') {
    throw new Error('candidate omitted the launchd deployment payload');
  }
  const checksumLines = fs.readFileSync(path.join(output, 'SHA256SUMS'), 'utf8').trim().split('\n');
  for (const line of checksumLines) {
    const match = line.match(/^([0-9a-f]{64})  ([A-Za-z0-9.-]+)$/);
    if (!match || sha256(path.join(output, match[2])) !== match[1]) {
      throw new Error('candidate checksum evidence is invalid');
    }
  }
  process.stdout.write('Vellum release-candidate builder fixture passed\n');
} finally {
  fs.rmSync(fixture, { recursive: true, force: true });
}
