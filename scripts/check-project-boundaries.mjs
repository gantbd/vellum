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
const failures = [];
if (metadata.packages.length !== 1 || metadata.packages[0]?.name !== 'vellum-engine') {
  failures.push('engine repository must contain exactly vellum-engine');
}
const engine = metadata.packages.find((pkg) => pkg.name === 'vellum-engine');
for (const dependency of engine?.dependencies ?? []) {
  if (dependency.name.startsWith('vellum-')) {
    failures.push(`engine has forbidden upward dependency on ${dependency.name}`);
  }
}

if (failures.length !== 0) {
  for (const failure of failures) process.stderr.write(`engine boundary: ${failure}\n`);
  process.exit(1);
}
process.stdout.write('engine boundary closed; 1 crate, 0 upward Vellum edges\n');
