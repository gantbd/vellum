import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { createInterface } from 'node:readline';
import { Client } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';

const PROTOCOL_VERSION = '2026-07-28';
const ORDINARY_TOOLS = [
  'vellum.snapshot',
  'vellum.save',
  'vellum.replace',
  'vellum.replace_and_save',
  'vellum.workspace_edit',
  'vellum.poll_changes',
  'vellum.resolve_locator',
  'vellum.membership',
];
const DURABLE_TOOLS = [
  'vellum.durable_execute',
  'vellum.durable_receipt',
  'vellum.durable_retire',
  'vellum.durable_status',
];
const daemonBinary = process.env.VELLUM_DAEMON_BIN;
const mcpBinary = process.env.VELLUM_MCP_BIN;
const timeoutMs = 30_000;

const requireAbsoluteBinary = (value, name) => {
  assert.equal(typeof value, 'string', `${name} is required`);
  assert.equal(path.isAbsolute(value), true, `${name} must be absolute`);
  const metadata = fs.lstatSync(value);
  assert.equal(metadata.isFile(), true, `${name} must be a regular file`);
  assert.equal(metadata.isSymbolicLink(), false, `${name} must not be a symbolic link`);
};
requireAbsoluteBinary(daemonBinary, 'VELLUM_DAEMON_BIN');
requireAbsoluteBinary(mcpBinary, 'VELLUM_MCP_BIN');

const withTimeout = async (promise, label) => {
  let timer;
  try {
    return await Promise.race([
      promise,
      new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`${label} timed out`)), timeoutMs); }),
    ]);
  } finally {
    clearTimeout(timer);
  }
};

const waitForReady = async child => {
  const lines = createInterface({ input: child.stdout });
  try {
    if (child.exitCode !== null || child.signalCode !== null) {
      throw new Error(`daemon exited before readiness: ${child.exitCode ?? child.signalCode}`);
    }
    const [line] = await withTimeout(Promise.race([
      once(lines, 'line'),
      once(child, 'exit').then(([code, signal]) => {
        throw new Error(`daemon exited before readiness: ${code ?? signal}`);
      }),
    ]), 'daemon readiness');
    assert.equal(line, 'vellum-daemon ready');
  } finally {
    lines.close();
  }
};

const startDaemon = async (directory, credential) => {
  const socket = path.join(directory, 'service.sock');
  const control = path.join(directory, 'control.sock');
  const source = path.join(directory, 'source.txt');
  fs.writeFileSync(source, 'alpha', { mode: 0o600 });
  const args = ['--socket', socket, '--control', control, '--allow-edit', '--allow-save'];
  if (credential !== undefined) args.push('--credential', credential);
  args.push('--', source);
  const child = spawn(daemonBinary, args, { stdio: ['ignore', 'pipe', 'pipe'] });
  let stderr = '';
  child.stderr.setEncoding('utf8');
  child.stderr.on('data', chunk => { stderr = `${stderr}${chunk}`.slice(-4096); });
  try {
    await waitForReady(child);
  } catch (error) {
    child.kill('SIGKILL');
    if (child.exitCode === null && child.signalCode === null) {
      await withTimeout(once(child, 'exit'), 'failed daemon reap');
    }
    throw new Error(`daemon startup failed: ${error.message}; ${stderr}`);
  }
  return { child, socket, control, source, stderr: () => stderr };
};

const stopDaemon = async daemon => {
  const stopped = spawnSync(daemonBinary, ['--shutdown', daemon.control], { encoding: 'utf8', timeout: timeoutMs });
  assert.equal(stopped.status, 0, `daemon shutdown failed: ${stopped.stderr ?? ''}`);
  const [code, signal] = daemon.child.exitCode === null && daemon.child.signalCode === null
    ? await withTimeout(once(daemon.child, 'exit'), 'daemon exit')
    : [daemon.child.exitCode, daemon.child.signalCode];
  assert.equal(signal, null, 'daemon was signalled during qualified shutdown');
  assert.equal(code, 0, `daemon exited unsuccessfully: ${daemon.stderr()}`);
};

