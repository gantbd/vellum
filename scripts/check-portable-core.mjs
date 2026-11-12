import fs from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const expectedRoots = ['/engine/', '/host/', '/protocol/', '/service/', '/client/', '/sdk/'];
const expectedRunners = ['macos-15', 'ubuntu-24.04', 'windows-2022'];
const canonical = (values) => [...values].sort((left, right) => left.localeCompare(right));

export function validatePolicy(policy, discoveredPackages) {
  const failures = [];
  if (policy.schemaVersion !== 1) failures.push('schemaVersion must be 1');
  if (policy.claim !== 'host-library-compilation-only') {
    failures.push('claim must remain host-library-compilation-only');
  }
  if (JSON.stringify(policy.packages) !== JSON.stringify(canonical(policy.packages ?? []))) {
    failures.push('packages must be sorted and unique');
  }
  if (JSON.stringify(policy.runnerMatrix) !== JSON.stringify(expectedRunners)) {
    failures.push('runner matrix must be the exact macOS/Linux/Windows set');
  }
  if (JSON.stringify(policy.packages) !== JSON.stringify(canonical(discoveredPackages))) {
    failures.push('package inventory differs from the complete portable categorical closure');
  }
  return failures;
}

export function validateWorkflow(policy, workflow) {
  const failures = [];
  for (const runner of expectedRunners) {
    if (!workflow.includes(runner)) failures.push(`workflow is missing runner ${runner}`);
  }
  for (const packageName of policy.packages ?? []) {
    if (!workflow.includes(`-p ${packageName}`)) {
      failures.push(`workflow is missing portable package ${packageName}`);
    }
  }
  for (const required of [
    'permissions:\n  contents: read',
    'cargo check --locked --lib',
    'persist-credentials: false',
  ]) {
    if (!workflow.includes(required)) failures.push(`workflow is missing ${required}`);
  }
  for (const forbidden of [
    'cargo publish',
    'cargo install',
    'actions/upload-artifact',
    'gh release',
    'contents: write',
    '--all-targets',
  ]) {
    if (workflow.includes(forbidden)) failures.push(`workflow contains forbidden effect ${forbidden}`);
  }
  return failures;
}

function main() {
  const policy = JSON.parse(fs.readFileSync('release/portable-core.json', 'utf8'));
  const cargo = spawnSync(
    'cargo',
    ['metadata', '--offline', '--locked', '--no-deps', '--format-version', '1'],
    { encoding: 'utf8' },
  );
  if (cargo.status !== 0) {
    process.stderr.write(cargo.stderr);
    process.exit(cargo.status ?? 1);
  }
  const metadata = JSON.parse(cargo.stdout);
  const portable = metadata.packages.filter((pkg) =>
    expectedRoots.some((root) => pkg.manifest_path.includes(root)),
  );
  const failures = validatePolicy(policy, portable.map((pkg) => pkg.name));
  for (const pkg of portable) {
    if (pkg.publish?.length !== 0) failures.push(`${pkg.name} must remain non-publishable`);
    if (!pkg.targets.some((target) => target.kind.includes('lib'))) {
      failures.push(`${pkg.name} has no library target`);
    }
  }
  const workflow = fs.readFileSync('.github/workflows/portable-core.yml', 'utf8');
  failures.push(...validateWorkflow(policy, workflow));
  if (failures.length !== 0) {
    for (const failure of failures) console.error(`portable core: ${failure}`);
    process.exit(1);
  }
  console.log(`Portable core contract closed over ${portable.length} libraries and 3 host OS runners`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
