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
const locations = new Map([
  ['vellum-engine', '/engine/'],
  ['vellum-host', '/host/'],
  ['vellum-session', '/host/'],
  ['vellum-editor', '/host/'],
  ['vellum-protocol', '/protocol/'],
  ['vellum-wire', '/protocol/'],
  ['vellum-service', '/service/'],
  ['vellum-client', '/client/'],
  ['vellum-transport', '/transport/'],
  ['vellum-adapter-local-fs', '/adapters/'],
  ['vellum-adapter-annotation-json', '/adapters/'],
  ['vellum-adapter-unicode-icu', '/adapters/'],
  ['vellum-adapter-clipboard-macos', '/adapters/'],
  ['vellum-mcp', '/connectors/'],
  ['vellum-daemon', '/applications/'],
  ['vellum-sdk-embedded', '/sdk/'],
  ['vellum-sdk-client', '/sdk/'],
]);

for (const [name, location] of locations) {
  const pkg = packages.get(name);
  if (pkg === undefined) {
    failures.push(`missing workspace crate ${name}`);
  } else if (!pkg.manifest_path.includes(location)) {
    failures.push(`${name} is outside categorical directory ${location}`);
  }
}
if (packages.size !== locations.size) {
  failures.push(`workspace has ${packages.size} crates; taxonomy declares ${locations.size}`);
}

const dependencies = (name, includeDev = true) =>
  new Set(
    (packages.get(name)?.dependencies ?? [])
      .filter((dependency) => includeDev || (dependency.kind ?? 'normal') !== 'dev')
      .map((dependency) => dependency.name),
  );
const requireEdges = (name, required, includeDev = true) => {
  const edges = dependencies(name, includeDev);
  for (const dependency of required) {
    if (!edges.has(dependency)) failures.push(`${name} must depend on ${dependency}`);
  }
};
const forbidEdges = (name, forbidden, includeDev = true) => {
  const edges = dependencies(name, includeDev);
  for (const dependency of forbidden) {
    if (edges.has(dependency)) failures.push(`${name} has forbidden dependency on ${dependency}`);
  }
};

for (const dependency of dependencies('vellum-engine')) {
  if (dependency.startsWith('vellum-')) {
    failures.push(`engine has forbidden upward dependency on ${dependency}`);
  }
}
if (dependencies('vellum-protocol').size !== 0) {
  failures.push('protocol must remain dependency-free');
}
forbidEdges(
  'vellum-client',
  ['vellum-engine', 'vellum-host', 'vellum-session', 'vellum-editor', 'vellum-service'],
);
requireEdges('vellum-wire', ['keystone-codec', 'vellum-protocol']);
forbidEdges(
  'vellum-wire',
  ['vellum-engine', 'vellum-host', 'vellum-session', 'vellum-editor', 'vellum-service'],
);
requireEdges('vellum-transport', ['vellum-protocol', 'vellum-wire']);
requireEdges(
  'vellum-daemon',
  ['vellum-service', 'vellum-transport', 'vellum-editor', 'vellum-adapter-local-fs'],
);
requireEdges('vellum-mcp', ['vellum-client', 'vellum-protocol', 'vellum-transport'], false);
forbidEdges(
  'vellum-mcp',
  ['vellum-engine', 'vellum-host', 'vellum-session', 'vellum-editor', 'vellum-service'],
  false,
);
requireEdges('vellum-sdk-embedded', ['vellum-host', 'vellum-session', 'vellum-editor']);
requireEdges(
  'vellum-sdk-client',
  ['vellum-client', 'vellum-protocol', 'vellum-wire', 'vellum-transport'],
);

for (const pkg of packages.values()) {
  for (const dependency of pkg.dependencies) {
    const kind = dependency.kind ?? 'normal';
    if (['vellum-tui', 'vellum-gui-macos'].includes(dependency.name)) {
      failures.push(`${pkg.name} has forbidden presentation dependency on ${dependency.name}`);
    }
    if (
      kind !== 'dev' &&
      dependency.name.startsWith('vellum-adapter-') &&
      pkg.name !== 'vellum-daemon' &&
      !pkg.name.startsWith('vellum-adapter-')
    ) {
      failures.push(`${pkg.name} has forbidden concrete-adapter dependency on ${dependency.name}`);
    }
  }
}

for (const sdk of ['vellum-sdk-embedded', 'vellum-sdk-client']) {
  if (packages.get(sdk)?.publish?.length !== 0) {
    failures.push(`${sdk} facade must remain unpublished until release approval`);
  }
}

const qualificationCargo = spawnSync(
  'cargo',
  [
    'metadata', '--offline', '--locked', '--no-deps', '--format-version', '1',
    '--manifest-path', 'qualification/clipboard-macos/Cargo.toml',
  ],
  { encoding: 'utf8' },
);
if (qualificationCargo.status !== 0) {
  failures.push(`clipboard qualification metadata failed: ${qualificationCargo.stderr}`);
} else {
  const qualificationMetadata = JSON.parse(qualificationCargo.stdout);
  const qualification = qualificationMetadata.packages.find(
    (pkg) => pkg.name === 'vellum-clipboard-macos-qualification',
  );
  if (qualification === undefined) {
    failures.push('clipboard qualification package is missing');
  } else if (qualification.publish?.length !== 0) {
    failures.push('clipboard qualification must remain unpublished');
  }
  if (qualificationMetadata.workspace_members.length !== 1) {
    failures.push('clipboard qualification must remain a standalone workspace');
  }
}

const unicodeCargo = spawnSync(
  'cargo',
  [
    'metadata', '--offline', '--locked', '--no-deps', '--format-version', '1',
    '--manifest-path', 'qualification/unicode-reference/Cargo.toml',
  ],
  { encoding: 'utf8' },
);
if (unicodeCargo.status !== 0) {
  failures.push(`Unicode qualification metadata failed: ${unicodeCargo.stderr}`);
} else {
  const unicodeMetadata = JSON.parse(unicodeCargo.stdout);
  const unicodeReference = unicodeMetadata.packages.find(
    (pkg) => pkg.name === 'vellum-unicode-reference',
  );
  if (unicodeReference === undefined) {
    failures.push('Unicode qualification package is missing');
  } else if (unicodeReference.publish?.length !== 0) {
    failures.push('Unicode qualification must remain unpublished');
  }
  if (unicodeMetadata.workspace_members.length !== 1) {
    failures.push('Unicode qualification must remain a standalone workspace');
  }
}

const rootManifest = fs.readFileSync('Cargo.toml', 'utf8');
const qualificationManifest = fs.readFileSync(
  'qualification/clipboard-macos/Cargo.toml',
  'utf8',
);
if (/^\[patch\.crates-io\]/m.test(rootManifest)) {
  failures.push('production workspace must not patch crates.io');
}
if (!/^\[patch\.crates-io\][\s\S]*^objc2-app-kit\s*=\s*\{\s*path\s*=\s*"\.\.\/\.\.\/vendor\/objc2-app-kit"\s*\}/m.test(qualificationManifest)) {
  failures.push('clipboard qualification must own the isolated AppKit patch');
}

if (failures.length !== 0) {
  for (const failure of failures) process.stderr.write(`Vellum boundary: ${failure}\n`);
  process.exit(1);
}
process.stdout.write(
  `Vellum boundaries closed; ${packages.size} crates, 2 zero-authority SDK facades, 1 declared production adapter edge\n`,
);
