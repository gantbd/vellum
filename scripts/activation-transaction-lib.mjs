import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const JOURNAL_FILE = 'activation-journal.json';
const POINTER_FILE = 'desired-deployment.json';
const VERSIONS_DIRECTORY = 'versions';
const DIGEST = /^[0-9a-f]{64}$/;
const TRANSACTION = /^[0-9a-f]{32}$/;

export const SCRATCH_FAILPOINTS = Object.freeze([
  'none',
  'after-prepared',
  'after-candidate-pointer',
  'after-candidate-terminal',
  'after-rollback-pending',
  'after-prior-pointer',
  'after-rollback-terminal',
]);

export const canonicalActivationJson = (value) => `${JSON.stringify(value, null, 2)}\n`;

const fail = (message) => {
  throw new Error(`activation transaction: ${message}`);
};

const exactKeys = (value, keys, label) => {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    fail(`${label} is not an object`);
  }
  if (Object.keys(value).sort().join(',') !== [...keys].sort().join(',')) {
    fail(`${label} fields are incompatible`);
  }
};

const sha256File = (file) => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const permissions = (file) => fs.statSync(file).mode & 0o777;

const validatePrivateDirectory = (directory, label, expectedMode) => {
  const absolute = path.resolve(directory);
  const stat = fs.lstatSync(absolute);
  if (stat.isSymbolicLink() || !stat.isDirectory()) fail(`${label} is not a real directory`);
  if (fs.realpathSync.native(absolute) !== absolute) fail(`${label} is not canonical`);
  if (stat.uid !== process.geteuid() || permissions(absolute) !== expectedMode) {
    fail(`${label} owner or mode is incompatible`);
  }
  return absolute;
};

const validateRegularInput = (file, label, expectedDigest, executable) => {
  if (!DIGEST.test(expectedDigest)) fail(`${label} digest is invalid`);
  const absolute = path.resolve(file);
  const stat = fs.lstatSync(absolute);
  if (stat.isSymbolicLink() || !stat.isFile()) fail(`${label} is not a regular file`);
  if (fs.realpathSync.native(absolute) !== absolute) fail(`${label} is not canonical`);
  if (stat.uid !== process.geteuid() || (permissions(absolute) & 0o022) !== 0) {
    fail(`${label} owner or mode is incompatible`);
  }
  if (executable && (permissions(absolute) & 0o100) === 0) fail(`${label} is not owner executable`);
  if (sha256File(absolute) !== expectedDigest) fail(`${label} payload digest mismatch`);
  return absolute;
};

const validateVersion = (version, label) => {
  exactKeys(version, ['manifestSha256', 'daemon', 'plist'], label);
  if (!DIGEST.test(version.manifestSha256)) fail(`${label} manifest digest is invalid`);
  const result = { manifestSha256: version.manifestSha256 };
  for (const [name, executable] of [['daemon', true], ['plist', false]]) {
    exactKeys(version[name], ['path', 'sha256'], `${label}.${name}`);
    result[name] = {
      path: validateRegularInput(
        version[name].path,
        `${label}.${name}`,
        version[name].sha256,
        executable,
      ),
      sha256: version[name].sha256,
    };
  }
  return result;
};

const readCanonicalJson = (file, label, expectedMode) => {
  const absolute = path.resolve(file);
  const stat = fs.lstatSync(absolute);
  if (stat.isSymbolicLink() || !stat.isFile()) fail(`${label} is not a regular file`);
  if (fs.realpathSync.native(absolute) !== absolute) fail(`${label} is not canonical`);
  if (stat.uid !== process.geteuid() || permissions(absolute) !== expectedMode) {
    fail(`${label} owner or mode is incompatible`);
  }
  const text = fs.readFileSync(absolute, 'utf8');
  let value;
  try {
    value = JSON.parse(text);
  } catch (error) {
    fail(`${label} JSON is invalid: ${error.message}`);
  }
  if (text !== canonicalActivationJson(value)) fail(`${label} JSON is not canonical`);
  return value;
};

const validateApproval = (file, candidate, prior) => {
  const approval = readCanonicalJson(file, 'activation approval', 0o600);
  exactKeys(
    approval,
    [
      'schema',
      'channel',
      'transactionId',
      'requestedAction',
      'candidateManifestSha256',
      'priorManifestSha256',
      'activationAuthorized',
    ],
    'activation approval',
  );
  if (
    approval.schema !== 1
    || approval.channel !== 'local-activation-approval'
    || !TRANSACTION.test(approval.transactionId)
    || approval.requestedAction !== 'select-candidate-for-external-activation'
    || approval.activationAuthorized !== true
    || approval.candidateManifestSha256 !== candidate.manifestSha256
    || approval.priorManifestSha256 !== prior.manifestSha256
    || approval.candidateManifestSha256 === approval.priorManifestSha256
  ) fail('activation approval refused');
  return approval;
};

