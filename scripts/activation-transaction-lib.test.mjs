import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  SCRATCH_FAILPOINTS,
  activationTransition,
  canonicalActivationJson,
  inspectScratchActivation,
  runScratchActivation,
} from './activation-transaction-lib.mjs';

const sha256 = (file) => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const mode = (file) => fs.statSync(file).mode & 0o777;

const fixture = () => {
  const parent = fs.realpathSync.native(fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-activation-')));
  fs.chmodSync(parent, 0o700);
  const root = path.join(parent, 'installation');
  fs.mkdirSync(root, { mode: 0o700 });
  fs.chmodSync(root, 0o700);
  const payload = path.join(parent, 'payload');
  fs.mkdirSync(payload, { mode: 0o700 });
  const version = (role, marker) => {
    const directory = path.join(payload, role);
    fs.mkdirSync(directory, { mode: 0o700 });
    const files = {};
    for (const [name, fileMode] of [['vellum-daemon', 0o500], ['vellum-mcp', 0o500], ['vellum.plist', 0o400]]) {
      const file = path.join(directory, name);
      fs.writeFileSync(file, `${role}:${name}:${marker}\n`, { mode: fileMode });
      fs.chmodSync(file, fileMode);
      files[name] = { path: file, sha256: sha256(file) };
    }
    const manifestSha256 = crypto.createHash('sha256').update(`${role}:${marker}`).digest('hex');
    return {
      manifestSha256,
      daemon: files['vellum-daemon'],
      mcp: files['vellum-mcp'],
      plist: files['vellum.plist'],
    };
  };
  const candidate = version('candidate', 'v2');
  const prior = version('prior', 'v1');
  const approval = path.join(parent, 'approval.json');
  fs.writeFileSync(approval, canonicalActivationJson({
    schema: 1,
    channel: 'local-activation-approval',
    transactionId: 'a'.repeat(32),
    requestedAction: 'select-candidate-for-external-activation',
    candidateManifestSha256: candidate.manifestSha256,
    priorManifestSha256: prior.manifestSha256,
    activationAuthorized: true,
  }), { mode: 0o600 });
  fs.chmodSync(approval, 0o600);
  return { parent, root, approval, candidate, prior };
};

const clean = ({ parent }) => {
  const makeRemovable = (directory) => {
    if (!fs.existsSync(directory)) return;
    const stat = fs.lstatSync(directory);
    if (!stat.isDirectory() || stat.isSymbolicLink()) return;
    fs.chmodSync(directory, 0o700);
    for (const entry of fs.readdirSync(directory)) makeRemovable(path.join(directory, entry));
  };
  makeRemovable(parent);
  fs.rmSync(parent, { recursive: true, force: true });
};

{
  let state = activationTransition('prepared', 'begin-candidate');
  assert.deepEqual(state, { phase: 'candidate-effect-started', outcome: null });
  assert.deepEqual(
    activationTransition(state.phase, 'candidate-confirmed'),
    { phase: 'terminal', outcome: 'candidate-active' },
  );
  state = activationTransition(state.phase, 'candidate-refused');
  assert.deepEqual(state, { phase: 'rollback-pending', outcome: null });
  state = activationTransition(state.phase, 'begin-rollback');
  assert.deepEqual(state, { phase: 'rollback-effect-started', outcome: null });
  assert.deepEqual(
    activationTransition(state.phase, 'rollback-confirmed'),
    { phase: 'terminal', outcome: 'prior-restored' },
  );
  assert.deepEqual(
    activationTransition('candidate-effect-started', 'candidate-unknown'),
    { phase: 'terminal', outcome: 'outcome-unknown' },
  );
  assert.deepEqual(
    activationTransition('rollback-effect-started', 'rollback-refused'),
    { phase: 'terminal', outcome: 'rollback-refused' },
  );
  assert.throws(() => activationTransition('terminal', 'begin-candidate'), /closed transition refused/);
  assert.throws(() => activationTransition('prepared', 'candidate-confirmed'), /closed transition refused/);
}

{
  const f = fixture();
  try {
    const receipt = runScratchActivation({
      root: f.root,
      approvalFile: f.approval,
      candidate: f.candidate,
      prior: f.prior,
      candidateDisposition: 'confirmed',
    });
    assert.equal(receipt.outcome, 'candidate-selected');
    assert.equal(receipt.launchdEffect, 'not-attempted');
    assert.equal(receipt.atomicity, 'one-desired-state-pointer-only');
    assert.equal(JSON.stringify(receipt).includes(f.parent), false);
    const inspected = inspectScratchActivation(f.root);
    assert.equal(inspected.outcome, 'candidate-selected');
    assert.equal(inspected.selectedManifestSha256, f.candidate.manifestSha256);
    assert.equal(inspected.pointerGeneration, 1);
    const versionRoot = path.join(f.root, 'versions', f.candidate.manifestSha256);
    assert.equal(mode(versionRoot), 0o500);
    assert.equal(mode(path.join(versionRoot, 'vellum-daemon')), 0o500);
    assert.equal(mode(path.join(versionRoot, 'vellum-mcp')), 0o500);
    assert.equal(mode(path.join(versionRoot, 'vellum.plist')), 0o400);
    assert.equal(sha256(path.join(versionRoot, 'vellum-daemon')), f.candidate.daemon.sha256);
  } finally {
    clean(f);
  }
}

{
  const f = fixture();
  try {
    runScratchActivation({
      root: f.root,
      approvalFile: f.approval,
      candidate: f.candidate,
      prior: f.prior,
    });
    const pointerFile = path.join(f.root, 'desired-deployment.json');
    const pointer = JSON.parse(fs.readFileSync(pointerFile, 'utf8'));
    pointer.daemonSha256 = '0'.repeat(64);
    fs.writeFileSync(pointerFile, canonicalActivationJson(pointer), { mode: 0o600 });
    assert.throws(() => inspectScratchActivation(f.root), /immutable version identity differs/);
  } finally {
    clean(f);
  }
}

{
  const f = fixture();
  try {
    const receipt = runScratchActivation({
      root: f.root,
      approvalFile: f.approval,
      candidate: f.candidate,
      prior: f.prior,
      candidateDisposition: 'refused',
      rollbackDisposition: 'confirmed',
    });
    assert.equal(receipt.outcome, 'prior-restored');
    assert.equal(receipt.selectedManifestSha256, f.prior.manifestSha256);
    assert.equal(receipt.pointerGeneration, 2);
    assert.equal(inspectScratchActivation(f.root).outcome, 'prior-restored');
  } finally {
    clean(f);
  }
}

{
  const f = fixture();
  try {
    const receipt = runScratchActivation({
      root: f.root,
      approvalFile: f.approval,
      candidate: f.candidate,
      prior: f.prior,
      candidateDisposition: 'unknown',
    });
    assert.equal(receipt.outcome, 'outcome-unknown');
    assert.equal(receipt.selectedManifestSha256, f.candidate.manifestSha256);
    assert.equal(receipt.launchdEffect, 'not-attempted');
  } finally {
    clean(f);
  }
}

for (const [failpoint, expected] of [
  ['after-prepared', 'prepared'],
  ['after-candidate-pointer', 'outcome-unknown'],
  ['after-rollback-pending', 'rollback-pending'],
  ['after-prior-pointer', 'outcome-unknown'],
]) {
  const f = fixture();
  try {
    assert.throws(
      () => runScratchActivation({
        root: f.root,
        approvalFile: f.approval,
        candidate: f.candidate,
        prior: f.prior,
        candidateDisposition: 'refused',
        rollbackDisposition: 'confirmed',
        failpoint,
      }),
      new RegExp(`simulated crash at ${failpoint}`),
    );
    const inspected = inspectScratchActivation(f.root);
    assert.equal(inspected.outcome, expected);
    assert.equal(JSON.stringify(inspected).includes(f.parent), false);
  } finally {
    clean(f);
  }
}

for (const [failpoint, candidateDisposition, expected] of [
  ['after-candidate-terminal', 'confirmed', 'candidate-selected'],
  ['after-rollback-terminal', 'refused', 'prior-restored'],
]) {
  const f = fixture();
  try {
    assert.throws(
      () => runScratchActivation({
        root: f.root,
        approvalFile: f.approval,
        candidate: f.candidate,
        prior: f.prior,
        candidateDisposition,
        rollbackDisposition: 'confirmed',
        failpoint,
      }),
      new RegExp(`simulated crash at ${failpoint}`),
    );
    assert.equal(inspectScratchActivation(f.root).outcome, expected);
  } finally {
    clean(f);
  }
}

assert.deepEqual(SCRATCH_FAILPOINTS, [
  'none',
  'after-prepared',
  'after-candidate-pointer',
  'after-candidate-terminal',
  'after-rollback-pending',
  'after-prior-pointer',
  'after-rollback-terminal',
]);

{
  const f = fixture();
  try {
    const approval = JSON.parse(fs.readFileSync(f.approval, 'utf8'));
    approval.activationAuthorized = false;
    fs.writeFileSync(f.approval, canonicalActivationJson(approval), { mode: 0o600 });
    assert.throws(
      () => runScratchActivation({ root: f.root, approvalFile: f.approval, candidate: f.candidate, prior: f.prior }),
      /activation approval refused/,
    );
    assert.deepEqual(fs.readdirSync(f.root), []);
  } finally {
    clean(f);
  }
}

{
  const f = fixture();
  try {
    f.candidate.daemon.sha256 = '0'.repeat(64);
    assert.throws(
      () => runScratchActivation({ root: f.root, approvalFile: f.approval, candidate: f.candidate, prior: f.prior }),
      /payload digest mismatch/,
    );
    assert.deepEqual(fs.readdirSync(f.root), []);
  } finally {
    clean(f);
  }
}

process.stdout.write('Vellum activation transaction algebra and scratch mutants passed\n');
