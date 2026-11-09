import fs from 'node:fs';
import { spawnSync } from 'node:child_process';

const cargo = spawnSync(
  'cargo',
  ['metadata', '--offline', '--locked', '--no-deps', '--format-version', '1'],
  { encoding: 'utf8' },
);
if (cargo.status !== 0) {
  process.stderr.write(cargo.stderr);
  process.exit(cargo.status ?? 1);
}

const metadata = JSON.parse(cargo.stdout);
const packages = new Map(metadata.packages.map((pkg) => [pkg.name, pkg]));
const failures = [];
const required = [
  'vellum-adapter-local-fs',
  'vellum-adapter-annotation-json',
  'vellum-adapter-unicode-icu',
  'vellum-adapter-clipboard-macos',
  'vellum-unicode-reference',
];
for (const name of required) {
  if (!packages.has(name)) failures.push(`missing adapter project crate ${name}`);
}

const unicodeReference = packages.get('vellum-unicode-reference');
if (unicodeReference && unicodeReference.publish?.length !== 0) {
  failures.push('Unicode reference/qualification package must remain unpublished');
}

const qualificationCargo = spawnSync(
  'cargo',
  [
    'metadata', '--offline', '--locked', '--no-deps', '--format-version', '1',
    '--manifest-path', 'qualification/vellum-clipboard-macos-qualification/Cargo.toml',
  ],
  { encoding: 'utf8' },
);
if (qualificationCargo.status !== 0) {
  failures.push(`standalone clipboard qualification metadata failed: ${qualificationCargo.stderr}`);
} else {
  const qualificationMetadata = JSON.parse(qualificationCargo.stdout);
  const qualification = qualificationMetadata.packages.find(
    (pkg) => pkg.name === 'vellum-clipboard-macos-qualification',
  );
  if (qualification === undefined) {
    failures.push('standalone clipboard qualification package is missing');
  } else if (qualification.publish?.length !== 0) {
    failures.push('live clipboard qualification package must remain unpublished');
  }
  if (qualificationMetadata.workspace_members.length !== 1) {
    failures.push('clipboard qualification must remain a single-package standalone workspace');
  }
}

const rootManifest = fs.readFileSync('Cargo.toml', 'utf8');
const qualificationManifest = fs.readFileSync(
  'qualification/vellum-clipboard-macos-qualification/Cargo.toml',
  'utf8',
);
if (/^\[patch\.crates-io\]/m.test(rootManifest)) {
  failures.push('production adapter workspace must not patch crates.io');
}
if (!/^\[patch\.crates-io\][\s\S]*^objc2-app-kit\s*=\s*\{\s*path\s*=\s*"\.\.\/\.\.\/vendor\/objc2-app-kit"\s*\}/m.test(qualificationManifest)) {
  failures.push('standalone clipboard qualification must own the isolated AppKit patch');
}

for (const pkg of packages.values()) {
  for (const dependency of pkg.dependencies) {
    if (dependency.path === null) continue;
    if (['vellum', 'vellum-iced'].includes(dependency.name)) {
      failures.push(`${pkg.name} has forbidden application dependency on ${dependency.name}`);
    }
  }
}

if (failures.length !== 0) {
  for (const failure of failures) process.stderr.write(`adapter boundary: ${failure}\n`);
  process.exit(1);
}
process.stdout.write('adapter boundaries closed; 0 application edges remain\n');
