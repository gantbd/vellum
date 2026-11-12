import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { evaluateSupplyChain } from './supply-chain-lib.mjs';

const root = process.cwd();
const cargo = spawnSync(
  'cargo',
  ['metadata', '--offline', '--locked', '--format-version', '1'],
  { cwd: root, encoding: 'utf8' },
);
if (cargo.status !== 0) throw new Error('locked offline Cargo metadata fixture failed');
const metadata = JSON.parse(cargo.stdout);
const lockText = fs.readFileSync(path.join(root, 'Cargo.lock'), 'utf8');
const policy = JSON.parse(fs.readFileSync(path.join(root, 'release/dependency-policy.json'), 'utf8'));
const clone = (value) => structuredClone(value);
const replacePackageField = (text, name, field, value) => {
  const pattern = new RegExp(
    `(\\[\\[package\\]\\]\\nname = "${name}"[\\s\\S]*?\\n${field} = )"[^"]+"`,
  );
  const replaced = text.replace(pattern, `$1"${value}"`);
  if (replaced === text) throw new Error(`test could not alter ${name}.${field}`);
  return replaced;
};
const removePackageField = (text, name, field) => {
  const pattern = new RegExp(
    `(\\[\\[package\\]\\]\\nname = "${name}"[\\s\\S]*?)\\n${field} = "[^"]+"`,
  );
  const replaced = text.replace(pattern, '$1');
  if (replaced === text) throw new Error(`test could not remove ${name}.${field}`);
  return replaced;
};
const expectFailure = (label, fixture, expected) => {
  try {
    evaluateSupplyChain({ root, ...fixture });
  } catch (error) {
    if (error.message.includes(expected)) return;
    throw new Error(`${label} failed for the wrong reason: ${error.message}`);
  }
  throw new Error(`${label} was accepted`);
};

const first = evaluateSupplyChain({ metadata, lockText, policy, root });
const second = evaluateSupplyChain({ metadata, lockText, policy, root });
if (first.serialized !== second.serialized || first.sbomSha256 !== second.sbomSha256) {
  throw new Error('source SBOM is not deterministic');
}
if (
  first.serialized.includes(root)
  || (process.env.HOME !== undefined && first.serialized.includes(process.env.HOME))
) {
  throw new Error('source SBOM contains a local filesystem identity');
}
if (first.blockers.length !== 0) {
  throw new Error('qualified immutable Keystone inputs were reported as blockers');
}
{
  const emitted = spawnSync(
    'node',
    ['scripts/check-supply-chain.mjs', '--emit=-'],
    { cwd: root, encoding: 'utf8', maxBuffer: 1024 * 1024 },
  );
  if (emitted.status !== 0 || emitted.stdout !== first.serialized) {
    throw new Error('stdout SBOM emission was truncated or changed');
  }
  JSON.parse(emitted.stdout);
}

{
  const altered = clone(metadata);
  altered.packages.find((pkg) => pkg.name === 'bitflags').source =
    'git+https://example.invalid/bitflags#1111111111111111111111111111111111111111';
  expectFailure(
    'unapproved Git source',
    {
      metadata: altered,
      policy,
      lockText: replacePackageField(
        lockText,
        'bitflags',
        'source',
        'git+https://example.invalid/bitflags#1111111111111111111111111111111111111111',
      ),
    },
    'unapproved Git source',
  );
}
{
  const altered = clone(metadata);
  altered.packages.find((pkg) => pkg.name === 'bitflags').source =
    'registry+https://example.invalid/index';
  expectFailure(
    'unknown registry',
    {
      metadata: altered,
      policy,
      lockText: replacePackageField(
        lockText,
        'bitflags',
        'source',
        'registry+https://example.invalid/index',
      ),
    },
    'unapproved registry or unknown source',
  );
}
{
  const altered = clone(metadata);
  altered.packages.find((pkg) => pkg.name === 'bitflags').license = null;
  expectFailure(
    'missing license',
    { metadata: altered, policy, lockText },
    'unapproved or missing license expression',
  );
}
{
  const altered = clone(metadata);
  altered.packages.find((pkg) => pkg.name === 'vellum-engine').manifest_path =
    '/etc/hosts';
  expectFailure(
    'escaped path dependency',
    { metadata: altered, policy, lockText },
    'workspace package escapes the workspace root',
  );
}
expectFailure(
  'missing registry checksum',
  {
    metadata,
    policy,
    lockText: removePackageField(lockText, 'bitflags', 'checksum'),
  },
  'lacks a SHA-256 checksum',
);

process.stdout.write('Vellum supply-chain adversarial fixtures passed\n');
