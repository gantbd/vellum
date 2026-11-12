import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-clean-candidate-test-'));
const vellum = path.join(scratch, 'vellum');
const keystone = path.join(scratch, 'keystone');
const output = path.join(scratch, 'output');
const qualifier = path.resolve('scripts/qualify-clean-release-candidate.mjs');

const run = (command, args, cwd, env = process.env) => {
  const result = spawnSync(command, args, { cwd, env, encoding: 'utf8' });
  if (result.status !== 0) {
    process.stderr.write(result.stdout ?? '');
    process.stderr.write(result.stderr ?? '');
    throw new Error(`${command} failed`);
  }
  return (result.stdout ?? '').trim();
};
const write = (root, name, contents) => {
  const file = path.join(root, ...name.split('/'));
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, contents);
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
const copyScript = (name) => {
  write(vellum, `scripts/${name}`, fs.readFileSync(path.resolve('scripts', name), 'utf8'));
};

try {
  initialize(keystone);
  write(keystone, 'README.md', 'exact local Keystone fixture\n');
  const keystoneRevision = commit(keystone);

  initialize(vellum);
  write(
    vellum,
    'Cargo.toml',
    '[workspace]\nmembers = ["distribution/cargo-install", "distribution/launchd-render", "distribution/launchd-activate"]\nresolver = "3"\n',
  );
  write(
    vellum,
    'distribution/cargo-install/Cargo.toml',
    '[package]\nname = "vellum"\nversion = "0.1.0"\nedition = "2024"\npublish = false\n'
      + '[[bin]]\nname = "vellum-daemon"\npath = "src/bin/vellum-daemon.rs"\n'
      + '[[bin]]\nname = "vellum-mcp"\npath = "src/bin/vellum-mcp.rs"\n',
  );
  for (const name of ['vellum-daemon', 'vellum-mcp']) {
    write(
      vellum,
      `distribution/cargo-install/src/bin/${name}.rs`,
      `fn main() { println!("${name} --help"); }\n`,
    );
  }
  for (const name of ['vellum-launchd-render', 'vellum-launchd-activate']) {
    const directory = `distribution/${name.replace('vellum-', '')}`;
    write(
      vellum,
      `${directory}/Cargo.toml`,
      `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\npublish = false\n`
        + `[lib]\npath = "src/lib.rs"\n[[bin]]\nname = "${name}"\npath = "src/main.rs"\n`,
    );
    write(vellum, `${directory}/src/lib.rs`, 'pub const QUALIFIED: bool = true;\n');
    write(vellum, `${directory}/src/main.rs`, `fn main() { println!("${name} --help"); }\n`);
  }
  write(vellum, 'release/compatibility-baseline.json', 'fixture compatibility\n');
  write(vellum, 'release/dependency-policy.json', 'fixture dependency policy\n');
  write(vellum, 'release/portable-core.json', 'fixture portable core policy\n');
  write(vellum, 'release/source-sbom.manifest.json', 'fixture SBOM\n');
  write(vellum, 'rust-toolchain.toml', fs.readFileSync(path.resolve('rust-toolchain.toml'), 'utf8'));
  for (const name of [
    'build-release-candidate.mjs',
    'release-candidate-lib.mjs',
    'compare-release-candidates.mjs',
    'release-reproducibility-lib.mjs',
    'qualify-clean-release-candidate.mjs',
  ]) copyScript(name);
  write(
    vellum,
    '.github/workflows/release-candidate.yml',
    fs.readFileSync(path.resolve('.github/workflows/release-candidate.yml'), 'utf8'),
  );
  write(
    vellum,
    '.github/workflows/portable-core.yml',
    fs.readFileSync(path.resolve('.github/workflows/portable-core.yml'), 'utf8'),
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
        crates: [],
      },
      sourceInstallProfiles: [{
        package: 'vellum',
        path: 'distribution/cargo-install',
        binaries: ['vellum-daemon', 'vellum-mcp'],
        platform: 'macos',
      }],
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
  run('cargo', ['generate-lockfile', '--offline'], vellum);
  const vellumRevision = commit(vellum);

  // Dirty working-tree material must not enter the exact committed-source build.
  write(vellum, 'UNTRACKED-SHOULD-NOT-ENTER', 'uncommitted\n');
  const report = run(
    'node',
    [qualifier, `--vellum-ref=${vellumRevision}`, `--output-dir=${output}`],
    vellum,
    { ...process.env, KEYSTONE_SOURCE: keystone },
  );
  if (!report.includes(vellumRevision) || !report.includes('no remote or publication effect')) {
    throw new Error('clean qualification omitted its exact source identity or nonclaim');
  }
  const candidate = path.join(output, 'candidate');
  const candidateFiles = fs.readdirSync(candidate).sort().join(',');
  if (
    candidateFiles
      !== 'SHA256SUMS,manifest.json,vellum-daemon,vellum-launchd-activate,vellum-launchd-render,vellum-mcp'
  ) {
    throw new Error('clean qualification emitted the wrong candidate closure');
  }
  const manifest = JSON.parse(fs.readFileSync(path.join(candidate, 'manifest.json'), 'utf8'));
  const evidence = JSON.parse(
    fs.readFileSync(path.join(output, 'candidate-byte-equivalence.json'), 'utf8'),
  );
  const provenanceText = fs.readFileSync(path.join(output, 'source-provenance.json'), 'utf8');
  const provenance = JSON.parse(provenanceText);
  if (
    manifest.source.vellum.revision !== vellumRevision
    || evidence.source.vellumRevision !== vellumRevision
    || provenance.source.vellum.revision !== vellumRevision
    || provenance.source.vellum.tree.length !== 40
    || provenance.source.keystone.tree.length !== 40
    || provenance.subject.manifestSha256 !== evidence.manifestSha256
    || provenance.nonClaims[0] !== 'remote-reachability'
    || evidence.manifestSha256.length !== 64
    || JSON.stringify({ manifest, evidence, provenance }).includes(scratch)
  ) {
    throw new Error('clean qualification evidence is incomplete or path-disclosing');
  }

  const repeated = spawnSync(
    process.execPath,
    [qualifier, `--vellum-ref=${vellumRevision}`, `--output-dir=${output}`],
    { cwd: vellum, env: { ...process.env, KEYSTONE_SOURCE: keystone }, encoding: 'utf8' },
  );
  if (repeated.status === 0 || !fs.existsSync(path.join(output, 'candidate', 'manifest.json'))) {
    throw new Error('existing output was not refused without damaging prior evidence');
  }

  process.stdout.write('Vellum exact clean-source candidate qualification passed\n');
} finally {
  fs.rmSync(scratch, { recursive: true, force: true });
}
