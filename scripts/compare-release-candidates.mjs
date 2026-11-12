#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

import { canonicalJson, validateCandidate } from './release-candidate-lib.mjs';
import { compareCandidateProducts } from './release-reproducibility-lib.mjs';

process.on('uncaughtException', (error) => {
  process.stderr.write(`${error.message}\n`);
  process.exit(1);
});

const argumentsByName = new Map();
for (const argument of process.argv.slice(2)) {
  const separator = argument.indexOf('=');
  if (separator < 3 || !argument.startsWith('--')) {
    throw new Error(`release reproducibility: unsupported argument: ${argument}`);
  }
  argumentsByName.set(argument.slice(2, separator), argument.slice(separator + 1));
}
for (const name of argumentsByName.keys()) {
  if (!['candidate-a', 'candidate-b', 'evidence-file', 'repository-root'].includes(name)) {
    throw new Error(`release reproducibility: unsupported argument: --${name}`);
  }
}
for (const required of ['candidate-a', 'candidate-b', 'evidence-file']) {
  if (!argumentsByName.has(required)) {
    throw new Error(`release reproducibility: --${required} is required`);
  }
}

const evidenceFile = path.resolve(argumentsByName.get('evidence-file'));
if (fs.existsSync(evidenceFile)) {
  throw new Error('release reproducibility: evidence file already exists');
}
const evidenceParent = fs.lstatSync(path.dirname(evidenceFile));
if (evidenceParent.isSymbolicLink() || !evidenceParent.isDirectory()) {
  throw new Error('release reproducibility: evidence parent must be a real directory');
}

const rustc = spawnSync('rustc', ['-vV'], { encoding: 'utf8' });
if (rustc.status !== 0) throw new Error('release reproducibility: rustc target discovery failed');
const runtimeTarget = rustc.stdout.match(/^host: (.+)$/m)?.[1];
if (!runtimeTarget) throw new Error('release reproducibility: rustc did not report a host target');

const repositoryRoot = path.resolve(argumentsByName.get('repository-root') ?? process.cwd());
const left = validateCandidate(path.resolve(argumentsByName.get('candidate-a')), {
  repositoryRoot,
  runtimeTarget,
});
const right = validateCandidate(path.resolve(argumentsByName.get('candidate-b')), {
  repositoryRoot,
  runtimeTarget,
});
const evidence = compareCandidateProducts(left, right);
fs.writeFileSync(evidenceFile, canonicalJson(evidence), { flag: 'wx', mode: 0o600 });
process.stdout.write(
  `Vellum candidate byte equivalence verified: ${evidence.manifestSha256}; reproducible build not claimed\n`,
);
