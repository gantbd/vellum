#!/usr/bin/env node

import { createHash } from "node:crypto";
import { lstatSync, readFileSync, readdirSync, realpathSync, writeFileSync } from "node:fs";
import { dirname, join, posix, relative, resolve, sep } from "node:path";

const DEFAULT_BASELINE = "release/compatibility-baseline.json";
const ROOT_MANIFEST = "Cargo.toml";

function fail(message) {
  throw new Error(`compatibility baseline: ${message}`);
}

function normalizedPath(path) {
  return path.split(sep).join(posix.sep);
}

function resolveRegularFile(root, repositoryPath) {
  if (typeof repositoryPath !== "string" || repositoryPath.length === 0) {
    fail("an empty or non-string repository path was supplied");
  }
  if (repositoryPath.startsWith("/") || repositoryPath.includes("\\")) {
    fail(`non-canonical repository path ${JSON.stringify(repositoryPath)}`);
  }
  const normalized = posix.normalize(repositoryPath);
  if (normalized !== repositoryPath || normalized === ".." || normalized.startsWith("../")) {
    fail(`repository path escapes or is not canonical: ${repositoryPath}`);
  }
  const absolute = resolve(root, ...repositoryPath.split("/"));
  const rootReal = realpathSync(root);
  const parentReal = realpathSync(dirname(absolute));
  if (parentReal !== rootReal && !parentReal.startsWith(`${rootReal}${sep}`)) {
    fail(`repository path escapes through a link: ${repositoryPath}`);
  }
  const stat = lstatSync(absolute);
  if (stat.isSymbolicLink() || !stat.isFile()) {
    fail(`expected a regular, non-symbolic-link file: ${repositoryPath}`);
  }
  return absolute;
}

