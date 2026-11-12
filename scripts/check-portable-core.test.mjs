import assert from 'node:assert/strict';
import fs from 'node:fs';
import { validatePolicy, validateWorkflow } from './check-portable-core.mjs';

const policy = JSON.parse(fs.readFileSync('release/portable-core.json', 'utf8'));
const workflow = fs.readFileSync('.github/workflows/portable-core.yml', 'utf8');
const packages = [...policy.packages];

assert.deepEqual(validatePolicy(policy, packages), []);
assert.deepEqual(validateWorkflow(policy, workflow), []);

const mutants = [
  () => validatePolicy({ ...policy, schemaVersion: 2 }, packages),
  () => validatePolicy({ ...policy, claim: 'cross-platform-runtime-qualified' }, packages),
  () => validatePolicy({ ...policy, packages: packages.slice(1) }, packages),
  () => validatePolicy({ ...policy, runnerMatrix: policy.runnerMatrix.slice(1) }, packages),
  () => validateWorkflow(policy, workflow.replace('windows-2022', 'ubuntu-24.04')),
  () => validateWorkflow(policy, `${workflow}\n# cargo publish\n`),
  () => validateWorkflow(policy, workflow.replace('cargo check --locked --lib', 'cargo check --locked --all-targets')),
];
for (const [index, mutate] of mutants.entries()) {
  assert.notDeepEqual(mutate(), [], `mutant ${index + 1} must be rejected`);
}
console.log(`Portable core contract mutants: ${mutants.length}/${mutants.length} rejected`);
