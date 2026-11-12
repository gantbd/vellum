import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { evaluateSupplyChain } from './supply-chain-lib.mjs';

process.on('uncaughtException', (error) => {
  process.stderr.write(`Vellum supply chain: ${error.message}\n`);
  process.exit(1);
});

const root = process.cwd();
const emitArgument = process.argv.slice(2).find((argument) => argument.startsWith('--emit='));
const qualificationEmitArgument = process.argv
  .slice(2)
  .find((argument) => argument.startsWith('--emit-qualification='));
const unknown = process.argv.slice(2).filter(
  (argument) => !argument.startsWith('--emit=') && !argument.startsWith('--emit-qualification='),
);
if (unknown.length !== 0) throw new Error(`unknown argument: ${unknown[0]}`);
const policy = JSON.parse(fs.readFileSync(path.join(root, 'release/dependency-policy.json'), 'utf8'));
const sourcePolicy = JSON.parse(fs.readFileSync(path.join(root, 'release/github-source.json'), 'utf8'));
const keystoneGit = policy.gitPolicy?.sources?.[0];
if (
  keystoneGit?.name !== 'keystone'
  || keystoneGit.repository !== sourcePolicy.keystone?.repository
  || keystoneGit.revision !== sourcePolicy.keystone?.revision
  || keystoneGit.source
    !== `git+${sourcePolicy.keystone.repository}?rev=${sourcePolicy.keystone.revision}#${sourcePolicy.keystone.revision}`
  || (policy.pathPolicy?.external ?? []).length !== 0
) {
  throw new Error('dependency policy drifted from the immutable Keystone Git source');
}
const externalRoot = path.resolve(root, sourcePolicy.keystone.relativePath);
const externalHead = spawnSync(
  'git',
  ['-C', externalRoot, 'rev-parse', 'HEAD'],
  { encoding: 'utf8' },
);
if (externalHead.status !== 0 || externalHead.stdout.trim() !== keystoneGit.revision) {
  throw new Error('Keystone checkout does not match the coordinated revision');
}
for (const pkg of keystoneGit.packages ?? []) {
  const atRevision = spawnSync(
    'git',
    ['-C', externalRoot, 'cat-file', '-e', `${keystoneGit.revision}:${pkg.manifest}`],
    { encoding: 'utf8' },
  ).status === 0;
  if (
    pkg.revisionStatus !== 'present-at-pinned-revision'
    || !atRevision
  ) {
    throw new Error(`Keystone revision status drifted for ${pkg.name}`);
  }
}
const apiClosure = sourcePolicy.keystone?.apiClosure ?? [];
const apiPackages = new Set();
const apiBlockerPackages = new Set();
for (const contract of apiClosure) {
  if (
    !['blocker', 'qualified'].includes(contract.status)
    || !Array.isArray(contract.requiredSourceFiles)
    || contract.requiredSourceFiles.length === 0
    || !Array.isArray(contract.requiredPublicItems)
    || contract.requiredPublicItems.length === 0
  ) {
    throw new Error('coordinated Keystone API-closure blocker is malformed');
  }
  if (apiPackages.has(contract.package)) {
    throw new Error(`duplicate Keystone API-closure blocker for ${contract.package}`);
  }
  apiPackages.add(contract.package);
  if (contract.status === 'blocker') apiBlockerPackages.add(contract.package);
  const dependencyPackage = (keystoneGit.packages ?? []).find(
    (pkg) => pkg.name === contract.package,
  );
  const expectedRevisionStatus = contract.status === 'blocker'
    ? 'manifest-present-api-closure-blocker'
    : 'present-at-pinned-revision';
  if (dependencyPackage?.revisionStatus !== expectedRevisionStatus) {
    throw new Error(`Keystone API-closure policy drifted for ${contract.package}`);
  }
  const absentRequiredFiles = contract.requiredSourceFiles.filter((file) => spawnSync(
    'git', ['-C', externalRoot, 'cat-file', '-e', `${keystoneGit.revision}:${file}`],
    { encoding: 'utf8' },
  ).status !== 0);
  const absentRequiredItems = contract.requiredPublicItems.filter((item) => spawnSync(
    'git',
    ['-C', externalRoot, 'grep', '-F', '-e', item, keystoneGit.revision, '--', `crates/${contract.package}/src`],
    { encoding: 'utf8' },
  ).status !== 0);
  if (
    contract.status === 'blocker'
    && absentRequiredFiles.length === 0
    && absentRequiredItems.length === 0
  ) {
    throw new Error(
      `Keystone API-closure blocker for ${contract.package} lacks pinned absence evidence`,
    );
  }
  if (
    contract.status === 'qualified'
    && (absentRequiredFiles.length !== 0 || absentRequiredItems.length !== 0)
  ) {
    throw new Error(`Keystone API closure is incomplete for ${contract.package}`);
  }
}
for (const pkg of keystoneGit.packages ?? []) {
  if (
    pkg.revisionStatus === 'manifest-present-api-closure-blocker'
    && !apiBlockerPackages.has(pkg.name)
  ) {
    throw new Error(`Keystone API-closure blocker is undeclared for ${pkg.name}`);
  }
}
const cargo = spawnSync(
  'cargo',
  ['metadata', '--offline', '--locked', '--format-version', '1'],
  { cwd: root, encoding: 'utf8' },
);
if (cargo.status !== 0) throw new Error('locked offline Cargo metadata failed');
const lockText = fs.readFileSync(path.join(root, policy.lockfile), 'utf8');
const result = evaluateSupplyChain({
  metadata: JSON.parse(cargo.stdout),
  lockText,
  policy,
  root,
});

