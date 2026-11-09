// Semantic family/reference routing only; not the Rust API or argument parser.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath,pathToFileURL} from 'node:url';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const read=()=>JSON.parse(fs.readFileSync(path.join(root,'formal/operation-families.json'),'utf8'));
const sinks={query:['read'],localCommand:['sourceCoordinator','viewOwner','annotationCoordinator','configurationCoordinator','lifecycleOwner','controlOwner','observationOwner'],
  effectRequest:['hostRequest'],effectObservation:['evidenceOwner','derivedOwner']};
const text=x=>typeof x==='string'&&x.trim().length>0;
function facadeInventory(){
  const requirements=fs.readFileSync(path.join(root,'requirements.md'),'utf8');
  const section=/^### 3\.3 Public facade\n([\s\S]*?)(?=^### 3\.4 )/m.exec(requirements)?.[1];
  assert(section,'Missing facade source section');
  const bullets=[...section.matchAll(/^- /gm)].length;
  const paragraphs=section.split(/\n\s*\n/).map(s=>s.trim()).filter(s=>s&&!s.startsWith('- '));
  const presentation=paragraphs.findIndex(s=>s.startsWith('Viewport/scrolling,'));
  assert(presentation>=0,'Missing optional presentation ownership paragraph');
  return {bullets,references:new Set([...Array.from({length:bullets},(_,i)=>`3.3/b${i+1}`),`3.3/p${presentation+1}`])};
}
const protectedById={F06:['minimal-control-status'],F18:['request-cancellation','shutdown-disposition'],
  F19:['stop-observation','revoke-observation-grant'],F26:['task-cancellation']};
