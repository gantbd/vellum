import fs from 'node:fs';
import { spawnSync } from 'node:child_process';

const policy = JSON.parse(fs.readFileSync('release/installability.json', 'utf8'));
const failures = [];
if (policy.schema !== 1) failures.push('unsupported installability schema');
if (policy.distributionPackage !== 'vellum' || policy.defaultBinary !== 'vellum') {
  failures.push('canonical distribution package/binary drifted');
}
const requiredCommands = [
  'cargo install vellum --locked',
  'cargo install vellum --locked --features desktop',
];
if (JSON.stringify(policy.commands) !== JSON.stringify(requiredCommands)) {
  failures.push('canonical cargo install commands drifted');
}

const cargo = spawnSync(
  'cargo',
  ['metadata', '--offline', '--no-deps', '--format-version', '1'],
  { encoding: 'utf8' },
);
if (cargo.status !== 0) {
  process.stderr.write(cargo.stderr);
  process.exit(cargo.status ?? 1);
}
const metadata = JSON.parse(cargo.stdout);
const packages = new Map(metadata.packages.map((pkg) => [pkg.name, pkg]));
const distribution = packages.get(policy.distributionPackage);
if (distribution === undefined) {
  failures.push('canonical distribution package is missing');
} else {
  const binary = (name) => distribution.targets.find(
    (target) => target.name === name && target.kind.includes('bin'),
  );
  if (binary(policy.defaultBinary) === undefined) failures.push('default binary is missing');
  const desktop = binary(policy.desktopBinary);
  if (desktop === undefined) failures.push('desktop binary is missing');
  else if (JSON.stringify(desktop['required-features']) !== JSON.stringify(['desktop'])) {
    failures.push('desktop binary must require exactly the desktop feature');
  }
  if (!Object.hasOwn(distribution.features, 'desktop')) {
    failures.push('desktop install feature is missing');
  }
}

for (const pkg of packages.values()) {
  if (pkg.rust_version !== '1.95') failures.push(`${pkg.name} has non-release MSRV`);
  if (pkg.license !== 'MIT OR Apache-2.0') failures.push(`${pkg.name} has missing license`);
  if (pkg.repository !== 'https://github.com/gantbd/vellum') {
    failures.push(`${pkg.name} has unexpected migration repository`);
  }
  for (const dependency of pkg.dependencies) {
    if (!dependency.path) continue;
    const expected = dependency.name.startsWith('vellum-') ? '=0.1.0'
      : dependency.name === 'keystone-observe' ? '=0.0.1' : null;
    if (expected === null) failures.push(`${pkg.name} has unclassified path dependency ${dependency.name}`);
    else if (dependency.req !== expected) {
      failures.push(`${pkg.name} -> ${dependency.name} must be pinned ${expected}`);
    }
  }
}

const actualBlockers = new Set();
const installClosure = [...packages.values()].filter((pkg) => pkg.name !== 'vellum-unicode-reference');
if (installClosure.some((pkg) => Array.isArray(pkg.publish) && pkg.publish.length === 0)) {
  actualBlockers.add('install-closure-publish-disabled');
}
if ([...packages.values()].some((pkg) => pkg.dependencies.some((dep) => dep.name === 'keystone-observe' && dep.path))) {
  actualBlockers.add('keystone-observe-unpublished');
}
const unicode = packages.get('vellum-adapter-unicode-icu');
if (unicode?.dependencies.some((dep) => dep.name === 'rust_icu_sys')) {
  actualBlockers.add('native-icu4c-discovery');
}

const root = fs.readFileSync('Cargo.toml', 'utf8');
for (const name of ['iced_core', 'iced_graphics', 'iced_widget', 'objc2-app-kit']) {
  if (new RegExp(`^${name.replace('-', '\\-')}\\s*=`, 'm').test(root)) {
    actualBlockers.add(`patch-${name.replace('_', '-')}`);
  }
}

const reverseEdges = [];
for (const pkg of packages.values()) {
  for (const dependency of pkg.dependencies) {
    if (['vellum-session', 'vellum-model'].includes(pkg.name)
      && dependency.kind !== 'dev'
      && dependency.name.startsWith('vellum-adapter-')) {
      reverseEdges.push(`${pkg.name} -> ${dependency.name}`);
    }
  }
}
if (reverseEdges.length !== 0) actualBlockers.add('reverse-adapter-edges');
if (policy.layout === 'monorepo-migration') actualBlockers.add('project-extraction-pending');

const admitted = new Set(policy.admittedBlockers);
for (const blocker of actualBlockers) {
  if (!admitted.has(blocker)) failures.push(`unadmitted install blocker ${blocker}`);
}
for (const blocker of admitted) {
  if (!actualBlockers.has(blocker)) failures.push(`stale admitted install blocker ${blocker}`);
}
if (new Set(policy.admittedBlockers).size !== policy.admittedBlockers.length) {
  failures.push('duplicate admitted install blocker');
}

if (failures.length !== 0) {
  for (const failure of failures) process.stderr.write(`installability: ${failure}\n`);
  process.exit(1);
}
process.stdout.write(
  `installability closed; ${actualBlockers.size} admitted blockers, ${reverseEdges.length} reverse edges remain\n`,
);