const qualifyClient = async ({ socket, credential, negotiation, durable, locator }) => {
  const args = ['--socket', socket];
  if (credential !== undefined) args.push('--credential', credential);
  const transport = new StdioClientTransport({
    command: mcpBinary,
    args,
    stderr: 'pipe',
    env: { PATH: process.env.PATH ?? '/usr/bin:/bin' },
    maxBufferSize: 1_048_576,
  });
  let stderr = '';
  transport.stderr?.setEncoding?.('utf8');
  transport.stderr?.on?.('data', chunk => { stderr = `${stderr}${chunk}`.slice(-4096); });
  const client = new Client(
    { name: 'vellum-official-client-qualification', version: '0.1.0' },
    { capabilities: {}, versionNegotiation: negotiation, enforceStrictCapabilities: true },
  );
  try {
    await withTimeout(client.connect(transport), 'official client connect');
    assert.equal(client.getProtocolEra(), 'modern', 'official client fell back to initialize');
    assert.equal(client.getNegotiatedProtocolVersion(), PROTOCOL_VERSION);
    const discover = client.getDiscoverResult();
    assert.ok(discover, 'modern connect omitted server/discover evidence');
    assert.deepEqual(discover.supportedVersions, [PROTOCOL_VERSION]);

    const listed = await withTimeout(client.listTools(), 'tools/list');
    const names = listed.tools.map(tool => tool.name);
    assert.deepEqual(names, durable ? [...ORDINARY_TOOLS, ...DURABLE_TOOLS] : ORDINARY_TOOLS);

    const resolution = await withTimeout(client.callTool({
      name: 'vellum.resolve_locator',
      arguments: { locator },
    }), 'locator resolution');
    assert.equal(resolution.isError, false);
    assert.equal(resolution.structuredContent?.kind, 'locator_resolved');
    assert.equal(typeof resolution.structuredContent?.document, 'number');
    assert.equal(resolution.structuredContent?.root?.kind, 'file');
    assert.equal(JSON.stringify(resolution).includes(locator), false, 'locator leaked into resolution');

    const resources = await withTimeout(client.listResources(), 'resources/list');
    const document = resources.resources.find(resource => resource.uri.includes('/document/'));
    assert.ok(document, 'workspace did not expose its startup document');
    const match = /^vellum:\/\/workspace\/(\d+)\/document\/(\d+)$/.exec(document.uri);
    assert.ok(match, 'official client received a noncanonical document URI');
    const content = await withTimeout(client.readResource({ uri: document.uri }), 'resources/read');
    assert.equal(content.contents[0]?.text, 'alpha');

    const snapshot = await withTimeout(client.callTool({
      name: 'vellum.snapshot',
      arguments: { workspace: Number(match[1]), document: Number(match[2]) },
    }), 'tools/call');
    assert.equal(snapshot.isError, false);
    assert.equal(snapshot.structuredContent?.kind, 'snapshot');
    assert.equal(snapshot.structuredContent?.text, 'alpha');
    const edit = await withTimeout(client.callTool({
      name: 'vellum.workspace_edit',
      arguments: {
        documents: [{
          workspace: Number(match[1]),
          document: Number(match[2]),
          expected_revision: snapshot.structuredContent?.revision,
          replacements: [{ start: 0, end: 5, inserted: 'bravo' }],
        }],
      },
    }), 'workspace edit');
    assert.equal(edit.isError, false);
    assert.equal(edit.structuredContent?.kind, 'workspace_edited');
    assert.equal(edit.structuredContent?.persistence, 'not_attempted');
    assert.equal(edit.structuredContent?.documents?.[0]?.text, 'bravo');

    const save = await withTimeout(client.callTool({
      name: 'vellum.save',
      arguments: {
        workspace: Number(match[1]),
        document: Number(match[2]),
        expected_revision: edit.structuredContent?.documents?.[0]?.revision,
      },
    }), 'document save');
    assert.equal(save.isError, false);
    assert.equal(save.structuredContent?.kind, 'saved');
    assert.equal(save.structuredContent?.persistence?.status, 'durable');
    assert.equal(fs.readFileSync(locator, 'utf8'), 'bravo');

    const edited = await withTimeout(client.readResource({ uri: document.uri }), 'edited resources/read');
    assert.equal(edited.contents[0]?.text, 'bravo');
    return {
      toolCount: names.length,
      resourceCount: resources.resources.length,
      workspaceEdit: 'observed',
      save: 'durable',
    };
  } finally {
    await withTimeout(client.close(), 'official client close');
    assert.equal(transport.pid, null, `MCP child did not close cleanly: ${stderr}`);
  }
};

// Unix socket paths are bounded by the daemon profile. Keep the private
// qualification root short even when the platform's temporary root is long.
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'v-'));
fs.chmodSync(root, 0o700);
let ordinaryDaemon;
let authenticatedDaemon;
try {
  const ordinary = path.join(root, 'ordinary');
  fs.mkdirSync(ordinary, { mode: 0o700 });
  ordinaryDaemon = await startDaemon(ordinary);
  const ordinaryReceipt = await qualifyClient({
    socket: ordinaryDaemon.socket,
    locator: ordinaryDaemon.source,
    negotiation: { mode: 'auto', probe: { timeoutMs: 2_000, maxRetries: 0 } },
    durable: false,
  });
  await stopDaemon(ordinaryDaemon);
  ordinaryDaemon = undefined;

  const authenticated = path.join(root, 'authenticated');
  fs.mkdirSync(authenticated, { mode: 0o700 });
  const credential = path.join(authenticated, 'credential');
  const provisioned = spawnSync(daemonBinary, ['--provision-credential', credential], { encoding: 'utf8', timeout: timeoutMs });
  assert.equal(provisioned.status, 0, 'credential provisioning failed');
  authenticatedDaemon = await startDaemon(authenticated, credential);
  const authenticatedReceipt = await qualifyClient({
    socket: authenticatedDaemon.socket,
    locator: authenticatedDaemon.source,
    credential,
    negotiation: { mode: { pin: PROTOCOL_VERSION }, probe: { timeoutMs: 2_000, maxRetries: 0 } },
    durable: true,
  });
  await stopDaemon(authenticatedDaemon);
  authenticatedDaemon = undefined;

  process.stdout.write(`${JSON.stringify({
    schema: 1,
    officialClient: '@modelcontextprotocol/client@2.4.0',
    protocolVersion: PROTOCOL_VERSION,
    protocolEra: 'modern',
    autoNegotiation: 'modern-no-fallback',
    pinnedNegotiation: 'modern-no-fallback',
    ordinary: ordinaryReceipt,
    authenticated: authenticatedReceipt,
    cleanEofDisconnect: true,
  })}\n`);
} finally {
  for (const daemon of [ordinaryDaemon, authenticatedDaemon]) {
    if (daemon?.child.exitCode === null) daemon.child.kill('SIGKILL');
  }
  fs.rmSync(root, { recursive: true, force: true });
}
