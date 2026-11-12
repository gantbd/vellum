import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const source = path.dirname(fileURLToPath(import.meta.url));
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-mcp-source-profile-'));
const fixture = path.join(root, 'fixture');
const verifier = path.join(fixture, 'verify-source.mjs');

const canonical = value => `${JSON.stringify(value, null, 2)}\n`;
const policy = JSON.parse(fs.readFileSync(path.join(source, 'source-policy.json'), 'utf8'));
const manifest = JSON.parse(fs.readFileSync(path.join(source, 'package.json'), 'utf8'));
const baseLock = {
  name: manifest.name,
  version: manifest.version,
  lockfileVersion: 3,
  requires: true,
  packages: {
    '': {
      name: manifest.name,
      version: manifest.version,
      license: manifest.license,
      dependencies: { '@modelcontextprotocol/client': '2.4.0' },
    },
    'node_modules/@modelcontextprotocol/client': {
      ...policy.packages['@modelcontextprotocol/client'],
    },
    'node_modules/@modelcontextprotocol/core': {
      ...policy.packages['@modelcontextprotocol/core'],
    },
  },
};

const run = (...args) => spawnSync(process.execPath, [verifier, ...args], {
  cwd: fixture,
  encoding: 'utf8',
});

const reset = () => {
  fs.rmSync(fixture, { recursive: true, force: true });
  fs.mkdirSync(fixture, { mode: 0o700 });
  for (const name of ['verify-source.mjs', 'source-policy.json', 'package.json']) {
    fs.copyFileSync(path.join(source, name), path.join(fixture, name));
  }
  fs.writeFileSync(path.join(fixture, 'package-lock.json'), canonical(baseLock));
  const emitted = run('--emit-sbom=-');
  assert.equal(emitted.status, 0, emitted.stderr);
  fs.writeFileSync(path.join(fixture, 'source-sbom.json'), emitted.stdout);
  const accepted = run();
  assert.equal(accepted.status, 0, accepted.stderr);
};

const mutateLock = mutation => {
  const lockPath = path.join(fixture, 'package-lock.json');
  const lock = JSON.parse(fs.readFileSync(lockPath, 'utf8'));
  mutation(lock);
  fs.writeFileSync(lockPath, canonical(lock));
};

const rejected = (label, mutation) => {
  reset();
  mutation();
  const result = run();
  assert.notEqual(result.status, 0, `${label} was accepted`);
  assert.match(result.stderr, /Vellum official MCP client source closure:/, label);
};

try {
  reset();
  rejected('lifecycle script', () => mutateLock(lock => {
    lock.packages['node_modules/@modelcontextprotocol/core'].hasInstallScript = true;
  }));
  rejected('non-registry source', () => mutateLock(lock => {
    lock.packages['node_modules/@modelcontextprotocol/core'].resolved = 'git+https://example.invalid/core.git';
  }));
  rejected('weak integrity', () => mutateLock(lock => {
    lock.packages['node_modules/@modelcontextprotocol/core'].integrity = 'sha1-deadbeef';
  }));
  rejected('linked package', () => mutateLock(lock => {
    lock.packages['node_modules/@modelcontextprotocol/core'].link = true;
  }));
  rejected('official package substitution', () => mutateLock(lock => {
    lock.packages['node_modules/@modelcontextprotocol/client'].version = '2.4.1';
  }));
  rejected('unreviewed transitive component', () => mutateLock(lock => {
    lock.packages['node_modules/unreviewed'] = { version: '1.0.0', license: 'MIT' };
  }));
  rejected('missing reviewed lock', () => {
    fs.unlinkSync(path.join(fixture, 'package-lock.json'));
  });
  rejected('missing reviewed SBOM', () => {
    fs.unlinkSync(path.join(fixture, 'source-sbom.json'));
  });
  rejected('SBOM drift', () => {
    const sbomPath = path.join(fixture, 'source-sbom.json');
    const sbom = JSON.parse(fs.readFileSync(sbomPath, 'utf8'));
    sbom.components.pop();
    fs.writeFileSync(sbomPath, canonical(sbom));
  });
  process.stdout.write('Official MCP client source-profile mutants: 9/9 rejected\n');
} finally {
  fs.rmSync(root, { recursive: true, force: true });
}
