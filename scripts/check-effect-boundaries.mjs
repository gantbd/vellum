import fs from 'node:fs';
import path from 'node:path';

const productionRoots = [
  'engine', 'host', 'service', 'protocol', 'client', 'sdk', 'transport',
  'adapters', 'applications', 'connectors',
  'distribution',
  // The excluded live-provider qualifier is still an authority-bearing
  // composition root and therefore participates in the exact effect ledger.
  'qualification/semantic-lsp-provider',
];

const mechanisms = new Map([
  ['filesystem', /(?:\bstd::fs\b|\buse std::\{[^;]*\bfs\b|\bfs::\{|\brustix::fs\b)/ms],
  ['local-network', /(?:\bstd::net\b|\bnet::\{[^}]*Unix|\bos::unix::net\b)/ms],
  ['process-launch', /(?:\bCommand::new\s*\(|\bstd::process::Command\b)/m],
  ['native-ffi', /(?:\bextern\s+"C"|\blibc::)/m],
]);

// Exact, mechanism-specific trusted sets. Both a new caller and a stale
// declaration fail. This is not a claim that textual scanning closes every
// possible effect mechanism.
const trusted = new Map([
  ['filesystem', new Set([
    'adapters/clipboard-macos/src/integration/qualification.rs',
    'adapters/local-authentication/src/unix.rs',
    'adapters/local-fs/src/append_journal.rs',
    'adapters/local-fs/src/export_copy.rs',
    'adapters/local-fs/src/macos.rs',
    'adapters/local-fs/src/request_journal.rs',
    'adapters/local-fs/src/workspace.rs',
    'adapters/local-fs/src/workspace_identity.rs',
    'applications/vellum-daemon/src/lib.rs',
    'applications/vellum-semantic-local/src/lib.rs',
    'applications/vellum-semantic-local/src/service.rs',
    'distribution/launchd-activate/src/lib.rs',
    'distribution/launchd-render/src/lib.rs',
    'distribution/launchd-render/src/main.rs',
    'qualification/semantic-lsp-provider/src/lib.rs',
    'qualification/semantic-lsp-provider/src/scenario.rs',
    'transport/vellum-semantic-transport/src/unix.rs',
  ])],
  ['local-network', new Set([
    'applications/vellum-daemon/src/lib.rs',
    // The semantic-local composition root clones only its accepted Unix
    // stream so SIGINT/SIGTERM can interrupt a blocked authenticated read.
    'applications/vellum-semantic-local/src/main.rs',
    'applications/vellum-semantic-local/src/service.rs',
    'connectors/vellum-semantic-mcp/src/main.rs',
    'transport/vellum-transport/src/lib.rs',
    'transport/vellum-semantic-transport/src/unix.rs',
  ])],
  ['process-launch', new Set([
    // Test-only FIFO construction lives in the production module's cfg(test)
    // body; the textual gate deliberately classifies the whole source file.
    'adapters/local-authentication/src/unix.rs',
    'applications/vellum-semantic-local/src/lib.rs',
    'distribution/launchd-activate/src/lib.rs',
  ])],
  ['native-ffi', new Set([
    'adapters/local-fs/src/macos.rs',
  ])],
]);

const walk = (directory, output = []) => {
  if (!fs.existsSync(directory)) return output;
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const child = path.join(directory, entry.name);
    if (entry.isDirectory()) walk(child, output);
    else if (
      entry.isFile()
      && child.endsWith('.rs')
      && child.includes(`${path.sep}src${path.sep}`)
      && !child.endsWith(`${path.sep}tests.rs`)
      && !child.includes(`${path.sep}tests${path.sep}`)
    ) {
      output.push(child.split(path.sep).join('/'));
    }
  }
  return output;
};

const classify = (sources) => {
  const actual = new Map([...mechanisms].map(([name]) => [name, new Set()]));
  for (const [source, sourceText] of sources) {
    for (const [name, pattern] of mechanisms) {
      if (pattern.test(sourceText)) actual.get(name).add(source);
    }
  }
  return actual;
};

const compare = (actual, expected = trusted) => {
  const failures = [];
  for (const [name] of mechanisms) {
    const observed = actual.get(name) ?? new Set();
    const declared = expected.get(name) ?? new Set();
    for (const source of observed) {
      if (!declared.has(source)) failures.push(`${name}: undeclared effect caller ${source}`);
    }
    for (const source of declared) {
      if (!observed.has(source)) failures.push(`${name}: stale trusted member ${source}`);
    }
  }
  return failures;
};

const emptyTrust = () => new Map([...mechanisms].map(([name]) => [name, new Set()]));
const selfTest = () => {
  const safe = classify(new Map([['engine/example/src/lib.rs', 'pub fn pure() {}']]));
  if (compare(safe, emptyTrust()).length !== 0) throw new Error('pure fixture was rejected');

  const mutants = new Map([
    ['filesystem', 'use std::fs;'],
    ['local-network', 'use std::os::unix::net::UnixStream;'],
    ['process-launch', 'pub fn escape() { std::process::Command::new("x"); }'],
    ['native-ffi', 'unsafe extern "C" { fn escape(); }'],
  ]);
  for (const [mechanism, sourceText] of mutants) {
    const source = `engine/${mechanism}/src/lib.rs`;
    const escaped = classify(new Map([[source, sourceText]]));
    if (!compare(escaped, emptyTrust())
      .includes(`${mechanism}: undeclared effect caller ${source}`)) {
      throw new Error(`${mechanism} escape mutant survived`);
    }
  }

  const expected = emptyTrust();
  expected.get('filesystem').add('adapters/missing/src/lib.rs');
  if (!compare(safe, expected)
    .includes('filesystem: stale trusted member adapters/missing/src/lib.rs')) {
    throw new Error('stale trusted-member mutant survived');
  }
};

selfTest();
if (process.argv.includes('--self-test')) {
  process.stdout.write('Vellum effect-boundary adversarial self-test passed\n');
  process.exit(0);
}

const files = productionRoots.flatMap((root) => walk(root)).sort();
const sources = new Map(files.map((source) => [source, fs.readFileSync(source, 'utf8')]));
const actual = classify(sources);
const failures = compare(actual);
if (failures.length !== 0) {
  for (const failure of failures) process.stderr.write(`Vellum effect boundary: ${failure}\n`);
  process.exit(1);
}

const counts = [...actual].map(([name, members]) => `${name}=${members.size}`).join(', ');
process.stdout.write(`Vellum declared effect mechanisms closed: ${counts}\n`);
