import { spawnSync } from 'node:child_process';

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
const failures = [];
const required = [
  'vellum-adapter-local-fs',
  'vellum-adapter-annotation-json',
  'vellum-adapter-unicode-icu',
  'vellum-adapter-clipboard-macos',
  'vellum-clipboard-macos-qualification',
  'vellum-unicode-reference',
];
for (const name of required) {
  if (!packages.has(name)) failures.push(`missing adapter project crate ${name}`);
}

const qualification = packages.get('vellum-clipboard-macos-qualification');
if (qualification && qualification.publish?.length !== 0) {
  failures.push('live clipboard qualification package must remain unpublished');
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
