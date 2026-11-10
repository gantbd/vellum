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
for (const name of [
  'vellum-host',
  'vellum-session',
  'vellum-editor',
  'vellum-protocol',
  'vellum-service',
  'vellum-client',
  'vellum-wire',
  'vellum-transport',
  'vellum-daemon',
]) {
  if (!packages.has(name)) failures.push(`missing SDK project crate ${name}`);
}
const dependencyNames = (name) =>
  new Set((packages.get(name)?.dependencies ?? []).map((dependency) => dependency.name));
if (dependencyNames('vellum-protocol').size !== 0) {
  failures.push('protocol must not depend on engine, editor, service, adapters, or transports');
}
for (const forbidden of ['vellum-engine', 'vellum-host', 'vellum-session', 'vellum-editor', 'vellum-service']) {
  if (dependencyNames('vellum-client').has(forbidden)) {
    failures.push(`client has forbidden inward dependency on ${forbidden}`);
  }
}
for (const required of ['vellum-service', 'vellum-transport', 'vellum-editor', 'vellum-adapter-local-fs']) {
  if (!dependencyNames('vellum-daemon').has(required)) {
    failures.push(`daemon composition root must depend on ${required}`);
  }
}
for (const dependency of dependencyNames('vellum-daemon')) {
  if (dependency.startsWith('vellum-adapter-') && dependency !== 'vellum-adapter-local-fs') {
    failures.push(`daemon has undeclared concrete adapter ${dependency}`);
  }
}
for (const required of ['vellum-protocol', 'vellum-wire']) {
  if (!dependencyNames('vellum-transport').has(required)) {
    failures.push(`transport must depend on ${required}`);
  }
}
for (const required of ['vellum-protocol', 'keystone-codec']) {
  if (!dependencyNames('vellum-wire').has(required)) {
    failures.push(`wire must depend on ${required}`);
  }
}
for (const forbidden of ['vellum-engine', 'vellum-host', 'vellum-session', 'vellum-editor', 'vellum-service']) {
  if (dependencyNames('vellum-wire').has(forbidden)) {
    failures.push(`wire has forbidden semantic dependency on ${forbidden}`);
  }
}

for (const pkg of packages.values()) {
  for (const dependency of pkg.dependencies) {
    if (dependency.path === null) continue;
    const kind = dependency.kind ?? 'normal';
    if (['vellum-tui', 'vellum-gui-macos'].includes(dependency.name)) {
      failures.push(`${pkg.name} has forbidden application dependency on ${dependency.name}`);
    }
    if (kind !== 'dev' && dependency.name.startsWith('vellum-adapter-') && pkg.name !== 'vellum-daemon') {
      failures.push(`${pkg.name} has forbidden concrete-adapter dependency on ${dependency.name}`);
    }
  }
}

if (failures.length !== 0) {
  for (const failure of failures) process.stderr.write(`SDK boundary: ${failure}\n`);
  process.exit(1);
}
process.stdout.write('SDK boundaries closed; 1 declared daemon composition-root adapter edge\n');
