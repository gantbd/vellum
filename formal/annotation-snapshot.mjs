// Design reference for one selected JSON profile, not a production JSON library.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {mapAttachment} from './mapping-profiles.mjs';
const schema=JSON.parse(fs.readFileSync(new URL('./annotation-snapshot.schema.json',import.meta.url),'utf8'));
const bytes=s=>new TextEncoder().encode(s),copy=structuredClone;
const scalar=s=>typeof s==='string'&&[...s].every(c=>{const n=c.codePointAt(0);return n<0xd800||n>0xdfff;});
const freeze=v=>{if(v&&typeof v==='object'){for(const child of Object.values(v))freeze(child);Object.freeze(v);}return v;};
const need=x=>{if(!x)throw Error('invalid');};
const limitKeys=['maxBytes','maxNodes','maxDepth','maxStringBytes','maxRecords','maxDigits','maxBodyBytes','maxTotalBodyBytes'];
function limitsValid(l) {return l&&limitKeys.every(k=>Number.isSafeInteger(l[k])&&l[k]>=0)&&l.maxDepth<=32;}

// Only objects, arrays and strings occur in the selected format. Other valid
// JSON primitive types are intentionally outside this profile, not coerced.
function parseProfile(input,l) {
  need(input instanceof Uint8Array&&input.length<=l.maxBytes);
  const text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(input);
  let at=0,nodes=0;
  const ws=()=>{while(at<text.length&&' \t\r\n'.includes(text[at]))at++;};
  const string=()=>{
    need(text[at]==='"');const start=at++;let escaped=false;
    while(at<text.length) {
      const c=text[at++];
      if(c==='"'&&!escaped) {
        const value=JSON.parse(text.slice(start,at));need(scalar(value)&&bytes(value).length<=l.maxStringBytes);return value;
      }
      escaped=!escaped&&c==='\\';
    }
    throw Error('invalid');
  };
  const value=depth=>{
    need(depth<=l.maxDepth&&++nodes<=l.maxNodes);ws();
    if(text[at]==='"')return string();
    if(text[at]==='{') {
      at++;ws();const out=Object.create(null),keys=new Set();
      if(text[at]==='}'){at++;return out;}
      while(true) {
        ws();const key=string();need(!keys.has(key));keys.add(key);ws();need(text[at++]===':');
        out[key]=value(depth+1);ws();const c=text[at++];if(c==='}')return out;need(c===',');
      }
    }
    if(text[at]==='[') {
      at++;ws();const out=[];if(text[at]===']'){at++;return out;}
      while(true){out.push(value(depth+1));ws();const c=text[at++];if(c===']')return out;need(c===',');}
    }
    throw Error('invalid');
  };
  const result=value(0);ws();need(at===text.length);return result;
}
// Schema-specific checks. This is not a general JSON Schema interpreter.
function exact(v,name) {
  const shape=name==='Root'?schema:schema.$defs[name];
  need(v&&typeof v==='object'&&!Array.isArray(v));
  need(equal(Object.keys(v).sort(),[...shape.required].sort()));
}
const textId=v=>need(scalar(v)&&v.length>0);
function natural(v,l,positive=false) {
  need(typeof v==='string'&&v.length<=l.maxDigits&&new RegExp(schema.$defs[positive?'Positive':'Nat'].pattern).test(v));
  return BigInt(v);
}
function source(s,l) {exact(s,'Source');textId(s.domain);textId(s.token);natural(s.byteLength,l);}
const sameSource=(a,b)=>a.domain===b.domain&&a.token===b.token&&a.byteLength===b.byteLength;
function target(t,l) {
  need(t&&['document','point','range'].includes(t.kind));
  exact(t,{document:'DocumentTarget',point:'PointTarget',range:'RangeTarget'}[t.kind]);source(t.source,l);
  const size=natural(t.source.byteLength,l);
  if(t.kind==='point'){need(['before','after'].includes(t.affinity));need(['position','content'].includes(t.profile));need(natural(t.offset,l)<=size);}
  if(t.kind==='range'){const lo=natural(t.start,l),hi=natural(t.end,l);need(lo<hi&&hi<=size);}
}
function shape(v,l) {
  exact(v,'Root');need(v.format===schema.properties.format.const&&v.version===schema.properties.version.const);
  exact(v.document,'Document');textId(v.document.namespace);textId(v.document.value);source(v.source,l);
  need(Array.isArray(v.records)&&v.records.length<=l.maxRecords);const identities=new Set();let bodyBytes=0;
  for(const r of v.records) {
    exact(r,'Record');exact(r.identity,'Identity');textId(r.identity.namespace);textId(r.identity.value);
    natural(r.identity.incarnation,l,true);natural(r.contentVersion,l,true);natural(r.targetGeneration,l,true);
    const identity=JSON.stringify([r.identity.namespace,r.identity.value]);need(!identities.has(identity));identities.add(identity);
    need(scalar(r.body));const size=bytes(r.body).length;need(size<=l.maxBodyBytes);bodyBytes+=size;need(bodyBytes<=l.maxTotalBodyBytes);
    target(r.authoredTarget,l);
    if(r.placement?.state==='detached')exact(r.placement,'Detached');
    else {exact(r.placement,'Located');need(['attached','affected'].includes(r.placement.state));target(r.placement.target,l);
      need(sameSource(r.placement.target.source,v.source));}
  }
}
export function decodeSnapshot(input,limits) {
  if(!limitsValid(limits))return {status:'invalidProfile'};
  try {const value=parseProfile(input,limits);shape(value,limits);return {status:'decoded',value};}
  catch{return {status:'invalid'};}
}
export function stageSnapshot(input,limits,current,gates) {
  const decoded=decodeSnapshot(input,limits);
  if(decoded.status!=='decoded')return {status:decoded.status,current};
  const v=decoded.value;
  const flags=['authorized','resources','notStopped','targetEvidence','identityRestorable','annotationSetCurrent'];
  if(!flags.every(k=>gates[k])||v.document.namespace!==current.document.namespace||v.document.value!==current.document.value||
      !sameSource(v.source,current.source)||BigInt(v.source.byteLength)!==BigInt(current.bytes.length))return {status:'refused',current};
  const boundary=n=>n===current.bytes.length||(current.bytes[n]&0xc0)!==0x80;
  try {
    new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(current.bytes);
    for(const r of v.records)for(const t of [r.authoredTarget,...(r.placement.state==='detached'?[]:[r.placement.target])])
      if(t.source.domain===current.source.domain&&t.source.token===current.source.token){
        need(t.source.byteLength===current.source.byteLength);
        if(t.kind==='point')need(boundary(Number(t.offset)));
        if(t.kind==='range')need(boundary(Number(t.start))&&boundary(Number(t.end)));
      }
  }catch{return {status:'refused',current};}
  return {status:'staged',candidate:freeze(copy(v)),dependencies:freeze(copy({document:current.document,
    source:current.source,annotationSet:current.annotationSet})),current};
}

