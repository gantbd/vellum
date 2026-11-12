import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const sha256 = (value) => crypto.createHash('sha256').update(value).digest('hex');
const inside = (parent, candidate) => {
  const relative = path.relative(parent, candidate);
  return relative === '' || (!relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative));
};
const requireDirectory = (candidate, label) => {
  const metadata = fs.lstatSync(candidate);
  if (metadata.isSymbolicLink() || !metadata.isDirectory()) {
    throw new Error(`${label} must be a non-symlink directory`);
  }
};
const requireManifest = (candidate, label) => {
  const metadata = fs.lstatSync(candidate);
  if (metadata.isSymbolicLink() || !metadata.isFile()) {
    throw new Error(`${label} manifest must be a regular non-symlink file`);
  }
};
const tomlString = (value, label) => {
  try {
    return JSON.parse(value);
  } catch {
    throw new Error(`unsupported Cargo.lock ${label}`);
  }
};

export const parseCargoLock = (text) => {
  const packages = [];
  let current;
  for (const line of text.split(/\r?\n/)) {
    if (line === '[[package]]') {
      current = {};
      packages.push(current);
      continue;
    }
    if (current === undefined) continue;
    const match = line.match(/^(name|version|source|checksum) = (".*")$/);
    if (match) current[match[1]] = tomlString(match[2], match[1]);
  }
  for (const pkg of packages) {
    if (typeof pkg.name !== 'string' || typeof pkg.version !== 'string') {
      throw new Error('Cargo.lock contains an incomplete package entry');
    }
  }
  return packages;
};

const lockKey = (pkg) => `${pkg.name}\u0000${pkg.version}\u0000${pkg.source ?? 'path'}`;
const packageRef = (pkg, sourceKind) => {
  if (sourceKind === 'registry') return `pkg:cargo/${pkg.name}@${pkg.version}`;
  if (sourceKind === 'workspace') return `urn:vellum:workspace:${pkg.name}@${pkg.version}`;
  if (sourceKind === 'git') return `urn:vellum:git:${pkg.name}@${pkg.version}`;
  return `urn:vellum:external:keystone:${pkg.name}@${pkg.version}`;
};

