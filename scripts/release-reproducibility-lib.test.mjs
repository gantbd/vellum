import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { canonicalJson, sha256File } from './release-candidate-lib.mjs';
import { compareCandidateProducts } from './release-reproducibility-lib.mjs';

const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-release-equivalence-'));
const leftRoot = path.join(scratch, 'left');
const rightRoot = path.join(scratch, 'right');
const revision = 'a'.repeat(40);

const makeProduct = (root) => {
  fs.mkdirSync(root);
  fs.writeFileSync(path.join(root, 'payload'), 'same product\n', { mode: 0o755 });
  fs.writeFileSync(path.join(root, 'manifest.json'), '{"fixture":true}\n');
  const manifest = {
    source: {
      vellum: { revision },
      keystone: { revision: 'b'.repeat(40) },
    },
    build: {
      target: 'aarch64-apple-darwin',
      rustc: 'rustc 1.95.0',
      cargo: 'cargo 1.95.0',
      profile: 'release',
      features: 'default',
    },
  };
  return {
    root: fs.realpathSync(root),
    manifest,
    manifestSha256: sha256File(path.join(root, 'manifest.json')),
  };
};

const refuse = (label, action) => {
  try {
    action();
  } catch (error) {
    if (error.message.startsWith('release reproducibility:')) return;
    throw error;
  }
  throw new Error(`${label} was not refused`);
};

try {
  const left = makeProduct(leftRoot);
  const right = makeProduct(rightRoot);
  const evidence = compareCandidateProducts(left, right);
  if (
    evidence.channel !== 'local-release-candidate-byte-equivalence'
    || evidence.source.vellumRevision !== revision
    || evidence.files.map(({ file }) => file).join(',') !== 'manifest.json,payload'
    || !evidence.nonClaims.includes('reproducible-build')
    || canonicalJson(evidence).includes(scratch)
  ) {
    throw new Error('valid comparison omitted bounded content-safe evidence');
  }

  refuse('same-root alias', () => compareCandidateProducts(left, left));

  fs.appendFileSync(path.join(rightRoot, 'payload'), 'drift\n');
  refuse('payload drift', () => compareCandidateProducts(left, right));
  fs.writeFileSync(path.join(rightRoot, 'payload'), 'same product\n', { mode: 0o755 });

  fs.writeFileSync(path.join(rightRoot, 'extra'), 'extra\n');
  refuse('closure drift', () => compareCandidateProducts(left, right));
  fs.unlinkSync(path.join(rightRoot, 'extra'));

  fs.unlinkSync(path.join(rightRoot, 'payload'));
  fs.linkSync(path.join(leftRoot, 'payload'), path.join(rightRoot, 'payload'));
  refuse('hard-link alias', () => compareCandidateProducts(left, right));

  process.stdout.write('Vellum release-candidate byte-equivalence mutants passed\n');
} finally {
  fs.rmSync(scratch, { recursive: true, force: true });
}
