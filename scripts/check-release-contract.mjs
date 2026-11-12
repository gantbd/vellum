import fs from 'node:fs';
import path from 'node:path';

const failures = [];
const fail = (message) => failures.push(message);
const read = (file) => {
  try {
    return fs.readFileSync(file, 'utf8');
  } catch (error) {
    fail(`cannot read ${file}: ${error.message}`);
    return '';
  }
};

const requiredFiles = [
  'README.md',
  'CHANGELOG.md',
  'SECURITY.md',
  'LICENSE-APACHE',
  'LICENSE-MIT',
  'rust-toolchain.toml',
  'docs/release-policy.md',
  'docs/mcp-extraction.md',
  'docs/semantic-mcp-extraction.md',
  'docs/security-threat-model.md',
  'docs/supply-chain.md',
  'docs/compatibility-baseline.md',
  'docs/artifact-lifecycle.md',
  'docs/launchd-distribution.md',
  'docs/launchd-activation.md',
  'release/dependency-policy.json',
  'release/github-source.json',
  'release/source-sbom.manifest.json',
  'release/semantic-provider-qualification-sbom.manifest.json',
  'release/compatibility-baseline.json',
  'scripts/check-compatibility-baseline.mjs',
  'scripts/check-compatibility-baseline.test.mjs',
  'scripts/check-supply-chain.mjs',
  'scripts/check-supply-chain.test.mjs',
  'scripts/supply-chain-lib.mjs',
  'release/vellum-mcp-extraction.json',
  'scripts/verify-mcp-extraction.mjs',
  'scripts/emit-mcp-peer-source.mjs',
  'release/vellum-semantic-mcp-extraction.json',
  'scripts/verify-semantic-mcp-extraction.mjs',
  'scripts/emit-semantic-mcp-peer-source.mjs',
  'scripts/build-release-candidate.mjs',
  'scripts/build-release-candidate.test.mjs',
  'scripts/release-candidate-lib.mjs',
  'scripts/qualify-release-candidate.mjs',
  'scripts/qualify-release-candidate.test.mjs',
  'scripts/activation-transaction-lib.mjs',
  'scripts/activation-transaction-lib.test.mjs',
  'scripts/cargo-install-qualification-lib.mjs',
  'scripts/qualify-github-source.mjs',
  'scripts/qualify-github-source.test.mjs',
  'distribution/launchd-render/Cargo.toml',
  'distribution/launchd-render/src/lib.rs',
  'distribution/launchd-render/src/main.rs',
  'distribution/launchd-render/tests/qualification.rs',
  'distribution/launchd-activate/Cargo.toml',
  'distribution/launchd-activate/src/lib.rs',
  'distribution/launchd-activate/src/main.rs',
  'distribution/launchd-activate/tests/qualification.rs',
  'distribution/cargo-install/Cargo.toml',
  'distribution/cargo-install/src/bin/vellum-daemon.rs',
  'distribution/cargo-install/src/bin/vellum-mcp.rs',
  'qualification/semantic-lsp-provider/profiles/rust-analyzer-v1.json',
  '.github/workflows/release-contract.yml',
  '.github/workflows/release-candidate.yml',
  '.github/workflows/semantic-provider-qualification.yml',
  '.github/workflows/ci.yml',
];
for (const file of requiredFiles) {
  if (!fs.existsSync(file) || !fs.statSync(file).isFile()) {
    fail(`required distribution file is missing: ${file}`);
  }
}