export const evaluateSupplyChain = ({ metadata, lockText, policy, root }) => {
  if (policy.schema !== 1 || policy.sbom?.format !== 'CycloneDX') {
    throw new Error('unsupported dependency policy schema or SBOM format');
  }
  const allowedRegistries = new Set(policy.allowedRegistries ?? []);
  const allowedGit = new Set(policy.allowedGitSources ?? []);
  const allowedLicenses = new Set(policy.allowedLicenseExpressions ?? []);
  const workspaceIds = new Set(metadata.workspace_members ?? []);
  const workspaceRoot = path.resolve(root, policy.pathPolicy?.workspaceRoot ?? '.');
  requireDirectory(workspaceRoot, 'workspace root');
  const canonicalWorkspace = fs.realpathSync(workspaceRoot);
  const gitPolicies = policy.gitPolicy?.sources ?? [];
  const gitPackages = new Map();
  for (const source of gitPolicies) {
    if (
      source.releaseStatus !== 'qualified-immutable-source'
      || !allowedGit.has(source.source)
      || typeof source.repository !== 'string'
      || !/^[0-9a-f]{40}$/.test(source.revision ?? '')
      || !source.source.endsWith(`#${source.revision}`)
    ) {
      throw new Error(`invalid immutable Git policy for ${source.name ?? 'unknown source'}`);
    }
    for (const pkg of source.packages ?? []) {
      const key = `${pkg.name}\u0000${pkg.version}`;
      if (gitPackages.has(key)) throw new Error(`duplicate Git policy for ${pkg.name}`);
      gitPackages.set(key, { ...pkg, source });
    }
  }
  const externalPolicies = policy.pathPolicy?.external ?? [];
  const externalPackages = new Map();
  for (const external of externalPolicies) {
    if (external.releaseStatus !== 'blocker') {
      throw new Error(`external path ${external.name} must remain an explicit release blocker`);
    }
    const externalRoot = path.resolve(root, external.root);
    requireDirectory(externalRoot, `external path ${external.name}`);
    const canonicalExternal = fs.realpathSync(externalRoot);
    for (const pkg of external.packages ?? []) {
      const key = `${pkg.name}\u0000${pkg.version}`;
      if (externalPackages.has(key)) throw new Error(`duplicate external path policy for ${pkg.name}`);
      externalPackages.set(key, {
        ...pkg,
        external,
        expectedManifest: path.resolve(externalRoot, pkg.manifest),
        canonicalExternal,
      });
    }
  }

  const locked = parseCargoLock(lockText);
  const lockByKey = new Map();
  for (const pkg of locked) {
    const key = lockKey(pkg);
    if (lockByKey.has(key)) throw new Error(`ambiguous locked package ${pkg.name} ${pkg.version}`);
    lockByKey.set(key, pkg);
    if (pkg.source?.startsWith('git+')) {
      const approved = gitPackages.get(`${pkg.name}\u0000${pkg.version}`);
      if (!allowedGit.has(pkg.source) || approved?.source.source !== pkg.source) {
        throw new Error(`unapproved Git source for ${pkg.name}`);
      }
    } else if (pkg.source !== undefined && !allowedRegistries.has(pkg.source)) {
      throw new Error(`unapproved registry or unknown source for ${pkg.name}`);
    }
    if (
      pkg.source?.startsWith('registry+')
      && policy.requireRegistryChecksums
      && !/^[0-9a-f]{64}$/.test(pkg.checksum ?? '')
    ) {
      throw new Error(`locked registry package ${pkg.name} lacks a SHA-256 checksum`);
    }
  }

  const classified = new Map();
  const seenMetadataKeys = new Set();
  const observedGitKeys = new Set();
  const observedExternalKeys = new Set();
  for (const pkg of metadata.packages ?? []) {
    if (!allowedLicenses.has(pkg.license)) {
      throw new Error(`unapproved or missing license expression for ${pkg.name}`);
    }
    const key = lockKey(pkg);
    if (!lockByKey.has(key)) throw new Error(`metadata package is absent from Cargo.lock: ${pkg.name}`);
    if (seenMetadataKeys.has(key)) throw new Error(`ambiguous metadata package ${pkg.name}`);
    seenMetadataKeys.add(key);

    let sourceKind;
    let sourceStatus;
    if (pkg.source !== null) {
      if (pkg.source.startsWith('git+')) {
        const gitKey = `${pkg.name}\u0000${pkg.version}`;
        const approved = gitPackages.get(gitKey);
        if (!allowedGit.has(pkg.source) || approved?.source.source !== pkg.source) {
          throw new Error(`unapproved Git source for ${pkg.name}`);
        }
        sourceKind = 'git';
        sourceStatus = 'approved-immutable-source';
        observedGitKeys.add(gitKey);
      } else {
        if (!allowedRegistries.has(pkg.source)) {
          throw new Error(`unapproved registry or unknown source for ${pkg.name}`);
        }
        sourceKind = 'registry';
        sourceStatus = 'locked-registry-checksum';
      }
    } else if (workspaceIds.has(pkg.id)) {
      const manifest = path.resolve(pkg.manifest_path);
      requireManifest(manifest, pkg.name);
      if (!inside(workspaceRoot, manifest) || !inside(canonicalWorkspace, fs.realpathSync(manifest))) {
        throw new Error(`workspace package escapes the workspace root: ${pkg.name}`);
      }
      sourceKind = 'workspace';
      sourceStatus = 'workspace-source';
    } else {
      const external = externalPackages.get(`${pkg.name}\u0000${pkg.version}`);
      const manifest = path.resolve(pkg.manifest_path);
      if (external !== undefined && manifest === external.expectedManifest) {
        requireManifest(manifest, pkg.name);
      }
      if (
        external === undefined
        || manifest !== external.expectedManifest
        || !inside(external.canonicalExternal, fs.realpathSync(manifest))
      ) {
        throw new Error(`unapproved or escaped external path dependency: ${pkg.name}`);
      }
      sourceKind = 'external-path';
      sourceStatus = external.revisionStatus === 'working-tree-only-blocker'
        ? 'release-blocker-working-tree-only'
        : 'release-blocker-unqualified-path';
      observedExternalKeys.add(`${pkg.name}\u0000${pkg.version}`);
    }
    classified.set(pkg.id, { pkg, locked: lockByKey.get(key), sourceKind, sourceStatus });
  }
  if (seenMetadataKeys.size !== lockByKey.size) {
    const missing = [...lockByKey].find(([key]) => !seenMetadataKeys.has(key));
    throw new Error(`Cargo.lock package is absent from metadata: ${missing?.[1].name ?? 'unknown'}`);
  }
  if (
    observedGitKeys.size !== gitPackages.size
    || [...gitPackages.keys()].some((key) => !observedGitKeys.has(key))
  ) {
    throw new Error('immutable Git policy and observed package closure differ');
  }
  if (
    observedExternalKeys.size !== externalPackages.size
    || [...externalPackages.keys()].some((key) => !observedExternalKeys.has(key))
  ) {
    throw new Error('external path policy and observed package closure differ');
  }

  const components = [...classified.values()].map(({ pkg, locked: lock, sourceKind, sourceStatus }) => {
    const normalizedLicense = policy.licenseNormalization?.[pkg.license] ?? pkg.license;
    const component = {
      type: 'library',
      'bom-ref': packageRef(pkg, sourceKind),
      name: pkg.name,
      version: pkg.version,
      licenses: [{ expression: normalizedLicense }],
      properties: [
        { name: 'vellum:source-kind', value: sourceKind },
        { name: 'vellum:source-status', value: sourceStatus },
      ],
    };
    if (normalizedLicense !== pkg.license) {
      component.properties.push({ name: 'vellum:declared-license', value: pkg.license });
    }
    if (sourceKind === 'registry') {
      component.purl = `pkg:cargo/${pkg.name}@${pkg.version}`;
      component.hashes = [{ alg: 'SHA-256', content: lock.checksum }];
    }
    return component;
  }).sort((left, right) => left['bom-ref'].localeCompare(right['bom-ref']));
  const referenceById = new Map(
    [...classified].map(([id, value]) => [id, packageRef(value.pkg, value.sourceKind)]),
  );
  const dependencies = (metadata.resolve?.nodes ?? []).map((node) => ({
    ref: referenceById.get(node.id),
    dependsOn: (node.dependencies ?? []).map((id) => referenceById.get(id)).filter(Boolean).sort(),
  })).filter((entry) => entry.ref !== undefined).sort((left, right) => left.ref.localeCompare(right.ref));
  dependencies.unshift({
    ref: 'urn:vellum:workspace-root',
    dependsOn: [...workspaceIds].map((id) => referenceById.get(id)).filter(Boolean).sort(),
  });
  const blockers = externalPolicies.map((external) => ({
    name: external.name,
    repository: external.repository,
    revision: external.revision,
    reason: 'external sibling path is not a self-contained source identity',
  })).sort((left, right) => left.name.localeCompare(right.name));
  for (const entry of externalPackages.values()) {
    if (entry.revisionStatus === 'working-tree-only-blocker') {
      blockers.push({
        name: entry.name,
        repository: entry.external.repository,
        revision: entry.external.revision,
        reason: 'package is absent from the pinned revision and exists only in the working tree',
      });
    }
  }
  blockers.sort((left, right) => left.name.localeCompare(right.name));
  const sbom = {
    bomFormat: 'CycloneDX',
    specVersion: policy.sbom.specVersion,
    version: 1,
    metadata: {
      component: {
        type: 'application',
        'bom-ref': 'urn:vellum:workspace-root',
        name: 'vellum-source-workspace',
        version: '0.1.0',
      },
      properties: [
        { name: 'vellum:artifact-kind', value: 'source-closure' },
        { name: 'vellum:provenance-claim', value: 'unsigned;unattested;not-a-binary-sbom' },
        { name: 'vellum:release-eligible', value: blockers.length === 0 ? 'true' : 'false' },
      ],
    },
    components,
    dependencies,
    properties: blockers.map((blocker) => ({
      name: `vellum:blocker:${blocker.name}`,
      value: `${blocker.repository}@${blocker.revision}: ${blocker.reason}`,
    })),
  };
  const serialized = `${JSON.stringify(sbom, null, 2)}\n`;
  for (const forbidden of [root, osHome(), 'file://']) {
    if (forbidden && serialized.includes(forbidden)) {
      throw new Error('generated SBOM contains a local filesystem identity');
    }
  }
  return {
    sbom,
    serialized,
    sbomSha256: sha256(serialized),
    lockSha256: sha256(lockText),
    componentCount: components.length,
    blockers,
  };
};

const osHome = () => process.env.HOME ?? '';
