#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';

import { verifyCandidateBundle } from './release-candidate-bundle-lib.mjs';

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
  if (!['bundle-file', 'repository-root'].includes(name)) throw new Error(`unsupported argument: --${name}`);
}
if (!argumentsByName.has('bundle-file')) throw new Error('--bundle-file is required');

const repositoryRoot = fs.realpathSync(argumentsByName.get('repository-root') ?? process.cwd());
const report = verifyCandidateBundle(path.resolve(argumentsByName.get('bundle-file')), { repositoryRoot });
process.stdout.write(
  `Vellum downloaded candidate bundle verified: ${report.manifestSha256}; ${report.fileCount} files; native smoke passed\n`,
);
