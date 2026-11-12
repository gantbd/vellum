import fs from 'node:fs';
import path from 'node:path';

import { canonicalJson, sha256Bytes } from './release-candidate-lib.mjs';
import { prepareCandidateBundleInstall } from './release-candidate-bundle-lib.mjs';

const PRODUCT_MODE = 0o500;
const DIRECTORY_MODE = 0o700;
const RECEIPT_MODE = 0o400;
const RECEIPT_FILE = 'install-receipt.json';
const PRODUCT_DIRECTORY = 'bin';
const FAULTS = new Set([
  'after-bin',
  'after-vellum-daemon',
  'after-vellum-mcp',
]);

const fail = (message) => {
  throw new Error(`release candidate install: ${message}`);
};

const effectiveUid = () => {
  if (typeof process.geteuid !== 'function') fail('effective owner is unavailable');
  return process.geteuid();
};

const canonicalExistingPath = (value, label) => {
  if (typeof value !== 'string' || !path.isAbsolute(value)) fail(`${label} must be absolute`);
  const normalized = path.normalize(value);
  if (normalized !== value) fail(`${label} must be canonical and non-symbolic`);
  const stat = fs.lstatSync(normalized);
  if (stat.isSymbolicLink() || fs.realpathSync(normalized) !== normalized) {
    fail(`${label} must be canonical and non-symbolic`);
  }
  return { path: normalized, stat };
};

const validateOwnedDirectory = (value, label, mode) => {
  const resolved = canonicalExistingPath(value, label);
  if (!resolved.stat.isDirectory() || (resolved.stat.mode & 0o777) !== mode) {
    fail(`${label} must be a directory with mode ${mode.toString(8)}`);
  }
  if (resolved.stat.uid !== effectiveUid()) fail(`${label} must be owned by the effective user`);
  return resolved.path;
};

const validateOwnedFile = (value, label, mode) => {
  const resolved = canonicalExistingPath(value, label);
  if (
    !resolved.stat.isFile()
    || resolved.stat.nlink !== 1
    || (resolved.stat.mode & 0o777) !== mode
  ) {
    fail(`${label} must be one single-link regular file with mode ${mode.toString(8)}`);
  }
  if (resolved.stat.uid !== effectiveUid()) fail(`${label} must be owned by the effective user`);
  return resolved.path;
};

const syncPath = (value) => {
  const descriptor = fs.openSync(value, 'r');
  try {
    fs.fsyncSync(descriptor);
  } finally {
    fs.closeSync(descriptor);
  }
};

const writeExclusiveDurable = (destination, bytes, mode) => {
  const descriptor = fs.openSync(destination, 'wx', mode);
  try {
    fs.writeFileSync(descriptor, bytes);
    fs.fchmodSync(descriptor, mode);
    fs.fsyncSync(descriptor);
  } finally {
    fs.closeSync(descriptor);
  }
};

const receiptFor = (prepared) => ({
  schema: 1,
  channel: 'local-release-candidate-installation',
  manifestSha256: prepared.report.manifestSha256,
  claims: ['verified-bundle', 'daemon-mcp-only', 'durable-local-prefix'],
  nonClaims: [
    'path-configuration',
    'launchd-installation',
    'activation',
    'upgrade',
    'uninstall',
    'signature',
    'attestation',
  ],
  products: prepared.products.map((product) => ({
    file: `${PRODUCT_DIRECTORY}/${product.name}`,
    bytes: product.byteLength,
    sha256: product.sha256,
  })),
  activationPerformed: false,
});

const validateInstalled = (root, prepared) => {
  const rootClosure = fs.readdirSync(root).sort();
  if (rootClosure.join(',') !== `${PRODUCT_DIRECTORY},${RECEIPT_FILE}`) {
    fail('installation root closure is incomplete or contains extra entries');
  }
  const productRoot = validateOwnedDirectory(
    path.join(root, PRODUCT_DIRECTORY),
    'installed product directory',
    DIRECTORY_MODE,
  );
  const expectedNames = prepared.products.map((product) => product.name).sort();
  if (fs.readdirSync(productRoot).sort().join(',') !== expectedNames.join(',')) {
    fail('installed product closure is incompatible');
  }
  for (const product of prepared.products) {
    const file = validateOwnedFile(
      path.join(productRoot, product.name),
      `installed ${product.name}`,
      PRODUCT_MODE,
    );
    const bytes = fs.readFileSync(file);
    if (bytes.length !== product.byteLength || sha256Bytes(bytes) !== product.sha256) {
      fail(`installed ${product.name} identity is incompatible`);
    }
  }
  const receiptFile = validateOwnedFile(
    path.join(root, RECEIPT_FILE),
    'installation receipt',
    RECEIPT_MODE,
  );
  if (fs.readFileSync(receiptFile, 'utf8') !== canonicalJson(receiptFor(prepared))) {
    fail('installation receipt is incompatible');
  }
  syncPath(productRoot);
  syncPath(root);
};

const injectFault = (configured, point) => {
  if (configured === point) fail(`injected interruption at ${point}`);
};

const sameIdentity = (left, right) => left.dev === right.dev && left.ino === right.ino;

export function installCandidateBundle(bundleFile, installRoot, options = {}) {
  const bundle = canonicalExistingPath(bundleFile, 'bundle');
  if (!bundle.stat.isFile() || bundle.stat.nlink !== 1 || bundle.stat.uid !== effectiveUid()) {
    fail('bundle must be one effective-user-owned regular file');
  }
  const root = validateOwnedDirectory(installRoot, 'installation root', DIRECTORY_MODE);
  const rootIdentity = fs.lstatSync(root);
  if (options.faultAfter !== undefined && !FAULTS.has(options.faultAfter)) {
    fail('unknown injected interruption');
  }

  // Validation and native smoke complete before the first installation write.
  const prepared = prepareCandidateBundleInstall(bundle.path, options);
  const observedBundle = canonicalExistingPath(bundle.path, 'bundle');
  const observedRoot = canonicalExistingPath(root, 'installation root');
  if (!sameIdentity(bundle.stat, observedBundle.stat)) fail('bundle identity changed during validation');
  if (!sameIdentity(rootIdentity, observedRoot.stat)) {
    fail('installation root identity changed during validation');
  }
  const existing = fs.readdirSync(root);
  if (existing.length !== 0) {
    validateInstalled(root, prepared);
    return {
      status: 'already-installed',
      manifestSha256: prepared.report.manifestSha256,
      productCount: prepared.products.length,
      activationPerformed: false,
    };
  }

  const productRoot = path.join(root, PRODUCT_DIRECTORY);
  fs.mkdirSync(productRoot, { mode: DIRECTORY_MODE });
  fs.chmodSync(productRoot, DIRECTORY_MODE);
  syncPath(root);
  injectFault(options.faultAfter, 'after-bin');

  for (const product of prepared.products) {
    writeExclusiveDurable(path.join(productRoot, product.name), product.bytes, PRODUCT_MODE);
    injectFault(options.faultAfter, `after-${product.name}`);
  }
  syncPath(productRoot);

  const receipt = canonicalJson(receiptFor(prepared));
  writeExclusiveDurable(path.join(root, RECEIPT_FILE), receipt, RECEIPT_MODE);
  syncPath(root);
  validateInstalled(root, prepared);
  return {
    status: 'installed',
    manifestSha256: prepared.report.manifestSha256,
    productCount: prepared.products.length,
    activationPerformed: false,
  };
}

