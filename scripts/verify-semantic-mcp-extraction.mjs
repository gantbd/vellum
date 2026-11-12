import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const root = process.cwd();
const contract = JSON.parse(fs.readFileSync(
  path.join(root, 'release/vellum-semantic-mcp-extraction.json'),
  'utf8',
));
if (
  contract.schema !== 1
  || contract.archiveRoot !== 'vellum-semantic-mcp'
  || contract.targetSibling !== '../vellum-semantic-mcp'
  || contract.publication !== 'none'
) throw new Error('unsupported semantic MCP extraction contract');
const expectedTools = [
  'vellum.semantic.symbols',
  'vellum.semantic.definition',
  'vellum.semantic.references',
  'vellum.semantic.implementations',
  'vellum.semantic.incoming_calls',
  'vellum.semantic.outgoing_calls',
  'vellum.semantic.diagnostics',
  'vellum.semantic.type_at_point',
  'vellum.semantic.plan_rename',
  'vellum.semantic.release_request',
];
if (JSON.stringify(contract.protocol?.tools) !== JSON.stringify(expectedTools)) {
  throw new Error('semantic MCP extraction catalog drifted from the qualified ten-tool surface');
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
const requireInside = (parent, candidate, label) => {
  const relative = path.relative(parent, candidate);
  if (relative === '' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    throw new Error(`extraction mapping escapes its root: ${label}`);
  }
};
const requireRegular = (candidate, label) => {
  const metadata = fs.lstatSync(candidate);
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw new Error(`extraction source must be a regular non-symlink file: ${label}`);
  }
};

const metadata = JSON.parse(run(
  'cargo',
  ['metadata', '--offline', '--locked', '--no-deps', '--format-version', '1'],
));
const connector = metadata.packages.find((pkg) => pkg.name === 'vellum-semantic-mcp');
if (connector === undefined) throw new Error('vellum-semantic-mcp package is missing');
const actualDependencies = connector.dependencies
  .filter((dependency) => (dependency.kind ?? 'normal') === 'normal')
  .map((dependency) => dependency.name)
  .sort();
if (actualDependencies.join(',') !== [...contract.normalDependencies].sort().join(',')) {
  throw new Error('semantic MCP normal dependencies drifted from the extraction contract');
}
const allowedVellum = new Set(contract.vellumDependencies.map((dependency) => dependency.package));
for (const dependency of actualDependencies.filter((name) => name.startsWith('vellum-'))) {
  if (!allowedVellum.has(dependency)) {
    throw new Error(`semantic MCP bypasses its SDK boundary through ${dependency}`);
  }
}
for (const required of [
  'vellum-adapter-local-authentication',
  'vellum-sdk-client',
  'vellum-sdk-semantic-client',
]) {
  if (!allowedVellum.has(required)) throw new Error(`missing facade dependency ${required}`);
}
for (const source of [
  'connectors/vellum-semantic-mcp/src/lib.rs',
  'connectors/vellum-semantic-mcp/src/main.rs',
]) {
  const text = fs.readFileSync(path.join(root, source), 'utf8');
  if (!text.includes('vellum_sdk_semantic_client')) {
    throw new Error(`${source} does not enter through the semantic SDK facade`);
  }
}

const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-semantic-mcp-extraction-'));
try {
  const regular = path.join(temporary, 'regular');
  const symlink = path.join(temporary, 'symlink');
  fs.writeFileSync(regular, 'probe');
  fs.symlinkSync(regular, symlink);
  let rejectedSymlink = false;
  try {
    requireRegular(symlink, 'self-test');
  } catch {
    rejectedSymlink = true;
  }
  if (!rejectedSymlink) throw new Error('regular-source symlink self-test failed');

  const staged = path.join(temporary, contract.archiveRoot);
  for (const mapping of contract.copy) {
    const source = path.join(root, mapping.from);
    const destination = path.join(staged, mapping.to);
    requireInside(root, source, mapping.from);
    requireInside(staged, destination, mapping.to);
    requireRegular(source, mapping.from);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.copyFileSync(source, destination);
  }
  const manifest = `[workspace]\n\n[package]\nname = "vellum-semantic-mcp"\nversion = "0.1.0"\nedition = "2024"\nrust-version = "1.95"\nlicense = "MIT OR Apache-2.0"\nrepository = "${contract.targetRepository}"\nreadme = "README.md"\npublish = false\ndescription = "Bounded MCP projection for Vellum semantic queries"\n\n[lib]\nname = "vellum_semantic_mcp"\n\n[[bin]]\nname = "vellum-semantic-mcp"\npath = "src/main.rs"\n\n[dependencies]\ngetrandom = "=0.4.3"\nserde = "=1.0.229"\nserde_json = "=1.0.151"\ntokio = { version = "1", features = ["io-util", "io-std", "rt"] }\nvellum-adapter-local-authentication = { version = "=0.1.0", path = "${path.join(root, 'adapters/local-authentication')}" }\nvellum-sdk-client = { version = "=0.1.0", path = "${path.join(root, 'sdk/client')}" }\nvellum-sdk-semantic-client = { version = "=0.1.0", path = "${path.join(root, 'sdk/semantic-client')}" }\n`;
  fs.writeFileSync(path.join(staged, 'Cargo.toml'), manifest);

  const archive = path.join(temporary, 'vellum-semantic-mcp.tar');
  run('tar', ['-cf', archive, '-C', temporary, contract.archiveRoot]);
  const unpacked = path.join(temporary, 'unpacked');
  fs.mkdirSync(unpacked);
  run('tar', ['-xf', archive, '-C', unpacked]);
  const extracted = path.join(unpacked, contract.archiveRoot);
  const target = path.join(temporary, 'target');
  run('cargo', ['generate-lockfile', '--offline'], { cwd: extracted });
  run('cargo', ['test', '--offline', '--locked', '--all-targets', '--target-dir', target], {
    cwd: extracted,
  });
  run('cargo', [
    'clippy', '--offline', '--locked', '--all-targets', '--target-dir', target,
    '--', '-D', 'warnings',
  ], { cwd: extracted });
  const install = path.join(temporary, 'install');
  run('cargo', [
    'install', '--offline', '--locked', '--path', extracted,
    '--root', install, '--target-dir', target,
  ], { cwd: extracted });
  const help = run(path.join(install, 'bin/vellum-semantic-mcp'), ['--help']);
  if (!help.includes('vellum-semantic-mcp --semantic-socket')) {
    throw new Error('extracted semantic MCP binary help smoke failed');
  }
  process.stdout.write(
    'Vellum semantic MCP extraction passed: native and semantic SDK facades, credential adapter, locked tests, strict Clippy and path install; remote Git reachability not claimed\n',
  );
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