const fsyncDirectory = (directory) => {
  const fd = fs.openSync(directory, fs.constants.O_RDONLY);
  try {
    fs.fsyncSync(fd);
  } finally {
    fs.closeSync(fd);
  }
};

const writeDurableFile = (file, bytes, mode) => {
  const fd = fs.openSync(file, fs.constants.O_WRONLY | fs.constants.O_CREAT | fs.constants.O_EXCL, mode);
  try {
    fs.writeFileSync(fd, bytes);
    fs.fsyncSync(fd);
  } finally {
    fs.closeSync(fd);
  }
  fs.chmodSync(file, mode);
};

const replaceDurableJson = (root, name, value, transactionId) => {
  const destination = path.join(root, name);
  const stage = path.join(root, `.${name}.${transactionId}.next`);
  if (fs.existsSync(stage)) fail(`stale ${name} stage refuses replacement`);
  writeDurableFile(stage, canonicalActivationJson(value), 0o600);
  fs.renameSync(stage, destination);
  fsyncDirectory(root);
};

const versionIdentity = (role, version) => ({
  schema: 2,
  channel: 'vellum-immutable-deployment-version',
  role,
  manifestSha256: version.manifestSha256,
  daemonSha256: version.daemon.sha256,
  plistSha256: version.plist.sha256,
});

const validateStagedVersion = (directory, identity) => {
  validatePrivateDirectory(directory, 'immutable version', 0o500);
  const expected = ['identity.json', 'vellum-daemon', 'vellum.plist'];
  if (fs.readdirSync(directory).sort().join(',') !== expected.join(',')) {
    fail('immutable version file closure differs');
  }
  const installedIdentity = readCanonicalJson(path.join(directory, 'identity.json'), 'version identity', 0o400);
  if (canonicalActivationJson(installedIdentity) !== canonicalActivationJson(identity)) {
    fail('immutable version identity differs');
  }
  for (const [name, digest, fileMode] of [
    ['vellum-daemon', identity.daemonSha256, 0o500],
    ['vellum.plist', identity.plistSha256, 0o400],
  ]) {
    const file = path.join(directory, name);
    if (permissions(file) !== fileMode || sha256File(file) !== digest) {
      fail(`immutable version ${name} differs`);
    }
  }
};

const stageVersion = (versions, role, version, transactionId) => {
  const identity = versionIdentity(role, version);
  const destination = path.join(versions, version.manifestSha256);
  if (fs.existsSync(destination)) {
    validateStagedVersion(destination, identity);
    return identity;
  }
  const stage = path.join(versions, `.stage-${transactionId}-${role}`);
  fs.mkdirSync(stage, { mode: 0o700 });
  try {
    for (const [source, name, fileMode] of [
      [version.daemon.path, 'vellum-daemon', 0o500],
      [version.plist.path, 'vellum.plist', 0o400],
    ]) {
      const destinationFile = path.join(stage, name);
      fs.copyFileSync(source, destinationFile, fs.constants.COPYFILE_EXCL);
      fs.chmodSync(destinationFile, fileMode);
      const fd = fs.openSync(destinationFile, fs.constants.O_RDONLY);
      try {
        fs.fsyncSync(fd);
      } finally {
        fs.closeSync(fd);
      }
    }
    writeDurableFile(path.join(stage, 'identity.json'), canonicalActivationJson(identity), 0o400);
    fsyncDirectory(stage);
    fs.chmodSync(stage, 0o500);
    fs.renameSync(stage, destination);
    fsyncDirectory(versions);
  } catch (error) {
    if (fs.existsSync(stage)) fs.rmSync(stage, { recursive: true, force: true });
    throw error;
  }
  validateStagedVersion(destination, identity);
  return identity;
};

const pointer = (generation, selectedRole, identity) => ({
  schema: 2,
  channel: 'vellum-local-desired-deployment',
  generation,
  selectedRole,
  manifestSha256: identity.manifestSha256,
  daemonSha256: identity.daemonSha256,
  plistSha256: identity.plistSha256,
  atomicity: 'one-desired-state-pointer-only',
  launchdEffect: 'not-attempted',
});

const journal = (approval, phase, outcome, selected, generation) => ({
  schema: 1,
  channel: 'vellum-local-activation-transaction',
  transactionId: approval.transactionId,
  candidateManifestSha256: approval.candidateManifestSha256,
  priorManifestSha256: approval.priorManifestSha256,
  phase,
  outcome,
  selectedManifestSha256: selected,
  pointerGeneration: generation,
  atomicity: 'one-desired-state-pointer-only',
  launchdEffect: 'not-attempted',
});

