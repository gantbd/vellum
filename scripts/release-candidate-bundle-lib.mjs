import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import {
  ARTIFACT_NAMES,
  CHECKSUM_FILE,
  DEPLOYMENT_NAMES,
  MANIFEST_FILE,
  canonicalJson,
  sha256File,
  smokeCandidate,
  smokeLaunchdActivationExecutor,
  smokeLaunchdDeployment,
  validateCandidate,
} from './release-candidate-lib.mjs';

const BLOCK = 512;
const MAX_BUNDLE_BYTES = 512 * 1024 * 1024;
const FILES = [
  ...ARTIFACT_NAMES,
  ...DEPLOYMENT_NAMES,
  MANIFEST_FILE,
  CHECKSUM_FILE,
].sort();
const EXPECTED = [
  { name: 'candidate/', mode: 0o700, type: '5' },
  ...FILES.map((name) => ({
    name: `candidate/${name}`,
    mode: [...ARTIFACT_NAMES, ...DEPLOYMENT_NAMES].includes(name) ? 0o755 : 0o600,
    type: '0',
  })),
  { name: 'candidate-byte-equivalence.json', mode: 0o600, type: '0' },
];

const fail = (message) => {
  throw new Error(`release candidate bundle: ${message}`);
};

const exactKeys = (value, keys, label) => {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    fail(`${label} is not an object`);
  }
  if (Object.keys(value).sort().join(',') !== [...keys].sort().join(',')) {
    fail(`${label} fields are incompatible`);
  }
};

const exactMode = (file, mode, label) => {
  const stat = fs.lstatSync(file);
  if (stat.isSymbolicLink() || !stat.isFile() || (stat.mode & 0o777) !== mode) {
    fail(`${label} must be a regular file with mode ${mode.toString(8)}`);
  }
  return stat;
};

const writeOctal = (header, offset, length, value) => {
  const encoded = value.toString(8).padStart(length - 1, '0');
  if (encoded.length !== length - 1) fail('archive numeric field overflow');
  header.write(encoded, offset, length - 1, 'ascii');
  header[offset + length - 1] = 0;
};

const headerFor = (entry) => {
  const header = Buffer.alloc(BLOCK);
  if (!/^[A-Za-z0-9./-]+$/.test(entry.name) || Buffer.byteLength(entry.name) > 100) {
    fail('archive entry name is not canonical');
  }
  header.write(entry.name, 0, 100, 'ascii');
  writeOctal(header, 100, 8, entry.mode);
  writeOctal(header, 108, 8, 0);
  writeOctal(header, 116, 8, 0);
  writeOctal(header, 124, 12, entry.bytes.length);
  writeOctal(header, 136, 12, 0);
  header.fill(0x20, 148, 156);
  header.write(entry.type, 156, 1, 'ascii');
  header.write('ustar\0', 257, 6, 'ascii');
  header.write('00', 263, 2, 'ascii');
  header.write('root', 265, 32, 'ascii');
  header.write('root', 297, 32, 'ascii');
  writeOctal(header, 329, 8, 0);
  writeOctal(header, 337, 8, 0);
  const checksum = header.reduce((sum, byte) => sum + byte, 0);
  header.write(checksum.toString(8).padStart(6, '0'), 148, 6, 'ascii');
  header[154] = 0;
  header[155] = 0x20;
  return header;
};

const encode = (entries) => {
  const chunks = [];
  let total = BLOCK * 2;
  for (const entry of entries) {
    const padding = (BLOCK - (entry.bytes.length % BLOCK)) % BLOCK;
    total += BLOCK + entry.bytes.length + padding;
    if (total > MAX_BUNDLE_BYTES) fail('archive exceeds the bounded size');
    chunks.push(headerFor(entry), entry.bytes, Buffer.alloc(padding));
  }
  chunks.push(Buffer.alloc(BLOCK * 2));
  return Buffer.concat(chunks, total);
};

const readString = (header, offset, length) => {
  const bytes = header.subarray(offset, offset + length);
  const zero = bytes.indexOf(0);
  const end = zero < 0 ? bytes.length : zero;
  return bytes.subarray(0, end).toString('ascii');
};

const readOctal = (header, offset, length, label) => {
  const raw = readString(header, offset, length).trim();
  if (!/^[0-7]+$/.test(raw)) fail(`${label} is not canonical octal`);
  const value = Number.parseInt(raw, 8);
  if (!Number.isSafeInteger(value)) fail(`${label} exceeds the safe range`);
  return value;
};

