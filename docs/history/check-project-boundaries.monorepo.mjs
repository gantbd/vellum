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
  'vellum-engine',
  'vellum-sdk',
  'vellum-session',
  'vellum-model',
  'vellum-adapter-local-fs',
  'vellum-adapter-annotation-json',
  'vellum-adapter-unicode-icu',
  'vellum-adapter-clipboard-macos',
  'vellum',
  'vellum-iced',
];
for (const name of required) {
  if (!packages.has(name)) failures.push(`missing declared project crate ${name}`);
}

const localEdges = [];
for (const pkg of packages.values()) {
  for (const dependency of pkg.dependencies) {
    if (dependency.path !== null) {
      localEdges.push({
        from: pkg.name,
        to: dependency.name,
        kind: dependency.kind ?? 'normal',
      });
    }
  }
}

const engine = localEdges.filter(({ from }) => from === 'vellum-engine');
for (const { to } of engine) {
  if (to.startsWith('vellum-')) {
    failures.push(`engine has forbidden upward dependency on ${to}`);
  }
}

for (const { from, to } of localEdges) {
  if (from === 'vellum-sdk' && to.startsWith('vellum-adapter-')) {
    failures.push(`SDK has forbidden concrete-adapter dependency on ${to}`);
  }
}

for (const { from, to } of localEdges) {
  if (from.startsWith('vellum-adapter-') && ['vellum', 'vellum-iced'].includes(to)) {
    failures.push(`${from} has forbidden application dependency on ${to}`);
  }
  if (['vellum-session', 'vellum-model'].includes(from) && ['vellum', 'vellum-iced'].includes(to)) {
    failures.push(`${from} has forbidden application dependency on ${to}`);
  }
}

// These are the complete, bounded pre-inversion edges in the accepted migration
// contract. Removal is progress; additions or substitutions are regressions.
const admittedDebt = new Set([
  'vellum-session -> vellum-adapter-local-fs',
  'vellum-session -> vellum-adapter-annotation-json',
  'vellum-model -> vellum-adapter-local-fs',
  'vellum-model -> vellum-adapter-annotation-json',
  'vellum-model -> vellum-adapter-unicode-icu',
]);
const presentDebt = new Set();
for (const { from, to, kind } of localEdges) {
  // Conformance tests may instantiate a concrete implementation of a port.
  // Only shipped dependency edges participate in project closure.
  if (kind === 'dev') continue;
  if (!['vellum-session', 'vellum-model'].includes(from) || !to.startsWith('vellum-adapter-')) {
    continue;
  }
  const edge = `${from} -> ${to}`;
  if (!admittedDebt.has(edge)) {
    failures.push(`unadmitted ${kind} reverse dependency: ${edge}`);
  } else {
    presentDebt.add(edge);
  }
}

if (failures.length !== 0) {
  for (const failure of failures) process.stderr.write(`project boundary: ${failure}\n`);
  process.exit(1);
}

process.stdout.write(
  `project boundaries closed; ${presentDebt.size} admitted migration edges remain\n`,
);