const receipt = (value) => ({
  schema: 1,
  channel: 'vellum-local-activation-scratch-receipt',
  transactionId: value.transactionId,
  phase: value.phase,
  outcome: value.outcome,
  candidateManifestSha256: value.candidateManifestSha256,
  priorManifestSha256: value.priorManifestSha256,
  selectedManifestSha256: value.selectedManifestSha256,
  pointerGeneration: value.pointerGeneration,
  atomicity: 'one-desired-state-pointer-only',
  launchdEffect: 'not-attempted',
  contentDisclosed: false,
});

export function activationTransition(phase, event) {
  const transitions = {
    'prepared:begin-candidate': { phase: 'candidate-effect-started', outcome: null },
    'candidate-effect-started:candidate-confirmed': { phase: 'terminal', outcome: 'candidate-active' },
    'candidate-effect-started:candidate-refused': { phase: 'rollback-pending', outcome: null },
    'candidate-effect-started:candidate-unknown': { phase: 'terminal', outcome: 'outcome-unknown' },
    'rollback-pending:begin-rollback': { phase: 'rollback-effect-started', outcome: null },
    'rollback-effect-started:rollback-confirmed': { phase: 'terminal', outcome: 'prior-restored' },
    'rollback-effect-started:rollback-refused': { phase: 'terminal', outcome: 'rollback-refused' },
    'rollback-effect-started:rollback-unknown': { phase: 'terminal', outcome: 'outcome-unknown' },
  };
  const next = transitions[`${phase}:${event}`];
  if (next === undefined) fail(`closed transition refused: ${phase} + ${event}`);
  return next;
}

const crash = (selected, failpoint) => {
  if (selected === failpoint) throw new Error(`activation transaction: simulated crash at ${failpoint}`);
};

const validateDisposition = (value, label) => {
  if (!['confirmed', 'refused', 'unknown'].includes(value)) fail(`${label} disposition is incompatible`);
};

export function runScratchActivation({
  root,
  approvalFile,
  candidate,
  prior,
  candidateDisposition = 'confirmed',
  rollbackDisposition = 'confirmed',
  failpoint = 'none',
}) {
  if (!SCRATCH_FAILPOINTS.includes(failpoint)) fail('unknown crash failpoint');
  validateDisposition(candidateDisposition, 'candidate');
  validateDisposition(rollbackDisposition, 'rollback');
  const privateRoot = validatePrivateDirectory(root, 'scratch installation root', 0o700);
  if (fs.readdirSync(privateRoot).length !== 0) fail('scratch installation root is not empty');
  const checkedCandidate = validateVersion(candidate, 'candidate version');
  const checkedPrior = validateVersion(prior, 'prior version');
  const approval = validateApproval(approvalFile, checkedCandidate, checkedPrior);

  const versions = path.join(privateRoot, VERSIONS_DIRECTORY);
  fs.mkdirSync(versions, { mode: 0o700 });
  fs.chmodSync(versions, 0o700);
  const candidateIdentity = stageVersion(versions, 'candidate', checkedCandidate, approval.transactionId);
  const priorIdentity = stageVersion(versions, 'prior', checkedPrior, approval.transactionId);

  replaceDurableJson(privateRoot, POINTER_FILE, pointer(0, 'prior', priorIdentity), approval.transactionId);
  let current = journal(approval, 'prepared', 'prepared', priorIdentity.manifestSha256, 0);
  replaceDurableJson(privateRoot, JOURNAL_FILE, current, approval.transactionId);
  crash(failpoint, 'after-prepared');

  activationTransition('prepared', 'begin-candidate');
  current = journal(approval, 'candidate-effect-started', null, priorIdentity.manifestSha256, 0);
  replaceDurableJson(privateRoot, JOURNAL_FILE, current, approval.transactionId);
  replaceDurableJson(privateRoot, POINTER_FILE, pointer(1, 'candidate', candidateIdentity), approval.transactionId);
  crash(failpoint, 'after-candidate-pointer');

  if (candidateDisposition === 'confirmed') {
    activationTransition('candidate-effect-started', 'candidate-confirmed');
    current = journal(approval, 'terminal', 'candidate-selected', candidateIdentity.manifestSha256, 1);
    replaceDurableJson(privateRoot, JOURNAL_FILE, current, approval.transactionId);
    crash(failpoint, 'after-candidate-terminal');
    return receipt(current);
  }
  if (candidateDisposition === 'unknown') {
    activationTransition('candidate-effect-started', 'candidate-unknown');
    current = journal(approval, 'terminal', 'outcome-unknown', candidateIdentity.manifestSha256, 1);
    replaceDurableJson(privateRoot, JOURNAL_FILE, current, approval.transactionId);
    crash(failpoint, 'after-candidate-terminal');
    return receipt(current);
  }

  activationTransition('candidate-effect-started', 'candidate-refused');
  current = journal(approval, 'rollback-pending', 'rollback-pending', candidateIdentity.manifestSha256, 1);
  replaceDurableJson(privateRoot, JOURNAL_FILE, current, approval.transactionId);
  crash(failpoint, 'after-rollback-pending');
  activationTransition('rollback-pending', 'begin-rollback');
  current = journal(approval, 'rollback-effect-started', null, candidateIdentity.manifestSha256, 1);
  replaceDurableJson(privateRoot, JOURNAL_FILE, current, approval.transactionId);
  replaceDurableJson(privateRoot, POINTER_FILE, pointer(2, 'prior', priorIdentity), approval.transactionId);
  crash(failpoint, 'after-prior-pointer');

  const event = `rollback-${rollbackDisposition}`;
  activationTransition('rollback-effect-started', event);
  const outcome = rollbackDisposition === 'confirmed'
    ? 'prior-restored'
    : rollbackDisposition === 'refused' ? 'rollback-refused' : 'outcome-unknown';
  current = journal(approval, 'terminal', outcome, priorIdentity.manifestSha256, 2);
  replaceDurableJson(privateRoot, JOURNAL_FILE, current, approval.transactionId);
  crash(failpoint, 'after-rollback-terminal');
  return receipt(current);
}

