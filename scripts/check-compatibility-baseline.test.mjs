#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  symlinkSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const repository = resolve(import.meta.dirname, "..");
const checker = join(repository, "scripts/check-compatibility-baseline.mjs");

function run(root, ...args) {
  return spawnSync(process.execPath, [checker, `--root=${root}`, ...args], { encoding: "utf8" });
}

function copyInputs(root) {
  const rootManifest = readFileSync(join(repository, "Cargo.toml"), "utf8");
  const members = [...rootManifest.matchAll(/"([^"\n]+)"/g)]
    .map((match) => match[1])
    .filter((path) => existsSync(join(repository, path, "Cargo.toml")));
  mkdirSync(root, { recursive: true });
  cpSync(join(repository, "Cargo.toml"), join(root, "Cargo.toml"));
  mkdirSync(join(root, "release"), { recursive: true });
  cpSync(
    join(repository, "release/compatibility-baseline.json"),
    join(root, "release/compatibility-baseline.json"),
  );
  for (const member of members) {
    mkdirSync(join(root, member), { recursive: true });
    cpSync(join(repository, member, "Cargo.toml"), join(root, member, "Cargo.toml"));
    const source = join(repository, member, "src");
    if (existsSync(source)) cpSync(source, join(root, member, "src"), { recursive: true });
  }
  mkdirSync(join(root, "protocol/vellum-wire/tests"), { recursive: true });
  cpSync(
    join(repository, "protocol/vellum-wire/tests/fixtures"),
    join(root, "protocol/vellum-wire/tests/fixtures"),
    { recursive: true },
  );
  mkdirSync(join(root, "protocol/vellum-semantic-wire/tests"), { recursive: true });
  cpSync(
    join(repository, "protocol/vellum-semantic-wire/tests/fixtures"),
    join(root, "protocol/vellum-semantic-wire/tests/fixtures"),
    { recursive: true },
  );
}

const mutantRoot = mkdtempSync(join(tmpdir(), "vellum-compatibility-mutants-"));
copyInputs(mutantRoot);

function assertRejected(label, mutate) {
  const restore = mutate(mutantRoot);
  try {
    const result = run(mutantRoot);
    if (result.status === 0) throw new Error(`${label} mutant was accepted`);
  } finally {
    restore();
  }
}

const clean = run(repository);
if (clean.status !== 0) throw new Error(`clean baseline failed:\n${clean.stderr}`);

assertRejected("Rust contract source", (root) => {
  const path = join(root, "sdk/client/src/lib.rs");
  const current = readFileSync(path, "utf8");
  writeFileSync(path, `${current}\n// compatibility mutant\n`);
  return () => writeFileSync(path, current);
});

assertRejected("wire fixture", (root) => {
  const path = join(root, "protocol/vellum-wire/tests/fixtures/connect-request-v1.hex");
  const current = readFileSync(path, "utf8");
  writeFileSync(path, `${current.trim()}00\n`);
  return () => writeFileSync(path, current);
});

assertRejected("semantic wire fixture", (root) => {
  const path = join(
    root,
    "protocol/vellum-semantic-wire/tests/fixtures/describe-command-v1.hex",
  );
  const current = readFileSync(path, "utf8");
  writeFileSync(path, `${current.trim()} 00\n`);
  return () => writeFileSync(path, current);
});

assertRejected("package metadata", (root) => {
  const path = join(root, "sdk/client/Cargo.toml");
  const current = readFileSync(path, "utf8");
  writeFileSync(path, current.replace("publish = false", "publish = true"));
  return () => writeFileSync(path, current);
});

assertRejected("new workspace member", (root) => {
  const path = join(root, "Cargo.toml");
  const current = readFileSync(path, "utf8");
  writeFileSync(path, current.replace('members = [', 'members = [\n    "sdk/client",'));
  return () => writeFileSync(path, current);
});

assertRejected("symbolic-link source", (root) => {
  const path = join(root, "sdk/client/src/lib.rs");
  const content = readFileSync(path, "utf8");
  const replacement = join(root, "replacement.rs");
  const link = `${path}.link`;
  writeFileSync(replacement, content);
  symlinkSync(replacement, link);
  return () => {
    unlinkSync(link);
    unlinkSync(replacement);
  };
});

assertRejected("malformed baseline", (root) => {
  const path = join(root, "release/compatibility-baseline.json");
  const current = readFileSync(path, "utf8");
  writeFileSync(path, '{"schemaVersion":1}\n');
  return () => writeFileSync(path, current);
});

process.stdout.write("compatibility baseline mutants: 7/7 rejected\n");
