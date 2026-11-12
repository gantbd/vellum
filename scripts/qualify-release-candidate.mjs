#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import {
  canonicalJson,
  smokeCandidate,
  smokeLaunchdDeployment,
  validateCandidate,
} from './release-candidate-lib.mjs';

process.on('uncaughtException', (error) => {
  process.stderr.write(`Vellum artifact lifecycle: ${error.message}\n`);
  process.exit(1);
});

const argumentsByName = new Map();
for (const argument of process.argv.slice(2)) {
  const separator = argument.indexOf('=');
  if (separator < 3 || !argument.startsWith('--')) throw new Error(`unsupported argument: ${argument}`);
  argumentsByName.set(argument.slice(2, separator), argument.slice(separator + 1));
}
for (const name of argumentsByName.keys()) {
  if (!['candidate-dir', 'prior-candidate-dir', 'evidence-file', 'repository-root'].includes(name)) {
    throw new Error(`unsupported argument: --${name}`);
  }
}
for (const required of ['candidate-dir', 'prior-candidate-dir', 'evidence-file']) {
  if (!argumentsByName.has(required)) throw new Error(`--${required} is required`);
}

const repositoryRoot = path.resolve(argumentsByName.get('repository-root') ?? process.cwd());
const evidenceFile = path.resolve(argumentsByName.get('evidence-file'));
if (fs.existsSync(evidenceFile)) throw new Error('evidence file already exists');
const evidenceParent = path.dirname(evidenceFile);
const parent = fs.lstatSync(evidenceParent);
if (parent.isSymbolicLink() || !parent.isDirectory()) throw new Error('evidence parent must be a real directory');

const rustc = spawnSync('rustc', ['-vV'], { encoding: 'utf8' });
if (rustc.status !== 0) throw new Error('rustc target discovery failed');
const runtimeTarget = rustc.stdout.match(/^host: (.+)$/m)?.[1];
if (!runtimeTarget) throw new Error('rustc did not report a host target');

const candidate = validateCandidate(path.resolve(argumentsByName.get('candidate-dir')), {
  repositoryRoot,
  runtimeTarget,
});
const prior = validateCandidate(path.resolve(argumentsByName.get('prior-candidate-dir')), {
  runtimeTarget,
});
if (candidate.manifestSha256 === prior.manifestSha256) {
  throw new Error('prior candidate must be a distinct immutable version');
}
if (candidate.manifest.source.vellum.revision === prior.manifest.source.vellum.revision) {
  throw new Error('prior candidate must identify a different Vellum revision');
}

const candidateSmoke = smokeCandidate(candidate);
const priorSmoke = smokeCandidate(prior);
const candidateLaunchdSmoke = smokeLaunchdDeployment(candidate);
const priorLaunchdSmoke = smokeLaunchdDeployment(prior);
const evidence = {
  schema: 1,
  channel: 'local-release-candidate-qualification',
  state: 'ready-for-manual-activation-review',
  activationAuthorized: false,
  claims: [
    'candidate-integrity-verified',
    'candidate-policy-compatible',
    'candidate-help-smoke-passed',
    'candidate-launchd-render-smoke-passed',
    'prior-integrity-verified',
    'prior-help-smoke-passed',
    'prior-launchd-render-smoke-passed',
    'rollback-version-selected',
  ],
  nonClaims: ['activation', 'installation', 'signature', 'attestation', 'github-release'],
  candidate: {
    manifestSha256: candidate.manifestSha256,
    vellumRevision: candidate.manifest.source.vellum.revision,
    artifacts: candidate.manifest.artifacts.map(({ file, sha256 }) => ({ file, sha256 })),
    deployment: candidate.manifest.deployment.map(({ file, sha256 }) => ({ file, sha256 })),
    smoke: candidateSmoke,
    launchdSmoke: candidateLaunchdSmoke,
  },
  rollback: {
    selectedPriorManifestSha256: prior.manifestSha256,
    selectedPriorVellumRevision: prior.manifest.source.vellum.revision,
    artifacts: prior.manifest.artifacts.map(({ file, sha256 }) => ({ file, sha256 })),
    deployment: prior.manifest.deployment.map(({ file, sha256 }) => ({ file, sha256 })),
    smoke: priorSmoke,
    launchdSmoke: priorLaunchdSmoke,
    trigger: 'post-activation readiness or health gate fails',
  },
  activationPlan: [
    'independently approve the exact candidate manifest identity',
    'quiesce the managed service and preserve its current installation identity',
    'install artifacts only through a separately authorized atomic activation mechanism',
    'require post-activation readiness and health evidence before declaring success',
    'on failure, atomically restore the selected prior artifacts and re-run readiness checks',
  ],
};
fs.writeFileSync(evidenceFile, canonicalJson(evidence), { flag: 'wx', mode: 0o600 });
process.stdout.write(
  `Vellum candidate qualified for manual activation review; activation not performed; rollback ${prior.manifestSha256}\n`,
);