export function inspectScratchActivation(root) {
  const privateRoot = validatePrivateDirectory(root, 'scratch installation root', 0o700);
  const value = readCanonicalJson(path.join(privateRoot, JOURNAL_FILE), 'activation journal', 0o600);
  const selected = readCanonicalJson(path.join(privateRoot, POINTER_FILE), 'desired deployment pointer', 0o600);
  exactKeys(
    value,
    [
      'schema', 'channel', 'transactionId', 'candidateManifestSha256', 'priorManifestSha256',
      'phase', 'outcome', 'selectedManifestSha256', 'pointerGeneration', 'atomicity', 'launchdEffect',
    ],
    'activation journal',
  );
  exactKeys(
    selected,
    [
      'schema', 'channel', 'generation', 'selectedRole', 'manifestSha256', 'daemonSha256',
      'plistSha256', 'atomicity', 'launchdEffect',
    ],
    'desired deployment pointer',
  );
  const commonValid = value.schema === 1
    && value.channel === 'vellum-local-activation-transaction'
    && TRANSACTION.test(value.transactionId)
    && DIGEST.test(value.candidateManifestSha256)
    && DIGEST.test(value.priorManifestSha256)
    && value.atomicity === 'one-desired-state-pointer-only'
    && value.launchdEffect === 'not-attempted'
    && selected.schema === 2
    && selected.channel === 'vellum-local-desired-deployment'
    && Number.isSafeInteger(selected.generation)
    && selected.generation >= 0
    && selected.generation <= 2
    && ['candidate', 'prior'].includes(selected.selectedRole)
    && DIGEST.test(selected.manifestSha256)
    && DIGEST.test(selected.daemonSha256)
    && DIGEST.test(selected.plistSha256)
    && selected.atomicity === 'one-desired-state-pointer-only'
    && selected.launchdEffect === 'not-attempted';
  if (!commonValid) fail('journal or pointer policy is incompatible');
  const closedState = (
    (value.phase === 'prepared' && value.outcome === 'prepared')
    || (value.phase === 'candidate-effect-started' && value.outcome === null)
    || (value.phase === 'rollback-pending' && value.outcome === 'rollback-pending')
    || (value.phase === 'rollback-effect-started' && value.outcome === null)
    || (value.phase === 'terminal' && [
      'candidate-selected', 'prior-restored', 'rollback-refused', 'outcome-unknown',
    ].includes(value.outcome))
  );
  const selectedIdentity = selected.selectedRole === 'candidate'
    ? value.candidateManifestSha256
    : value.priorManifestSha256;
  if (!closedState || selected.manifestSha256 !== selectedIdentity) {
    fail('journal or pointer state is outside the closed algebra');
  }
  validateStagedVersion(
    path.join(privateRoot, VERSIONS_DIRECTORY, selected.manifestSha256),
    {
      schema: 2,
      channel: 'vellum-immutable-deployment-version',
      role: selected.selectedRole,
      manifestSha256: selected.manifestSha256,
      daemonSha256: selected.daemonSha256,
      plistSha256: selected.plistSha256,
    },
  );
  if (
    value.selectedManifestSha256 !== selected.manifestSha256
    || value.pointerGeneration !== selected.generation
  ) {
    return receipt({
      ...value,
      phase: 'terminal',
      outcome: 'outcome-unknown',
      selectedManifestSha256: selected.manifestSha256,
      pointerGeneration: selected.generation,
    });
  }
  return receipt(value);
}
