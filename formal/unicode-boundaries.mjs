// Reference provider/tool bridge only. No Java process is required of the production engine.
import fs from 'node:fs';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {isDeepStrictEqual as equal} from 'node:util';
import {validateCanonicalBatch} from './text-oracles.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./unicode-boundaries.json',import.meta.url),'utf8'));
const owned=new WeakMap(),copy=structuredClone;
const hash=file=>crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const boundary=(s,p)=>Number.isSafeInteger(p)&&p>=0&&p<=s.length&&(p===s.length||(s[p]&0xc0)!==0x80);
const limitsValid=l=>l&&catalog.limits.every(k=>Number.isSafeInteger(l[k])&&l[k]>=0);
export function runUnicodeReference(texts,sourceFile=new URL('./UnicodeBoundary.java',import.meta.url)){
  const jar=process.env.VELLUM_ICU4J_JAR;
  if(!jar||hash(jar)!==catalog.icuJarSha256)throw Error('Use the pinned ICU4J 79.1 reference jar');
  if(!Array.isArray(texts)||!texts.length||texts.length>catalog.toolBounds.maxBatch||texts.some(s=>typeof s!=='string'||[...s].length>catalog.toolBounds.maxScalars||
      Buffer.from(s,'utf8').toString('utf8')!==s||Buffer.from(s,'utf8').toString('base64').length>catalog.toolBounds.maxBase64Line))throw Error('Reference tool input bound/scalar violation');
  const p=spawnSync('java',['--class-path',jar,fileURLToPath(sourceFile)],{input:texts.map(s=>Buffer.from(s,'utf8').toString('base64')).join('\n')+'\n',
    encoding:'utf8',timeout:30000,maxBuffer:8*1024*1024});
  if(p.error||p.signal||p.status!==0||p.stderr)throw Error('Unicode reference tool failed: '+(p.stderr||p.error||p.signal));
  const lines=p.stdout.trim().split(/\r?\n/),version=JSON.parse(lines.shift());
  if(!equal(version,{icu:catalog.icuVersion,unicode:catalog.unicode,uax29:catalog.uax29})||lines.length!==texts.length)throw Error('Unicode reference tool version/cardinality mismatch');
  const rows=lines.map(JSON.parse);
  if(rows.some(row=>!Array.isArray(row)||row.length!==4))throw Error('Unicode reference tool row shape');
  return {version,jarSha256:catalog.icuJarSha256,rows};
}
function within(source,table,limits){
  if(!limitsValid(limits)||source.length>limits.maxRootBytes||!Array.isArray(table)||table.length!==4)return false;
  try{validateCanonicalBatch(source,[]);}catch{return false;}
  const [graphemes,words,significant,dictionary]=table;
  for(const vector of [graphemes,words,dictionary])if(!Array.isArray(vector)||vector[0]!==0||vector.at(-1)!==source.length||vector.some((p,i)=>!boundary(source,p)||i>0&&p<=vector[i-1]))return false;
  if(!Array.isArray(significant)||significant.length>limits.maxWords||significant.some((r,i)=>!Array.isArray(r)||r.length!==2||r[0]>=r[1]||
      !words.includes(r[0])||words[words.indexOf(r[0])+1]!==r[1]||i>0&&r[0]<significant[i-1][1]))return false;
  return graphemes.length+words.length+dictionary.length+2*significant.length<=limits.maxBoundaries&&Buffer.byteLength(JSON.stringify(table),'utf8')<=limits.maxBoundaryBytes;
}
export function acquireBoundaryBatch(snapshots,limits){
  if(!limitsValid(limits)||!Array.isArray(snapshots)||!snapshots.length||snapshots.some(s=>!Array.isArray(s.source)||s.source.length>limits.maxRootBytes))return null;
  try{for(const s of snapshots)validateCanonicalBatch(s.source,[]);}catch{return null;}
  const result=runUnicodeReference(snapshots.map(s=>Buffer.from(s.source).toString('utf8')));
  if(snapshots.some((s,i)=>!within(s.source,result.rows[i],limits)))return null;
  return snapshots.map((s,i)=>{const ticket=Object.freeze({profile:catalog.profile});owned.set(ticket,{ref:copy(s.ref),source:copy(s.source),table:copy(result.rows[i])});return ticket;});
}
export function resolveBoundaries(ticket,ref,source,limits){
  const p=owned.get(ticket);return p&&equal(p.ref,ref)&&equal(p.source,source)&&within(source,p.table,limits)?copy(p.table):null;
}
export {catalog as unicodeReferenceCatalog};
