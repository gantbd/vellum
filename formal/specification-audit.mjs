// Structural validation of manually reviewed source-unit coverage, not closure.
// No semantic coverage, product-policy approval or implementation proof is inferred.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const read = file => fs.readFileSync(path.join(root, file), 'utf8');
const hash = text => crypto.createHash('sha256').update(text).digest('hex');

export function sourceUnits(text, selected) {
  let section = '', paragraph = 0, bullet = 0, table = 0, block = [];
  const units = new Map();
  const flush = () => {
    if (!block.length) return;
    const body = block.join(' ').trim();
    const key = /^[-*] /.test(body) || /^\d+\. /.test(body) ? `b${++bullet}` :
      /^\|/.test(body) ? `t${++table}` : `p${++paragraph}`;
    if (selected.has(section)) units.set(`${section}/${key}`, body);
    block = [];
  };
  for (const line of text.split('\n')) {
    const heading = /^#{2,4} (\d+(?:\.\d+)*)(?:\.| )/.exec(line);
    if (heading) {
      flush();
      section = heading[1];
      paragraph = bullet = table = 0;
    } else if (!line.trim()) flush();
    else {
      if (/^(?:[-*] |\d+\. |\|)/.test(line) && block.length) flush();
      block.push(line.trim());
    }
  }
  flush();
  return units;
}

export function validateSpecificationAudit({
  requirementsText = read('requirements.md'),
  auditText = read('specification-audit.md'),
  specificationText = read('specification.md'),
  decisionsText = read('specification-decisions.md'),
  requireFull = false,
} = {}) {
  const fingerprint = /^Source SHA256: ([a-f0-9]{64})$/m.exec(auditText)?.[1];
  if (!fingerprint || fingerprint !== hash(requirementsText))
    throw Error('Clause audit source fingerprint is missing or stale');
  const declared = /^Audited sections: (.+)$/m.exec(auditText)?.[1].split(',').map(s => s.trim());
  if (!declared?.length || new Set(declared).size !== declared.length ||
      declared.some(s => !/^\d+(?:\.\d+)*$/.test(s)))
    throw Error('Clause audit has invalid/duplicate declared sections');
  const sourceSectionList = [...requirementsText.matchAll(/^#{2,4} (\d+(?:\.\d+)*)(?:\.| )/gm)].map(m => m[1]);
  const sourceSections = new Set(sourceSectionList);
  if (sourceSections.size !== sourceSectionList.length)
    throw Error('Duplicate numbered source section');
  if (declared.some(s => !sourceSections.has(s)))
    throw Error('Clause audit declares a nonexistent source section');
  const sourceCensusComplete = declared.length === sourceSections.size;
  if (requireFull && !sourceCensusComplete)
    throw Error('Clause audit omits source sections from required full census');
  const units = sourceUnits(requirementsText, new Set(declared));
  const specSections = new Set([...specificationText.matchAll(/^#{2,4} (\d+(?:\.\d+)*)(?:\.| )/gm)].map(m => m[1]));
  const decisions = new Set([...decisionsText.matchAll(/^\| (D\d{2})\b/gm)].map(m => m[1]));
  const rows = new Map();
  for (const match of auditText.matchAll(/^\| (\d+(?:\.\d+)*\/[pbt]\d+) \| (.+)$/gm)) {
    const [key, body] = [match[1], match[2]];
    if (rows.has(key)) throw Error(`Duplicate clause audit row ${key}`);
    if (!units.has(key)) throw Error(`Unknown clause audit source unit ${key}`);
    const cells = body.split('|').map(c => c.trim());
    if (cells.length !== 4 || !cells[0] || !cells[1] || !cells[2] || cells[3])
      throw Error(`Malformed clause audit row ${key}`);
    const owners = [...cells[1].matchAll(/§(\d+(?:\.\d+)*)/g)].map(m => m[1]);
    if (!owners.length || owners.some(s => !specSections.has(s)))
      throw Error(`Missing/unknown specification owner for ${key}`);
    for (const decision of body.matchAll(/\bD\d{2}\b/g))
      if (!decisions.has(decision[0])) throw Error(`Unknown decision ${decision[0]} for ${key}`);
    rows.set(key, body);
  }
  const missing = [...units.keys()].filter(key => !rows.has(key));
  if (missing.length) throw Error(`Missing clause audit rows: ${missing.join(', ')}`);
  return {
    sections: declared,
    sourceUnits: units.size,
    reviewRows: rows.size,
    sourceCensusComplete,
    sourceSha256: fingerprint,
    status: 'partial-draft-coverage-only',
    scope: 'Source-unit census/reference integrity only; not semantic closure, atomic-clause proof or independent review',
  };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const inputs = {
    requirementsText: read('requirements.md'), auditText: read('specification-audit.md'),
    specificationText: read('specification.md'), decisionsText: read('specification-decisions.md'),
    requireFull: process.argv.includes('--require-full'),
  };
  const report = validateSpecificationAudit(inputs);
  if (process.argv.includes('--self-test')) {
    const row = /^\| 3\.1\/p1 \| .+$/m.exec(inputs.auditText)[0];
    const duplicateSource = inputs.requirementsText + '\n## 19. Duplicate\n';
    const cases = [
      ['stale source', {...inputs, requirementsText: inputs.requirementsText + '\n'}, 'fingerprint'],
      ['missing row', {...inputs, auditText: inputs.auditText.replace(row, '')}, 'Missing clause'],
      ['duplicate row', {...inputs, auditText: inputs.auditText + '\n' + row}, 'Duplicate clause'],
      ['unknown unit', {...inputs, auditText: inputs.auditText.replace('3.1/p1 |', '3.1/p999 |')}, 'Unknown clause'],
      ['unknown owner', {...inputs, auditText: inputs.auditText.replace('§25', '§9999')}, 'specification owner'],
      ['missing source section', {...inputs, requireFull: true,
        auditText: inputs.auditText.replace(', 19.9\n', '\n')}, 'omits source sections'],
      ['duplicate source heading', {...inputs, requirementsText: duplicateSource,
        auditText: inputs.auditText.replace(hash(inputs.requirementsText), hash(duplicateSource))}, 'Duplicate numbered'],
      ['unknown decision', {...inputs, auditText: inputs.auditText.replace('D43', 'D99')}, 'Unknown decision'],
    ];
    for (const [name, changed, expected] of cases) {
      let caught;
      try { validateSpecificationAudit(changed); } catch (error) { caught = error; }
      if (!caught?.message.includes(expected)) throw Error(`Sensitivity control failed: ${name}`);
    }
    report.sensitivityControls = cases.map(([name]) => name);
  }
  console.log(JSON.stringify(report, null, 2));
}