const limits={maxBytes:20000,maxNodes:1000,maxDepth:16,maxStringBytes:2000,maxRecords:8,maxDigits:32,maxBodyBytes:2000,maxTotalBodyBytes:4000};
const current={document:{namespace:'host',value:'doc'},source:{domain:'host-source',token:'s1',byteLength:'4'},bytes:bytes('aéb'),annotationSet:'set1'};
const flags=['authorized','resources','notStopped','targetEvidence','identityRestorable','annotationSetCurrent'];
const gates=Object.fromEntries(flags.map(k=>[k,true]));
const docTarget={kind:'document',source:copy(current.source)};
const fixture={format:'vellum.annotations',version:'1',document:copy(current.document),source:copy(current.source),records:[{
  identity:{namespace:'author',value:'note',incarnation:'9007199254740993'},contentVersion:'9007199254740995',targetGeneration:'1',
  body:'note',authoredTarget:copy(docTarget),placement:{state:'attached',target:copy(docTarget)}}]};
const wire=v=>bytes(JSON.stringify(v));
export function checkAnnotationSnapshot() {
  let roundTrips=0,stageCases=0,pointMappingCases=0;const controls=[];
  const check=(name,ok)=>{assert(ok,name);controls.push(name);};
  const positions=[0,1,3,4];const targets=[docTarget,
    ...positions.flatMap(offset=>['before','after'].flatMap(affinity=>['position','content'].map(profile=>({kind:'point',source:copy(current.source),offset:String(offset),affinity,profile})))),
    ...positions.flatMap((lo,i)=>positions.slice(i+1).map(hi=>({kind:'range',source:copy(current.source),start:String(lo),end:String(hi)})))];
  for(const t of targets)for(const state of ['attached','affected','detached'])for(const historical of [false,true])
    for(const body of ['', 'a\r\nb','é','\u0000','"\\','```mermaid\ngraph LR\n```']) {
      const f=copy(fixture);f.records[0].body=body;f.records[0].authoredTarget=copy(t);
      if(historical)f.records[0].authoredTarget.source.token='older-source';
      f.records[0].placement=state==='detached'?{state}:{state,target:copy(t)};
      const input=wire(f),before=input.slice(),decoded=decodeSnapshot(input,limits);assert.equal(decoded.status,'decoded');
      assert.deepEqual(JSON.parse(JSON.stringify(decoded.value)),f);
      const staged=stageSnapshot(input,limits,current,gates);assert.equal(staged.status,'staged');assert.deepEqual(JSON.parse(JSON.stringify(staged.candidate)),f);assert.deepEqual(input,before);roundTrips++;
    }
  for(let mask=0;mask<64;mask++)for(const doc of [true,false])for(const src of [true,false]) {
    const f=copy(fixture);if(!doc)f.document.value='other';if(!src){f.source.token='old';f.records[0].placement.target.source.token='old';}
    const g=Object.fromEntries(flags.map((k,i)=>[k,!!(mask&(1<<i))]));const input=wire(f),before=copy({input,current,g});
    const result=stageSnapshot(input,limits,current,g);assert.equal(result.status,mask===63&&doc&&src?'staged':'refused');
    assert.deepEqual(result.current,current);assert.deepEqual({input,current,g},before);stageCases++;
  }
  const invalid=(name,input,l=limits)=>{const before=input.slice();check(name,decodeSnapshot(input,l).status!=='decoded');assert.deepEqual(input,before);};
  invalid('root-duplicate',bytes(JSON.stringify(fixture).replace('"version":"1"','"version":"1","version":"1"')));
  invalid('escaped-key-duplicate',bytes(JSON.stringify(fixture).replace('"version":"1"','"version":"1","ver\\u0073ion":"1"')));
  invalid('nested-duplicate',bytes(JSON.stringify(fixture).replace('"domain":"host-source"','"domain":"host-source","domain":"host-source"')));
  invalid('malformed-utf8',new Uint8Array([0xc3]));invalid('json-bom-not-silently-consumed',bytes('\ufeff'+JSON.stringify(fixture)));
  invalid('lone-surrogate',wire({...fixture,records:[{...fixture.records[0],body:'\ud800'}]}));
  invalid('numeric-version-not-coerced',wire({...fixture,version:1}));
  invalid('unknown-version',wire({...fixture,version:'2'}));invalid('unknown-field',wire({...fixture,metadata:{}}));
  invalid('trailing-token',bytes(JSON.stringify(fixture)+' {}'));
  invalid('trailing-comma',bytes(JSON.stringify(fixture).slice(0,-1)+',}'));
  invalid('invalid-string-escape',bytes(JSON.stringify(fixture).replace('"body":"note"','"body":"\\q"')));
  invalid('raw-control-in-string',bytes(JSON.stringify(fixture).replace('"body":"note"','"body":"a\nb"')));
  invalid('prototype-name-not-executable',bytes('{"__proto__":{"polluted":"yes"},'+JSON.stringify(fixture).slice(1)));
  assert.equal({}.polluted,undefined);
  invalid('duplicate-live-id-different-incarnation',wire({...fixture,records:[fixture.records[0],{...fixture.records[0],identity:{...fixture.records[0].identity,incarnation:'2'}}]}));
  for(const key of Object.keys(fixture)){const f=copy(fixture);delete f[key];invalid(`missing-root-${key}`,wire(f));}
  for(const key of Object.keys(fixture.records[0])){const f=copy(fixture);delete f.records[0][key];invalid(`missing-record-${key}`,wire(f));}
  for(const value of ['01','-1','1e3','1.0','0','1\n','1\u2028','900719925474099300000000000000000000']){
    const f=copy(fixture);f.records[0].contentVersion=value;invalid(`invalid-positive-${value}`,wire(f));
  }
  for(const [name,t] of [
    ['empty-range',{kind:'range',source:copy(current.source),start:'1',end:'1'}],
    ['inverted-range',{kind:'range',source:copy(current.source),start:'3',end:'1'}],
    ['beyond-source',{kind:'point',source:copy(current.source),offset:'5',affinity:'before',profile:'position'}],
    ['unknown-affinity',{kind:'point',source:copy(current.source),offset:'1',affinity:'nearest',profile:'content'}]]) {
    const f=copy(fixture);f.records[0].authoredTarget=t;invalid(name,wire(f));
  }
  for(const key of limitKeys)invalid(`exhausted-${key}`,wire(fixture),{...limits,[key]:0});
  invalid('profile-depth-reference-bound',wire(fixture),{...limits,maxDepth:33});
  for(const field of ['authoredTarget','placement'])for(const profile of [undefined,'nearest','Position','',null,1]){
    const f=copy(fixture),t={kind:'point',source:copy(current.source),offset:'1',affinity:'after',profile};
    if(field==='authoredTarget')f.records[0].authoredTarget=t;else f.records[0].placement={state:'attached',target:t};
    invalid(`invalid-point-profile-${field}-${String(profile)}`,wire(f));
  }
  for(const authoredProfile of ['position','content'])for(const locatedProfile of ['position','content']){
    const f=copy(fixture),t={kind:'point',source:copy(current.source),offset:'1',affinity:'after'};
    f.records[0].authoredTarget={...t,profile:authoredProfile};f.records[0].placement={state:'affected',target:{...t,profile:locatedProfile}};
    const staged=stageSnapshot(wire(f),limits,current,gates);check(`independent-profile-fields-${authoredProfile}-${locatedProfile}`,staged.status==='staged'&&staged.candidate.records[0].authoredTarget.profile===authoredProfile&&staged.candidate.records[0].placement.target.profile===locatedProfile);
  }
  for(const affinity of ['before','after']){
    const mapped={};for(const profile of ['position','content']){
      const f=copy(fixture),t={kind:'point',source:copy(current.source),offset:'1',affinity,profile};f.records[0].authoredTarget=t;f.records[0].placement={state:'attached',target:copy(t)};
      const staged=stageSnapshot(wire(f),limits,current,gates);assert.equal(staged.status,'staged');const p=staged.candidate.records[0].placement.target;
      const target={lo:Number(p.offset),hi:Number(p.offset),affinity:p.affinity==='before'?'left':'right',profile:p.profile};
      mapped[profile]=mapAttachment(Array.from(current.bytes),[{lo:0,hi:4,expected:Array.from(current.bytes),ins:[88]}],target);assert.deepEqual(mapped[profile],mapAttachment(Array.from(current.bytes),[{lo:0,hi:4,expected:Array.from(current.bytes),ins:[88]}],{...target,profile}));pointMappingCases++;
    }
    check(`wire-preserves-point-meaning-${affinity}`,mapped.content.evidence==='detached'&&mapped.position.evidence!=='detached');
  }
  const split=copy(fixture);split.records[0].placement.target={kind:'point',source:copy(current.source),offset:'2',affinity:'after',profile:'content'};
  check('scalar-split-not-rounded',stageSnapshot(wire(split),limits,current,gates).status==='refused');
  const authored=copy(fixture);authored.records[0].authoredTarget=copy(split.records[0].placement.target);
  check('current-authored-scalar-split-refused',stageSnapshot(wire(authored),limits,current,gates).status==='refused');
  const inconsistent=copy(fixture);inconsistent.records[0].authoredTarget.source.byteLength='5';
  check('same-source-token-inconsistent-length-refused',stageSnapshot(wire(inconsistent),limits,current,gates).status==='refused');
  const detached=copy(split);detached.records[0].placement={state:'detached'};
  const staged=stageSnapshot(wire(detached),limits,current,gates);
  check('detached-does-not-reattach',staged.status==='staged'&&staged.candidate.records[0].placement.state==='detached');
  check('exact-integer-string-preserved',staged.candidate.records[0].contentVersion==='9007199254740995');
  check('staging-is-not-publication',staged.current.annotationSet==='set1');
  const f=copy(fixture),input=wire(f),owned=stageSnapshot(input,limits,current,gates);input.fill(0);
  check('input-mutation-cannot-rewrite-candidate',owned.candidate.records[0].body==='note');
  assert.throws(()=>{owned.candidate.records[0].body='mutation';});controls.push('candidate-is-immutable');
  const live=copy(current),captured=stageSnapshot(wire(fixture),limits,live,gates);live.annotationSet='set2';
  check('set-dependency-captured-not-reborrowed',captured.dependencies.annotationSet==='set1');
  const emoji=copy(fixture);emoji.records[0].body='😀e\u0301';
  check('scalar-pair-and-decomposed-body-preserved',decodeSnapshot(wire(emoji),limits).value.records[0].body==='😀e\u0301');
  return {status:'annotation-snapshot-reference-only',roundTrips,stageCases,pointMappingCases,controls,
    productionDecoderProved:false,hostIdentityProved:false,publicationRefinement:false};
}
function checkSchemaArtifact() {
  const cases=[{value:copy(fixture),valid:true}];
  for(const placement of [{state:'detached'},{state:'affected',target:copy(docTarget)}]){
    const f=copy(fixture);f.records[0].placement=placement;cases.push({value:f,valid:true});
  }
  for(const target of [
    ...['position','content'].map(profile=>({kind:'point',source:copy(current.source),offset:'1',affinity:'before',profile})),
    {kind:'range',source:copy(current.source),start:'0',end:'4'}]) {
    const f=copy(fixture);f.records[0].authoredTarget=target;cases.push({value:f,valid:true});
  }
  for(const key of Object.keys(fixture)){const f=copy(fixture);delete f[key];cases.push({value:f,valid:false});}
  for(const f of [{...fixture,version:1},{...fixture,version:'2'},{...fixture,metadata:{}},
    {...fixture,records:[{...fixture.records[0],contentVersion:'1\n'}]},
    {...fixture,records:[{...fixture.records[0],placement:{state:'detached',target:docTarget}}]}])cases.push({value:f,valid:false});
  for(const field of ['authoredTarget','placement'])for(const profile of [undefined,'nearest','Position','',null,1]){
    const f=copy(fixture),t={kind:'point',source:copy(current.source),offset:'1',affinity:'after',profile};if(field==='authoredTarget')f.records[0].authoredTarget=t;else f.records[0].placement={state:'attached',target:t};cases.push({value:f,valid:false});
  }
  const program=`import json,sys,importlib.metadata
from jsonschema import Draft202012Validator
assert importlib.metadata.version('jsonschema') == '4.25.1'
p=json.load(sys.stdin)
Draft202012Validator.check_schema(p['schema'])
v=Draft202012Validator(p['schema'])
for c in p['cases']:
    assert v.is_valid(c['value']) == c['valid']
print(json.dumps({'status':'annotation-jsonschema-artifact-only','cases':len(p['cases']),'version':'4.25.1'}))
`;
  need(process.env.VELLUM_JSONSCHEMA_PYTHON);
  const p=spawnSync(process.env.VELLUM_JSONSCHEMA_PYTHON,['-c',program],{input:JSON.stringify({schema,cases}),encoding:'utf8',timeout:30000});
  if(p.status!==0||p.error)throw Error(`Pinned schema check failed: ${p.stderr||p.error||p.status}`);
  const r=JSON.parse(p.stdout);need(r.cases===28&&r.status==='annotation-jsonschema-artifact-only');return r;
}
export async function checkAnnotationSnapshotMutations(){
  const raw=fs.readFileSync(new URL('./annotation-snapshot.mjs',import.meta.url),'utf8'),mutations=[
    ['unknown-point-profile-admitted',"need(['position','content'].includes(t.profile));",''],
    ['profile-defaulted-on-decode','const value=parseProfile(input,limits);',"const value=parseProfile(input,limits);for(const r of value.records??[])for(const t of [r.authoredTarget,r.placement?.target])if(t?.kind==='point'&&t.profile===undefined)t.profile='position';"],
    ['content-coerced-to-position','const value=parseProfile(input,limits);',"const value=parseProfile(input,limits);for(const r of value.records??[])for(const t of [r.authoredTarget,r.placement?.target])if(t?.kind==='point')t.profile='position';"],
    ['staged-profile-erased','candidate:freeze(copy(v))',"candidate:freeze(JSON.parse(JSON.stringify(v,(key,value)=>key==='profile'?undefined:value)))"],
    ['staged-profile-swapped','candidate:freeze(copy(v))',"candidate:freeze(JSON.parse(JSON.stringify(v,(key,value)=>key==='profile'?(value==='position'?'content':'position'):value)))"]
  ];
  const load=async(m=null)=>{let source=raw;if(m){assert(source.includes(m[1]),m[0]);source=source.replace(m[1],m[2]);}source=source.replace(/from '(\.\/[^']+)'/g,(_,r)=>`from ${JSON.stringify(new URL(r,import.meta.url).href)}`).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,r)=>`new URL(${JSON.stringify(new URL(r,import.meta.url).href)},import.meta.url)`);return import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}#${m?.[0]??'baseline'}`);};
  (await load()).checkAnnotationSnapshot();const killed=[];for(const m of mutations){let failed=false;try{(await load(m)).checkAnnotationSnapshot();}catch(e){assert.equal(e.code,'ERR_ASSERTION',m[0]+': '+e.message);failed=true;}assert(failed,m[0]);killed.push(m[0]);}return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(process.argv.includes('--schema-check')?checkSchemaArtifact():{...checkAnnotationSnapshot(),mutations:await checkAnnotationSnapshotMutations(),mutationBaselineChecked:true},null,2));