export function validateOperationCatalog(rows) {
  assert(Array.isArray(rows)&&rows.length===28);
  assert.deepEqual(rows.map(r=>r.id),Array.from({length:28},(_,i)=>`F${String(i+1).padStart(2,'0')}`));
  assert.equal(new Set(rows.map(r=>r.name)).size,28);
  const spec=fs.readFileSync(path.join(root,'specification.md'),'utf8');
  const model=fs.readFileSync(path.join(root,'model.md'),'utf8');
  const sections=new Set([...spec.matchAll(/^#{2,4} (\d+(?:\.\d+)*)(?:[ .])/gm)].map(m=>m[1]));
  const observations=new Set([...model.matchAll(/^\| (INS-\d{2})(?:\s*\||:)/gm)].map(m=>m[1]));
  const facade=facadeInventory().references;
  for(const r of rows) {
    assert.deepEqual(Object.keys(r).sort(),['id','name','kind','surface','sections','facade','target','variants','sink','observations',
      ...(Object.hasOwn(protectedById,r.id)?['protectedVariants']:[])].sort());
    if(Object.hasOwn(protectedById,r.id))assert.deepEqual(r.protectedVariants,protectedById[r.id]);
    assert(text(r.name)&&/^[a-z]+(?:-[a-z]+)*$/.test(r.name));assert(text(r.target));
    assert(Object.hasOwn(sinks,r.kind)&&sinks[r.kind].includes(r.sink));
    assert(['required','conditional'].includes(r.surface));
    for(const [key,valid] of [['sections',v=>sections.has(v)],['facade',v=>facade.has(v)],
      ['observations',v=>observations.has(v)],['variants',v=>text(v)&&/^[a-z]+(?:-[a-z]+)*$/.test(v)]]) {
      assert(Array.isArray(r[key])&&r[key].length>0&&new Set(r[key]).size===r[key].length);
      assert(r[key].every(v=>typeof v==='string'&&valid(v)));
    }
  }
  assert.deepEqual([...new Set(rows.flatMap(r=>r.facade))].sort(),[...facade].sort());
  assert.equal(rows.filter(r=>r.kind==='query').length,10);
  assert.equal(rows.filter(r=>r.kind==='localCommand').length,9);
  assert.equal(rows.filter(r=>r.kind==='effectRequest').length,7);
  assert.equal(rows.filter(r=>r.kind==='effectObservation').length,2);
}
const shape=(r,keys)=>r&&typeof r==='object'&&!Array.isArray(r)&&
  JSON.stringify(Object.keys(r).sort())===JSON.stringify([...keys].sort());
export function decodeTyped(rows,r) {
  if(!shape(r,['family','variant','version']))return null;
  if(!Number.isInteger(r.family)||r.family<0||r.family>=rows.length||r.version!==1)return null;
  const d=rows[r.family];return d.variants.includes(r.variant)?{descriptor:d,variant:r.variant}:null;
}
export function decodeDynamic(rows,r) {
  if(!shape(r,['name','variant','version'])||r.version!==1)return null;
  const d=rows.find(d=>d.name===r.name);return d&&d.variants.includes(r.variant)?{descriptor:d,variant:r.variant}:null;
}
export function planRoute(decoded,c) {
  if(!decoded)return {status:'unsupported',sink:null};
  if(!c.schemaValid)return {status:'invalid',sink:null};
  const d=decoded.descriptor;
  if(d.kind==='effectObservation') {
    if(!c.trustedProducer||!c.attributed)return {status:'invalidEvidence',sink:null};
    if(d.sink==='evidenceOwner')return c.protectedCapacity?{status:'routed',sink:'evidenceOwner'}:
      {status:'retentionRequired',sink:'responsibleHost'};
    if(!c.enabled||!c.current||!c.grant)return {status:'staleDerived',sink:null};
    return c.ordinaryCapacity?{status:'routed',sink:'derivedOwner'}:{status:'capacity',sink:null};
  }
  const protectedControl=d.protectedVariants?.includes(decoded.variant);
  if(protectedControl) {
    // current includes validated admitted target/retained contract and capability.
    if(!c.grant)return {status:'denied',sink:null};
    if(!c.current)return {status:'stale',sink:null};
    if(d.kind==='query'&&!c.releaseAllowed)return {status:'withheld',sink:null};
    if(!c.protectedCapacity)return d.kind==='query'?{status:'protectedUnavailable',sink:null}:
      {status:'retentionRequired',sink:'responsibleHost'};
    return {status:'routed',sink:d.sink};
  }
  if(!c.enabled)return {status:'unsupported',sink:null};
  if(!c.grant)return {status:'denied',sink:null};
  if(!c.current)return {status:'stale',sink:null};
  if(!c.ordinaryCapacity)return {status:'capacity',sink:null};
  if(d.kind==='query'&&!c.releaseAllowed)return {status:'withheld',sink:null};
  return {status:'routed',sink:d.sink};
}
function expectedRoute(d,variant,c) {
  // Independent decision table: no descriptor-driven executable handler is called.
  if(!c.schemaValid)return {status:'invalid',sink:null};
  if(d.id==='F27') {
    if(!(c.attributed&&c.trustedProducer))return {status:'invalidEvidence',sink:null};
    return c.protectedCapacity?{status:'routed',sink:'evidenceOwner'}:{status:'retentionRequired',sink:'responsibleHost'};
  }
  if(d.id==='F28') {
    if(!(c.attributed&&c.trustedProducer))return {status:'invalidEvidence',sink:null};
    if(!(c.enabled&&c.current&&c.grant))return {status:'staleDerived',sink:null};
    return c.ordinaryCapacity?{status:'routed',sink:'derivedOwner'}:{status:'capacity',sink:null};
  }
  if((d.id==='F06'&&variant==='minimal-control-status')||d.id==='F18'||
    (d.id==='F19'&&['stop-observation','revoke-observation-grant'].includes(variant))||
    (d.id==='F26'&&variant==='task-cancellation')) {
    if(!c.grant)return {status:'denied',sink:null};
    if(!c.current)return {status:'stale',sink:null};
    if(d.id==='F06'&&!c.releaseAllowed)return {status:'withheld',sink:null};
    return c.protectedCapacity?{status:'routed',sink:d.sink}:d.id==='F06'?{status:'protectedUnavailable',sink:null}:
      {status:'retentionRequired',sink:'responsibleHost'};
  }
  const refusal=!c.enabled?'unsupported':!c.grant?'denied':!c.current?'stale':!c.ordinaryCapacity?'capacity':
    d.id<='F10'&&!c.releaseAllowed?'withheld':null;
  return refusal?{status:refusal,sink:null}:{status:'routed',sink:d.sink};
}
export function checkOperationCatalog() {
  const rows=read();validateOperationCatalog(rows);
  const flags=['enabled','grant','schemaValid','current','ordinaryCapacity','protectedCapacity','trustedProducer','attributed','releaseAllowed'];
  let variantCases=0,routingCases=0;
  for(let i=0;i<rows.length;i++)for(const variant of rows[i].variants) {
    const t={family:i,variant,version:1},d={name:rows[i].name,variant,version:1};
    assert.deepEqual(decodeTyped(rows,t),decodeDynamic(rows,d));variantCases++;
    for(let mask=0;mask<512;mask++) {
      const c=Object.fromEntries(flags.map((key,bit)=>[key,Boolean(mask&(1<<bit))]));
      const before=structuredClone({rows,t,d,c});
      const typed=planRoute(decodeTyped(rows,t),c),dynamic=planRoute(decodeDynamic(rows,d),c);
      assert.deepEqual(typed,dynamic);assert.deepEqual(typed,expectedRoute(rows[i],variant,c));
      if(rows[i].kind==='query')assert(typed.sink===null||typed.sink==='read');
      assert.deepEqual({rows,t,d,c},before);routingCases++;
    }
  }
  const controls=[];const reject=(name,mutate)=>{const changed=structuredClone(rows);mutate(changed);assert.throws(()=>validateOperationCatalog(changed));controls.push(name);};
  reject('missing-family',r=>r.pop());reject('duplicate-name',r=>{r[1].name=r[0].name;});
  reject('query-mutation-sink',r=>{r[0].sink='sourceCoordinator';});
  reject('observation-new-work-lane',r=>{r[26].sink='hostRequest';});
  reject('unknown-section',r=>{r[0].sections=['[0-9]+'];});
  reject('unknown-observation',r=>{r[0].observations=['INS-99|.*'];});
  reject('missing-variant',r=>{r[0].variants=[];});reject('object-target',r=>{r[0].target={};});
  reject('removed-facade-bullet',r=>{r[0].facade=['3.3/b12'];});
  reject('new-grant-not-protected-revocation',r=>{r[18].protectedVariants.push('establish-bounded-stream');});
  const control=(name,ok)=>{assert(ok);controls.push(name);};
  const c=Object.fromEntries(flags.map(k=>[k,true]));
  control('unknown-verb-no-route',planRoute(decodeDynamic(rows,{name:'run-anything',variant:'x',version:1}),c).sink===null);
  control('unknown-variant-no-route',decodeDynamic(rows,{name:rows[0].name,variant:'execute-script',version:1})===null);
  control('unknown-version-no-route',decodeTyped(rows,{family:0,variant:rows[0].variants[0],version:2})===null);
  control('caller-cannot-reclassify',decodeDynamic(rows,{name:rows[10].name,variant:rows[10].variants[0],version:1,kind:'query'})===null);
  control('discovery-not-current-grant',planRoute({descriptor:rows[10]}, {...c,grant:false}).status==='denied');
  control('historical-evidence-survives-observer-revocation',planRoute({descriptor:rows[26]}, {...c,grant:false,current:false,ordinaryCapacity:false,enabled:false}).sink==='evidenceOwner');
  control('forged-evidence-not-retained-as-truth',planRoute({descriptor:rows[26]}, {...c,trustedProducer:false}).status==='invalidEvidence');
  control('protected-exhaustion-not-silent-drop',planRoute({descriptor:rows[26]}, {...c,protectedCapacity:false}).status==='retentionRequired');
  control('stale-derived-result-not-historical-write',planRoute({descriptor:rows[27]}, {...c,current:false}).status==='staleDerived');
  control('query-no-hidden-launch',planRoute({descriptor:rows[4]},c).sink==='read');
  for(const [index,variant] of [[17,'request-cancellation'],[17,'shutdown-disposition'],[18,'stop-observation'],
    [18,'revoke-observation-grant'],[25,'task-cancellation'],[5,'minimal-control-status']])
    control(`protected-${variant}-survives-ordinary-pressure`,planRoute({descriptor:rows[index],variant},
      {...c,ordinaryCapacity:false,enabled:false}).status==='routed');
  control('protected-control-still-validates-target',planRoute({descriptor:rows[17],variant:'request-cancellation'},
    {...c,current:false,ordinaryCapacity:false}).status==='stale');
  control('protected-control-retains-responsibility',planRoute({descriptor:rows[17],variant:'request-cancellation'},
    {...c,ordinaryCapacity:false,protectedCapacity:false}).status==='retentionRequired');
  control('new-observation-not-promoted',planRoute({descriptor:rows[18],variant:'establish-bounded-stream'},
    {...c,ordinaryCapacity:false}).status==='capacity');
  return {status:'semantic-operation-catalog-only',families:rows.length,facadeBullets:facadeInventory().bullets,optionalPresentationAnchors:1,variantCases,routingCases,controls,
    concreteVocabularyClosed:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(checkOperationCatalog(),null,2));
