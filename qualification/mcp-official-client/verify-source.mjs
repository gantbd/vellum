import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.dirname(fileURLToPath(import.meta.url));
const fail = message => { throw new Error(`Vellum official MCP client source closure: ${message}`); };
const readJson = name => JSON.parse(fs.readFileSync(path.join(root, name), 'utf8'));
const canonical = value => `${JSON.stringify(value, null, 2)}\n`;
const policy = readJson('source-policy.json');
const manifest = readJson('package.json');
const lockPath = path.join(root, 'package-lock.json');

if (
  policy.schema !== 1
  || policy.protocolVersion !== '2026-07-28'
  || policy.source?.repository !== 'https://github.com/modelcontextprotocol/typescript-sdk'
  || policy.source?.releaseTag !== 'v2.4.0'
  || policy.source?.releaseCommitPrefix !== '951fe56'
  || policy.source?.releaseCommitVerifiedByGitHub !== true
) fail('source revision policy drifted');
if (
  manifest.private !== true
  || manifest.dependencies?.['@modelcontextprotocol/client'] !== '2.4.0'
  || Object.keys(manifest.dependencies ?? {}).length !== 1
) fail('package manifest must retain one exact official-client dependency');
if (!fs.existsSync(lockPath)) {
  fail('package-lock.json is absent; run the documented package-lock-only acquisition and review it before qualification');
}

const lock = readJson('package-lock.json');
if (lock.lockfileVersion !== 3 || typeof lock.packages !== 'object' || lock.packages === null) {
  fail('only npm package-lock schema 3 is accepted');
}
if (
  lock.packages['']?.dependencies?.['@modelcontextprotocol/client'] !== '2.4.0'
  || Object.keys(lock.packages['']?.dependencies ?? {}).length !== 1
) fail('lock root dependency does not exactly match package.json');

for (const [name, product] of Object.entries(policy.packages)) {
  const locked = lock.packages[`node_modules/${name}`];
  if (
    locked?.version !== product.version
    || locked?.resolved !== product.resolved
    || locked?.integrity !== product.integrity
    || locked?.license !== product.license
  ) fail(`${name} identity, integrity, source, or license differs from reviewed policy`);
}

const components = [];
for (const [location, product] of Object.entries(lock.packages)) {
  if (location === '') continue;
  if (!location.startsWith('node_modules/') || product.link === true) fail(`non-registry or linked package entry ${location}`);
  if (
    typeof product.version !== 'string'
    || typeof product.resolved !== 'string'
    || !product.resolved.startsWith(policy.lockPolicy.registryPrefix)
    || typeof product.integrity !== 'string'
    || !product.integrity.startsWith('sha512-')
    || typeof product.license !== 'string'
    || product.license.length === 0
  ) fail(`incomplete bounded source evidence for ${location}`);
  if (product.hasInstallScript === true) fail(`lifecycle script is forbidden for ${location}`);
  const name = location.slice('node_modules/'.length);
  const digest = Buffer.from(product.integrity.slice('sha512-'.length), 'base64').toString('hex');
  components.push({
    type: 'library', name, version: product.version,
    licenses: [{ license: { id: product.license } }],
    hashes: [{ alg: 'SHA-512', content: digest }],
    purl: `pkg:npm/${name.replace('@', '%40')}@${product.version}`,
    externalReferences: [{ type: 'distribution', url: product.resolved }],
  });
}
components.sort((left, right) => `${left.name}@${left.version}`.localeCompare(`${right.name}@${right.version}`));
const sbom = {
  bomFormat: 'CycloneDX', specVersion: '1.5', version: 1,
  metadata: {
    component: { type: 'application', name: manifest.name, version: manifest.version },
    properties: [
      { name: 'vellum:qualification-only', value: 'true' },
      { name: 'vellum:mcp-protocol', value: policy.protocolVersion },
      { name: 'vellum:source-release', value: `${policy.source.repository}@${policy.source.releaseTag}` },
    ],
  },
  components,
};

const arguments_ = process.argv.slice(2);
if (arguments_.some(argument => argument !== '--emit-sbom=-')) fail(`unknown argument ${arguments_.join(' ')}`);
if (arguments_.includes('--emit-sbom=-')) {
  process.stdout.write(canonical(sbom));
  process.exit(0);
}
const sbomPath = path.join(root, policy.sbom.file);
if (!fs.existsSync(sbomPath)) fail('reviewed source-sbom.json is absent; emit and review the candidate after locking');
if (canonical(readJson(policy.sbom.file)) !== canonical(sbom)) {
  fail('source-sbom.json differs from the exact locked dependency closure');
}
process.stdout.write(`Official MCP client source closure verified: ${components.length} locked components\n`);
