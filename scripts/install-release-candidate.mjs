#!/usr/bin/env node

import fs from 'node:fs';

import { installCandidateBundle } from './release-candidate-install-lib.mjs';

process.on('uncaughtException', (error) => {
  process.stderr.write(`${error.message}\n`);
  process.exit(1);
});

const argumentsByName = new Map();
for (const argument of process.argv.slice(2)) {
  const separator = argument.indexOf('=');
  if (separator < 3 || !argument.startsWith('--')) throw new Error(`unsupported argument: ${argument}`);
  const name = argument.slice(2, separator);
  if (argumentsByName.has(name)) throw new Error(`duplicate argument: --${name}`);
  argumentsByName.set(name, argument.slice(separator + 1));
}
for (const name of argumentsByName.keys()) {
  if (!['bundle-file', 'install-root'].includes(name)) throw new Error(`unsupported argument: --${name}`);
}
for (const required of ['bundle-file', 'install-root']) {
  if (!argumentsByName.has(required)) throw new Error(`--${required} is required`);
}

const report = installCandidateBundle(
  argumentsByName.get('bundle-file'),
  argumentsByName.get('install-root'),
  { repositoryRoot: fs.realpathSync(process.cwd()) },
);
process.stdout.write(
  `Vellum candidate ${report.status}: ${report.manifestSha256}; ${report.productCount} products; activation not performed\n`,
);
