import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = process.cwd();
const argument = process.argv.find((value) => value.startsWith('--output='));
if (argument === undefined || process.argv.length !== 3) {
  throw new Error('usage: node scripts/emit-semantic-mcp-peer-source.mjs --output=PATH');
}
const output = path.resolve(root, argument.slice('--output='.length));
if (fs.existsSync(output)) throw new Error(`output already exists: ${output}`);

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
const requireInside = (parent, candidate, label) => {
  const relative = path.relative(parent, candidate);
  if (relative === '' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    throw new Error(`mapping escapes its root: ${label}`);
  }
};
const requireFile = (candidate, label) => {
  const metadata = fs.lstatSync(candidate);
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw new Error(`source must be a regular non-symlink file: ${label}`);
  }
};

const revision = run('git', ['rev-parse', 'HEAD^{commit}']);
if (!/^[0-9a-f]{40}$/.test(revision)) throw new Error('Vellum HEAD is not a full commit');
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-semantic-mcp-peer-'));
try {
  const archive = path.join(temporary, 'source.tar');
  const sourceRoot = path.join(temporary, 'source');
  fs.mkdirSync(sourceRoot);
  run('git', ['archive', '--format=tar', `--output=${archive}`, revision]);
  run('tar', ['-xf', archive, '-C', sourceRoot]);
  const contract = JSON.parse(fs.readFileSync(
    path.join(sourceRoot, 'release/vellum-semantic-mcp-extraction.json'),
    'utf8',
  ));
  if (
    contract.schema !== 1
    || contract.archiveRoot !== 'vellum-semantic-mcp'
    || contract.publication !== 'none'
  ) throw new Error('unsupported semantic MCP extraction contract');
  const keystone = contract.qualifiedImmutableDependencies?.[0];
  if (
    keystone?.repository !== 'https://github.com/gantbd/keystone'
    || !/^[0-9a-f]{40}$/.test(keystone.revision ?? '')
  ) throw new Error('exact Keystone dependency is absent');

  const staged = path.join(temporary, contract.archiveRoot);
  for (const mapping of contract.copy) {
    const source = path.join(sourceRoot, mapping.from);
    const destination = path.join(staged, mapping.to);
    requireInside(sourceRoot, source, mapping.from);
    requireInside(staged, destination, mapping.to);
    requireFile(source, mapping.from);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.copyFileSync(source, destination);
  }
  const vellumDependency = (name) => `${name} = { version = "=0.1.0", git = "${contract.sourceRepository}", rev = "${revision}" }`;
  const manifest = `[workspace]\n\n[package]\nname = "vellum-semantic-mcp"\nversion = "0.1.0"\nedition = "2024"\nrust-version = "1.95"\nlicense = "MIT OR Apache-2.0"\nrepository = "${contract.targetRepository}"\nreadme = "README.md"\npublish = false\ndescription = "Bounded MCP projection for Vellum semantic queries"\n\n[lib]\nname = "vellum_semantic_mcp"\n\n[[bin]]\nname = "vellum-semantic-mcp"\npath = "src/main.rs"\n\n[dependencies]\ngetrandom = "=0.4.3"\nserde = "=1.0.229"\nserde_json = "=1.0.151"\ntokio = { version = "1", features = ["io-util", "io-std", "rt"] }\n${contract.vellumDependencies.map((dependency) => vellumDependency(dependency.package)).join('\n')}\n\n[dev-dependencies]\nsha2 = "=0.10.9"\n`;
  fs.writeFileSync(path.join(staged, 'Cargo.toml'), manifest);
  fs.copyFileSync(path.join(sourceRoot, 'Cargo.lock'), path.join(staged, 'Cargo.lock'));
  fs.writeFileSync(path.join(staged, '.gitignore'), '/target/\n');
  fs.mkdirSync(path.join(staged, 'release'));
  fs.writeFileSync(path.join(staged, 'release/source.json'), `${JSON.stringify({
    schema: 1,
    source: { repository: contract.sourceRepository, revision },
    immutableDependencies: contract.qualifiedImmutableDependencies,
    install: {
      package: 'vellum-semantic-mcp',
      repository: contract.targetRepository,
      command: `cargo install --locked --git ${contract.targetRepository} --rev RELEASE_COMMIT vellum-semantic-mcp`,
    },
    publication: 'none',
    retainedUpstreamQualification: contract.retainedVellumQualification,
  }, null, 2)}\n`);
  fs.appendFileSync(path.join(staged, 'README.md'), `\n## Exact Git source installation\n\nThis peer is bound to Vellum commit \`${revision}\` and Keystone commit\n\`${keystone.revision}\`. After a reviewed peer commit exists:\n\n\`\`\`sh\ncargo install --locked --git ${contract.targetRepository} --rev RELEASE_COMMIT vellum-semantic-mcp\n\`\`\`\n\nNo floating branch, crates.io package, remote network service or provider sandbox\nis implied by this source contract.\n`);
  fs.appendFileSync(path.join(staged, 'README.md'), `\nThe deterministic \`release/payload.json\` closes the complete repository file\nset, byte lengths and SHA-256 digests. Its contract test rejects added, removed,\nchanged or symlinked payload entries while excluding only the manifest itself.\n`);
  fs.writeFileSync(path.join(staged, 'CONTRIBUTING.md'), '# Contributing\n\nUse Rust 1.95. Run `cargo fmt --check`, `cargo test --locked --all-targets`, and `cargo clippy --locked --all-targets -- -D warnings`. Keep engine, editor, provider and policy authority outside this peer. Exact Vellum/Keystone revisions, the lockfile and `release/source.json` must change together.\n');
  const sourceContract = `use std::{fs, path::Path};\n\nfn read(root: &Path, relative: &str) -> String { fs::read_to_string(root.join(relative)).unwrap() }\nfn full_hex(value: &str) -> bool { value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) }\n\n#[test]\nfn manifest_lock_and_receipt_bind_exact_sources() {\n    let root = Path::new(env!("CARGO_MANIFEST_DIR"));\n    let receipt: serde_json::Value = serde_json::from_str(&read(root, "release/source.json")).unwrap();\n    assert_eq!(receipt["schema"], 1);\n    assert_eq!(receipt["publication"], "none");\n    let vellum = receipt["source"]["revision"].as_str().unwrap();\n    assert!(full_hex(vellum));\n    let keystone = receipt["immutableDependencies"][0]["revision"].as_str().unwrap();\n    assert!(full_hex(keystone));\n    let manifest = read(root, "Cargo.toml");\n    assert_eq!(manifest.matches(&format!("rev = \\\"{vellum}\\\"")).count(), 3);\n    assert_eq!(manifest.matches("git = \\\"https://github.com/gantbd/vellum\\\"").count(), 3);\n    assert!(manifest.contains("publish = false"));\n    let lock = read(root, "Cargo.lock");\n    assert!(lock.contains(&format!("git+https://github.com/gantbd/vellum?rev={vellum}#{vellum}")));\n    assert!(lock.contains(&format!("git+https://github.com/gantbd/keystone?rev={keystone}#{keystone}")));\n}\n`;
  fs.writeFileSync(path.join(staged, 'tests/source_contract.rs'), sourceContract);
  const payloadContract = `use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn collect(root: &Path, directory: &Path, files: &mut Vec<String>) {
    for entry in fs::read_dir(directory).expect("payload directory must be readable") {
        let entry = entry.expect("payload entry must be readable");
        let path = entry.path();
        let relative = path.strip_prefix(root).expect("payload path must remain below root");
        if relative == Path::new(".git") || relative == Path::new("target") {
            continue;
        }
        let metadata = fs::symlink_metadata(&path).expect("payload metadata must be readable");
        assert!(!metadata.file_type().is_symlink(), "payload must contain no symlinks");
        if metadata.is_dir() {
            collect(root, &path, files);
        } else if relative != Path::new("release/payload.json") {
            files.push(relative.to_string_lossy().replace('\\\\', "/"));
        }
    }
}

#[test]
fn payload_manifest_closes_the_repository_tree() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let receipt: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("release/source.json")).expect("source receipt must exist"),
    ).expect("source receipt must be valid JSON");
    let payload: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("release/payload.json")).expect("payload manifest must exist"),
    ).expect("payload manifest must be valid JSON");
    assert_eq!(payload["schema"], 1);
    assert_eq!(payload["sourceRevision"], receipt["source"]["revision"]);
    let entries = payload["files"].as_array().expect("payload files must be an array");
    let declared: Vec<_> = entries.iter().map(|entry| {
        let relative = entry["path"].as_str().expect("payload path must be text");
        let expected_bytes = entry["bytes"].as_u64().expect("payload bytes must be u64");
        let expected_hash = entry["sha256"].as_str().expect("payload hash must be text");
        assert!(expected_hash.len() == 64 && expected_hash.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
        let bytes = fs::read(root.join(relative)).expect("declared payload file must exist");
        assert_eq!(u64::try_from(bytes.len()).unwrap(), expected_bytes);
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), expected_hash);
        relative.to_owned()
    }).collect();
    assert!(declared.windows(2).all(|pair| pair[0] < pair[1]), "payload paths must be canonical");
    let mut observed = Vec::new();
    collect(root, root, &mut observed);
    observed.sort();
    assert_eq!(observed, declared, "payload file closure drifted");
}
`;
  fs.writeFileSync(path.join(staged, 'tests/payload_contract.rs'), payloadContract);
  const workflow = `name: Rust CI\n\non:\n  pull_request:\n  push:\n    branches: [main]\n\npermissions:\n  contents: read\n\njobs:\n  locked-source:\n    runs-on: ubuntu-latest\n    timeout-minutes: 30\n    steps:\n      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683\n        with:\n          persist-credentials: false\n      - run: cargo fmt --check\n      - run: cargo test --locked --all-targets\n      - run: cargo clippy --locked --all-targets -- -D warnings\n`;
  fs.mkdirSync(path.join(staged, '.github/workflows'), { recursive: true });
  fs.writeFileSync(path.join(staged, '.github/workflows/ci.yml'), workflow);

  const keystoneRoot = path.resolve(root, '../keystone');
  if (!fs.existsSync(path.join(keystoneRoot, '.git'))) {
    throw new Error(`local Keystone checkout required for qualification: ${keystoneRoot}`);
  }
  const gitConfig = path.join(temporary, 'gitconfig');
  fs.writeFileSync(gitConfig, `[url "${pathToFileURL(root).href}"]\n\tinsteadOf = ${contract.sourceRepository}\n[url "${pathToFileURL(keystoneRoot).href}"]\n\tinsteadOf = ${keystone.repository}\n`);
  const env = {
    ...process.env,
    GIT_CONFIG_GLOBAL: gitConfig,
    GIT_CONFIG_NOSYSTEM: '1',
    CARGO_NET_GIT_FETCH_WITH_CLI: 'true',
  };
  run('cargo', ['generate-lockfile'], { cwd: staged, env });
  run('cargo', ['fmt'], { cwd: staged, env });
  const payloadFiles = [];
  const walkPayload = (directory) => {
    for (const name of fs.readdirSync(directory).sort()) {
      const file = path.join(directory, name);
      const relative = path.relative(staged, file).split(path.sep).join('/');
      if (relative === 'release/payload.json') continue;
      const metadata = fs.lstatSync(file);
      if (metadata.isSymbolicLink()) throw new Error(`peer payload contains a symlink: ${relative}`);
      if (metadata.isDirectory()) {
        walkPayload(file);
      } else if (metadata.isFile()) {
        const bytes = fs.readFileSync(file);
        payloadFiles.push({
          path: relative,
          bytes: bytes.length,
          sha256: crypto.createHash('sha256').update(bytes).digest('hex'),
        });
      } else {
        throw new Error(`peer payload contains a non-regular entry: ${relative}`);
      }
    }
  };
  walkPayload(staged);
  payloadFiles.sort((left, right) => (left.path < right.path ? -1 : left.path > right.path ? 1 : 0));
  fs.writeFileSync(path.join(staged, 'release/payload.json'), `${JSON.stringify({
    schema: 1,
    sourceRevision: revision,
    files: payloadFiles,
  }, null, 2)}\n`);
  const target = path.join(temporary, 'target');
  run('cargo', ['test', '--locked', '--all-targets', '--target-dir', target], { cwd: staged, env });
  run('cargo', ['clippy', '--locked', '--all-targets', '--target-dir', target, '--', '-D', 'warnings'], { cwd: staged, env });

  const repository = path.join(temporary, 'repository');
  fs.cpSync(staged, repository, { recursive: true });
  run('git', ['init', '--quiet'], { cwd: repository });
  run('git', ['config', 'user.name', 'Vellum qualification'], { cwd: repository });
  run('git', ['config', 'user.email', 'qualification@invalid'], { cwd: repository });
  run('git', ['add', '.'], { cwd: repository });
  run('git', ['commit', '--quiet', '-m', 'qualification source'], { cwd: repository });
  const peerRevision = run('git', ['rev-parse', 'HEAD'], { cwd: repository });
  fs.appendFileSync(gitConfig, `[url "${pathToFileURL(repository).href}"]\n\tinsteadOf = ${contract.targetRepository}\n`);
  const install = path.join(temporary, 'install');
  run('cargo', [
    'install', '--locked', '--git', contract.targetRepository, '--rev', peerRevision,
    'vellum-semantic-mcp', '--root', install,
    '--target-dir', path.join(temporary, 'install-target'),
  ], { cwd: temporary, env });
  const help = run(path.join(install, 'bin/vellum-semantic-mcp'), ['--help']);
  if (!help.includes('vellum-semantic-mcp --semantic-socket')) {
    throw new Error('Git-installed semantic MCP binary help smoke failed');
  }

  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.cpSync(staged, output, { recursive: true });
  process.stdout.write(`emitted ${output}\nVellum ${revision}\nKeystone ${keystone.revision}\n`);
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
