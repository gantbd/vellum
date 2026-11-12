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
  ['vellum-semantic-protocol', '/protocol/'],
  ['vellum-semantic-wire', '/protocol/'],
  ['vellum-service', '/service/'],
  ['vellum-semantic-service', '/service/'],
  ['vellum-semantic-session-broker', '/service/'],
  ['vellum-semantic-session-actor', '/service/'],
  ['vellum-semantic-apply-service', '/service/'],
  ['vellum-client', '/client/'],
  ['vellum-semantic-client', '/client/'],
  ['vellum-semantic-session-client', '/client/'],
  ['vellum-transport', '/transport/'],
  ['vellum-semantic-transport', '/transport/'],
  ['vellum-adapter-local-authentication', '/adapters/'],
  ['vellum-adapter-local-fs', '/adapters/'],
  ['vellum-adapter-annotation-json', '/adapters/'],
  ['vellum-adapter-unicode-icu', '/adapters/'],
  ['vellum-adapter-semantic-lsp', '/adapters/'],
  ['vellum-adapter-clipboard-macos', '/adapters/'],
  ['vellum-mcp', '/connectors/'],
  ['vellum-semantic-mcp', '/connectors/'],
  ['vellum-daemon', '/applications/'],
  ['vellum-semantic-local', '/applications/'],
  ['vellum', '/distribution/'],
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
requireEdges('vellum-semantic-service', ['vellum-host']);
requireEdges('vellum-semantic-session-broker', ['vellum-host', 'vellum-semantic-protocol']);
forbidEdges(
  'vellum-semantic-session-broker',
  ['vellum-engine', 'vellum-editor', 'vellum-session', 'vellum-service', 'vellum-semantic-service'],
);
requireEdges('vellum-semantic-session-actor', [
  'vellum-host',
  'vellum-semantic-protocol',
  'vellum-semantic-session-broker',
]);
forbidEdges(
  'vellum-semantic-session-actor',
  [
    'vellum-engine',
    'vellum-editor',
    'vellum-session',
    'vellum-service',
    'vellum-semantic-service',
    'vellum-wire',
    'vellum-semantic-wire',
    'vellum-transport',
    'vellum-mcp',
    'vellum-semantic-mcp',
    'vellum-semantic-local',
  ],
);
forbidEdges(
  'vellum-semantic-service',
  ['vellum-engine', 'vellum-editor', 'vellum-session', 'vellum-service'],
);
requireEdges(
  'vellum-semantic-apply-service',
  ['vellum-engine', 'vellum-editor', 'vellum-host', 'vellum-semantic-service'],
  false,
);
forbidEdges(
  'vellum-semantic-apply-service',
  [
    'vellum-client',
    'vellum-semantic-client',
    'vellum-service',
    'vellum-session',
    'vellum-adapter-local-authentication',
    'vellum-adapter-local-fs',
    'vellum-adapter-annotation-json',
    'vellum-adapter-unicode-icu',
    'vellum-adapter-semantic-lsp',
    'vellum-adapter-clipboard-macos',
    'vellum-mcp',
    'vellum-protocol',
    'vellum-wire',
    'vellum-transport',
  ],
  false,
);
requireEdges('vellum-semantic-client', [
  'vellum-host',
  'vellum-semantic-service',
  'vellum-semantic-protocol',
  'vellum-semantic-session-client',
]);
requireEdges('vellum-semantic-session-client', ['vellum-host', 'vellum-semantic-protocol']);
forbidEdges(
  'vellum-semantic-session-client',
  [
    'vellum-engine',
    'vellum-editor',
    'vellum-session',
    'vellum-service',
    'vellum-semantic-service',
    'vellum-semantic-session-broker',
    'vellum-wire',
    'vellum-semantic-wire',
    'vellum-transport',
  ],
);
forbidEdges(
  'vellum-semantic-client',
  ['vellum-engine', 'vellum-editor', 'vellum-session', 'vellum-service'],
);
requireEdges('vellum-wire', ['keystone-codec', 'vellum-protocol']);
forbidEdges(
  'vellum-wire',
  ['vellum-engine', 'vellum-host', 'vellum-session', 'vellum-editor', 'vellum-service'],
);
requireEdges('vellum-semantic-protocol', ['vellum-host']);
forbidEdges(
  'vellum-semantic-protocol',
  ['vellum-engine', 'vellum-editor', 'vellum-session', 'vellum-service', 'vellum-semantic-service'],
);
requireEdges('vellum-semantic-wire', [
  'keystone-codec',
  'vellum-host',
  'vellum-semantic-protocol',
]);
forbidEdges(
  'vellum-semantic-wire',
  ['vellum-engine', 'vellum-editor', 'vellum-session', 'vellum-service', 'vellum-semantic-service'],
);
requireEdges('vellum-transport', ['vellum-protocol', 'vellum-wire']);
requireEdges('vellum-semantic-transport', [
  'vellum-semantic-protocol',
  'vellum-semantic-session-client',
  'vellum-semantic-wire',
]);
forbidEdges(
  'vellum-semantic-transport',
  [
    'vellum-engine',
    'vellum-editor',
    'vellum-host',
    'vellum-session',
    'vellum-service',
    'vellum-semantic-service',
    'vellum-semantic-session-broker',
    'vellum-semantic-session-actor',
    'vellum-mcp',
    'vellum-semantic-mcp',
    'vellum-semantic-local',
  ],
);
requireEdges(
  'vellum-daemon',
  [
    'vellum-service',
    'vellum-transport',
    'vellum-editor',
    'vellum-adapter-local-fs',
    'vellum-adapter-local-authentication',
  ],
);
requireEdges(
  'vellum-semantic-local',
  ['vellum-adapter-semantic-lsp', 'vellum-host'],
  false,
);
forbidEdges(
  'vellum-semantic-local',
  ['vellum-engine', 'vellum-editor', 'vellum-session', 'vellum-service', 'vellum-mcp'],
  false,
);
requireEdges(
  'vellum-mcp',
  ['vellum-sdk-client', 'vellum-adapter-local-authentication'],
  false,
);
requireEdges(
  'vellum-semantic-mcp',
  [
    'vellum-sdk-client',
    'vellum-semantic-client',
    'vellum-semantic-service',
    'vellum-adapter-local-authentication',
  ],
  false,
);
forbidEdges(
  'vellum-semantic-mcp',
  [
    'vellum-host',
    'vellum-semantic-apply-service',
    'vellum-adapter-semantic-lsp',
    'vellum-editor',
    'vellum-engine',
    'vellum-session',
    'vellum-service',
    'vellum-semantic-local',
  ],
  false,
);
forbidEdges(
  'vellum-mcp',
  [
    'vellum-client',
    'vellum-protocol',
    'vellum-transport',
    'vellum-engine',
    'vellum-host',
    'vellum-session',
    'vellum-editor',
    'vellum-service',
  ],
  false,
);
requireEdges('vellum-sdk-embedded', ['vellum-host', 'vellum-session', 'vellum-editor']);
requireEdges(
  'vellum-sdk-client',
  ['vellum-client', 'vellum-protocol', 'vellum-wire', 'vellum-transport'],
);
requireEdges('vellum', ['vellum-daemon', 'vellum-mcp'], false);
forbidEdges(
  'vellum',
  [
    'vellum-engine',
    'vellum-editor',
    'vellum-host',
    'vellum-service',
    'vellum-transport',
    'vellum-adapter-local-fs',
    'vellum-adapter-local-authentication',
  ],
  false,
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
      ![
        'vellum-daemon',
        'vellum-mcp',
        'vellum-semantic-local',
        'vellum-semantic-mcp',
      ].includes(pkg.name) &&
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

const semanticProviderCargo = spawnSync(
  'cargo',
  [
    'metadata', '--offline', '--locked', '--no-deps', '--format-version', '1',
    '--manifest-path', 'qualification/semantic-lsp-provider/Cargo.toml',
  ],
  { encoding: 'utf8' },
);
if (semanticProviderCargo.status !== 0) {
  failures.push(`semantic-provider qualification metadata failed: ${semanticProviderCargo.stderr}`);
} else {
  const semanticProviderMetadata = JSON.parse(semanticProviderCargo.stdout);
  const qualification = semanticProviderMetadata.packages.find(
    (pkg) => pkg.name === 'vellum-semantic-lsp-provider-qualification',
  );
  if (qualification === undefined) {
    failures.push('semantic-provider qualification package is missing');
  } else {
    if (qualification.publish?.length !== 0) {
      failures.push('semantic-provider qualification must remain unpublished');
    }
    const edges = new Set(qualification.dependencies.map((dependency) => dependency.name));
    if (!edges.has('vellum-semantic-local')) {
      failures.push('semantic-provider qualification must exercise vellum-semantic-local');
    }
    for (const forbidden of ['vellum-mcp', 'vellum-semantic-mcp', 'vellum-tui', 'vellum-gui-macos']) {
      if (edges.has(forbidden)) {
        failures.push(`semantic-provider qualification has forbidden presentation edge ${forbidden}`);
      }
    }
  }
  if (semanticProviderMetadata.workspace_members.length !== 1) {
    failures.push('semantic-provider qualification must remain a standalone workspace');
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
  `Vellum boundaries closed; ${packages.size} crates, 2 zero-authority SDK facades, 4 adapter-selecting composition roots, 3 standalone qualification packages\n`,
);
