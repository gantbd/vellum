import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as args from './native-operation-observation-arguments.mjs';
import * as admission from './observation-admission.mjs';
import * as observation from './native-operation-observation.mjs';
import * as kernel from './invocation-publication.mjs';
import * as discovery from './native-discovery.mjs';
import {nativeOperationObservationFixture,nativeOperationObservationLimits as limits,nativeOperationObservationGates} from './native-operation-observation-checks.mjs';
import {observationAdmissionProfile,observationAdmissionLimits} from './observation-admission-checks.mjs';
import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';
import {validateStandardSchemas} from './standard-schema-validator.mjs';
const copy=structuredClone,no={status:'unavailable'},kinds=['source-load','annotation-load','source-save','annotation-save'],good={...nativeOperationObservationGates,capacityActivationAuthorized:true},inputLimits={maxRequestBytes:8192,maxInputNodes:2048,maxInputDepth:32};
function profile(){const p=observationAdmissionProfile();for(const s of ['host','origin']){p.scope[s].hard.bytes.ceiling=67108864;p.scope[s].softBytes=67108864;}return p;}
function fixture(kind,A=admission,N=args,opt={}){
 const z=nativeOperationObservationFixture(kind),pool=A.openObservationAdmission(z.domain,profile(),observationAdmissionLimits,good,true,opt.nativeEnabled===false?null:'native-operation-observation/1');assert(pool);
 const admit=(c=z.c,p=z.p,g=good)=>A.admitPooledNativeOperationObservation(pool,kind,z.owner,z.task,c,p,limits,g,inputLimits),r=opt.noAdmission?null:admit();if(r)assert.equal(r.status,'context');
 const form=(op,extra={})=>z.form(op,{context:r?.context,...extra}),envelope=(op,route='typed',extra={})=>({route,request:z.envelope(form(op,extra),route)});
 const query=(op='status',route='typed',g=good,p=z.p,c=z.c,e=envelope(op,route))=>A.queryDecodedPooledObservation(pool,r.reservation,e,c,p,g);
 return {z,pool,r,admit,form,envelope,query,A,N};
}
export function checkNativeOperationObservationIntegration(A=admission,N=args,O=observation){
 const controls=[];let routes=0,schemaCases=0;const check=(n,f)=>{f();controls.push(n);};
 check('all-kinds-decoded-through-original-pool-with-eight-prepaid-frames',()=>{
  for(const kind of kinds){const x=fixture(kind,A,N),u=copy(A.observationAdmissionUsage(x.pool)),charge=observation.nativeOperationObservationCharge(limits),workspace=8*inputLimits.maxRequestBytes;
   assert.equal(A.observationDecoderUsage(x.pool,x.r.reservation).declaredWorkspaceBytes,workspace);
   for(const s of ['host','origin']){assert.equal(u.state.used[s].bytes,u.baseBytes+charge+workspace);assert.equal(u.state.used[s].slots,1);assert.equal(u.state.used[s].work,1);}
   for(const route of ['typed','dynamic'])for(const op of ['status','retained-outcome','minimal-control-status']){assert.equal(x.query(op,route).status,op==='retained-outcome'?'native-operation-receipt':'native-execution');routes++;assert.deepEqual(A.observationAdmissionUsage(x.pool),u);}
   assert.equal(A.retireObservationReservation(x.pool,x.r.reservation,good).status,'retired');const end=A.observationAdmissionUsage(x.pool);assert.equal(end.reservations,0);for(const s of ['host','origin']){assert.equal(end.state.used[s].bytes,end.baseBytes);assert.equal(end.state.used[s].work,1);}
  }
 });
 check('typed-dynamic-canonical-meaning-and-opaque-role-identity',()=>{
  const x=fixture('source-load',A,N);for(const op of ['status','retained-outcome','minimal-control-status','bounded-observation-wait']){
   const status=x.query(),extra=op==='bounded-observation-wait'?{after:status.observation,budget:2}:{},a=N.normalizeNativeOperationObservationArguments(x.envelope(op,'typed',extra),x.z.c,inputLimits,8192),b=N.normalizeNativeOperationObservationArguments(x.envelope(op,'dynamic',extra),x.z.c,inputLimits,8192);assert(a&&b);assert(N.sameNativeOperationObservationArguments(a,b));assert.equal(a.task,x.z.task);assert.equal(a.context,x.r.context);
   const producer=N.nativeOperationObservationProducerRequest(a);assert.equal(producer.task,x.z.task);assert.equal(producer.context,x.r.context);assert.equal(producer.after,extra.after);
   assert(!N.sameNativeOperationObservationArguments(a,{...b,task:{...b.task}}));assert(!N.sameNativeOperationObservationArguments(a,{...b,context:{...b.context}}));
  }
 });
 check('closed-data-decoder-does-not-evaluate-accessors-or-traverse-opaque-roles',()=>{
  const x=fixture('source-load',A,N);let calls=0;
  for(const field of ['contract','operation','task','context','profile','facet']){const e=x.envelope('retained-outcome');Object.defineProperty(e.request.arguments,field,{enumerable:true,get(){calls++;return 'surprise';}});assert.equal(N.normalizeNativeOperationObservationArguments(e,x.z.c,inputLimits,8192),null);}
  const e=x.envelope('retained-outcome');Object.defineProperty(e.request.arguments.facet,'kind',{enumerable:true,get(){calls++;return 'source-load';}});assert.equal(N.normalizeNativeOperationObservationArguments(e,x.z.c,inputLimits,8192),null);assert.equal(calls,0);
  const opaque={get forbidden(){calls++;throw Error('must not traverse');}},r=x.envelope('status');r.request.arguments.task=opaque;r.request.arguments.context=opaque;const n=N.normalizeNativeOperationObservationArguments(r,x.z.c,inputLimits,8192);assert(n);assert.equal(n.task,opaque);assert.equal(n.context,opaque);assert.equal(calls,0);assert.deepEqual(x.query('status','typed',good,x.z.p,x.z.c,r),no);
  const wait=x.envelope('bounded-observation-wait','typed',{after:opaque,budget:1});assert(N.normalizeNativeOperationObservationArguments(wait,x.z.c,inputLimits,8192));assert.deepEqual(x.query('bounded-observation-wait','typed',good,x.z.p,x.z.c,wait),no);assert.equal(calls,0);assert.equal(O.nativeOperationObservationUsage(x.r.context).receiptFaulted,false);
 });
 check('bad-profile-fields-versions-context-and-independent-copy-bounds-refuse',()=>{
  const x=fixture('annotation-load',A,N),e=x.envelope('retained-outcome');for(const change of [{contract:'execution-receipt/1'},{facet:{kind:'annotation'}},{facet:{kind:'annotation-load',extra:true}},{extra:true},{task:'not-opaque'}]){const r=copy(e);r.request.arguments={...e.request.arguments,...change};assert.equal(N.normalizeNativeOperationObservationArguments(r,x.z.c,inputLimits,8192),null);}
  for(const value of [2,-1,undefined]){const r={...e,request:{...e.request,version:value}};assert.equal(N.normalizeNativeOperationObservationArguments(r,x.z.c,inputLimits,8192),null);}
  for(const k of Object.keys(x.z.c)){const c=copy(x.z.c);delete c[k];assert.equal(N.normalizeNativeOperationObservationArguments(e,c,inputLimits,8192),null);}
  for(const b of [{maxRequestBytes:1},{maxInputNodes:1},{maxInputDepth:1}])assert.equal(N.normalizeNativeOperationObservationArguments(e,x.z.c,{...inputLimits,...b},8192),null);
  const n=N.normalizeNativeOperationObservationArguments(e,x.z.c,inputLimits,8192);assert(n);e.request.arguments.facet.kind='source-save';assert.equal(n.record.request.facet.kind,'annotation-load');assert.equal(n.task,x.z.task);
 });
 check('native-profile-is-explicit-not-a-silent-upgrade-or-a-cross-domain-route',()=>{
  const x=fixture('source-load',A,N,{nativeEnabled:false,noAdmission:true});assert.deepEqual(x.admit(),no);
  const y=nativeOperationObservationFixture('source-load'),u=kernel.invocationUsage(y.domain);for(const value of [true,false,'native-operation-observation/2','execution-observation/1',{}]){assert.equal(A.openObservationAdmission(y.domain,profile(),observationAdmissionLimits,good,true,value),null);assert.deepEqual(kernel.invocationUsage(y.domain),u);}assert(A.openObservationAdmission(y.domain,profile(),observationAdmissionLimits,good,true,'native-operation-observation/1'));
  const other=fixture('annotation-load',A,N),before=copy(A.observationAdmissionUsage(other.pool));assert.deepEqual(A.admitPooledNativeOperationObservation(other.pool,x.z.kind,x.z.owner,x.z.task,x.z.c,x.z.p,limits,good,inputLimits),no);assert.deepEqual(A.observationAdmissionUsage(other.pool),before);
  const r=other.envelope('status');r.request.arguments.contract='execution-observation/1';assert.deepEqual(other.query('status','typed',good,other.z.p,other.z.c,r),no);
 });
 check('stopped-pool-and-no-ordinary-facts-preserve-prepaid-decoded-control',()=>{
  for(const kind of kinds){const x=fixture(kind,A,N),u=copy(A.observationAdmissionUsage(x.pool));assert.equal(A.stopObservationAdmission(x.pool,good).status,'stopped');assert.deepEqual(x.admit(),no);assert.deepEqual(x.query('retained-outcome','dynamic',{...good,operationReceiptAllowed:false}),no);
   const out=x.query('minimal-control-status','dynamic',{...good,ordinaryReady:false,resourcesReady:false,operationReceiptAllowed:false});assert.equal(out.status,'native-execution');assert.equal(out.stopped,false);const next=A.observationAdmissionUsage(x.pool);assert.deepEqual(next.state,u.state);assert.equal(A.retireObservationReservation(x.pool,x.r.reservation,good).status,'retired');
  }
 });
 check('actual-wait-lifetime-governs-reservation-retirement-without-producer-stop',()=>{
  for(const kind of kinds){const x=fixture(kind,A,N),s=x.query(),w=x.query('bounded-observation-wait','typed',good,x.z.p,x.z.c,x.envelope('bounded-observation-wait','typed',{after:s.observation,budget:1}));assert.equal(w.status,'waiting');const held=copy(A.observationAdmissionUsage(x.pool));assert.deepEqual(A.retireObservationReservation(x.pool,x.r.reservation,good),no);assert.deepEqual(A.observationAdmissionUsage(x.pool),held);assert.equal(O.advanceNativeOperationWait(x.r.context,w.wait,'tick',x.z.c,x.z.p,good).status,'observation-ended');assert.deepEqual(A.retireObservationReservation(x.pool,x.r.reservation,good),no);assert.equal(O.retireNativeOperationWait(x.r.context,w.wait,good).status,'retired');assert.equal(A.retireObservationReservation(x.pool,x.r.reservation,good).status,'retired');assert.equal(observation.queryNativeOperationObservation(x.z.h,x.z.form('status'),x.z.c,x.z.p,good).stopped,false);}
 });
 check('native-admission-private-input-and-actual-section-precede-current-facts',()=>{
  const x=fixture('source-load',A,N,{noAdmission:true}),c=copy(x.z.c),p=copy(x.z.p);let called=false;const g={...good,get authorized(){if(!called){called=true;c.recipient+='-changed';p.effect=false;assert.deepEqual(x.admit(),no);}return true;}};const r=x.admit(c,p,g);assert.equal(r.status,'context');assert(called);assert.equal(A.queryDecodedPooledObservation(x.pool,r.reservation,{route:'typed',request:x.z.envelope(x.z.form('retained-outcome',{context:r.context}),'typed')},x.z.c,x.z.p,good).status,'native-operation-receipt');
 });
 check('input-and-facts-accessors-are-refusals-not-evaluated-or-result-faults',()=>{
  const x=fixture('source-save',A,N);let count=0;const p={...x.z.p,get effect(){count++;return true;}},g={...good,get scope(){count++;return true;}};assert.deepEqual(x.query('status','typed',good,p),no);assert.deepEqual(x.query('status','typed',g),no);assert.equal(count,0);assert.equal(A.observationDecoderUsage(x.pool,x.r.reservation).faulted,false);assert.equal(x.query('minimal-control-status').status,'native-execution');
 });
 check('standard-schema-describes-four-forms-but-not-opaque-authority',()=>{
  const x=fixture('source-load',A,N),ds=N.nativeOperationObservationEngineDescriptors();assert.equal(ds.length,2);const cases=[];
  for(const d of ds){assert.equal(d.family,'F06');for(const v of d.variants){const value=x.form(v.id,v.id==='bounded-observation-wait'?{after:{},budget:1}:{}),schema=v.requirements.schema;assert(Object.isFrozen(schema));assert.equal(v.input.contract,d.contract);cases.push({schema,value,valid:true},{schema,value:{...value,extra:true},valid:false},{schema,value:{...value,contract:'wrong'},valid:false},{schema,value:{...value,task:{}},valid:true});if(v.id==='retained-outcome')cases.push({schema,value:{...value,facet:{kind:'annotation'}},valid:false});}}
  for(const d of ds)for(const v of d.variants){const extra=v.id==='bounded-observation-wait'?{after:x.query().observation,budget:2}:{},value=x.query(v.id,'typed',good,x.z.p,x.z.c,x.envelope(v.id,'typed',extra)),schema=v.outcomes.result;assert.notEqual(value.status,'unavailable');cases.push({schema,value,valid:true},{schema,value:{...value,extra:true},valid:false},{schema,value:no,valid:true},{schema,value:{status:'invented-result'},valid:false});
   if(v.id==='bounded-observation-wait'){const pc=v.outcomes.privateContinuation;x.z.stop();const observed=O.advanceNativeOperationWait(x.r.context,value.wait,'poll',x.z.c,x.z.p,good);assert.equal(observed.status,'observed');cases.push({schema:pc,value:observed,valid:true},{schema:pc,value:{status:'observation-ended'},valid:true},{schema:pc,value,valid:true},{schema:pc,value:{...observed,extra:true},valid:false});}
  }
  const resultSchema=ds.find(d=>d.contract==='native-operation-receipt/1').variants[0].outcomes.result;
  for(const kind of kinds){const y=fixture(kind,A,N),z=y.z.x;if(kind.endsWith('load')){if(kind==='source-load')z.events.set(y.z.task,z.request());assert.equal(z.submit(y.z.task).status,'submitted');z.complete(y.z.task);}else{assert.equal(z.dispatch(y.z.task).status,'dispatched');assert.equal((kind==='source-save'?z.written(y.z.task):z.x.result(y.z.task)).status,'recorded');}const value=y.query('retained-outcome');assert.equal(value.status,'native-operation-receipt');cases.push({schema:resultSchema,value,valid:true},{schema:resultSchema,value:{...value,facet:{kind:kind==='source-load'?'annotation-load':'source-load'}},valid:false});}
  schemaCases=validateStandardSchemas(cases);assert.equal(schemaCases,45);
 });
 check('accepted-registry-releases-owned-descriptors-without-granting-execution',()=>{
  const x=fixture('source-load',A,N),template=nativeDiscoveryFixture(),c=kernel.invocationAuthorityContext(x.z.domain),scope={...c,purpose:'discover-contracts',policy:'native-observation-policy',profile:'owned-contract-discovery/1'},g=template.g;
  const l={...template.l,maxContexts:1,maxPreparations:1,maxEntries:2,maxVariants:4,maxSelectors:32,maxPageRows:2,maxRootBytes:65536,maxPacketBytes:131072,maxCaptureBytes:131072,maxRowBytes:32768,maxReplyBytes:131072,maxMetadataBytes:32768,maxMetadataDepth:24,maxHeldBytes:1048576};
  const owner=discovery.openNativeDiscoveryRegistry(x.z.domain,l,g());assert(owner);const ds=N.nativeOperationObservationEngineDescriptors(),rows=ds.map(({family,...d},i)=>({command:{authority:c.authority,id:'native-observation-'+i,incarnation:1},...d})),target={identity:'native-observation-target',basis:{}};
  const cut=()=>{const h=discovery.prepareNativeDiscoveryCut(owner,scope,rows,template.visibility(rows),target,template.advice(rows),g());assert(h);return h;};let h=cut();const r=discovery.acquireNativeDiscovery(owner,scope,h,g());assert.equal(r.status,'context');discovery.retireNativeDiscoveryCut(owner,h,g());
  const read=request=>{const h=cut();try{return discovery.queryNativeDiscovery(owner,{contract:'contract-discovery/1',profile:scope.profile,context:r.context,...request},scope,h,g());}finally{discovery.retireNativeDiscoveryCut(owner,h,g());}};
  const page=read({operation:'list',limit:2,continuation:null});assert.equal(page.status,'page');assert.equal(page.entries.length,2);let variants=0;for(const e of page.entries){const d=read({operation:'inspect',entry:e.reference});assert.equal(d.status,'descriptor');variants+=d.descriptor.variants.length;assert.deepEqual(x.query('retained-outcome','typed',good,x.z.p,x.z.c,{route:'typed',request:x.z.envelope(x.form('retained-outcome',{context:e.reference}),'typed')}),no);}assert.equal(variants,4);assert.deepEqual(x.query('retained-outcome','typed',{...good,operationReceiptAllowed:false}),no);assert.equal(x.query('retained-outcome').status,'native-operation-receipt');
 });
 return {scope:'native-operation-observation-decoder-and-discovery',controls,routes,schemaCases,sharedEightFramePoolReused:true,oldProfilesOrProducerSemanticsChanged:false,completeF19F18HostileWirePhysicalOrIndependentProof:false};
}
export async function checkNativeOperationObservationIntegrationMutations(){
 const files=Object.fromEntries([['args','native-operation-observation-arguments.mjs'],['admission','observation-admission.mjs'],['observation','native-operation-observation.mjs']].map(([k,f])=>[k,fs.readFileSync(new URL(f,import.meta.url),'utf8')]));
 const edits=[
  ['native-profile-not-opt-in','admission','!o?.nativeEnabled','!o'],
  ['wrong-actual-pool-domain','admission','nativeOperationObservationParent(kind,owner,task,c)!==o.domain','false',2],
  ['decoder-workspace-not-funded','admission','const workspace=invocationObservationDecoderFrames.length*capturedLimits.maxRequestBytes;','const workspace=0;'],
  ['native-reservation-kind-forgotten','admission','nativeKind:kind,decoder:','nativeKind:undefined,decoder:'],
  ['native-admission-input-not-captured','admission','c=copy(c);ceiling=copy(ceiling);limits=copy(limits);',''],
  ['native-retirement-bypassed','admission',"if(usage(r.context)!==null&&retire(r.context,g).status!=='retired')return unavailable();\n if(usage(r.context)!==null)return unavailable();",''],
  ['query-facts-not-privately-copied','admission','copySettingsData({policy:p,facts:g},d.limits)','{value:{policy:p,facts:g}}',2],
  ['task-role-copied','args','task:a.task,context:a.context','task:copySettingsData(a.task,cap).value,context:a.context'],
  ['argument-extra-fields-ignored','args','!exact(a,f.fields)','false'],
  ['progress-role-traversed-by-native-bound','observation','size({request:shadow,context:c,policy:p})','size({request:r,context:c,policy:p})'],
  ['schema-allows-extra-fields','args','required:[...f.fields],additionalProperties:false','required:[...f.fields],additionalProperties:true'],
  ['schema-advertises-old-local-receipt-kind','args','enum:[...nativeOperationKinds()]',"enum:[...nativeOperationKinds(),'annotation']"],
 ];
 const url=(s,tag,deps={})=>'data:text/javascript;base64,'+Buffer.from(s.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(deps[p]??new URL(p,import.meta.url).href)).replace(/new URL\('(\.\/[^']+)',import.meta.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,import.meta.url).href)+')')+'\n// '+tag).toString('base64');
 async function load(edit=null){const raw={...files};if(edit){const [name,k,before,after,count=1]=edit;assert.equal(raw[k].split(before).length-1,count,name+': exact anchor count');raw[k]=raw[k].replaceAll(before,after);}const tag=edit?.[0]??'baseline',ou=url(raw.observation,tag+'-observer'),nu=url(raw.args,tag+'-args',{'./native-operation-observation.mjs':ou}),au=url(raw.admission,tag+'-admission',{'./native-operation-observation.mjs':ou,'./native-operation-observation-arguments.mjs':nu});return {O:await import(ou),N:await import(nu),A:await import(au)};}
 const b=await load();checkNativeOperationObservationIntegration(b.A,b.N,b.O);const killed=[];for(const edit of edits){const a=await load(edit);let caught=false;try{checkNativeOperationObservationIntegration(a.A,a.N,a.O);}catch(e){assert.equal(e.code,'ERR_ASSERTION',edit[0]+': '+String(e.message).slice(0,160));caught=true;}assert(caught,edit[0]);killed.push(edit[0]);}return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{const baseline=checkNativeOperationObservationIntegration();console.log(JSON.stringify(process.argv.includes('--reference-only')?baseline:{baseline,mutations:await checkNativeOperationObservationIntegrationMutations(),coherentCopiedThreeModuleBaseline:true,constructionFailuresCounted:false}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
