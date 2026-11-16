import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const contract = JSON.parse(readFileSync(join(root, 'release/mcp-peers.json'), 'utf8'));
const fail = (message) => { throw new Error(`MCP peer contract: ${message}`); };
const fullRevision = (value) => typeof value === 'string' && /^[0-9a-f]{40}$/.test(value);

if (contract.schema !== 1 || contract.protocolVersion !== '2026-07-28') fail('unsupported schema or protocol');
if (contract.publication !== 'none') fail('publication must remain none');
if (!Array.isArray(contract.peers) || contract.peers.length !== 2) fail('exactly two peers are required');
for (const name of ['vellum-mcp', 'vellum-semantic-mcp']) {
  const peer = contract.peers.find((candidate) => candidate.package === name);
  if (!peer || peer.sourceOwner !== name || !fullRevision(peer.revision)) fail(`${name} identity is incomplete`);
  if (existsSync(join(root, 'connectors', name))) fail(`${name} production source remains in Vellum`);
  if (!Array.isArray(peer.retainedVellumQualification) || peer.retainedVellumQualification.length === 0) {
    fail(`${name} lost Vellum-owned coordinated qualification`);
  }
}
for (const relative of [
  'qualification/mcp-live-daemon',
  'qualification/mcp-official-client',
  'applications/vellum-semantic-local',
  'qualification/semantic-lsp-provider',
]) {
  if (!existsSync(join(root, relative))) fail(`missing retained qualification ${relative}`);
}
if (existsSync(join(root, 'distribution/cargo-install/src/bin/vellum-mcp.rs'))) {
  fail('Vellum source-install package still owns an MCP binary');
}

console.log('MCP peer ownership contract passed');
