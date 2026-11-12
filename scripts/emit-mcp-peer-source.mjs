import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = process.cwd();
const argument = process.argv.find((value) => value.startsWith('--output='));
if (argument === undefined || process.argv.length !== 3) {
  throw new Error('usage: node scripts/emit-mcp-peer-source.mjs --output=PATH');
}
const output = path.resolve(root, argument.slice('--output='.length));
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
const requireInside = (parent, candidate, label) => {
  const relative = path.relative(parent, candidate);
  if (relative === '' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    throw new Error(`mapping escapes its root: ${label}`);
  }
};
const requireFile = (candidate, label) => {
  const metadata = fs.lstatSync(candidate);
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw new Error(`source must be a regular non-symlink file: ${label}`);
  }
};
if (fs.existsSync(output)) throw new Error(`output already exists: ${output}`);

const revision = run('git', ['rev-parse', 'HEAD^{commit}']);
if (!/^[0-9a-f]{40}$/.test(revision)) throw new Error('Vellum HEAD is not a full commit');
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-mcp-peer-'));
try {
  // Source all payload bytes from one committed tree. Dirty worktree content can
  // neither leak into nor partially update the peer payload.
  const archive = path.join(temporary, 'source.tar');
  const sourceRoot = path.join(temporary, 'source');
  fs.mkdirSync(sourceRoot);
  run('git', ['archive', '--format=tar', `--output=${archive}`, revision]);
  run('tar', ['-xf', archive, '-C', sourceRoot]);
  const contract = JSON.parse(fs.readFileSync(
    path.join(sourceRoot, 'release/vellum-mcp-extraction.json'), 'utf8',
  ));
  if (
    contract.schema !== 1
    || contract.archiveRoot !== 'vellum-mcp'
    || contract.publication !== 'none'
    || contract.immutableDependencyBlockers.length !== 0
  ) throw new Error('unsupported extraction contract');
  const keystone = contract.qualifiedImmutableDependencies.find(
    (dependency) => dependency.dependency === 'keystone-codec',
  );
  if (keystone === undefined || !/^[0-9a-f]{40}$/.test(keystone.revision)) {
    throw new Error('exact Keystone dependency is absent');
  }

  const staged = path.join(temporary, contract.archiveRoot);
  for (const mapping of contract.copy) {
    const source = path.join(sourceRoot, mapping.from);
    const destination = path.join(staged, mapping.to);
    requireInside(sourceRoot, source, mapping.from);
    requireInside(staged, destination, mapping.to);
    requireFile(source, mapping.from);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.copyFileSync(source, destination);
  }
  const manifest = `[workspace]\n\n[package]\nname = "vellum-mcp"\nversion = "0.1.0"\nedition = "2024"\nrust-version = "1.95"\nlicense = "MIT OR Apache-2.0"\nrepository = "${contract.targetRepository}"\nreadme = "README.md"\npublish = false\ndescription = "Bounded current-protocol MCP projection for Vellum"\n\n[lib]\nname = "vellum_mcp"\n\n[[bin]]\nname = "vellum-mcp"\npath = "src/main.rs"\n\n[dependencies]\nserde_json = "=1.0.151"\n${contract.credentialAdapter.package} = { version = "=0.1.0", git = "${contract.credentialAdapter.repository}", rev = "${revision}" }\n${contract.publicSdk.package} = { version = "=0.1.0", git = "${contract.publicSdk.repository}", rev = "${revision}" }\n`;
  fs.writeFileSync(path.join(staged, 'Cargo.toml'), manifest);
  // Seed resolution from the same committed graph. Cargo then prunes unrelated
  // workspace packages while preserving the source revision's registry pins.
  fs.copyFileSync(path.join(sourceRoot, 'Cargo.lock'), path.join(staged, 'Cargo.lock'));
  fs.writeFileSync(path.join(staged, '.gitignore'), '/target/\n');
  fs.mkdirSync(path.join(staged, 'release'));
  fs.writeFileSync(path.join(staged, 'release/source.json'), `${JSON.stringify({
    schema: 1,
    source: { repository: contract.publicSdk.repository, revision },
    immutableDependencies: contract.qualifiedImmutableDependencies,
    install: {
      package: 'vellum-mcp',
      repository: contract.targetRepository,
      command: `cargo install --locked --git ${contract.targetRepository} --rev RELEASE_COMMIT vellum-mcp`,
    },
    publication: 'none',
    retainedUpstreamQualification: contract.retainedVellumQualification,
  }, null, 2)}\n`);
  fs.appendFileSync(path.join(staged, 'README.md'), `\n## Source installation\n\nThis peer source is bound to Vellum commit \`${revision}\` and Keystone commit\n\`${keystone.revision}\`. After a reviewed release commit exists here:\n\n\`\`\`sh\ncargo install --locked --git ${contract.targetRepository} --rev RELEASE_COMMIT vellum-mcp\n\`\`\`\n\nBuild and test locally with \`cargo test --locked\`. Coordinated live-daemon\nqualification remains in Vellum; this peer owns only the MCP projection.\n`);

  const keystoneRoot = path.resolve(root, '../keystone');
  if (!fs.existsSync(path.join(keystoneRoot, '.git'))) {
    throw new Error(`local Keystone checkout required for qualification: ${keystoneRoot}`);
  }
  const gitConfig = path.join(temporary, 'gitconfig');
  fs.writeFileSync(gitConfig, `[url "${pathToFileURL(root).href}"]\n\tinsteadOf = ${contract.publicSdk.repository}\n[url "${pathToFileURL(keystoneRoot).href}"]\n\tinsteadOf = ${keystone.repository}\n`);
  const env = {
    ...process.env,
    GIT_CONFIG_GLOBAL: gitConfig,
    GIT_CONFIG_NOSYSTEM: '1',
    CARGO_NET_GIT_FETCH_WITH_CLI: 'true',
  };
  // Fetches are URL-rewritten to the two local checkouts. This populates Cargo's
  // canonical Git source cache without contacting either remote.
  run('cargo', ['generate-lockfile'], { cwd: staged, env });
  const metadata = JSON.parse(run(
    'cargo', ['metadata', '--offline', '--locked', '--no-deps', '--format-version', '1'],
    { cwd: staged, env },
  ));
  const packageMetadata = metadata.packages.find((candidate) => candidate.name === 'vellum-mcp');
  for (const name of [contract.publicSdk.package, contract.credentialAdapter.package]) {
    const dependency = packageMetadata?.dependencies.find((candidate) => candidate.name === name);
    if (dependency?.source !== `git+${contract.publicSdk.repository}?rev=${revision}`) {
      throw new Error(`${name} is not exact-revision Vellum Git source`);
    }
  }
  const target = path.join(temporary, 'target');
  run('cargo', ['test', '--locked', '--target-dir', target], { cwd: staged, env });

  // Initialize a disposable peer Git repository so the qualification exercises
  // cargo install --git, not the weaker path-install approximation.
  const repository = path.join(temporary, 'repository');
  fs.cpSync(staged, repository, { recursive: true });
  run('git', ['init', '--quiet'], { cwd: repository });
  run('git', ['config', 'user.name', 'Vellum qualification'], { cwd: repository });
  run('git', ['config', 'user.email', 'qualification@invalid'], { cwd: repository });
  run('git', ['add', '.'], { cwd: repository });
  run('git', ['commit', '--quiet', '-m', 'qualification source'], { cwd: repository });
  const repositoryRevision = run('git', ['rev-parse', 'HEAD'], { cwd: repository });
  fs.appendFileSync(gitConfig, `[url "${pathToFileURL(repository).href}"]\n\tinsteadOf = ${contract.targetRepository}\n`);
  const install = path.join(temporary, 'install');
  run('cargo', [
    'install', '--locked', '--git', contract.targetRepository,
    '--rev', repositoryRevision, 'vellum-mcp', '--root', install,
    '--target-dir', path.join(temporary, 'install-target'),
  ], { cwd: temporary, env });
  if (!run(path.join(install, 'bin/vellum-mcp'), ['--help']).includes('vellum-mcp --socket')) {
    throw new Error('Git-installed binary help smoke failed');
  }

  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.cpSync(staged, output, { recursive: true });
  process.stdout.write(`emitted ${output}\nVellum ${revision}\nKeystone ${keystone.revision}\n`);
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