const parse = (bytes) => {
  if (bytes.length === 0 || bytes.length > MAX_BUNDLE_BYTES || bytes.length % BLOCK !== 0) {
    fail('archive length is invalid');
  }
  const entries = [];
  let cursor = 0;
  let terminated = false;
  while (cursor + BLOCK <= bytes.length) {
    const header = bytes.subarray(cursor, cursor + BLOCK);
    if (header.every((byte) => byte === 0)) {
      if (
        cursor + BLOCK * 2 !== bytes.length
        || !bytes.subarray(cursor + BLOCK).every((byte) => byte === 0)
      ) fail('archive terminator or trailing content is invalid');
      terminated = true;
      break;
    }
    const storedChecksum = readOctal(header, 148, 8, 'archive checksum');
    const checksumHeader = Buffer.from(header);
    checksumHeader.fill(0x20, 148, 156);
    const computedChecksum = checksumHeader.reduce((sum, byte) => sum + byte, 0);
    if (storedChecksum !== computedChecksum) fail('archive header checksum is invalid');
    if (readString(header, 257, 6) !== 'ustar' || readString(header, 263, 2) !== '00') {
      fail('archive is not canonical USTAR');
    }
    const entry = {
      name: readString(header, 0, 100),
      mode: readOctal(header, 100, 8, 'archive mode'),
      uid: readOctal(header, 108, 8, 'archive uid'),
      gid: readOctal(header, 116, 8, 'archive gid'),
      size: readOctal(header, 124, 12, 'archive size'),
      mtime: readOctal(header, 136, 12, 'archive mtime'),
      type: readString(header, 156, 1) || '0',
      uname: readString(header, 265, 32),
      gname: readString(header, 297, 32),
    };
    if (entry.uid !== 0 || entry.gid !== 0 || entry.mtime !== 0) {
      fail('archive ownership or time fields are not canonical');
    }
    if (entry.uname !== 'root' || entry.gname !== 'root') {
      fail('archive owner names are not canonical');
    }
    if (!header.equals(headerFor({ ...entry, bytes: { length: entry.size } }))) {
      fail('archive header is not canonical');
    }
    const dataStart = cursor + BLOCK;
    const dataEnd = dataStart + entry.size;
    const next = dataStart + Math.ceil(entry.size / BLOCK) * BLOCK;
    if (dataEnd > bytes.length || next > bytes.length) fail('archive entry is truncated');
    if (!bytes.subarray(dataEnd, next).every((byte) => byte === 0)) {
      fail('archive entry padding is not canonical');
    }
    entry.bytes = bytes.subarray(dataStart, dataEnd);
    entries.push(entry);
    cursor = next;
  }
  if (!terminated) fail('archive terminator is absent');
  if (entries.length !== EXPECTED.length) fail('archive entry closure is incompatible');
  for (const [index, expected] of EXPECTED.entries()) {
    const actual = entries[index];
    if (
      actual.name !== expected.name
      || actual.mode !== expected.mode
      || actual.type !== expected.type
      || (actual.type === '5' && actual.size !== 0)
    ) fail(`archive entry is incompatible at ordinal ${index}`);
  }
  return entries;
};

const validateEvidence = (file, candidate) => {
  const text = fs.readFileSync(file, 'utf8');
  let evidence;
  try {
    evidence = JSON.parse(text);
  } catch (error) {
    fail(`byte-equivalence evidence is invalid JSON: ${error.message}`);
  }
  if (text !== canonicalJson(evidence)) fail('byte-equivalence evidence is not canonical');
  exactKeys(
    evidence,
    ['schema', 'channel', 'claims', 'nonClaims', 'source', 'build', 'manifestSha256', 'files'],
    'byte-equivalence evidence',
  );
  if (
    evidence.schema !== 1
    || evidence.channel !== 'local-release-candidate-byte-equivalence'
    || evidence.claims.join(',') !== 'distinct-materialization,exact-file-closure,byte-for-byte-equivalence'
    || evidence.nonClaims.join(',')
      !== 'independent-build-execution,reproducible-build,signature,attestation,github-release,activation'
    || evidence.manifestSha256 !== candidate.manifestSha256
  ) fail('byte-equivalence evidence identity is incompatible');
  exactKeys(evidence.source, ['vellumRevision', 'keystoneRevision'], 'evidence source');
  exactKeys(evidence.build, ['target', 'rustc', 'cargo', 'profile', 'features'], 'evidence build');
  if (
    evidence.source.vellumRevision !== candidate.manifest.source.vellum.revision
    || evidence.source.keystoneRevision !== candidate.manifest.source.keystone.revision
    || evidence.build.target !== candidate.manifest.build.target
    || evidence.build.rustc !== candidate.manifest.build.rustc
    || evidence.build.cargo !== candidate.manifest.build.cargo
    || evidence.build.profile !== candidate.manifest.build.profile
    || evidence.build.features !== candidate.manifest.build.features
  ) fail('byte-equivalence evidence source or build is incompatible');
  const observed = FILES.map((name) => {
    const candidateFile = path.join(candidate.root, name);
    return { file: name, bytes: fs.statSync(candidateFile).size, sha256: sha256File(candidateFile) };
  });
  if (canonicalJson(evidence.files) !== canonicalJson(observed)) {
    fail('byte-equivalence evidence file closure is incompatible');
  }
  return evidence;
};

