#!/usr/bin/env node

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';
import { canonicalJson, sha256File } from './release-candidate-lib.mjs';

process.on('uncaughtException', (error) => {
  process.stderr.write(`Vellum clean candidate: ${error.message}\n`);
  process.exit(1);
});

const root = fs.realpathSync(process.cwd());
const argumentsByName = new Map();
for (const argument of process.argv.slice(2)) {
  const separator = argument.indexOf('=');
  if (separator < 3 || !argument.startsWith('--')) {
    throw new Error(`unsupported argument: ${argument}`);
  }
  argumentsByName.set(argument.slice(2, separator), argument.slice(separator + 1));
}
for (const name of argumentsByName.keys()) {
  if (!['vellum-ref', 'output-dir'].includes(name)) {
    throw new Error(`unsupported argument: --${name}`);
  }
}
if (!argumentsByName.has('output-dir')) throw new Error('--output-dir is required');

const run = (command, args, options = {}) => {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? root,
    env: options.env ?? process.env,
    encoding: 'utf8',
  });
  if (result.status !== 0) {
    if (result.stdout) process.stderr.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);
    throw new Error(`${command} failed with status ${result.status ?? 'unknown'}`);
  }
  return (result.stdout ?? '').trim();
};
const commit = (repository, revision, label) => {
  const resolved = run('git', ['-C', repository, 'rev-parse', `${revision}^{commit}`]);
  if (!/^[0-9a-f]{40}$/.test(resolved)) throw new Error(`${label} is not a full commit`);
  return resolved;
};
const tree = (repository, revision, label) => {
  const resolved = run('git', ['-C', repository, 'rev-parse', `${revision}^{tree}`]);
  if (!/^[0-9a-f]{40}$/.test(resolved)) throw new Error(`${label} is not a full tree identity`);
  return resolved;
};
const cloneAt = (source, revision, destination) => {
  run('git', ['clone', '--quiet', '--no-hardlinks', '--no-checkout', source, destination]);
  run('git', ['-C', destination, 'checkout', '--quiet', '--detach', revision]);
  if (run('git', ['-C', destination, 'status', '--porcelain', '--untracked-files=all']) !== '') {
    throw new Error('fresh exact-source checkout is not clean');
  }
};

const policy = JSON.parse(fs.readFileSync(path.join(root, 'release/github-source.json'), 'utf8'));
const vellumRevision = commit(root, argumentsByName.get('vellum-ref') ?? 'HEAD', 'Vellum revision');
const keystoneSource = fs.realpathSync(
  path.resolve(root, process.env.KEYSTONE_SOURCE ?? policy.keystone.relativePath),
);
const keystoneRevision = commit(
  keystoneSource,
  policy.keystone.revision,
  'Keystone revision',
);
if (keystoneRevision !== policy.keystone.revision) {
  throw new Error('Keystone source does not contain the coordinated exact commit');
}

const requestedOutput = path.resolve(root, argumentsByName.get('output-dir'));
if (fs.existsSync(requestedOutput)) throw new Error('output directory already exists');
const outputParent = path.dirname(requestedOutput);
const outputParentStat = fs.lstatSync(outputParent);
if (outputParentStat.isSymbolicLink() || !outputParentStat.isDirectory()) {
  throw new Error('output parent must be a real directory');
}
const canonicalOutput = path.join(fs.realpathSync(outputParent), path.basename(requestedOutput));

const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-clean-candidate-'));
let outputCreated = false;
try {
  const vellum = path.join(temporary, 'vellum');
  const keystone = path.join(temporary, 'keystone');
  cloneAt(root, vellumRevision, vellum);
  cloneAt(keystoneSource, keystoneRevision, keystone);

  const gitConfiguration = path.join(temporary, 'gitconfig');
  fs.writeFileSync(
    gitConfiguration,
    `[url "${pathToFileURL(vellum).href}"]\n\tinsteadOf = ${policy.vellumRepository}\n`
      + `[url "${pathToFileURL(keystone).href}"]\n\tinsteadOf = ${policy.keystone.repository}\n`,
    { mode: 0o600 },
  );
  const environment = { ...process.env };
  environment.KEYSTONE_SOURCE = keystone;
  environment.GIT_CONFIG_GLOBAL = gitConfiguration;
  environment.GIT_CONFIG_NOSYSTEM = '1';
  environment.GIT_TERMINAL_PROMPT = '0';
  environment.CARGO_NET_GIT_FETCH_WITH_CLI = 'true';
  environment.CARGO_NET_OFFLINE = 'true';

  fs.mkdirSync(canonicalOutput, { mode: 0o700 });
  outputCreated = true;
  const candidate = path.join(canonicalOutput, 'candidate');
  const rebuilt = path.join(temporary, 'candidate-rebuild');
  run(
    'node',
    [
      'scripts/build-release-candidate.mjs',
      `--vellum-revision=${vellumRevision}`,
      `--output-dir=${candidate}`,
    ],
    { cwd: vellum, env: environment },
  );
  run(
    'node',
    [
      'scripts/build-release-candidate.mjs',
      `--vellum-revision=${vellumRevision}`,
      `--output-dir=${rebuilt}`,
    ],
    { cwd: vellum, env: environment },
  );
  run(
    'node',
    [
      'scripts/compare-release-candidates.mjs',
      `--candidate-a=${candidate}`,
      `--candidate-b=${rebuilt}`,
      `--evidence-file=${path.join(canonicalOutput, 'candidate-byte-equivalence.json')}`,
      `--repository-root=${vellum}`,
    ],
    { cwd: vellum, env: environment },
  );
  const manifest = JSON.parse(fs.readFileSync(path.join(candidate, 'manifest.json'), 'utf8'));
  const equivalence = path.join(canonicalOutput, 'candidate-byte-equivalence.json');
  const provenance = {
    schema: 1,
    channel: 'local-release-candidate-source-provenance',
    claims: [
      'exact-local-git-objects',
      'clean-detached-checkouts',
      'candidate-manifest-bound',
      'byte-equivalence-bound',
    ],
    nonClaims: [
      'remote-reachability',
      'signature',
      'attestation',
      'github-release',
      'independent-build-execution',
      'cross-host-reproducible-build',
    ],
    source: {
      vellum: {
        repository: policy.vellumRepository,
        revision: vellumRevision,
        tree: tree(vellum, vellumRevision, 'Vellum tree'),
      },
      keystone: {
        repository: policy.keystone.repository,
        revision: keystoneRevision,
        tree: tree(keystone, keystoneRevision, 'Keystone tree'),
      },
    },
    subject: {
      manifestSha256: sha256File(path.join(candidate, 'manifest.json')),
      byteEquivalenceSha256: sha256File(equivalence),
    },
  };
  if (
    manifest.source.vellum.revision !== provenance.source.vellum.revision
    || manifest.source.keystone.revision !== provenance.source.keystone.revision
  ) {
    throw new Error('candidate source identity differs from the clean Git objects');
  }
  fs.writeFileSync(
    path.join(canonicalOutput, 'source-provenance.json'),
    canonicalJson(provenance),
    { flag: 'wx', mode: 0o600 },
  );
  fs.chmodSync(canonicalOutput, 0o700);
  process.stdout.write(
    `Vellum clean exact-source candidate qualified: ${vellumRevision}; no remote or publication effect\n`,
  );
} catch (error) {
  if (outputCreated) fs.rmSync(canonicalOutput, { recursive: true, force: true });
  throw error;
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
