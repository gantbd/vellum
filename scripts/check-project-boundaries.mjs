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
for (const name of ['vellum-sdk', 'vellum-session', 'vellum-model']) {
  if (!packages.has(name)) failures.push(`missing SDK project crate ${name}`);
}

for (const pkg of packages.values()) {
  for (const dependency of pkg.dependencies) {
    if (dependency.path === null) continue;
    const kind = dependency.kind ?? 'normal';
    if (['vellum-tui', 'vellum-gui-macos'].includes(dependency.name)) {
      failures.push(`${pkg.name} has forbidden application dependency on ${dependency.name}`);
    }
    if (kind !== 'dev' && dependency.name.startsWith('vellum-adapter-')) {
      failures.push(`${pkg.name} has forbidden concrete-adapter dependency on ${dependency.name}`);
    }
  }
}

if (failures.length !== 0) {
  for (const failure of failures) process.stderr.write(`SDK boundary: ${failure}\n`);
  process.exit(1);
}
process.stdout.write('SDK boundaries closed; 0 shipped concrete-adapter edges remain\n');
