#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';

import { buildCandidateBundle, verifyCandidateBundle } from './release-candidate-bundle-lib.mjs';

process.on('uncaughtException', (error) => {
  process.stderr.write(`${error.message}\n`);
  process.exit(1);
});

const argumentsByName = new Map();
for (const argument of process.argv.slice(2)) {
  const separator = argument.indexOf('=');
  if (separator < 3 || !argument.startsWith('--')) throw new Error(`unsupported argument: ${argument}`);
  argumentsByName.set(argument.slice(2, separator), argument.slice(separator + 1));
}
for (const name of argumentsByName.keys()) {
  if (!['qualified-dir', 'bundle-file', 'repository-root'].includes(name)) {
    throw new Error(`unsupported argument: --${name}`);
  }
}
for (const required of ['qualified-dir', 'bundle-file']) {
  if (!argumentsByName.has(required)) throw new Error(`--${required} is required`);
}

const repositoryRoot = fs.realpathSync(argumentsByName.get('repository-root') ?? process.cwd());
const destination = path.resolve(argumentsByName.get('bundle-file'));
if (fs.existsSync(destination)) throw new Error('bundle destination already exists');
const parent = fs.lstatSync(path.dirname(destination));
if (parent.isSymbolicLink() || !parent.isDirectory()) throw new Error('bundle parent must be a real directory');

const bytes = buildCandidateBundle(path.resolve(argumentsByName.get('qualified-dir')), { repositoryRoot });
fs.writeFileSync(destination, bytes, { flag: 'wx', mode: 0o600 });
try {
  const report = verifyCandidateBundle(destination, { repositoryRoot });
  process.stdout.write(
    `Vellum release candidate bundle qualified: ${report.manifestSha256}; upload transport may flatten only the outer mode\n`,
  );
} catch (error) {
  fs.rmSync(destination, { force: true });
  throw error;
}
