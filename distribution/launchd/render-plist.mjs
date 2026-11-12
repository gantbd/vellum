#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';

const fail = (message) => { throw new Error(`launchd profile: ${message}`); };
const exactKeys = (value, expected) => {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) fail('configuration is not an object');
  if (Object.keys(value).sort().join(',') !== [...expected].sort().join(',')) fail('configuration fields are incompatible');
};
const unresolved = (value) => /\$\{|\{\{|\}\}|@@|<[^>]+>/.test(value);
const absolute = (value, label) => {
  if (typeof value !== 'string' || value.length === 0 || value.includes('\0') || unresolved(value)) {
    fail(`${label} is malformed or contains an unresolved placeholder`);
  }
  if (!path.isAbsolute(value) || path.normalize(value) !== value) fail(`${label} is not a normalized absolute path`);
  return value;
};
const inspect = (file, label) => {
  let stat;
  try { stat = fs.lstatSync(file); } catch (error) {
    if (error.code === 'ENOENT') return null;
    fail(`${label} could not be inspected`);
  }
  if (stat.isSymbolicLink()) fail(`${label} is a symbolic link`);
  return stat;
};
const requireCanonical = (file, label) => {
  let canonical;
  try { canonical = fs.realpathSync.native(file); } catch { fail(`${label} could not be resolved canonically`); }
  if (canonical !== file) fail(`${label} contains a symbolic-link or noncanonical ancestor`);
};
const requireOwner = (stat, ownerUid, label) => {
  if (stat.uid !== ownerUid) fail(`${label} has the wrong owner`);
};
const requirePrivateFile = (file, ownerUid, label) => {
  const stat = inspect(file, label);
  if (stat === null) return;
  if (!stat.isFile()) fail(`${label} is not a regular file`);
  requireOwner(stat, ownerUid, label);
  if ((stat.mode & 0o077) !== 0) fail(`${label} grants group or other permissions`);
};
const child = (directory, file, expectedName, label) => {
  if (path.dirname(file) !== directory || path.basename(file) !== expectedName) {
    fail(`${label} is not the exact ${expectedName} child of the state directory`);
  }
};
const escapeXml = (value) => value
  .replaceAll('&', '&amp;')
  .replaceAll('<', '&lt;')
  .replaceAll('>', '&gt;')
  .replaceAll('"', '&quot;')
  .replaceAll("'", '&apos;');

export function renderLaunchdProfile(configuration) {
  exactKeys(configuration, [
    'schema', 'label', 'binary', 'stateDirectory', 'dataSocket', 'controlSocket',
    'lockFile', 'metadataFile', 'stdoutLog', 'stderrLog', 'ownerUid',
  ]);
  if (configuration.schema !== 1) fail('configuration schema is incompatible');
  if (typeof configuration.label !== 'string' || !/^[A-Za-z0-9.-]{1,128}$/.test(configuration.label)) {
    fail('label is not canonical');
  }
  if (!Number.isSafeInteger(configuration.ownerUid) || configuration.ownerUid < 0) fail('ownerUid is invalid');
  if (typeof process.geteuid !== 'function' || process.geteuid() !== configuration.ownerUid) {
    fail('qualification must run as the declared owner');
  }

  const binary = absolute(configuration.binary, 'binary');
  const stateDirectory = absolute(configuration.stateDirectory, 'stateDirectory');
  const dataSocket = absolute(configuration.dataSocket, 'dataSocket');
  const controlSocket = absolute(configuration.controlSocket, 'controlSocket');
  const lockFile = absolute(configuration.lockFile, 'lockFile');
  const metadataFile = absolute(configuration.metadataFile, 'metadataFile');
  const stdoutLog = absolute(configuration.stdoutLog, 'stdoutLog');
  const stderrLog = absolute(configuration.stderrLog, 'stderrLog');

  const state = inspect(stateDirectory, 'stateDirectory');
  if (state === null || !state.isDirectory()) fail('stateDirectory is not a real directory');
  requireCanonical(stateDirectory, 'stateDirectory');
  requireOwner(state, configuration.ownerUid, 'stateDirectory');
  if ((state.mode & 0o777) !== 0o700) fail('stateDirectory mode is not exactly 0700');

  const executable = inspect(binary, 'binary');
  if (executable === null || !executable.isFile()) fail('binary is not a real regular file');
  requireCanonical(binary, 'binary');
  if (executable.uid !== 0 && executable.uid !== configuration.ownerUid) fail('binary owner is neither root nor the declared owner');
  if ((executable.mode & 0o111) === 0 || (executable.mode & 0o022) !== 0) fail('binary is not executable or is group/other writable');

  child(stateDirectory, dataSocket, 'vellum.sock', 'dataSocket');
  child(stateDirectory, controlSocket, 'vellum-control.sock', 'controlSocket');
  child(stateDirectory, lockFile, '.vellum-daemon.lock', 'lockFile');
  child(stateDirectory, metadataFile, 'workspace-identity-v1', 'metadataFile');
  child(stateDirectory, stdoutLog, 'stdout.log', 'stdoutLog');
  child(stateDirectory, stderrLog, 'stderr.log', 'stderrLog');
  if (new Set([dataSocket, controlSocket, lockFile, metadataFile, stdoutLog, stderrLog]).size !== 6) {
    fail('state paths are not distinct');
  }
  if (inspect(dataSocket, 'dataSocket') !== null || inspect(controlSocket, 'controlSocket') !== null) {
    fail('socket paths must be absent when rendering a new activation profile');
  }
  requirePrivateFile(lockFile, configuration.ownerUid, 'lockFile');
  requirePrivateFile(metadataFile, configuration.ownerUid, 'metadataFile');
  requirePrivateFile(stdoutLog, configuration.ownerUid, 'stdoutLog');
  requirePrivateFile(stderrLog, configuration.ownerUid, 'stderrLog');

  const args = [binary, '--socket', dataSocket, '--control', controlSocket];
  const array = args.map((argument) => `      <string>${escapeXml(argument)}</string>`).join('\n');
  return `<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0">\n  <dict>\n    <key>Label</key>\n    <string>${escapeXml(configuration.label)}</string>\n    <key>ProgramArguments</key>\n    <array>\n${array}\n    </array>\n    <key>RunAtLoad</key>\n    <true/>\n    <key>KeepAlive</key>\n    <false/>\n    <key>ProcessType</key>\n    <string>Background</string>\n    <key>Umask</key>\n    <integer>63</integer>\n    <key>StandardOutPath</key>\n    <string>${escapeXml(stdoutLog)}</string>\n    <key>StandardErrorPath</key>\n    <string>${escapeXml(stderrLog)}</string>\n  </dict>\n</plist>\n`;
}

const invoked = process.argv[1] !== undefined
  && path.resolve(process.argv[1]) === path.resolve(decodeURIComponent(new URL(import.meta.url).pathname));
if (invoked) {
  try {
    if (process.argv.length !== 3 || !process.argv[2].startsWith('--config=')) fail('usage: vellum-launchd-render --config=DEPLOYMENT.json');
    const configFile = absolute(process.argv[2].slice(9), 'config');
    const stat = inspect(configFile, 'config');
    if (stat === null || !stat.isFile()) fail('config is not a real regular file');
    requireCanonical(configFile, 'config');
    if ((stat.mode & 0o022) !== 0) fail('config is group or other writable');
    const text = fs.readFileSync(configFile, 'utf8');
    let config;
    try { config = JSON.parse(text); } catch { fail('config is not valid JSON'); }
    if (!Number.isSafeInteger(config?.ownerUid) || stat.uid !== config.ownerUid) fail('config has the wrong owner');
    process.stdout.write(renderLaunchdProfile(config));
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
