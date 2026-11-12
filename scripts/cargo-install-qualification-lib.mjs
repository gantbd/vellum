import fs from 'node:fs';
import path from 'node:path';

const fail = (message) => {
  throw new Error(message);
};

export const requireCanonicalDirectory = (directory, label) => {
  const resolved = path.resolve(directory);
  const metadata = fs.lstatSync(resolved);
  if (metadata.isSymbolicLink() || !metadata.isDirectory()) {
    fail(`${label} must be a real directory`);
  }
  return fs.realpathSync(resolved);
};

export const requireCanonicalRegularFile = (root, candidate, label) => {
  const canonicalRoot = requireCanonicalDirectory(root, `${label} root`);
  const resolved = path.resolve(candidate);
  const canonicalCandidate = fs.realpathSync(resolved);
  const relative = path.relative(canonicalRoot, canonicalCandidate);
  if (relative === '' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    fail(`${label} escapes its declared root`);
  }
  const metadata = fs.lstatSync(resolved);
  if (metadata.isSymbolicLink() || !metadata.isFile()) {
    fail(`${label} must be a regular non-symlink file`);
  }
  return metadata;
};

export const isolatedCargoEnvironment = (base, inherited = process.env) => {
  const temporary = requireCanonicalDirectory(base, 'qualification temporary root');
  const cargoHome = path.join(temporary, 'cargo-home');
  const target = path.join(temporary, 'cargo-target');
  fs.mkdirSync(cargoHome, { mode: 0o700 });
  fs.mkdirSync(target, { mode: 0o700 });
  const environment = { ...inherited };
  for (const variable of Object.keys(environment)) {
    if (variable.startsWith('CARGO_')) delete environment[variable];
  }
  for (const variable of [
    'RUSTFLAGS',
    'RUSTDOCFLAGS',
    'RUSTC_WRAPPER',
    'RUSTC_WORKSPACE_WRAPPER',
  ]) delete environment[variable];
  environment.CARGO_HOME = cargoHome;
  environment.CARGO_TARGET_DIR = target;
  environment.CARGO_NET_OFFLINE = 'true';
  return { environment, cargoHome, target };
};

export const validateInstalledBinaryClosure = (installRoot, expectedBinaries) => {
  const root = requireCanonicalDirectory(installRoot, 'Cargo install root');
  const bin = requireCanonicalDirectory(path.join(root, 'bin'), 'Cargo install bin directory');
  const expected = [...expectedBinaries].sort();
  if (new Set(expected).size !== expected.length || expected.length === 0) {
    fail('expected installed binary closure must be non-empty and unique');
  }
  const observed = fs.readdirSync(bin).sort();
  if (observed.join('\0') !== expected.join('\0')) {
    fail(`installed binary closure is incompatible: expected ${expected.join(',')}; observed ${observed.join(',')}`);
  }
  for (const name of expected) {
    if (name.includes('/') || name.includes('\\') || name === '.' || name === '..') {
      fail('installed binary identity is not a basename');
    }
    const file = path.join(bin, name);
    const metadata = requireCanonicalRegularFile(root, file, `installed binary ${name}`);
    if ((metadata.mode & 0o111) === 0) fail(`installed binary ${name} is not executable`);
    if ((metadata.mode & 0o022) !== 0) {
      fail(`installed binary ${name} remains group/world writable`);
    }
  }
  return expected.map((name) => path.join(bin, name));
};

export const assertContentSafeSmoke = (result, binary, forbiddenFragments) => {
  const output = `${result.stdout ?? ''}\n${result.stderr ?? ''}`;
  if (result.status === 0) fail(`${binary} accepted an empty startup invocation`);
  if (!output.includes(binary) && !output.includes('usage:')) {
    fail(`${binary} empty startup refusal is not self-identifying`);
  }
  for (const fragment of forbiddenFragments) {
    if (fragment.length !== 0 && output.includes(fragment)) {
      fail(`${binary} empty startup refusal disclosed a qualification path`);
    }
  }
};
