import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const root = process.cwd();
const contract = JSON.parse(
  fs.readFileSync(path.join(root, 'release/vellum-mcp-extraction.json'), 'utf8'),
);
if (
  contract.schema !== 1
  || contract.archiveRoot !== 'vellum-mcp'
  || contract.targetSibling !== '../vellum-mcp'
  || contract.publication !== 'none'
) {
  throw new Error('unsupported MCP extraction contract');
}
const expectedOrdinaryTools = [
  'vellum.snapshot',
  'vellum.replace',
  'vellum.replace_and_save',
  'vellum.workspace_edit',
  'vellum.poll_changes',
  'vellum.resolve_locator',
  'vellum.membership',
];
const expectedAuthenticatedAdditions = [
  'vellum.durable_execute',
  'vellum.durable_receipt',
  'vellum.durable_retire',
  'vellum.durable_status',
];
if (
  JSON.stringify(contract.protocol?.ordinaryTools) !== JSON.stringify(expectedOrdinaryTools)
  || JSON.stringify(contract.protocol?.authenticatedAdditions)
    !== JSON.stringify(expectedAuthenticatedAdditions)
) {
  throw new Error('MCP extraction catalog drifted from the qualified seven-plus-four surface');
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
const requireRegularSource = (source, label) => {
  const metadata = fs.lstatSync(source);
  if (metadata.isSymbolicLink() || !metadata.isFile()) {
    throw new Error(`extraction source must be a regular non-symlink file: ${label}`);
  }
};
const requireInside = (parent, candidate, label) => {
  const relative = path.relative(parent, candidate);
  if (relative === '' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    throw new Error(`extraction mapping escapes its root: ${label}`);
  }
};
const metadata = JSON.parse(
  run('cargo', ['metadata', '--offline', '--locked', '--no-deps', '--format-version', '1']),
);
const connector = metadata.packages.find((pkg) => pkg.name === 'vellum-mcp');
if (connector === undefined) throw new Error('vellum-mcp package is missing');
const wire = metadata.packages.find((pkg) => pkg.name === 'vellum-wire');
const keystoneCodec = wire?.dependencies.find((dependency) => dependency.name === 'keystone-codec');
const qualifiedKeystone = contract.qualifiedImmutableDependencies?.[0];
if (
  (contract.immutableDependencyBlockers ?? []).length !== 0
  || keystoneCodec?.path != null
  || qualifiedKeystone?.dependency !== 'keystone-codec'
  || qualifiedKeystone?.repository !== 'https://github.com/gantbd/keystone'
  || !/^[0-9a-f]{40}$/.test(qualifiedKeystone?.revision ?? '')
  || keystoneCodec?.source
    !== `git+${qualifiedKeystone.repository}?rev=${qualifiedKeystone.revision}`
) {
  throw new Error('transitive Keystone immutable Git source drifted from the extraction contract');
}
const normalDependencies = connector.dependencies
  .filter((dependency) => (dependency.kind ?? 'normal') === 'normal')
  .map((dependency) => dependency.name)
  .sort();
if (normalDependencies.join(',') !== [...contract.normalDependencies].sort().join(',')) {
  throw new Error('vellum-mcp normal dependencies drifted from the extraction contract');
}
const internalNormal = normalDependencies.filter(
  (name) => name.startsWith('vellum-')
    && name !== contract.publicSdk.package
    && name !== contract.credentialAdapter?.package,
);
if (internalNormal.length !== 0) {
  throw new Error('vellum-mcp bypasses the public client SDK');
}
for (const source of ['connectors/vellum-mcp/src/lib.rs', 'connectors/vellum-mcp/src/main.rs']) {
  const text = fs.readFileSync(path.join(root, source), 'utf8');
  if (/\bvellum_(client|protocol|transport)\b/.test(text)) {
    throw new Error(`${source} directly names an SDK implementation crate`);
  }
}

const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-mcp-extraction-'));
try {
  const symlinkProbeTarget = path.join(temporary, 'regular-probe');
  const symlinkProbe = path.join(temporary, 'symlink-probe');
  fs.writeFileSync(symlinkProbeTarget, 'probe');
  fs.symlinkSync(symlinkProbeTarget, symlinkProbe);
  let rejectedSymlink = false;
  try {
    requireRegularSource(symlinkProbe, 'self-test');
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
    requireRegularSource(source, mapping.from);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.copyFileSync(source, destination);
  }
  const fixtureManifest = `[workspace]\n\n[package]\nname = "vellum-mcp"\nversion = "0.1.0"\nedition = "2024"\nrust-version = "1.95"\nlicense = "MIT OR Apache-2.0"\nrepository = "${contract.targetRepository}"\nreadme = "README.md"\npublish = false\ndescription = "Bounded current-protocol MCP projection for Vellum"\n\n[lib]\nname = "vellum_mcp"\n\n[[bin]]\nname = "vellum-mcp"\npath = "src/main.rs"\n\n[dependencies]\nserde_json = "=1.0.151"\nvellum-adapter-local-authentication = { version = "=0.1.0", path = "${path.join(root, 'adapters/local-authentication')}" }\nvellum-sdk-client = { version = "=0.1.0", path = "${path.join(root, 'sdk/client')}" }\n`;
  fs.writeFileSync(path.join(staged, 'Cargo.toml'), fixtureManifest);

  const archive = path.join(temporary, 'vellum-mcp.tar');
  run('tar', ['-cf', archive, '-C', temporary, contract.archiveRoot]);
  const unpacked = path.join(temporary, 'unpacked');
  fs.mkdirSync(unpacked);
  run('tar', ['-xf', archive, '-C', unpacked]);
  const extracted = path.join(unpacked, contract.archiveRoot);
  const target = path.join(temporary, 'target');
  run('cargo', ['generate-lockfile', '--offline'], { cwd: extracted });
  run('cargo', ['test', '--offline', '--locked', '--target-dir', target], { cwd: extracted });
  const install = path.join(temporary, 'install');
  run(
    'cargo',
    ['install', '--offline', '--path', extracted, '--root', install, '--target-dir', target],
    { cwd: extracted },
  );
  const help = run(path.join(install, 'bin/vellum-mcp'), ['--help']);
  if (!help.includes('vellum-mcp --socket')) {
    throw new Error('extracted MCP binary help smoke failed');
  }
  process.stdout.write(
    'Vellum MCP extraction fixture passed: public client SDK, credential adapter and immutable Keystone source; remote Git reachability not claimed\n',
  );
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
