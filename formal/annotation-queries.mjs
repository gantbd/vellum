// Normalized owned-value reference. Host scope/visibility truth is a premise.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {compoundTargetShape} from './annotation-targets.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./annotation-queries.json',import.meta.url),'utf8'));
const clone=structuredClone;
const bytes=v=>Buffer.byteLength(JSON.stringify(v),'utf8'); // Fixture charge, not a wire size.
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const unavailable=()=>({status:'unavailable'});
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};

// Fixed metadata from the same declaration used by the native F03 reader.
export function annotationQueryInputForms(){return freeze(Object.entries(catalog.variants).map(([operation,fields])=>({family:catalog.family,contract:catalog.contract,variant:operation,operation,fields:[...Object.keys(catalog.common),...Object.keys(fields)]})));}

// Native owned-value products only. Shape does not authenticate an opaque alias,
// establish current grants or verify a placement against source bytes.
const nativeResultFields=freeze(Object.fromEntries(Object.entries(catalog.results).map(([status,fields])=>[status,['status',...fields]])));
const nativeValueFields=freeze({recordRequired:['reference'],recordOptional:['content','location'],content:['body','version'],location:['placement','targetGeneration'],detachedPlacement:['state'],attachedPlacement:['state','sourceRevision','target'],attachedStates:['attached','affected']});
const nativeExact=(v,keys)=>{if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const d=Object.getOwnPropertyDescriptors(v),ks=Reflect.ownKeys(d);return ks.length===keys.length&&keys.every(k=>Object.hasOwn(d,k)&&Object.hasOwn(d[k],'value')&&d[k].enumerable);};
const natural=n=>Number.isSafeInteger(n)&&n>=0,opaque=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
function nativeLocation(v){if(!nativeExact(v,nativeValueFields.location)||!natural(v.targetGeneration))return false;const p=v.placement;if(p?.state==='detached')return nativeExact(p,nativeValueFields.detachedPlacement);return nativeValueFields.attachedStates.includes(p?.state)&&nativeExact(p,nativeValueFields.attachedPlacement)&&natural(p.sourceRevision)&&compoundTargetShape(p.target);}
function nativeRecord(v){if(!v||typeof v!=='object')return false;const fields=[...nativeValueFields.recordRequired,...nativeValueFields.recordOptional.filter(k=>Object.hasOwn(v,k))];return nativeExact(v,fields)&&opaque(v.reference)&&(!Object.hasOwn(v,'content')||nativeExact(v.content,nativeValueFields.content)&&typeof v.content.body==='string'&&natural(v.content.version))&&(!Object.hasOwn(v,'location')||nativeLocation(v.location));}
export function annotationQueryResultForms(){return nativeResultFields;}
export function annotationQueryResultValueForms(){return nativeValueFields;}
export function annotationQueryResultValid(out,request){
 if(!out||typeof out.status!=='string'||!Object.hasOwn(nativeResultFields,out.status)||!nativeExact(out,nativeResultFields[out.status]))return false;
 if(out.status==='unavailable')return true;if(out.snapshot!==request.snapshot)return false;
 if(out.status==='page'){if(request.operation!=='enumerate'||!Array.isArray(out.records)||out.records.length>request.limit||!(out.continuation===null||opaque(out.continuation)))return false;for(let i=0;i<out.records.length;i++){if(!Object.hasOwn(out.records,i)||!nativeRecord(out.records[i])||out.records[i].reference===out.continuation)return false;for(let j=0;j<i;j++)if(out.records[j].reference===out.records[i].reference)return false;}return out.continuation===null||out.records.length===request.limit;}
 if(out.status==='record')return request.operation==='inspect'&&nativeRecord(out.record)&&out.record.reference===request.annotation;
 return request.operation==='target-location'&&out.reference===request.annotation&&nativeLocation(out.location);
}