export function buildCandidateBundle(qualifiedDirectory, options = {}) {
  const root = fs.realpathSync(qualifiedDirectory);
  const closure = fs.readdirSync(root, { withFileTypes: true });
  if (
    closure.length !== 2
    || !closure.some((entry) => entry.name === 'candidate' && entry.isDirectory() && !entry.isSymbolicLink())
    || !closure.some((entry) => entry.name === 'candidate-byte-equivalence.json' && entry.isFile() && !entry.isSymbolicLink())
  ) fail('qualified output closure is incompatible');
  const candidateRoot = path.join(root, 'candidate');
  if ((fs.lstatSync(candidateRoot).mode & 0o777) !== 0o700) {
    fail('candidate directory mode is incompatible');
  }
  const candidate = validateCandidate(candidateRoot, {
    repositoryRoot: options.repositoryRoot,
    runtimeTarget: options.runtimeTarget,
  });
  const evidenceFile = path.join(root, 'candidate-byte-equivalence.json');
  exactMode(evidenceFile, 0o600, 'byte-equivalence evidence');
  validateEvidence(evidenceFile, candidate);
  const entries = [{ ...EXPECTED[0], bytes: Buffer.alloc(0) }];
  for (const name of FILES) {
    const expected = EXPECTED.find((entry) => entry.name === `candidate/${name}`);
    const file = path.join(candidateRoot, name);
    exactMode(file, expected.mode, expected.name);
    entries.push({ ...expected, bytes: fs.readFileSync(file) });
  }
  entries.push({ ...EXPECTED.at(-1), bytes: fs.readFileSync(evidenceFile) });
  return encode(entries);
}

const inspectCandidateBundle = (bundleFile, options = {}, includeProducts = false) => {
  const stat = fs.lstatSync(bundleFile);
  if (stat.isSymbolicLink() || !stat.isFile() || stat.size > MAX_BUNDLE_BYTES) {
    fail('bundle must be one bounded regular file');
  }
  const entries = parse(fs.readFileSync(bundleFile));
  const scratch = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-candidate-bundle-')));
  try {
    const candidateRoot = path.join(scratch, 'candidate');
    fs.mkdirSync(candidateRoot, { mode: 0o700 });
    fs.chmodSync(candidateRoot, 0o700);
    for (const entry of entries.slice(1)) {
      const destination = entry.name === 'candidate-byte-equivalence.json'
        ? path.join(scratch, entry.name)
        : path.join(candidateRoot, path.basename(entry.name));
      fs.writeFileSync(destination, entry.bytes, { flag: 'wx', mode: entry.mode });
      fs.chmodSync(destination, entry.mode);
    }
    const candidate = validateCandidate(candidateRoot, {
      repositoryRoot: options.repositoryRoot,
      runtimeTarget: options.runtimeTarget,
    });
    validateEvidence(path.join(scratch, 'candidate-byte-equivalence.json'), candidate);
    const smoke = options.smoke === false ? null : {
      products: smokeCandidate(candidate),
      launchd: smokeLaunchdDeployment(candidate),
      activator: smokeLaunchdActivationExecutor(candidate),
    };
    const report = {
      manifestSha256: candidate.manifestSha256,
      vellumRevision: candidate.manifest.source.vellum.revision,
      fileCount: FILES.length,
      smoke,
    };
    const products = includeProducts ? ARTIFACT_NAMES.map((name) => {
      const artifact = candidate.manifest.artifacts.find((item) => item.file === name);
      const entry = entries.find((item) => item.name === `candidate/${name}`);
      if (artifact === undefined || entry === undefined) {
        fail('candidate product closure is incompatible');
      }
      return {
        name,
        bytes: entry.bytes,
        byteLength: artifact.bytes,
        sha256: artifact.sha256,
      };
    }) : [];
    return { report, products };
  } finally {
    fs.rmSync(scratch, { recursive: true, force: true });
  }
};

export function verifyCandidateBundle(bundleFile, options = {}) {
  return inspectCandidateBundle(bundleFile, options).report;
}

export function prepareCandidateBundleInstall(bundleFile, options = {}) {
  return inspectCandidateBundle(bundleFile, options, true);
}
