// First-admission design reference, not a production annotation coordinator.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {changeAnnotation} from './annotation-records.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./annotation-operations.json',import.meta.url),'utf8'));
const clone=structuredClone;
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
const exact=(v,keys)=>object(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const text=v=>typeof v==='string'&&[...v].every(c=>{const n=c.codePointAt(0);return n<0xd800||n>0xdfff;});
const label=v=>text(v)&&v.length>0;
const version=v=>Number.isSafeInteger(v)&&v>0; // Private finite fixture encoding.
const doc=v=>exact(v,['authority','id','incarnation'])&&label(v.authority)&&label(v.id)&&version(v.incarnation);
const annotation=v=>exact(v,['document','id','incarnation'])&&doc(v.document)&&label(v.id)&&version(v.incarnation);
const target=v=>exact(v,['document','sourceRevision','point','affinity'])&&doc(v.document)&&version(v.sourceRevision)&&
  Number.isSafeInteger(v.point)&&v.point>=0&&['before','after'].includes(v.affinity);
const dependency=v=>exact(v,catalog.dependencyFields)&&label(v.domain)&&label(v.identity)&&version(v.generation);
function shape(r,limits) {
  if(!limits||!['maxDependencies','maxBodyBytes'].every(k=>Number.isSafeInteger(limits[k])&&limits[k]>=0))return false;
  if(!object(r)||r.contract!==catalog.contract||!Object.hasOwn(catalog.variants,r.operation))return false;
  if(!exact(r,[...Object.keys(catalog.common),...Object.keys(catalog.variants[r.operation])]))return false;
  if(!label(r.invocation)||!doc(r.document)||!label(r.profile)||!Array.isArray(r.dependencies)||r.dependencies.length>limits.maxDependencies||!r.dependencies.every(dependency))return false;
  if(['create','update-body'].includes(r.operation)&&(typeof r.body!=='string'||r.body.length>limits.maxBodyBytes||
      !text(r.body)||new TextEncoder().encode(r.body).length>limits.maxBodyBytes))return false;
  if(['create','reattach'].includes(r.operation)&&(!target(r.target)||!equal(r.target.document,r.document)))return false;
  if(r.operation!=='create'&&(!annotation(r.annotation)||!equal(r.annotation.document,r.document)))return false;
  return (!Object.hasOwn(r,'expectedContent')||version(r.expectedContent))&&(!Object.hasOwn(r,'expectedTarget')||version(r.expectedTarget));
}
export function mergeDependencies(required,extra,current,maximum=Infinity) {
  const merged=[];
  for(const d of [...required,...extra]) {
    const old=merged.find(x=>x.domain===d.domain&&x.identity===d.identity);
    if(old&&old.generation!==d.generation)return false;
    if(!old)merged.push(d);
    if(merged.length>maximum)return false;
  }
  return merged.every(d=>{
    const matches=current.filter(x=>x.domain===d.domain&&x.identity===d.identity);
    return matches.length>0&&matches.every(x=>equal(x,d));
  });
}
export function decideAnnotation(state,request,host) {
  const refused=()=>({status:'not-published',state});
  if(state.live===false||!shape(request,host.limits)||!equal(request.document,state.document)||request.profile!==host.profile||
      !['documentLive','authorized','bounded','notStopped','interpretationCurrent','companionsReady'].every(k=>host[k]))return refused();
  if(!mergeDependencies(host.requiredDependencies,request.dependencies,state.contexts,host.limits.maxDependencies))return refused();
  const create=request.operation==='create';
  if(create&&(!host.allocationValid||!exact(host.fresh,['id','incarnation'])||!label(host.fresh.id)||!version(host.fresh.incarnation)))return refused();
  if(['create','reattach'].includes(request.operation)&&(!host.targetCurrent||request.target.sourceRevision!==state.sourceRevision))return refused();
  const lowered=create?{kind:'create',...host.fresh,body:request.body,target:request.target}:
    {kind:{'update-body':'body',reattach:'reattach',delete:'delete'}[request.operation],
      id:request.annotation.id,incarnation:request.annotation.incarnation,
      ...(Object.hasOwn(request,'expectedContent')?{expectedBody:request.expectedContent}:{}),
      ...(Object.hasOwn(request,'expectedTarget')?{expectedTarget:request.expectedTarget}:{}),
      ...(Object.hasOwn(request,'body')?{body:request.body}:{}),...(Object.hasOwn(request,'target')?{target:request.target}:{})};
  const result=changeAnnotation(state,lowered,{authorized:true,bounded:true,notStopped:true,targetCurrent:host.targetCurrent});
  if(result.status==='refused')return refused();
  if(result.status==='unchanged')return host.completeIdentity?{status:'unchanged',state}:refused();
  if(!host.publicationVersionAvailable||!host.setVersionAvailable||
      (['create','update-body'].includes(request.operation)&&!host.contentVersionAvailable)||
      (['create','reattach'].includes(request.operation)&&!host.targetVersionAvailable))return refused();
  return {status:'applied',state:{...result.state,annotationSet:state.annotationSet+1,publication:state.publication+1}};
}

const document={authority:'local',id:'doc',incarnation:1};
const point={document,sourceRevision:1,point:0,affinity:'after'};
const record={id:'note',incarnation:1,body:'old',bodyVersion:1,target:clone(point),targetGeneration:1};
const state={document,source:'same source',sourceRevision:1,record,retired:[],annotationSet:1,publication:1,
  contexts:[{domain:'interpretation',identity:'annotations',generation:1},{domain:'explicit-read',identity:'other',generation:1}]};
const host={profile:'reference',limits:{maxDependencies:2,maxBodyBytes:64},documentLive:true,authorized:true,bounded:true,notStopped:true,interpretationCurrent:true,companionsReady:true,
  completeIdentity:true,targetCurrent:true,allocationValid:true,fresh:{id:'fresh',incarnation:1},
  requiredDependencies:[state.contexts[0]],publicationVersionAvailable:true,setVersionAvailable:true,contentVersionAvailable:true,targetVersionAvailable:true};
const common={contract:catalog.contract,document,profile:host.profile,invocation:'invocation',dependencies:[]};
const ref={document,id:record.id,incarnation:record.incarnation};
const requests=[{...common,operation:'create',body:'',target:point},
  {...common,operation:'update-body',annotation:ref,expectedContent:1,body:'new'},
  {...common,operation:'reattach',annotation:ref,expectedTarget:1,target:{...point,point:1}},
  {...common,operation:'delete',annotation:ref,expectedContent:1,expectedTarget:1}];
export function checkAnnotationOperations() {
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  assert.equal(catalog.family,'F13');assert.deepEqual(Object.keys(catalog.variants).sort(),families.find(f=>f.id==='F13').variants.toSorted());
  assert.equal(catalog.contract,'annotation-authoring/1');
  assert.deepEqual(Object.keys(catalog.results).sort(),['applied','not-published','unchanged']);
  let gateCases=0;const controls=[];
  const control=(name,ok)=>{assert(ok,name);controls.push(name);};
  const stateFor=r=>r.operation==='create'?{...clone(state),record:null}:clone(state);
  const reject=(name,r,s=stateFor(r),h=host)=>{
    const before=clone({s,r,h}),out=decideAnnotation(s,r,h);assert.equal(out.status,'not-published');
    assert.deepEqual(out.state,s);assert.deepEqual({s,r,h},before);controls.push(name);
  };
  const gateKeys=['documentLive','authorized','bounded','notStopped','interpretationCurrent','companionsReady','publicationVersionAvailable','setVersionAvailable'];
  for(const r of requests)for(let mask=0;mask<256;mask++) {
    const s=stateFor(r),h={...clone(host),...Object.fromEntries(gateKeys.map((k,i)=>[k,!!(mask&(1<<i))]))};
    const before=clone({s,r,h}),out=decideAnnotation(s,r,h);assert.equal(out.status,mask===255?'applied':'not-published');
    assert.equal(out.state.source,s.source);assert.equal(out.state.sourceRevision,s.sourceRevision);
    if(out.status==='applied'){
      const row=r.operation==='create'?{id:h.fresh.id,incarnation:h.fresh.incarnation,body:r.body,bodyVersion:1,target:clone(r.target),targetGeneration:1}:
        r.operation==='update-body'?{...clone(s.record),body:r.body,bodyVersion:s.record.bodyVersion+1}:
        r.operation==='reattach'?{...clone(s.record),target:clone(r.target),targetGeneration:s.record.targetGeneration+1}:null;
      const retired=r.operation==='delete'?[...clone(s.retired),clone(s.record)]:clone(s.retired);
      assert.deepEqual(out.state,{...s,record:row,retired,annotationSet:s.annotationSet+1,publication:s.publication+1});
    }
    else assert.deepEqual(out.state,s);
    assert.deepEqual({s,r,h},before);gateCases++;
  }
  for(const r of requests)for(const key of Object.keys(r)){const bad=clone(r);delete bad[key];reject(`missing-${r.operation}-${key}`,bad,stateFor(r));}
  reject('unknown-verb',{...requests[1],operation:'execute-body'});
  reject('unknown-contract',{...requests[1],contract:'annotation-authoring/2'});
  reject('unknown-field',{...requests[1],metadata:{}});
  reject('foreign-authority',{...requests[1],document:{...document,authority:'other'}});
  reject('foreign-record-owner',{...requests[1],annotation:{...ref,document:{...document,id:'other'}}});
  reject('old-record-incarnation',{...requests[1],annotation:{...ref,incarnation:2}});
  reject('stale-content',{...requests[1],expectedContent:2});
  reject('stale-target',{...requests[2],expectedTarget:2});
  reject('delete-needs-content',{...requests[3],expectedContent:2});
  reject('delete-needs-target',{...requests[3],expectedTarget:2});
  reject('invalid-scalar-body',{...requests[1],body:'\ud800'});
  reject('body-byte-bound',requests[1],state,{...host,limits:{...host.limits,maxBodyBytes:2}});
  reject('multibyte-body-byte-bound',{...requests[1],body:'😀'},state,{...host,limits:{...host.limits,maxBodyBytes:3}});
  reject('dependency-input-bound',{...requests[1],dependencies:[state.contexts[0],state.contexts[0],state.contexts[0]]});
  reject('required-and-extra-union-bound',{...requests[1],dependencies:[state.contexts[1]]},state,{...host,limits:{...host.limits,maxDependencies:1}});
  reject('foreign-target',{...requests[2],target:{...point,document:{...document,id:'other'}}});
  reject('stale-source-target',{...requests[2],target:{...point,sourceRevision:2}});
  reject('source-target-proof-required',requests[2],state,{...host,targetCurrent:false});
  reject('fresh-identity-proof-required',requests[0],stateFor(requests[0]),{...host,allocationValid:false});
  reject('create-target-proof-required',requests[0],stateFor(requests[0]),{...host,targetCurrent:false});
  reject('create-content-counter-required',requests[0],stateFor(requests[0]),{...host,contentVersionAvailable:false});
  reject('create-target-counter-required',requests[0],stateFor(requests[0]),{...host,targetVersionAvailable:false});
  reject('caller-cannot-choose-live-create-identity',{...requests[0],id:'chosen',incarnation:1},stateFor(requests[0]));
  reject('unknown-profile',{...requests[1],profile:'unknown'});
  reject('retired-incarnation-not-revived',requests[0],{...stateFor(requests[0]),retired:[{id:'fresh',incarnation:1}]});
  reject('mandatory-dependency-cannot-be-omitted',requests[1],{...state,contexts:[]});
  reject('ambiguous-current-dependency-refused',requests[1],{...state,contexts:[...state.contexts,{...state.contexts[0],generation:2}]});
  reject('conflicting-duplicate-dependency',{...requests[1],dependencies:[{...state.contexts[0],generation:2}]});
  reject('extra-read-dependency-stale',{...requests[1],dependencies:[{...state.contexts[1],generation:2}]});
  control('equivalent-dependencies-idempotent',equal(decideAnnotation(state,requests[1],host),
    decideAnnotation(state,{...requests[1],dependencies:[state.contexts[0],state.contexts[0]]},host)));
  const updated=decideAnnotation(state,requests[1],host).state;
  control('body-preserves-target',equal(updated.record.target,state.record.target)&&updated.record.targetGeneration===1);
  const moved=decideAnnotation(state,requests[2],host).state;
  control('target-preserves-body',moved.record.body==='old'&&moved.record.bodyVersion===1);
  control('body-does-not-require-unrelated-target-freshness',decideAnnotation({...state,sourceRevision:2},requests[1],{...host,targetCurrent:false,targetVersionAvailable:false}).status==='applied');
  control('target-does-not-require-unrelated-body-counter',decideAnnotation(state,requests[2],{...host,contentVersionAvailable:false}).status==='applied');
  const deleted=decideAnnotation(state,requests[3],{...host,contentVersionAvailable:false,targetVersionAvailable:false});
  control('delete-retains-record-without-incrementing-it',deleted.status==='applied'&&deleted.state.record===null&&deleted.state.retired[0].body==='old');
  const identity={...requests[1],body:'old'},noop=decideAnnotation(state,identity,{...host,publicationVersionAvailable:false,setVersionAvailable:false,contentVersionAvailable:false});
  control('complete-noop-needs-no-new-versions',noop.status==='unchanged'&&equal(noop.state,state));
  reject('equal-body-not-complete-identity',identity,state,{...host,completeIdentity:false});
  reject('noop-still-needs-authority',identity,state,{...host,authorized:false});
  reject('noop-still-stops',identity,state,{...host,notStopped:false});
  reject('noop-still-needs-mandatory-dependency',identity,{...state,contexts:[]});
  const sameTarget={...requests[2],target:clone(point)};
  reject('equal-target-not-complete-identity',sameTarget,state,{...host,completeIdentity:false});
  const targetNoop=decideAnnotation(state,sameTarget,{...host,publicationVersionAvailable:false,setVersionAvailable:false,targetVersionAvailable:false});
  control('target-noop-preserves-complete-state',targetNoop.status==='unchanged'&&equal(targetNoop.state,state));
  control('source-write-not-required-for-comment-body',decideAnnotation(state,requests[1],{...host,sourceWritable:false}).status==='applied');
  reject('content-counter-exhaustion',requests[1],state,{...host,contentVersionAvailable:false});
  reject('target-counter-exhaustion',requests[2],state,{...host,targetVersionAvailable:false});
  for(const r of requests)control(`diagnostic-failure-not-authority-${r.operation}`,
    equal(decideAnnotation(stateFor(r),r,host),decideAnnotation(stateFor(r),r,{...host,diagnosticFailed:true})));
  reject('serialized-snapshot-is-not-command',{format:'vellum.annotations',version:'1',records:[]},state);
  const targetChanged=clone(state);targetChanged.record.targetGeneration=2;targetChanged.record.target={...point,point:2};
  targetChanged.annotationSet=2;targetChanged.publication=2;
  const lateBody=decideAnnotation(targetChanged,requests[1],host);
  control('body-merges-current-target-not-prepared-copy',lateBody.status==='applied'&&lateBody.state.record.target.point===2&&
    lateBody.state.record.targetGeneration===2&&lateBody.state.annotationSet===3&&lateBody.state.publication===3);
  return {status:'annotation-operation-products-only',variants:requests.length,gateCases,controls,
    fullCompanionsProved:false,replayRefinement:false,wireDecoderProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(checkAnnotationOperations(),null,2));
