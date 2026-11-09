import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as native from './annotation-editing.mjs';
import * as kernel from './invocation-publication.mjs';
import * as reversal from './history-reversal.mjs';
import * as observer from './publication-observation.mjs';
import * as bridge from './observation-reads.mjs';
import * as reads from './read-contexts.mjs';
import {annotationObservationFixture,annotationObservationContext as writer,annotationObservationPolicy as policy,annotationObservationGates as good} from './annotation-observation-checks.mjs';
const original={...native,...kernel,...reversal,...observer,...bridge,...reads},copy=structuredClone;
const catalog=JSON.parse(fs.readFileSync(new URL('./annotation-reads.json',import.meta.url))),rc=JSON.parse(fs.readFileSync(new URL('./observation-reads.json',import.meta.url)));
const document={authority:'a',id:'A',incarnation:1},unavailable={status:'unavailable'};
const limits={contexts:8,units:4096,selectors:64,holds:8,issuance:256,sourceBytes:1024,source:{maxRootBytes:1024,maxInputBytes:1024,maxContentBytes:1024,maxReplyBytes:8192},annotation:{maxInputRows:32,maxCaptureBytes:16384,maxRowBytes:8192,maxPageRows:8}};
const targetForms=[{kind:'document'},{kind:'range',start:1,end:3},...['position','content'].flatMap(profile=>['before','after'].map(affinity=>({kind:'point',offset:1,affinity,profile})))];
const variants=[{operation:'update-body'},...targetForms.map(target=>({operation:'create',target})),...targetForms.map(target=>({operation:'reattach',target})),{operation:'delete'}];
const release={...good,...Object.fromEntries(rc.releaseGates.map(k=>[k,true]))};
function fixture(api=original,options={}){
 const z=annotationObservationFixture(api,options),c=z.c;
 z.host={document:copy(document),recipient:c.recipient,purpose:c.purpose,destination:c.destination,policy:c.policy,continuity:c.continuity,profile:c.profile,...Object.fromEntries([...rc.readGates,'replyReady'].map(k=>[k,true]))};
 z.request={contract:'read-context/1',operation:'snapshot-handoff',document:copy(document),selection:{kind:'capture-current'},data:['annotations'],profile:c.profile};
 z.registry=api.newReadRegistry('read-issuer',limits);
 z.acquire=(m=z.registry,h=z.host,g=release,r=z.request,rl=limits)=>api.acquireObservationRead(m,z.owner,z.current(),r,c,h,rl,g,z.packet(z.current()));
 return z;
}
const query=(z,context,operation,fields={})=>({contract:'annotation-query/1',operation,document:copy(document),snapshot:context,profile:z.c.profile,...fields});
function enumerate(api,z,out,limit=1,host=z.host){
 assert.equal(out.result.status,'context');let m=out.state,continuation=null,records=[];
 for(let n=0;n<=32;n++){
  const q=api.queryReadContext(m,query(z,out.result.context,'enumerate',{limit,continuation}),host);assert.equal(q.result.status,'page');m=q.state;records.push(...q.result.records);continuation=q.result.continuation;
  if(continuation===null)return {state:m,result:{...q.result,records},context:out.result.context};
 }
 assert.fail('bounded complete enumeration did not terminate');
}
function checkCapture(api,z,out,expectedRoot=z.current(),visible=()=>true){
 const facts=z.facts(expectedRoot),q=enumerate(api,z,out),expected=expectedRoot.documents[0].records.filter(visible).map(r=>{const fact=facts.find(f=>f.id===r.id&&f.incarnation===r.incarnation);return {...(z.c.documents[0].collection.body&&fact.body?{content:{body:r.body,version:r.bodyVersion}}:{}),...(z.c.documents[0].collection.target&&fact.target?{location:{placement:copy(r.placement),targetGeneration:r.targetGeneration}}:{})};});
 const normalized=q.result.records.map(({reference,...row})=>row),sort=xs=>xs.map(x=>JSON.stringify(x)).sort();assert.deepEqual(sort(normalized),sort(expected));
 for(const row of q.result.records){assert.equal(typeof row.reference,'string');assert(row.reference.startsWith('read-issuer:private-'));assert(!['note','fresh','annotation-1','annotation-2'].includes(row.reference));
  const inspected=api.queryReadContext(q.state,query(z,q.context,'inspect',{annotation:row.reference}),z.host);assert.deepEqual(inspected.result,{status:'record',snapshot:q.context,record:row});
  const located=api.queryReadContext(q.state,query(z,q.context,'target-location',{annotation:row.reference}),z.host);assert.deepEqual(located.result,Object.hasOwn(row,'location')?{status:'location',snapshot:q.context,reference:row.reference,location:row.location}:unavailable);
 }
 return q;
}
const nativeTarget=p=>p.request.operation==='create'?{document:copy(document),id:p.checks.annotationAllocation.id,incarnation:p.checks.annotationAllocation.incarnation}:copy(p.request.annotation);
const grant=p=>({...good,annotationResultGrant:{invocation:p.reference,annotation:nativeTarget(p),identity:true,contentVersion:true,targetGeneration:true}});
export function checkAnnotationReads(api=original,short=false){
 let nativeCases=0,historyStages=0,readGateCases=0,releaseGateCases=0,pairedCases=0;const controls=[],control=(name,f)=>{f();controls.push(name);};
 const active=short?[variants[0],variants[1],variants[7],variants.at(-1)]:variants;
 for(const v of active)for(let facets=0;facets<4;facets++)for(let currentFacets=0;currentFacets<4;currentFacets++)for(const shown of [false,true]){
  const visible=()=>shown,z=fixture(api,{body:!!(facets&1),target:!!(facets&2),bodyVisible:!!(currentFacets&1),targetVisible:!!(currentFacets&2),visible}),p=z.prepare(v),before=z.current();
  z.execute(p,p.checks,grant(p));const receipt=copy(api.observeAnnotationResult(z.domain,p.ticket,writer,policy,grant(p))),usage=copy(z.usage());
  const first=checkCapture(api,z,z.acquire(),z.current(),visible);assert.deepEqual(z.usage(),usage);assert.equal(z.release().status,'empty');
  for(const op of ['undo','redo']){z.reverse(p,op);checkCapture(api,z,z.acquire(),z.current(),visible);assert.deepEqual(api.observeAnnotationResult(z.domain,p.ticket,writer,policy,grant(p)),receipt);const old=api.queryReadContext(first.state,query(z,first.context,'enumerate',{limit:8,continuation:null}),z.host);assert.equal(old.result.status,'page');assert.deepEqual(old.result.records,first.result.records);historyStages++;}
  assert.equal(before.publication,0);nativeCases++;
 }
 for(const v of [variants[0],variants[1],variants[7],variants.at(-1)])for(const [names,kind] of [[rc.readGates,'read'],[rc.releaseGates,'release']]){
  const z=fixture(api),p=z.prepare(v);z.execute(p);const usage=copy(z.usage()),root=copy(z.current()),all=(1<<names.length)-1;
  for(const mask of short?[0,...names.map((_,i)=>all^(1<<i)),all]:Array.from({length:all+1},(_,i)=>i)){
   const gates=Object.fromEntries(names.map((k,i)=>[k,!!(mask&(1<<i))])),out=z.acquire(z.registry,kind==='read'?{...z.host,...gates}:z.host,kind==='release'?{...release,...gates}:release);
   assert.equal(out.result.status,mask===all?'context':'unavailable');if(mask!==all)assert.deepEqual(out.state,z.registry);assert.deepEqual(z.current(),root);assert.deepEqual(z.usage(),usage);
   assert.equal(api.observeInvocation(z.domain,p.ticket,writer,policy,good).outcome.publication,'applied');if(kind==='read')readGateCases++;else releaseGateCases++;
  }
 }
 for(let n=0;n<(short?2:8);n++)for(const op of ['create','update-body','reattach','delete']){
  const seed=copy(fixture(api).current()),hidden={...copy(seed.documents[0].records[0]),id:'private',body:'secret-'+n,bodyVersion:n+1};seed.documents[0].records.push(hidden);seed.documents[0].annotationLifecycle.entries.push({id:'private',incarnation:1,phase:'present',bodyFloor:n+1,targetFloor:1,retained:null});
  const a=fixture(api,{visible:r=>r.id!=='private'}),b=fixture(api,{state:seed,visible:r=>r.id!=='private'}),p=a.prepare({operation:op}),q=b.prepare({operation:op});a.execute(p);b.execute(q);const x=checkCapture(api,a,a.acquire()),y=checkCapture(api,b,b.acquire(),b.current(),r=>r.id!=='private');assert.deepEqual(x.result,y.result);assert.deepEqual(a.usage(),b.usage());pairedCases++;
 }
 control('current-capture-is-not-the-last-delivered-event-or-original-receipt',()=>{const z=fixture(api),p=z.prepare({operation:'update-body'});z.execute(p,p.checks,grant(p));z.observe();const first=z.release();assert.equal(first.projection.documents[0].annotations[0].content.body,'changed');const receipt=copy(api.observeAnnotationResult(z.domain,p.ticket,writer,policy,grant(p)));const q=z.prepare({operation:'update-body'},{body:'newer'});z.execute(q);const read=checkCapture(api,z,z.acquire());assert.equal(read.result.records[0].content.body,'newer');assert.equal(read.result.records[0].content.version,3);assert.deepEqual(api.observeAnnotationResult(z.domain,p.ticket,writer,policy,grant(p)),receipt);assert.equal(receipt.result.contentVersion,2);assert.equal(z.release().status,'empty');});
 control('native-receipt-event-and-generation-values-cannot-substitute-for-read-selectors',()=>{const z=fixture(api),p=z.prepare({operation:'create'});z.execute(p,p.checks,grant(p));z.observe();const event=z.release(),receipt=api.observeAnnotationResult(z.domain,p.ticket,writer,policy,grant(p)),out=z.acquire(),q=checkCapture(api,z,out);const refs=[receipt.result.annotation,receipt.result.annotation.id,receipt.result.publication,event.after,...event.projection.documents[0].annotations.map(r=>r.reference)];for(const reference of refs){assert.deepEqual(api.queryReadContext(q.state,query(z,q.context,'inspect',{annotation:reference}),z.host).result,unavailable);assert.deepEqual(api.queryReadContext(q.state,query(z,reference,'enumerate',{limit:1,continuation:null}),z.host).result,unavailable);}});
 control('held-query-selectors-bind-the-original-context-and-cannot-cross-captures',()=>{const z=fixture(api),a=checkCapture(api,z,z.acquire()),b=checkCapture(api,z,z.acquire(a.state));assert.notEqual(a.context,b.context);assert.deepEqual(api.queryReadContext(b.state,query(z,b.context,'inspect',{annotation:a.result.records[0].reference}),z.host).result,unavailable);for(const key of ['recipient','purpose','destination','policy','continuity','profile','document'])assert.deepEqual(api.queryReadContext(b.state,query(z,b.context,'inspect',{annotation:b.result.records[0].reference}),{...z.host,[key]:key==='document'?{...document,incarnation:2}:'other'}).result,unavailable);for(const key of ['granted','live','continuous','transferReady','replyReady'])assert.deepEqual(api.queryReadContext(b.state,query(z,b.context,'inspect',{annotation:b.result.records[0].reference}),{...z.host,[key]:false}).result,unavailable);});
 control('current-read-capture-scope-cannot-be-rebound-to-another-recipient-context',()=>{const z=fixture(api),p=z.prepare({operation:'create'});z.execute(p);for(const key of ['recipient','purpose','destination','policy','continuity','profile','document']){const h={...z.host,[key]:key==='document'?{...document,incarnation:2}:'other'};assert.deepEqual(z.acquire(z.registry,h),{state:z.registry,result:unavailable});}});
 control('source-mapping-before-native-body-publication-is-captured-as-current-placement',()=>{const z=fixture(api),p=z.prepare({operation:'update-body'});z.source();z.execute(p);const q=checkCapture(api,z,z.acquire());assert.equal(q.result.records[0].location.placement.sourceRevision,2);assert.equal(q.result.records[0].location.placement.target.start,1);assert.equal(q.result.records[0].content.body,'changed');assert.equal(q.result.records[0].location.targetGeneration,1);});
 control('independent-authored-field-merges-are-visible-in-the-same-current-capture',()=>{for(const operation of ['update-body','reattach']){const z=fixture(api),p=z.prepare({operation}),q=z.prepare({operation:operation==='update-body'?'reattach':'update-body'});z.execute(q);z.execute(p);const r=checkCapture(api,z,z.acquire()).result.records[0];assert.equal(r.content.version,2);assert.equal(r.location.targetGeneration,2);assert.equal(r.content.body,'changed');assert.deepEqual(r.location.placement.target,{kind:'document'});}});
 control('delete-restoration-is-fresh-membership-and-old-read-stays-present',()=>{const z=fixture(api),old=checkCapture(api,z,z.acquire()),p=z.prepare({operation:'delete'});z.execute(p);const absent=checkCapture(api,z,z.acquire());assert.deepEqual(absent.result.records,[]);z.reverse(p,'undo');const restored=checkCapture(api,z,z.acquire());assert.equal(restored.result.records[0].content.version,2);assert.equal(restored.result.records[0].location.targetGeneration,2);assert.equal(api.queryReadContext(old.state,query(z,old.context,'inspect',{annotation:old.result.records[0].reference}),z.host).result.record.content.version,1);assert.deepEqual(api.queryReadContext(absent.state,query(z,absent.context,'enumerate',{limit:1,continuation:null}),z.host).result.records,[]);});
 control('annotation-only-read-never-infers-source-authority-or-drops-a-denied-class',()=>{const z=fixture(api),p=z.prepare({operation:'create'});z.execute(p);for(const data of [['source'],['source','annotations']])assert.deepEqual(z.acquire(z.registry,z.host,release,{...z.request,data}),{state:z.registry,result:unavailable});const q=checkCapture(api,z,z.acquire());assert.deepEqual(api.queryReadContext(q.state,{contract:'source-query/1',operation:'bounded-range',document,snapshot:q.context,profile:z.c.profile,lo:0,hi:1},z.host).result,unavailable);});
 control('read-admission-or-selector-pressure-never-reverts-the-committed-edit',()=>{const z=fixture(api),p=z.prepare({operation:'create'});z.execute(p);for(const key of ['contexts','units','selectors','issuance']){const m={...z.registry,limits:{...z.registry.limits,[key]:0}};assert.deepEqual(z.acquire(m),{state:m,result:unavailable});}const out=z.acquire(),m={...out.state,limits:{...out.state.limits,selectors:out.state.selectors.length}},q=api.queryReadContext(m,query(z,out.result.context,'enumerate',{limit:1,continuation:null}),z.host);assert.deepEqual(q,{state:m,result:unavailable});assert.equal(z.current().documents[0].records.length,2);assert.equal(api.observeInvocation(z.domain,p.ticket,writer,policy,good).outcome.publication,'applied');});
 control('stop-query-retirement-and-receipt-eviction-remain-independent',()=>{const z=fixture(api),p=z.prepare({operation:'create'});z.execute(p,p.checks,grant(p));const q=checkCapture(api,z,z.acquire());assert.equal(api.forgetInvocationResult(z.domain,p.ticket,{...good,retentionReleased:true,deliveryDrained:true}).status,'forgotten');checkCapture(api,z,z.acquire());assert.equal(api.stopPublicationObserver(z.owner,{ownerControlAllowed:true}).status,'stopped');assert.deepEqual(z.acquire().result,unavailable);assert.equal(api.queryReadContext(q.state,query(z,q.context,'inspect',{annotation:q.result.records[0].reference}),z.host).result.status,'record');const closed=api.closeReadContext(q.state,{contract:'read-context/1',operation:'stop-observation',context:q.context},z.host.recipient);assert.equal(closed.result.status,'stopped');assert.deepEqual(api.queryReadContext(closed.state,query(z,q.context,'inspect',{annotation:q.result.records[0].reference}),z.host).result,unavailable);assert.equal(z.current().documents[0].records.length,2);});
 control('read-copy-mutation-cannot-change-native-state-or-later-query',()=>{const z=fixture(api),p=z.prepare({operation:'reattach'});z.execute(p);const q=checkCapture(api,z,z.acquire()),r=q.result.records[0],before=copy(r);r.content.body='tampered';r.location.placement.state='detached';assert.deepEqual(api.queryReadContext(q.state,query(z,q.context,'inspect',{annotation:r.reference}),z.host).result.record,before);assert.equal(z.current().documents[0].records[0].body,'old');assert.equal(z.current().documents[0].records[0].placement.state,'attached');});
 control('writer-receipt-and-reader-delivery-are-independent-authority-products',()=>{const z=fixture(api),p=z.prepare({operation:'create'});z.execute(p);assert.deepEqual(api.observeAnnotationResult(z.domain,p.ticket,writer,policy,grant(p)),unavailable);assert.equal(checkCapture(api,z,z.acquire()).result.records.length,2);const a=fixture(api),q=a.prepare({operation:'create'});a.execute(q,q.checks,grant(q));assert.deepEqual(a.acquire(a.registry,{...a.host,granted:false}).result,unavailable);assert.equal(api.observeAnnotationResult(a.domain,q.ticket,writer,policy,grant(q)).result.status,'applied');});
 control('native-queue-gap-requires-explicit-recovery-without-changing-edits-or-held-reads',()=>{const z=fixture(api,{observationLimits:{maxQueue:1}}),old=checkCapture(api,z,z.acquire()),a=z.prepare({operation:'create'},{freshName:'first'});z.execute(a);assert.equal(z.observe().status,'queued');const b=z.prepare({operation:'create'},{freshName:'second'});z.execute(b);assert.equal(z.observe().status,'gap');assert.deepEqual(z.acquire().result,unavailable);assert.equal(api.queryReadContext(old.state,query(z,old.context,'inspect',{annotation:old.result.records[0].reference}),z.host).result.status,'record');assert.equal(z.current().documents[0].records.length,3);assert.equal(api.resyncPublicationObserver(z.owner,z.current(),z.c,release,z.packet(z.current())).status,'baseline');assert.equal(checkCapture(api,z,z.acquire()).result.records.length,3);});
 return {scope:catalog.scope,nativeCases,historyStages,readGateCases,releaseGateCases,pairedCases,controls,...catalog.scopeFlags};
}
function moduleURL(name,m,dependencies={}){let raw=fs.readFileSync(new URL(name,import.meta.url),'utf8');if(m){assert(raw.includes(m.from),m.name);raw=raw.replaceAll(m.from,m.to);}raw=raw.replace(/from '(\.\/[^']+)'/g,(_,r)=>`from ${JSON.stringify(dependencies[r]??new URL(r,import.meta.url).href)}`).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,r)=>`new URL(${JSON.stringify(new URL(r,import.meta.url).href)},import.meta.url)`);return `data:text/javascript;base64,${Buffer.from(raw).toString('base64')}#${m?.name??name}`;}
export async function checkAnnotationReadMutations(){
 const mutations=[
  ['hidden-native-row-exposed','publication-observation.mjs','&&r.visible)',')'],
  ['captured-body-ceiling-ignored','publication-observation.mjs','bodyVisible:d.collection.body&&fact.body','bodyVisible:true'],
  ['captured-target-ceiling-ignored','publication-observation.mjs','targetVisible:d.collection.target&&fact.target','targetVisible:true'],
  ['current-body-permission-ignored','publication-observation.mjs','bodyVisible:d.collection.body&&fact.body','bodyVisible:d.collection.body'],
  ['current-target-permission-ignored','publication-observation.mjs','targetVisible:d.collection.target&&fact.target','targetVisible:d.collection.target'],
  ['old-stream-baseline-read','publication-observation.mjs','selected.annotations.map','o.baseline.documents.find(d=>equal(d.document,document)).annotations.map'],
  ['annotation-read-infers-source-authority','publication-observation.mjs',"data.includes('source')&&!scope.source",'false'],
  ['read-installs-observer-bindings','publication-observation.mjs','const selected=projection.documents.find','o.refs=stage.refs;o.issued=stage.issued+1;const selected=projection.documents.find'],
  ['current-read-scope-bypassed','observation-reads.mjs','if(!Object.entries(captured.scope).every(([key,value])=>equal(value,current[key])))return fail();',''],
  ['read-stop-gate-bypassed','read-contexts.mjs','||!current.notStopped',''],
  ['record-selector-context-bypassed','read-contexts.mjs','record.context!==c.token','false'],
  ['selector-capacity-bypassed','read-contexts.mjs','if(!room(m,pending.length))return fail();',''],
  ['query-grant-bypassed','read-contexts.mjs','current.granted&&current.live&&current.continuous&&current.transferReady','current.live&&current.continuous&&current.transferReady'],
  ['mapped-source-revision-rolled-back','publication-observation.mjs','placement:copy(r.placement)','placement:{...copy(r.placement),sourceRevision:1}']
 ].map(([name,file,from,to])=>({name,file,from,to})),killed=[];
 const load=async(m=null)=>{const ou=moduleURL('publication-observation.mjs',m?.file==='publication-observation.mjs'?m:null),ru=moduleURL('read-contexts.mjs',m?.file==='read-contexts.mjs'?m:null),bu=moduleURL('observation-reads.mjs',m?.file==='observation-reads.mjs'?m:null,{'./publication-observation.mjs':ou,'./read-contexts.mjs':ru});return {...original,...await import(ou),...await import(ru),...await import(bu)};};
 checkAnnotationReads(await load(),true);
 for(const m of mutations){let failed=false;const loadedMutationApi=await load(m);try{checkAnnotationReads(loadedMutationApi,true);}catch(e){assert.equal(e.code,'ERR_ASSERTION',m.name+': '+e.message);failed=true;}assert(failed,m.name);killed.push(m.name);}return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkAnnotationReads(),mutations:await checkAnnotationReadMutations(),mutationBaselineChecked:true},null,2));