export function captureAnnotations(world,scope,limits) {
  if(world.length>limits.maxInputRows)return null;
  const rows=world.filter(r=>r.visible).map(r=>({reference:r.reference,
    ...(r.bodyVisible?{content:{body:r.body,version:r.contentVersion}}:{}),
    ...(r.targetVisible?{location:clone(r.location)}:{})}));
  // References/order are supplied projection-scoped identities, not global indices.
  rows.sort((a,b)=>a.reference<b.reference?-1:a.reference>b.reference?1:0);
  if(new Set(rows.map(r=>r.reference)).size!==rows.length||
      rows.some(r=>bytes(r)>limits.maxRowBytes)||bytes(rows)>limits.maxCaptureBytes)return null;
  return freeze({scope:clone(scope),rows:clone(rows),limits:clone(limits)});
}

// Each entry is a retained opaque selector binding the exact query and position.
// Issuance/entropy, actual registry lookup and transfer serialization are outside this reference.
export function readAnnotations(capture,request,current,cursors=[]) {
  if(!capture||!request||!Object.hasOwn(catalog.variants,request.operation)||
      !exact(request,[...Object.keys(catalog.common),...Object.keys(catalog.variants[request.operation])])||
      request.contract!==catalog.contract)return unavailable();
  const {scope,rows,limits}=capture;
  if(!equal(request.document,scope.document)||request.snapshot!==scope.snapshot||request.profile!==scope.profile||
      !equal(current.scope,scope)||!current.granted||!current.retained||!current.live||
      !current.continuous||!current.transferReady)return unavailable();
  if(request.operation==='enumerate') {
    if(!Number.isSafeInteger(request.limit)||request.limit<1||request.limit>limits.maxPageRows)return unavailable();
    let start=0;
    if(request.continuation!==null){
      const entries=cursors.filter(c=>c.selector===request.continuation);
      if(entries.length!==1)return unavailable();
      const c=entries[0];
      if(!equal(c.scope,scope)||c.limit!==request.limit||!Number.isSafeInteger(c.next)||c.next<1||c.next>=rows.length)return unavailable();
      start=c.next;
    }
    const end=Math.min(start+request.limit,rows.length),next=end<rows.length?end:null;
    // next is private issuance intent, never the caller-facing continuation token.
    return {status:'page',snapshot:scope.snapshot,records:clone(rows.slice(start,end)),next};
  }
  const row=rows.find(r=>r.reference===request.annotation);
  if(!row)return unavailable();
  if(request.operation==='inspect')return {status:'record',snapshot:scope.snapshot,record:clone(row)};
  return Object.hasOwn(row,'location')?
    {status:'location',snapshot:scope.snapshot,reference:row.reference,location:clone(row.location)}:unavailable();
}

