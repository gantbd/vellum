import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { renderLaunchdProfile } from './render-plist.mjs';

const root = fs.realpathSync.native(fs.mkdtempSync(path.join(os.tmpdir(), 'vellum-launchd-')));
const state = path.join(root, 'state & logs');
const binary = path.join(root, 'vellum-daemon');
fs.mkdirSync(state, { mode: 0o700 });
fs.writeFileSync(binary, '#!/bin/sh\nexit 0\n', { mode: 0o755 });
const uid = process.geteuid();
const config = {
  schema: 1, label: 'com.example.vellum', binary, stateDirectory: state,
  dataSocket: path.join(state, 'vellum.sock'), controlSocket: path.join(state, 'vellum-control.sock'),
  lockFile: path.join(state, '.vellum-daemon.lock'), metadataFile: path.join(state, 'workspace-identity-v1'),
  stdoutLog: path.join(state, 'stdout.log'), stderrLog: path.join(state, 'stderr.log'), ownerUid: uid,
};
const refusal = (name, mutate, prepare = () => {}) => {
  const candidate = structuredClone(config);
  prepare(candidate);
  mutate(candidate);
  try { renderLaunchdProfile(candidate); } catch { return; }
  throw new Error(`${name} was not refused`);
};

try {
  const plist = renderLaunchdProfile(config);
  if (!plist.includes(`${state.replace('&', '&amp;')}/vellum.sock`) || plist.includes('--allow-')) throw new Error('valid profile was not escaped or read-only');
  if (plist.includes('<key>Sockets</key>') || plist.includes('Health')) throw new Error('profile claimed socket activation or health');
  if (!plist.includes('<key>Umask</key>\n    <integer>63</integer>')) throw new Error('profile did not keep launchd-created logs private');
  const plistFile = path.join(root, 'qualified.plist');
  fs.writeFileSync(plistFile, plist, { mode: 0o600 });
  const lint = spawnSync('/usr/bin/plutil', ['-lint', plistFile], { encoding: 'utf8' });
  if (lint.status !== 0) throw new Error(`generated plist failed plutil: ${lint.stderr}`);
  refusal('unresolved-placeholder', (value) => { value.binary = '${VELLUM_BINARY}'; });
  refusal('unknown-field', (value) => { value.ambientAuthority = true; });
  refusal('escaped-state-child', (value) => { value.dataSocket = path.join(state, '..', 'other', 'vellum.sock'); });
  refusal('wrong-owner', (value) => { value.ownerUid = uid + 1; });
  refusal('world-writable-state', () => {}, () => fs.chmodSync(state, 0o777));
  fs.chmodSync(state, 0o700);
  refusal('symlink-binary', (value) => { value.binary = path.join(root, 'linked-daemon'); }, () => fs.symlinkSync(binary, path.join(root, 'linked-daemon')));
  const realBinaryParent = path.join(root, 'real-bin');
  const aliasBinaryParent = path.join(root, 'alias-bin');
  fs.mkdirSync(realBinaryParent);
  fs.copyFileSync(binary, path.join(realBinaryParent, 'vellum-daemon'));
  fs.chmodSync(path.join(realBinaryParent, 'vellum-daemon'), 0o755);
  fs.symlinkSync(realBinaryParent, aliasBinaryParent);
  refusal('symlink-binary-parent', (value) => { value.binary = path.join(aliasBinaryParent, 'vellum-daemon'); });
  const realState = path.join(root, 'real-state');
  const aliasState = path.join(root, 'alias-state');
  fs.mkdirSync(realState, { mode: 0o700 });
  fs.symlinkSync(realState, aliasState);
  refusal('symlink-state-parent', (value) => {
    value.stateDirectory = aliasState;
    value.dataSocket = path.join(aliasState, 'vellum.sock');
    value.controlSocket = path.join(aliasState, 'vellum-control.sock');
    value.lockFile = path.join(aliasState, '.vellum-daemon.lock');
    value.metadataFile = path.join(aliasState, 'workspace-identity-v1');
    value.stdoutLog = path.join(aliasState, 'stdout.log');
    value.stderrLog = path.join(aliasState, 'stderr.log');
  });
  refusal('existing-socket', () => {}, () => fs.writeFileSync(path.join(state, 'vellum.sock'), 'not a socket'));
  fs.unlinkSync(path.join(state, 'vellum.sock'));
  refusal('writable-lock', () => {}, () => fs.writeFileSync(path.join(state, '.vellum-daemon.lock'), '', { mode: 0o666 }));
  fs.unlinkSync(path.join(state, '.vellum-daemon.lock'));
  const configFile = path.join(root, 'deployment.json');
  fs.writeFileSync(configFile, `${JSON.stringify(config)}\n`, { mode: 0o600 });
  const configLink = path.join(root, 'deployment-link.json');
  fs.symlinkSync(configFile, configLink);
  const cli = spawnSync(process.execPath, [path.resolve('distribution/launchd/render-plist.mjs'), `--config=${configLink}`], { encoding: 'utf8' });
  if (cli.status === 0 || !cli.stderr.includes('symbolic link')) throw new Error('symlink config was not refused');
  const realConfigParent = path.join(root, 'real-config');
  const aliasConfigParent = path.join(root, 'alias-config');
  fs.mkdirSync(realConfigParent);
  fs.copyFileSync(configFile, path.join(realConfigParent, 'deployment.json'));
  fs.symlinkSync(realConfigParent, aliasConfigParent);
  const aliasedConfig = spawnSync(process.execPath, [path.resolve('distribution/launchd/render-plist.mjs'), `--config=${path.join(aliasConfigParent, 'deployment.json')}`], { encoding: 'utf8' });
  if (aliasedConfig.status === 0 || !aliasedConfig.stderr.includes('noncanonical ancestor')) throw new Error('symlink config parent was not refused');
  const relative = spawnSync(process.execPath, [path.resolve('distribution/launchd/render-plist.mjs'), '--config=deployment.json'], { cwd: root, encoding: 'utf8' });
  if (relative.status === 0 || !relative.stderr.includes('absolute path')) throw new Error('relative config path was not refused');
  process.stdout.write('Vellum launchd profile qualification mutants passed\n');
} finally {
  fs.rmSync(root, { recursive: true, force: true });
}