const qualificationManifestPath = path.join(
  root,
  'qualification/semantic-lsp-provider/Cargo.toml',
);
const qualificationCargo = spawnSync(
  'cargo',
  [
    'metadata',
    '--manifest-path', qualificationManifestPath,
    '--offline',
    '--locked',
    '--format-version', '1',
  ],
  { cwd: root, encoding: 'utf8' },
);
if (qualificationCargo.status !== 0) {
  throw new Error('locked offline semantic-provider qualification metadata failed');
}
const qualificationMetadata = JSON.parse(qualificationCargo.stdout);
const repositoryPrefix = `${fs.realpathSync(root)}${path.sep}`;
qualificationMetadata.workspace_members = qualificationMetadata.packages
  .filter((pkg) => {
    if (pkg.source !== null) return false;
    const manifest = fs.realpathSync(pkg.manifest_path);
    return manifest.startsWith(repositoryPrefix);
  })
  .map((pkg) => pkg.id);
const observedPackageNames = new Set(
  qualificationMetadata.packages.map((pkg) => pkg.name),
);
const qualificationPolicy = structuredClone(policy);
qualificationPolicy.pathPolicy.external = qualificationPolicy.pathPolicy.external.map(
  (entry) => ({
    ...entry,
    packages: entry.packages.filter((pkg) => observedPackageNames.has(pkg.name)),
  }),
);
qualificationPolicy.gitPolicy.sources = qualificationPolicy.gitPolicy.sources
  .map((entry) => ({
    ...entry,
    packages: entry.packages.filter((pkg) => observedPackageNames.has(pkg.name)),
  }))
  .filter((entry) => entry.packages.length !== 0);
const qualificationLockText = fs.readFileSync(
  path.join(root, 'qualification/semantic-lsp-provider/Cargo.lock'),
  'utf8',
);
const qualificationResult = evaluateSupplyChain({
  metadata: qualificationMetadata,
  lockText: qualificationLockText,
  policy: qualificationPolicy,
  root,
});

if (qualificationEmitArgument !== undefined) {
  const destination = qualificationEmitArgument.slice('--emit-qualification='.length);
  if (destination === '-') {
    fs.writeFileSync(1, qualificationResult.serialized);
  } else {
    fs.writeFileSync(destination, qualificationResult.serialized, { flag: 'wx', mode: 0o600 });
  }
  process.exit(0);
}

if (emitArgument !== undefined) {
  const destination = emitArgument.slice('--emit='.length);
  if (destination === '-') {
    fs.writeFileSync(1, result.serialized);
  } else {
    fs.writeFileSync(destination, result.serialized, { flag: 'wx', mode: 0o600 });
  }
  process.exit(0);
}

const manifest = JSON.parse(fs.readFileSync(path.join(root, policy.sbom.manifest), 'utf8'));
if (
  manifest.schema !== 1
  || manifest.format !== `CycloneDX ${policy.sbom.specVersion}`
  || manifest.lockSha256 !== result.lockSha256
  || manifest.sbomSha256 !== result.sbomSha256
  || manifest.componentCount !== result.componentCount
  || JSON.stringify(manifest.blockers) !== JSON.stringify(result.blockers)
) {
  throw new Error('source SBOM manifest drifted; review dependencies and regenerate deliberately');
}
const qualificationManifest = JSON.parse(fs.readFileSync(
  path.join(root, 'release/semantic-provider-qualification-sbom.manifest.json'),
  'utf8',
));
if (
  qualificationManifest.schema !== 1
  || qualificationManifest.format !== `CycloneDX ${policy.sbom.specVersion}`
  || qualificationManifest.scope !== 'semantic-lsp-provider-qualification'
  || qualificationManifest.lockSha256 !== qualificationResult.lockSha256
  || qualificationManifest.sbomSha256 !== qualificationResult.sbomSha256
  || qualificationManifest.componentCount !== qualificationResult.componentCount
  || JSON.stringify(qualificationManifest.blockers) !== JSON.stringify(qualificationResult.blockers)
) {
  throw new Error(
    `semantic-provider qualification SBOM manifest drifted; expected ${JSON.stringify({
      lockSha256: qualificationResult.lockSha256,
      sbomSha256: qualificationResult.sbomSha256,
      componentCount: qualificationResult.componentCount,
      blockers: qualificationResult.blockers,
    })}`,
  );
}
process.stdout.write(
  `Vellum supply-chain policy closed: ${result.componentCount} shipping and ${qualificationResult.componentCount} semantic-qualification locked components; ${result.blockers.length} shipping and ${qualificationResult.blockers.length} qualification source-SBOM blockers; ${apiBlockerPackages.size} additional GitHub API-closure blocker\n`,
);