function readText(root, path) {
  return readFileSync(resolveRegularFile(root, path), "utf8").replaceAll("\r\n", "\n");
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function stringsInArray(source, key) {
  const match = source.match(new RegExp(`^${key}\\s*=\\s*\\[([\\s\\S]*?)\\]`, "m"));
  if (!match) fail(`missing ${key} array`);
  return [...match[1].matchAll(/"([^"\\]*(?:\\.[^"\\]*)*)"/g)].map((item) =>
    JSON.parse(`"${item[1]}"`),
  );
}

function section(source, heading) {
  const escaped = heading.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = source.match(new RegExp(`^\\[${escaped}\\]\\s*$([\\s\\S]*?)(?=^\\[|(?![\\s\\S]))`, "m"));
  return match?.[1] ?? "";
}

function scalar(source, key) {
  const escaped = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = source.match(new RegExp(`^${escaped}\\s*=\\s*("(?:[^"\\\\]|\\\\.)*"|true|false)\\s*$`, "m"));
  if (!match) return undefined;
  return match[1].startsWith('"') ? JSON.parse(match[1]) : match[1] === "true";
}

function inheritedScalar(packageSection, workspaceSection, key) {
  const direct = scalar(packageSection, key);
  if (direct !== undefined) return direct;
  if (new RegExp(`^${key.replaceAll("-", "\\-")}\\.workspace\\s*=\\s*true\\s*$`, "m").test(packageSection)) {
    const inherited = scalar(workspaceSection, key);
    if (inherited === undefined) fail(`workspace package does not define inherited ${key}`);
    return inherited;
  }
  fail(`package does not define ${key}`);
}

function featureNames(manifest) {
  return [...section(manifest, "features").matchAll(/^([A-Za-z0-9_-]+)\s*=/gm)]
    .map((match) => match[1])
    .sort();
}

function explicitTargets(manifest, packageName, manifestDirectory, root) {
  const targets = [];
  const libSection = section(manifest, "lib");
  const defaultLib = normalizedPath(join(manifestDirectory, "src/lib.rs"));
  try {
    resolveRegularFile(root, defaultLib);
    targets.push({
      kind: "lib",
      name: scalar(libSection, "name") ?? packageName.replaceAll("-", "_"),
      path: scalar(libSection, "path")
        ? normalizedPath(join(manifestDirectory, scalar(libSection, "path")))
        : defaultLib,
    });
  } catch (error) {
    if (!String(error).includes("ENOENT")) throw error;
  }
  for (const match of manifest.matchAll(/^\[\[bin\]\]\s*$([\s\S]*?)(?=^\[|(?![\s\S]))/gm)) {
    const name = scalar(match[1], "name");
    const localPath = scalar(match[1], "path") ?? "src/main.rs";
    if (!name) fail(`${manifestDirectory}/Cargo.toml has a bin without a name`);
    const path = normalizedPath(join(manifestDirectory, localPath));
    resolveRegularFile(root, path);
    targets.push({ kind: "bin", name, path });
  }
  const defaultBin = normalizedPath(join(manifestDirectory, "src/main.rs"));
  if (!targets.some((target) => target.kind === "bin")) {
    try {
      resolveRegularFile(root, defaultBin);
      targets.push({ kind: "bin", name: packageName, path: defaultBin });
    } catch (error) {
      if (!String(error).includes("ENOENT")) throw error;
    }
  }
  return targets.sort((left, right) => `${left.kind}:${left.name}`.localeCompare(`${right.kind}:${right.name}`));
}

function walkRustFiles(root, directory, output = []) {
  const absolute = resolve(root, ...directory.split("/"));
  const rootReal = realpathSync(root);
  const directoryReal = realpathSync(absolute);
  if (directoryReal !== rootReal && !directoryReal.startsWith(`${rootReal}${sep}`)) {
    fail(`source directory escapes through a link: ${directory}`);
  }
  for (const entry of readdirSync(absolute, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    const path = normalizedPath(join(directory, entry.name));
    if (entry.isSymbolicLink()) fail(`symbolic links are forbidden in compatibility inputs: ${path}`);
    if (entry.isDirectory()) walkRustFiles(root, path, output);
    else if (entry.isFile() && entry.name.endsWith(".rs")) output.push(path);
    else if (!entry.isFile()) fail(`non-regular compatibility input: ${path}`);
  }
  return output;
}

function sourceTree(root, manifestDirectory, targets) {
  const library = targets.find((target) => target.kind === "lib");
  if (!library) return null;
  const directory = normalizedPath(join(manifestDirectory, "src"));
  if (library.path !== `${directory}/lib.rs` && !library.path.startsWith(`${directory}/`)) {
    fail(`library target is outside its conservatively hashed source tree: ${library.path}`);
  }
  const files = walkRustFiles(root, directory).sort();
  const digestInput = files.map((path) => `${path}\0${sha256(readFileSync(resolveRegularFile(root, path)))}\0`).join("");
  return { directory, fileCount: files.length, sha256: sha256(digestInput) };
}

function workspaceSnapshot(root) {
  const rootManifest = readText(root, ROOT_MANIFEST);
  const workspacePackage = section(rootManifest, "workspace.package");
  const members = stringsInArray(section(rootManifest, "workspace"), "members").sort();
  const packages = members.map((directory) => {
    const manifestPath = `${directory}/Cargo.toml`;
    const manifest = readText(root, manifestPath);
    const packageSection = section(manifest, "package");
    const name = scalar(packageSection, "name");
    if (!name) fail(`${manifestPath} has no package name`);
    const targets = explicitTargets(manifest, name, directory, root);
    return {
      manifest: manifestPath,
      name,
      version: inheritedScalar(packageSection, workspacePackage, "version"),
      edition: inheritedScalar(packageSection, workspacePackage, "edition"),
      rustVersion: inheritedScalar(packageSection, workspacePackage, "rust-version"),
      license: inheritedScalar(packageSection, workspacePackage, "license"),
      repository: inheritedScalar(packageSection, workspacePackage, "repository"),
      publish: inheritedScalar(packageSection, workspacePackage, "publish"),
      features: featureNames(manifest),
      featureTableSha256: sha256(section(manifest, "features")),
      targets,
      sourceTree: sourceTree(root, directory, targets),
    };
  }).sort((left, right) => left.name.localeCompare(right.name));
  return {
    workspace: {
      members,
      version: scalar(workspacePackage, "version"),
      edition: scalar(workspacePackage, "edition"),
      rustVersion: scalar(workspacePackage, "rust-version"),
      license: scalar(workspacePackage, "license"),
      repository: scalar(workspacePackage, "repository"),
      publish: scalar(workspacePackage, "publish"),
    },
    packages,
  };
}

function wireSnapshot(root, directory, label) {
  const absolute = resolve(root, ...directory.split("/"));
  const fixtures = readdirSync(absolute, { withFileTypes: true })
    .sort((a, b) => a.name.localeCompare(b.name))
    .map((entry) => {
      const path = `${directory}/${entry.name}`;
      if (!entry.isFile() || entry.isSymbolicLink() || !entry.name.endsWith(".hex")) {
        fail(`wire fixture directory contains a non-.hex regular file: ${path}`);
      }
      const text = readText(root, path);
      if (!/^[0-9a-f]{2}(?: [0-9a-f]{2})*(?:\n[0-9a-f]{2}(?: [0-9a-f]{2})*)*\n?$/.test(text)) {
        fail(`wire fixture is not canonical lowercase space-separated hex: ${path}`);
      }
      return { path, byteLength: text.trim().split(/\s+/).length, sha256: sha256(text) };
    });
  if (fixtures.length === 0) fail(`${label} wire fixture inventory is empty`);
  return { directory, fixtures };
}

function snapshot(root) {
  return {
    schemaVersion: 2,
    policy: {
      mode: "exact-conservative-change-detection",
      rustSemverAnalysis: false,
      rustSourceEvidence: "reviewed-change-tripwire-not-public-api",
      wireCompatibility: "exact-frozen-fixtures",
    },
    ...workspaceSnapshot(root),
    wire: wireSnapshot(root, "protocol/vellum-wire/tests/fixtures", "editing"),
    semanticWire: wireSnapshot(
      root,
      "protocol/vellum-semantic-wire/tests/fixtures",
      "semantic session",
    ),
  };
}

function canonicalJson(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function main() {
  let root = process.cwd();
  let baselinePath = DEFAULT_BASELINE;
  let emit = false;
  let update = false;
  for (const argument of process.argv.slice(2)) {
    if (argument === "--emit=-") emit = true;
    else if (argument === "--update-baseline") update = true;
    else if (argument.startsWith("--root=")) root = resolve(argument.slice("--root=".length));
    else if (argument.startsWith("--baseline=")) baselinePath = argument.slice("--baseline=".length);
    else fail(`unknown argument ${argument}`);
  }
  root = realpathSync(root);
  const actual = snapshot(root);
  if (emit) {
    process.stdout.write(canonicalJson(actual));
    return;
  }
  if (update) {
    const target = resolve(root, baselinePath);
    if (!target.startsWith(`${root}${sep}`)) {
      fail("baseline update target must remain inside the repository");
    }
    writeFileSync(target, canonicalJson(actual));
    process.stdout.write(`compatibility baseline: updated ${normalizedPath(relative(root, target))}\n`);
    return;
  }
  const baselineText = readText(root, baselinePath);
  let baseline;
  try {
    baseline = JSON.parse(baselineText);
  } catch (error) {
    fail(`invalid baseline JSON: ${error.message}`);
  }
  const expected = canonicalJson(actual);
  if (baselineText !== canonicalJson(baseline)) fail("baseline JSON is not canonical");
  if (baselineText !== expected) {
    fail("current package/source/wire snapshot differs; review compatibility and regenerate intentionally with --emit=-");
  }
  process.stdout.write(
    `compatibility baseline: ${actual.packages.length} packages, ${actual.wire.fixtures.length} editing and ${actual.semanticWire.fixtures.length} semantic wire fixtures verified\n`,
  );
}

try {
  main();
} catch (error) {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
}