const root = read('Cargo.toml');
const workspacePackage = root.match(/\[workspace\.package\]([\s\S]*?)(?=\n\[|$)/)?.[1] ?? '';
const requireWorkspaceValue = (key, value) => {
  const pattern = new RegExp(`^${key}\\s*=\\s*${value}\\s*$`, 'm');
  if (!pattern.test(workspacePackage)) fail(`workspace.package.${key} must be ${value}`);
};
requireWorkspaceValue('rust-version', '"1\\.95"');
requireWorkspaceValue('license', '"MIT OR Apache-2\\.0"');
requireWorkspaceValue('repository', '"https://github\\.com/gantbd/vellum"');
requireWorkspaceValue('publish', 'false');

const memberBlock = root.match(/members\s*=\s*\[([\s\S]*?)\]/)?.[1] ?? '';
const members = [...memberBlock.matchAll(/"([^"]+)"/g)].map((match) => match[1]);
if (members.length === 0) fail('workspace must declare at least one member');

for (const member of members) {
  const manifestPath = path.join(member, 'Cargo.toml');
  const manifest = read(manifestPath);
  const packageBlock = manifest.match(/\[package\]([\s\S]*?)(?=\n\[|$)/)?.[1] ?? '';
  if (packageBlock.length === 0) {
    fail(`${manifestPath} has no package table`);
    continue;
  }
  const inheritsClosedPolicy = /^publish\.workspace\s*=\s*true\s*$/m.test(packageBlock);
  const explicitlyClosed = /^publish\s*=\s*false\s*$/m.test(packageBlock);
  if (!inheritsClosedPolicy && !explicitlyClosed) {
    fail(`${manifestPath} must inherit workspace publish policy or set publish = false`);
  }
  if (/^publish\s*=\s*true\s*$/m.test(packageBlock)) {
    fail(`${manifestPath} must not enable publication`);
  }
  if (/^publish\s*=\s*\[/m.test(packageBlock)) {
    fail(`${manifestPath} must not allow a package registry`);
  }
}

const qualificationManifests = [
  'qualification/clipboard-macos/Cargo.toml',
  'qualification/unicode-reference/Cargo.toml',
  'qualification/semantic-lsp-provider/Cargo.toml',
];
for (const manifestPath of qualificationManifests) {
  const manifest = read(manifestPath);
  const packageBlock = manifest.match(/\[package\]([\s\S]*?)(?=\n\[|$)/)?.[1] ?? '';
  if (!/^publish\s*=\s*false\s*$/m.test(packageBlock)) {
    fail(`${manifestPath} must set publish = false`);
  }
}

const toolchain = read('rust-toolchain.toml');
if (!/^channel\s*=\s*"1\.95\.0"\s*$/m.test(toolchain)) {
  fail('rust-toolchain.toml must pin Rust 1.95.0 exactly');
}
for (const component of ['clippy', 'rustfmt']) {
  if (!toolchain.includes(`"${component}"`)) {
    fail(`rust-toolchain.toml must install ${component}`);
  }
}

const readme = read('README.md');
if (!/No crate is being released to\s+crates\.io yet/.test(readme)) {
  fail('README must state the no-crates.io policy');
}
const policy = read('docs/release-policy.md');
if (!policy.includes('no crates.io publication is authorized')) {
  fail('release policy must state that crates.io publication is unauthorized');
}
if (!read('Makefile').includes('check-cargo-install-qualification:')) {
  fail('Makefile must expose the isolated Cargo-install qualification fixture');
}
if (!read('Makefile').includes('check-activation-transaction:')) {
  fail('Makefile must expose the local activation-transaction qualification');
}
if (!read('Makefile').includes('check-launchd-activation:')) {
  fail('Makefile must expose the real launchd activation qualification');
}
for (const target of [
  'check-semantic-provider-qualification:',
  'qualify-semantic-provider:',
]) {
  if (!read('Makefile').includes(target)) {
    fail(`Makefile must expose ${target.slice(0, -1)}`);
  }
}
if (!read('Makefile').includes('--qualify-pinned-provider --profile')) {
  fail('live semantic-provider target must retain its explicit pinned-provider opt in');
}

let sourcePolicy;
try {
  sourcePolicy = JSON.parse(read('release/github-source.json'));
} catch (error) {
  fail(`release/github-source.json is invalid JSON: ${error.message}`);
}
if (sourcePolicy !== undefined) {
  if (sourcePolicy.schema !== 1 || sourcePolicy.channel !== 'github-coordinated-source') {
    fail('GitHub source policy has an unsupported schema or channel');
  }
  if (
    sourcePolicy.installLockContract
    !== 'workspace-build-and-git-install-locked; path-install-local-convenience'
  ) {
    fail('GitHub source policy must bind the locked Git install and keep path install as local convenience');
  }
  if (!/^[0-9a-f]{40}$/.test(sourcePolicy.keystone?.revision ?? '')) {
    fail('GitHub source policy must pin one full Keystone commit');
  }
  if (sourcePolicy.keystone?.relativePath !== '../keystone') {
    fail('GitHub source policy must preserve the local pinned-Keystone verification path');
  }
  const keystoneCrates = new Set(sourcePolicy.keystone?.crates ?? []);
  for (const crate of [
    'keystone-codec',
    'keystone-lifecycle',
    'keystone-observe',
    'keystone-replay',
  ]) {
    if (!keystoneCrates.has(crate)) fail(`GitHub source policy must declare ${crate}`);
  }
  const excluded = new Set(sourcePolicy.excludedPresentations ?? []);
  for (const presentation of ['vellum-tui', 'vellum-gui-macos']) {
    if (!excluded.has(presentation)) fail(`${presentation} must remain excluded from this release`);
  }
  const profiles = new Map(
    (sourcePolicy.sourceInstallProfiles ?? []).map((profile) => [profile.package, profile]),
  );
  const installProfile = profiles.get('vellum');
  if (
    profiles.size !== 1
    || installProfile?.path !== 'distribution/cargo-install'
    || (installProfile?.binaries ?? []).join(',') !== 'vellum-daemon,vellum-mcp'
  ) {
    fail('the source install unit must be exactly vellum yielding daemon and MCP binaries');
  }
  if (
    installProfile?.role
    !== 'installation-only composition of the daemon and its peer MCP connector'
  ) {
    fail('the GitHub source policy must preserve the install package as ownership-neutral');
  }

  const candidate = sourcePolicy.releaseCandidate ?? {};
  if (
    candidate.workflow !== '.github/workflows/release-candidate.yml'
    || candidate.platform !== 'macos'
    || (candidate.packages ?? []).join(',') !== 'vellum-daemon,vellum-mcp'
    || candidate.publishing !== 'none'
    || candidate.signing !== 'none'
    || candidate.attestation !== 'none'
  ) {
    fail('release-candidate policy must remain macOS daemon+MCP only and non-publishing');
  }

  const candidateWorkflow = read(candidate.workflow ?? '.github/workflows/release-candidate.yml');
  const repositorySlug = (repository) => {
    try {
      const url = new URL(repository);
      return url.hostname === 'github.com' ? url.pathname.replace(/^\/|\/$/g, '') : '';
    } catch {
      return '';
    }
  };
  for (const [label, expected] of [
    ['Vellum repository', `repository: ${repositorySlug(sourcePolicy.vellumRepository)}`],
    ['Keystone repository', `repository: ${repositorySlug(sourcePolicy.keystone?.repository)}`],
    ['Keystone revision', `ref: ${sourcePolicy.keystone?.revision}`],
  ]) {
    if (!candidateWorkflow.includes(expected)) {
      fail(`release-candidate workflow drifted from the ${label}`);
    }
  }
  for (const required of [
    'workflow_dispatch:',
    'runs-on: macos-15',
    'ref: ${{ github.sha }}',
    'scripts/build-release-candidate.mjs',
    'make check-artifact-lifecycle',
    'make check-activation-transaction',
    'make check-launchd-distribution',
    'make check-launchd-activation',
    'actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683',
    'actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02',
    '--output-dir=../candidate',
    'path: candidate',
  ]) {
    if (!candidateWorkflow.includes(required)) {
      fail(`release-candidate workflow is missing pinned contract: ${required}`);
    }
  }
  for (const forbidden of [
    'vellum-tui',
    'vellum-gui-macos',
    'actions/attest',
    'sigstore',
    'gh release',
    'cargo publish',
  ]) {
    if (candidateWorkflow.includes(forbidden)) {
      fail(`release-candidate workflow must not contain ${forbidden}`);
    }
  }
  const actionUses = [...candidateWorkflow.matchAll(/^\s*uses:\s*(\S+)\s*$/gm)]
    .map((match) => match[1]);
  const expectedActions = [
    'actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683',
    'actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683',
    'actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02',
  ];
  if (actionUses.join(',') !== expectedActions.join(',')) {
    fail('release-candidate workflow action closure must be exactly two checkouts and one upload');
  }
  const checkoutRepositories = [
    ...candidateWorkflow.matchAll(/^\s*repository:\s*(\S+)\s*$/gm),
  ].map((match) => match[1]);
  const checkoutRevisions = [
    ...candidateWorkflow.matchAll(/^\s*ref:\s*(.+?)\s*$/gm),
  ].map((match) => match[1]);
  if (
    checkoutRepositories.join(',')
      !== [
        repositorySlug(sourcePolicy.vellumRepository),
        repositorySlug(sourcePolicy.keystone?.repository),
      ].join(',')
    || checkoutRevisions.join(',')
      !== ['${{ github.sha }}', sourcePolicy.keystone?.revision].join(',')
  ) {
    fail('release-candidate checkout repositories or revisions drifted from source policy');
  }
  if (/^\s*cargo\s+/m.test(candidateWorkflow)) {
    fail('release-candidate workflow must delegate package scope to the policy-bound builder');
  }
}

let mcpExtraction;
try {
  mcpExtraction = JSON.parse(read('release/vellum-mcp-extraction.json'));
} catch (error) {
  fail(`release/vellum-mcp-extraction.json is invalid JSON: ${error.message}`);
}
if (mcpExtraction !== undefined) {
  const copied = new Set((mcpExtraction.copy ?? []).map((entry) => entry.from));
  const retained = new Set(mcpExtraction.retainedVellumQualification ?? []);
  if (
    mcpExtraction.schema !== 1
    || mcpExtraction.archiveRoot !== 'vellum-mcp'
    || mcpExtraction.targetSibling !== '../vellum-mcp'
    || mcpExtraction.publicSdk?.package !== 'vellum-sdk-client'
    || mcpExtraction.publicSdk?.repository !== sourcePolicy?.vellumRepository
    || mcpExtraction.credentialAdapter?.package !== 'vellum-adapter-local-authentication'
    || mcpExtraction.credentialAdapter?.repository !== sourcePolicy?.vellumRepository
    || (mcpExtraction.normalDependencies ?? []).sort().join(',')
      !== 'serde_json,vellum-adapter-local-authentication,vellum-sdk-client'
    || mcpExtraction.publication !== 'none'
  ) {
    fail('MCP extraction must remain a non-publishing public-client-SDK peer with the local credential adapter');
  }
  const blockers = mcpExtraction.immutableDependencyBlockers ?? [];
  const immutableDependencies = mcpExtraction.qualifiedImmutableDependencies ?? [];
  if (
    blockers.length !== 0
    || immutableDependencies.length !== 1
    || immutableDependencies[0]?.dependency !== 'keystone-codec'
    || immutableDependencies[0]?.repository !== sourcePolicy?.keystone?.repository
    || immutableDependencies[0]?.revision !== sourcePolicy?.keystone?.revision
  ) {
    fail('MCP extraction must retain the exact transitive Keystone Git source');
  }
  for (const source of [
    'connectors/vellum-mcp/CHANGELOG.md',
    'connectors/vellum-mcp/README.md',
    'connectors/vellum-mcp/SECURITY.md',
    'connectors/vellum-mcp/src/cli.rs',
    'connectors/vellum-mcp/src/lib.rs',
    'connectors/vellum-mcp/src/main.rs',
    'connectors/vellum-mcp/tests/mcp.rs',
  ]) {
    if (!copied.has(source)) fail(`MCP extraction is missing portable source ${source}`);
  }
  if (
    copied.has('connectors/vellum-mcp/tests/live_daemon.rs')
    || !retained.has('connectors/vellum-mcp/tests/live_daemon.rs')
  ) {
    fail('the real-daemon MCP test must remain Vellum-owned qualification');
  }
}

let semanticMcpExtraction;
try {
  semanticMcpExtraction = JSON.parse(read('release/vellum-semantic-mcp-extraction.json'));
} catch (error) {
  fail(`release/vellum-semantic-mcp-extraction.json is invalid JSON: ${error.message}`);
}
if (semanticMcpExtraction !== undefined) {
  const dependencies = semanticMcpExtraction.vellumDependencies ?? [];
  const dependencyNames = dependencies.map((dependency) => dependency.package).sort();
  const copied = new Set((semanticMcpExtraction.copy ?? []).map((entry) => entry.from));
  const immutable = semanticMcpExtraction.qualifiedImmutableDependencies ?? [];
  if (
    semanticMcpExtraction.schema !== 1
    || semanticMcpExtraction.archiveRoot !== 'vellum-semantic-mcp'
    || semanticMcpExtraction.targetSibling !== '../vellum-semantic-mcp'
    || semanticMcpExtraction.sourceRepository !== sourcePolicy?.vellumRepository
    || dependencyNames.join(',')
      !== [
        'vellum-adapter-local-authentication',
        'vellum-sdk-client',
        'vellum-sdk-semantic-client',
      ].join(',')
    || semanticMcpExtraction.publication !== 'none'
  ) {
    fail('semantic MCP extraction must remain a non-publishing SDK-facade peer');
  }
  if (
    immutable.length !== 1
    || immutable[0]?.repository !== sourcePolicy?.keystone?.repository
    || immutable[0]?.revision !== sourcePolicy?.keystone?.revision
    || (immutable[0]?.packages ?? []).sort().join(',')
      !== 'keystone-codec,keystone-lifecycle,keystone-observe,keystone-replay'
  ) {
    fail('semantic MCP extraction must retain the exact Keystone source closure');
  }
  for (const source of [
    'connectors/vellum-semantic-mcp/CHANGELOG.md',
    'connectors/vellum-semantic-mcp/README.md',
    'connectors/vellum-semantic-mcp/SECURITY.md',
    'connectors/vellum-semantic-mcp/src/lib.rs',
    'connectors/vellum-semantic-mcp/src/main.rs',
    'connectors/vellum-semantic-mcp/src/stdio.rs',
    'connectors/vellum-semantic-mcp/src/tests.rs',
    'connectors/vellum-semantic-mcp/tests/stdio_peer.rs',
  ]) {
    if (!copied.has(source)) fail(`semantic MCP extraction is missing portable source ${source}`);
  }
  const directImplementationDependencies = (semanticMcpExtraction.normalDependencies ?? [])
    .filter((dependency) => [
      'vellum-semantic-client',
      'vellum-semantic-protocol',
      'vellum-semantic-service',
      'vellum-semantic-session-client',
      'vellum-semantic-transport',
    ].includes(dependency));
  if (directImplementationDependencies.length !== 0) {
    fail('semantic MCP extraction bypasses the semantic SDK facade');
  }
}

let dependencyPolicy;
try {
  dependencyPolicy = JSON.parse(read('release/dependency-policy.json'));
} catch (error) {
  fail(`release/dependency-policy.json is invalid JSON: ${error.message}`);
}
if (dependencyPolicy !== undefined) {
  const external = dependencyPolicy.pathPolicy?.external ?? [];
  const expectedGitSource = `git+${sourcePolicy?.keystone?.repository}?rev=${sourcePolicy?.keystone?.revision}#${sourcePolicy?.keystone?.revision}`;
  const gitSources = dependencyPolicy.gitPolicy?.sources ?? [];
  const keystoneGit = gitSources[0];
  if (
    dependencyPolicy.schema !== 1
    || (dependencyPolicy.allowedGitSources ?? []).join(',') !== expectedGitSource
    || (dependencyPolicy.allowedRegistries ?? []).join(',')
      !== 'registry+https://github.com/rust-lang/crates.io-index'
    || external.length !== 0
    || gitSources.length !== 1
    || keystoneGit?.repository !== sourcePolicy?.keystone?.repository
    || keystoneGit?.revision !== sourcePolicy?.keystone?.revision
    || keystoneGit?.source !== expectedGitSource
    || keystoneGit?.releaseStatus !== 'qualified-immutable-source'
    || (keystoneGit?.packages ?? []).map((pkg) => pkg.name).sort().join(',')
      !== [...(sourcePolicy?.keystone?.crates ?? [])].sort().join(',')
  ) {
    fail('dependency policy must retain the exact immutable Keystone source closure');
  }
  const revisionStatuses = new Map(
    (keystoneGit?.packages ?? []).map((pkg) => [pkg.name, pkg.revisionStatus]),
  );
  for (const crate of sourcePolicy?.keystone?.crates ?? []) {
    if (revisionStatuses.get(crate) !== 'present-at-pinned-revision') {
      fail(`${crate} must remain qualified at the pinned Keystone revision`);
    }
  }
}

const releaseContractWorkflow = read('.github/workflows/release-contract.yml');
for (const required of [
  'permissions:\n  contents: read',
  'actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683',
  'make check-release-contract',
  'make check-cargo-install-qualification',
]) {
  if (!releaseContractWorkflow.includes(required)) {
    fail(`release-contract workflow is missing: ${required}`);
  }
}
if (/\bcargo\s+(publish|install)\b/.test(releaseContractWorkflow)) {
  fail('release-contract workflow must delegate qualification and never publish/install directly');
}

const ciWorkflow = read('.github/workflows/ci.yml');
for (const required of [
  'permissions:\n  contents: read',
  'timeout-minutes: 45',
  'repository: gantbd/vellum',
  'ref: ${{ github.sha }}',
  'repository: gantbd/keystone',
  `ref: ${sourcePolicy?.keystone?.revision ?? ''}`,
  'actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683',
  'cargo fetch --locked',
  'make check-cargo-install-qualification qualify-github-source',
  'check-supply-chain',
  'check-effect-boundaries',
  'check-compatibility',
  'cargo fmt --all -- --check',
  'cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings',
  'cargo test --workspace --all-targets --all-features --locked --offline',
  'RUSTDOCFLAGS: -D warnings',
  'cargo doc --workspace --all-features --no-deps --locked --offline',
  'cargo test --workspace --doc --locked --offline',
  'make qualify-unicode-portable',
  'make check-semantic-provider-qualification',
]) {
  if (!ciWorkflow.includes(required)) fail(`coordinated CI is missing: ${required}`);
}

const providerWorkflow = read('.github/workflows/semantic-provider-qualification.yml');
let providerProfile;
try {
  providerProfile = JSON.parse(read(
    'qualification/semantic-lsp-provider/profiles/rust-analyzer-v1.json',
  ));
} catch (error) {
  fail(`semantic-provider profile is invalid JSON: ${error.message}`);
}
const pinnedArtifact = providerProfile?.qualification?.artifact ?? {};
if (
  providerProfile?.schema_version !== 1
  || providerProfile?.qualification?.kind !== 'pinned'
  || providerProfile?.qualification?.platform?.os !== 'macos'
  || providerProfile?.qualification?.platform?.arch !== 'aarch64'
  || pinnedArtifact.compression !== 'gzip'
  || !/^https:\/\/github\.com\/rust-lang\/rust-analyzer\/releases\/download\//.test(
    pinnedArtifact.source_url ?? '',
  )
  || !/^[0-9a-f]{64}$/.test(pinnedArtifact.archive_sha256 ?? '')
  || !/^[0-9a-f]{64}$/.test(pinnedArtifact.sha256 ?? '')
  || providerProfile?.qualification?.provider?.readiness?.method
    !== 'experimental/serverStatus'
) {
  fail('semantic-provider profile must pin one exact official macOS ARM64 artifact and readiness schema');
}
for (const required of [
  'workflow_dispatch:',
  'schedule:',
  'permissions:\n  contents: read',
  'runs-on: macos-15',
  'repository: gantbd/vellum',
  'ref: ${{ github.sha }}',
  'repository: gantbd/keystone',
  `ref: ${sourcePolicy?.keystone?.revision ?? ''}`,
  'actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683',
  'SEMANTIC_PROVIDER_ARCHIVE_URL',
  'SEMANTIC_PROVIDER_ARCHIVE_SHA256',
  'SEMANTIC_PROVIDER_EXECUTABLE_SHA256',
  'rust-analyzer/releases/download/2026-11-09/rust-analyzer-aarch64-apple-darwin.gz',
  'e43a65008f8f4291320f33c0545dd577ad7623a7407b584a27155833d3722dfb',
  'f21b42b915d3f0d05a8c799f74ed54dda7b74de542a48f0ae5f8c394ec7f6b9f',
  'shasum -a 256 -c',
  'gzip -dc',
  'make qualify-semantic-provider',
]) {
  if (!providerWorkflow.includes(required)) {
    fail(`semantic-provider workflow is missing fail-closed contract: ${required}`);
  }
}
for (const value of [
  pinnedArtifact.source_url,
  pinnedArtifact.archive_sha256,
  pinnedArtifact.sha256,
]) {
  if (typeof value === 'string' && !providerWorkflow.includes(value)) {
    fail('semantic-provider workflow drifted from its checked-in artifact profile');
  }
}
for (const forbidden of [
  'continue-on-error:',
  'if:',
  '|| true',
  'cargo publish',
  'inputs.artifact_',
  'vars.SEMANTIC_PROVIDER_',
]) {
  if (providerWorkflow.includes(forbidden)) {
    fail(`semantic-provider workflow must not contain ${forbidden}`);
  }
}
const providerActions = [
  ...providerWorkflow.matchAll(/^\s*uses:\s*(\S+)\s*$/gm),
].map((match) => match[1]);
if (
  providerActions.join(',')
  !== [
    'actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683',
    'actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683',
  ].join(',')
) {
  fail('semantic-provider workflow action closure must be exactly two pinned checkouts');
}
const ciActions = [...ciWorkflow.matchAll(/^\s*uses:\s*(\S+)\s*$/gm)].map(
  (match) => match[1],
);
if (
  ciActions.join(',')
  !== [
    'actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683',
    'actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683',
  ].join(',')
) {
  fail('coordinated CI action closure must be exactly two pinned checkouts');
}
const sourceSlug = (repository) => {
  try {
    const url = new URL(repository);
    return url.hostname === 'github.com' ? url.pathname.replace(/^\/|\/$/g, '') : '';
  } catch {
    return '';
  }
};
const ciRepositories = [...ciWorkflow.matchAll(/^\s*repository:\s*(\S+)\s*$/gm)].map(
  (match) => match[1],
);
const ciRevisions = [...ciWorkflow.matchAll(/^\s*ref:\s*(.+?)\s*$/gm)].map(
  (match) => match[1],
);
if (
  ciRepositories.join(',')
    !== [sourceSlug(sourcePolicy?.vellumRepository), sourceSlug(sourcePolicy?.keystone?.repository)].join(',')
  || ciRevisions.join(',')
    !== ['${{ github.sha }}', sourcePolicy?.keystone?.revision].join(',')
) {
  fail('coordinated CI checkout repositories or revisions drifted from source policy');
}

for (const file of fs.readdirSync('.github/workflows')) {
  if (!/\.ya?ml$/.test(file)) continue;
  const workflowPath = path.join('.github/workflows', file);
  if (/\bcargo\s+publish\b/.test(read(workflowPath))) {
    fail(`${workflowPath} must not invoke cargo publish`);
  }
}

if (failures.length !== 0) {
  for (const failure of failures) process.stderr.write(`Vellum release contract: ${failure}\n`);
  process.exit(1);
}

process.stdout.write(
  `Vellum release contract closed; ${members.length} workspace packages and ${qualificationManifests.length} qualification packages are non-publishable\n`,
);