export function checkAnnotationQueries(){
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  assert.deepEqual(Object.keys(catalog.variants).sort(),families.find(f=>f.id==='F03').variants.toSorted());
  assert.equal(catalog.contract,'annotation-query/1');
  const limits={maxInputRows:8,maxCaptureBytes:4096,maxRowBytes:512,maxPageRows:2};
  const scope={document:{authority:'a',id:'d',incarnation:1},snapshot:'s',profile:'p',
    recipient:'reader',purpose:'display',destination:'view',policy:'epoch1',continuity:'c1'};
  const location={state:'attached',source:'source1',targetGeneration:1,target:{kind:'point',offset:0,affinity:'before'}};
  const row=(reference,visible=true)=>({reference,visible,bodyVisible:true,targetVisible:true,
    body:'comment',contentVersion:1,location:clone(location)});
  const world=[row('r3'),row('hidden',false),row('r1'),row('r2')];
  const cap=captureAnnotations(world,scope,limits);
  const current={scope:clone(scope),granted:true,retained:true,live:true,continuous:true,transferReady:true};
  const common={contract:catalog.contract,document:clone(scope.document),snapshot:'s',profile:'p'};
  const enumeration={...common,operation:'enumerate',limit:2,continuation:null};
  const inspect={...common,operation:'inspect',annotation:'r1'};
  const locate={...common,operation:'target-location',annotation:'r1'};
  const requests=[enumeration,inspect,locate],controls=[];
  const control=(name,ok)=>{assert(ok,name);controls.push(name);};
  const deny=(name,r=enumeration,c=current,capture=cap,cursors=[])=>control(name,equal(readAnnotations(capture,r,c,cursors),unavailable()));
  let releaseCases=0,pairedCases=0,traversalCases=0;
  const gates=['granted','retained','live','continuous','transferReady'];
  for(const r of requests)for(let mask=0;mask<32;mask++){
    const c={...clone(current),...Object.fromEntries(gates.map((k,i)=>[k,!!(mask&(1<<i))]))};
    const before=clone({capture:cap,r,c}),out=readAnnotations(cap,r,c);
    const expected=r.operation==='enumerate'?{status:'page',snapshot:'s',records:clone(cap.rows.slice(0,2)),next:2}:
      r.operation==='inspect'?{status:'record',snapshot:'s',record:clone(cap.rows[0])}:
      {status:'location',snapshot:'s',reference:'r1',location:clone(location)};
    assert.deepEqual(out,mask===31?expected:unavailable());
    assert.deepEqual({capture:cap,r,c},before);releaseCases++;
  }
  for(let visibility=0;visibility<16;visibility++)for(let secret=0;secret<8;secret++){
    const a=[row('r1'),row('r2'),row('r3')];
    a[0].bodyVisible=!!(visibility&1);a[0].targetVisible=!!(visibility&2);
    a[1].bodyVisible=!!(visibility&4);a[1].targetVisible=!!(visibility&8);
    const hidden=row(`private${secret}`,false);hidden.body='secret'.repeat(secret);
    const left=captureAnnotations(a,scope,limits),right=captureAnnotations([hidden,...clone(a)],scope,limits);
    for(let i=0;i<2;i++){
      assert.equal(Object.hasOwn(left.rows[i],'content'),a[i].bodyVisible);
      assert.equal(Object.hasOwn(left.rows[i],'location'),a[i].targetVisible);
    }
    assert.deepEqual(left,right);
    for(const r of requests){assert.deepEqual(readAnnotations(left,r,current),readAnnotations(right,r,current));pairedCases++;}
    const entry={selector:'paired-next',scope:clone(scope),limit:2,next:2};
    const continuation={...enumeration,continuation:entry.selector};
    assert.deepEqual(readAnnotations(left,continuation,current,[entry]),readAnnotations(right,continuation,current,[entry]));pairedCases++;
  }
  for(let n=0;n<=8;n++)for(let limit=1;limit<=2;limit++){
    const capture=captureAnnotations(Array.from({length:n},(_,i)=>row(`r${i}`)),scope,limits);
    const collected=[],cursors=[];let continuation=null,previous=0;
    for(let page=0;page<=n;page++){
      const result=readAnnotations(capture,{...enumeration,limit,continuation},current,cursors);
      assert.equal(result.status,'page');collected.push(...result.records);
      if(result.next===null)break;
      assert(result.next>previous&&result.next<n);previous=result.next;
      continuation=`cursor-${page}`;cursors.push({selector:continuation,scope:clone(scope),limit,next:result.next});
    }
    assert.deepEqual(collected,capture.rows);traversalCases++;
  }
  const cursor={selector:'opaque-next',scope:clone(scope),limit:2,next:2};
  const next={...enumeration,continuation:'opaque-next'};
  control('complete-visible-traversal',equal(readAnnotations(cap,next,current,[cursor]),
    {status:'page',snapshot:'s',records:[clone(cap.rows[2])],next:null}));
  control('empty-is-established-visible-end',equal(readAnnotations(captureAnnotations([],scope,limits),enumeration,current),
    {status:'page',snapshot:'s',records:[],next:null}));
  for(const key of Object.keys(scope)){
    const changed={...clone(scope),[key]:key==='document'?{...scope.document,incarnation:2}:'other'};
    deny(`current-scope-${key}`,next,{...current,scope:changed},cap,[cursor]);
    deny(`cursor-scope-${key}`,next,current,cap,[{...cursor,scope:changed}]);
  }
  deny('regrant-after-gap-is-not-continuity',next,{...current,scope:{...scope,policy:'epoch2',continuity:'c2'}},cap,[cursor]);
  deny('cursor-not-in-registry',next);
  deny('ambiguous-cursor',next,current,cap,[cursor,clone(cursor)]);
  deny('cursor-query-limit-bound',{...next,limit:1},current,cap,[cursor]);
  deny('cursor-end-is-not-extra-empty-page',next,current,cap,[{...cursor,next:3}]);
  deny('cursor-cannot-restart',next,current,cap,[{...cursor,next:0}]);
  deny('cursor-cannot-use-raw-offset',{...enumeration,continuation:2});
  deny('zero-limit',{...enumeration,limit:0});deny('over-limit',{...enumeration,limit:3});
  deny('fractional-limit',{...enumeration,limit:1.5});
  deny('unknown-contract',{...enumeration,contract:'annotation-query/2'});
  deny('unknown-variant',{...enumeration,operation:'execute'});
  deny('unknown-field',{...enumeration,offset:1});
  deny('missing-snapshot',Object.fromEntries(Object.entries(enumeration).filter(([k])=>k!=='snapshot')));
  deny('foreign-request-owner',{...enumeration,document:{...scope.document,id:'other'}});
  deny('new-snapshot-cannot-substitute',{...enumeration,snapshot:'s2'});
  deny('unknown-profile',{...enumeration,profile:'other'});
  deny('missing-record',{...inspect,annotation:'absent'});
  deny('hidden-record',{...inspect,annotation:'hidden'});
  deny('hidden-record-location',{...locate,annotation:'hidden'});
  const withheld=clone(world);withheld[2].bodyVisible=false;withheld[2].targetVisible=false;
  const restricted=captureAnnotations(withheld,scope,limits);
  control('withheld-fields-not-null-or-empty',equal(readAnnotations(restricted,inspect,current),
    {status:'record',snapshot:'s',record:{reference:'r1'}}));
  deny('withheld-location-common-unavailable',locate,current,restricted);
  const detached=clone(world);detached[2].location={state:'detached',targetGeneration:1};
  control('detached-is-not-withheld',equal(readAnnotations(captureAnnotations(detached,scope,limits),locate,current),
    {status:'location',snapshot:'s',reference:'r1',location:{state:'detached',targetGeneration:1}}));
  control('capture-owns-old-body-and-placement',equal(readAnnotations(cap,inspect,current).record,cap.rows[0])&&cap.rows[0].content.body==='comment'&&cap.rows[0].location.source==='source1');
  world[2].body='mutated';world[2].location.source='later';
  control('input-alias-cannot-change-capture',cap.rows[0].content.body==='comment'&&cap.rows[0].location.source==='source1');
  const output=readAnnotations(cap,inspect,current);output.record.content.body='recipient mutation';
  control('output-alias-cannot-change-capture',cap.rows[0].content.body==='comment');
  control('owned-capture-frozen',Object.isFrozen(cap.rows[0].location.target));
  control('input-row-bound',captureAnnotations(Array.from({length:9},(_,i)=>row(`r${i}`)),scope,limits)===null);
  control('capture-byte-bound',captureAnnotations([row('r1')],scope,{...limits,maxCaptureBytes:1})===null);
  control('row-byte-bound',captureAnnotations([row('r1')],scope,{...limits,maxRowBytes:1})===null);
  control('duplicate-visible-reference-refused',captureAnnotations([row('r1'),row('r1')],scope,limits)===null);
  deny('failed-capture-not-empty',enumeration,current,null);
  control('diagnostic-failure-not-authority',equal(readAnnotations(cap,inspect,current),readAnnotations(cap,inspect,{...current,diagnosticFailed:true})));
  return {status:'annotation-query-products-only',variants:3,releaseCases,pairedCases,traversalCases,controls,
    hostPolicyProved:false,cursorRegistryProved:false,implementationRefinement:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(checkAnnotationQueries(),null,2));
