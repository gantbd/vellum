// Scoped owned-observer lifetime evidence, never producer cancellation or host-policy proof.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as native from './annotation-editing.mjs';
import * as kernel from './invocation-publication.mjs';
import * as reversal from './history-reversal.mjs';
import * as observer from './publication-observation.mjs';
import * as histories from './history-contexts.mjs';
import * as observation from './invocation-observation.mjs';
import {invocationObservationFixture as fixture,invocationObservationGates as g,invocationObservationContext as c,invocationObservationPolicy as policy} from './invocation-observation-checks.mjs';
const original={...native,...kernel,...reversal,...observer,...histories,...observation},copy=structuredClone;
export function checkInvocationInvalidation(api=original){
 const setup=()=>{const z=fixture(api);assert.equal(z.admit().status,'accepted');const context=z.open();assert(context);const first=z.read(context);assert.equal(first.status,'execution');const wait=()=>api.startInvocationWait(context,z.request(context,'bounded-observation-wait',{after:first.observation,budget:2}),c,policy,g);return {z,context,first,wait};};
 let gaps=0,unknowns=0,transients=0,isolations=0,readinessCases=0,waitAdmissionCases=0,controlCases=0;
 for(const field of ['authorized','scope','continuous','live','retained','policy-grant']){
  const {z,context,wait}=setup(),a=wait(),b=wait();assert.equal(a.status,'waiting');assert.equal(b.status,'waiting');const before=api.invocationObservationUsage(context),root=z.current(),producer=copy(api.invocationUsage(z.domain));
  assert.equal(z.read(context,'status',field==='policy-grant'?{...policy,grant:false}:policy,field==='policy-grant'?g:{...g,[field]:false}).status,'unavailable');
  const after=api.invocationObservationUsage(context);assert.equal(after.live,false);assert.equal(after.references,0);assert.equal(after.waiting,0);assert.equal(after.waiters,2);assert.equal(after.issued,before.issued);assert.equal(after.declaredHeldBytes,before.declaredHeldBytes);assert.equal(z.current(),root);assert.deepEqual(api.invocationUsage(z.domain),producer);assert.equal(z.read(context).status,'unavailable');
  assert.equal(api.advanceInvocationWait(context,a.wait,'poll',c,policy,g).status,'unavailable');assert.equal(z.finish().status,'settled');assert.equal(api.observeInvocation(z.domain,z.p.ticket,c,policy,g).outcome.publication,'applied');assert.equal(z.read(context).status,'unavailable');const fresh=z.open();assert(fresh);assert.equal(z.read(fresh,'retained-outcome').outcome.publication,'applied');
  for(const token of [a.wait,b.wait])assert.equal(api.retireInvocationWait(context,token,{...g,authorized:false}).status,'retired');assert.equal(api.retireInvocationObservation(context,{...g,authorized:false}).status,'retired');gaps++;
 }
 for(const field of ['authorized','scope','continuous','live','retained'])for(const value of [undefined,1,null]){const {z,context}=setup();assert.equal(z.read(context,'status',policy,{...g,[field]:value}).status,'unavailable');assert.equal(api.invocationObservationUsage(context).live,true);assert.equal(z.read(context).status,'execution');unknowns++;}
 for(const field of ['transferReady','ordinaryReady','resourcesReady','controlReady']){const {z,context}=setup(),operation=field==='controlReady'?'minimal-control-status':'status';assert.equal(z.read(context,operation,policy,{...g,[field]:false}).status,'unavailable');assert.equal(z.read(context).status,'execution');transients++;for(const value of [1,undefined,null]){assert.equal(z.read(context,operation,policy,{...g,[field]:value}).status,'unavailable');assert.equal(api.invocationObservationUsage(context).live,true);readinessCases++;}}
 for(const field of Object.keys(c)){const {z,context}=setup();assert.equal(api.queryInvocationObservation(context,z.request(context),{...c,[field]:'foreign'},{...policy,grant:false},{...g,authorized:false}).status,'unavailable');assert.equal(z.read(context).status,'execution');isolations++;}
 for(const altered of [p=>({...p,recipient:'foreign',grant:false}),p=>({...p,epoch:p.epoch+1,grant:false}),p=>({...p,grant:1})]){const {z,context}=setup();assert.equal(z.read(context,'status',altered(policy)).status,'unavailable');assert.equal(z.read(context).status,'execution');isolations++;}
 for(const alter of [r=>({...r,extra:1}),r=>({...r,context:copy(r.context)}),r=>({...r,operation:'execute'}),r=>({...r,invocation:'other'})]){const {z,context}=setup();assert.equal(api.queryInvocationObservation(context,alter(z.request(context)),c,{...policy,grant:false},{...g,authorized:false}).status,'unavailable');assert.equal(z.read(context).status,'execution');isolations++;}
 for(const fields of [{after:{},budget:2},{after:null,budget:2},{budget:0},{budget:3.5}]){const {z,context,first}=setup(),request=z.request(context,'bounded-observation-wait',{after:first.observation,budget:2,...fields});assert.equal(api.startInvocationWait(context,request,c,{...policy,grant:false},g).status,'unavailable');assert.equal(z.read(context).status,'execution');isolations++;}
 for(const field of ['waitCapacity','waitClockValid','retentionReady'])for(const value of [1,undefined,null]){const {z,context,first}=setup();assert.equal(api.startInvocationWait(context,z.request(context,'bounded-observation-wait',{after:first.observation,budget:2}),c,policy,{...g,[field]:value}).status,'unavailable');assert.equal(api.invocationObservationUsage(context).waiters,0);assert.equal(z.read(context).status,'execution');waitAdmissionCases++;}
 for(const value of [1,undefined,null]){const {z,context,wait}=setup(),w=wait();assert.equal(w.status,'waiting');assert.equal(api.closeInvocationObservation(context,c,{...g,ownerControlAllowed:value}).status,'unavailable');assert.equal(api.advanceInvocationWait(context,w.wait,'stop',c,policy,{...g,ownerControlAllowed:value}).status,'unavailable');assert.equal(api.invocationObservationUsage(context).live,true);assert.equal(api.invocationObservationUsage(context).waiting,1);controlCases+=2;}
 const {z,context,wait}=setup(),w=wait();assert.equal(w.status,'waiting');assert.equal(api.closeInvocationObservation(context,c,g).status,'stopped');const held=copy(api.invocationObservationUsage(context));
 for(const field of ['ownerControlAllowed','retentionReleased','deliveryDrained'])for(const value of [false,1,undefined]){assert.equal(api.retireInvocationWait(context,w.wait,{...g,[field]:value}).status,'unavailable');assert.equal(api.retireInvocationObservation(context,{...g,[field]:value}).status,'unavailable');}
 assert.deepEqual(api.invocationObservationUsage(context),held);assert.equal(api.retireInvocationWait(context,w.wait,g).status,'retired');for(const field of ['ownerControlAllowed','retentionReleased','deliveryDrained'])for(const value of [false,1,undefined])assert.equal(api.retireInvocationObservation(context,{...g,[field]:value}).status,'unavailable');assert.equal(api.retireInvocationObservation(context,g).status,'retired');assert.equal(api.invocationUsage(z.domain).starts,0);assert.equal(z.finish().status,'settled');
 const forgotten=setup();forgotten.z.finish();assert.equal(forgotten.z.read(forgotten.context,'retained-outcome').outcome.publication,'applied');api.forgetInvocationResult(forgotten.z.domain,forgotten.z.p.ticket,g);assert.equal(forgotten.z.read(forgotten.context,'retained-outcome').status,'unavailable');assert.equal(api.invocationObservationUsage(forgotten.context).live,true);assert.equal(forgotten.z.read(forgotten.context,'minimal-control-status').phase,'terminal');
 const clock=setup(),cw=clock.wait();assert.equal(cw.status,'waiting');assert.equal(api.advanceInvocationWait(clock.context,cw.wait,'tick',c,policy,{...g,waitClockValid:1}).status,'unavailable');assert.deepEqual(api.invocationObservationUsage(clock.context).remaining,[2]);assert.equal(clock.z.read(clock.context).status,'execution');
 const polling=setup(),pa=polling.wait(),pb=polling.wait();assert.equal(api.advanceInvocationWait(polling.context,pa.wait,'poll',c,{...policy,grant:false},g).status,'unavailable');assert.equal(api.invocationObservationUsage(polling.context).waiting,0);assert.equal(api.advanceInvocationWait(polling.context,pb.wait,'poll',c,policy,g).status,'unavailable');assert.equal(polling.z.read(polling.context).status,'unavailable');
 return {gaps,unknowns,transients,isolations,readinessCases,waitAdmissionCases,controlCases,protectedDischargeCases:27,producerAndHistoricalOutcomeFramesChecked:true,allWaitersAbsorbedChecked:true};
}
export async function checkInvocationInvalidationMutations(){
 const raw=fs.readFileSync(new URL('./invocation-observation.mjs',import.meta.url),'utf8'),mutants=[
  ['observed-loss-ignored',"if(p.grant===false||['authorized','scope','continuous','live','retained'].some(k=>g?.[k]===false))",'if(false)'],
  ['unknown-loss-closes','.some(k=>g?.[k]===false)','.some(k=>g?.[k]!==true)'],
  ['delivery-selectors-retained','e.refs.clear();',''],
  ['other-waiters-not-closed',"for(const w of e.waiters.values())w.phase='ended';",''],
  ['closure-refunds-issuance','e.live=false;e.refs.clear();','e.live=false;e.issued=0;e.controlIssued=0;e.refs.clear();'],
  ['foreign-context-poisons','!equal(e.context,c)','false'],
  ['wrong-policy-poisons','p.recipient!==e.ceiling.recipient||p.epoch!==e.ceiling.epoch','false'],
  ['truthy-ordinary-readiness','g?.ordinaryReady===true','!!g?.ordinaryReady'],
  ['truthy-wait-capacity','g?.waitCapacity!==true','!g?.waitCapacity'],
  ['retirement-without-discharge','g?.retentionReleased!==true||g?.deliveryDrained!==true','false'],
  ['foreign-wait-reference-poisons',"request.operation!=='bounded-observation-wait'||e.refs.get(request.after)?.facet!=='ordinary'","request.operation!=='bounded-observation-wait'||!current(e,c,p,g,'ordinary')?.control||e.refs.get(request.after)?.facet!=='ordinary'"],
  ['forgotten-outcome-closes-context',"if(!outcome||outcome.observation!=='available')return null;","if(!outcome||outcome.observation!=='available'){closeContext(e);return null;}"],
  ['truthy-timer-spends-rank','if(g?.waitClockValid!==true)','if(!g?.waitClockValid)']
 ];
 const load=s=>import('data:text/javascript;base64,'+Buffer.from(s.replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+new URL(p,new URL('./invocation-observation.mjs',import.meta.url)).href+q)).toString('base64'));
 checkInvocationInvalidation({...original,...await load(raw)});const killed=[];for(const [name,a,b]of mutants){assert(raw.includes(a),name);const loadedMutationApi={...original,...await load(raw.replaceAll(a,b))};let failed=false;try{checkInvocationInvalidation(loadedMutationApi);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name+': '+e.message);failed=true;}assert(failed,name);killed.push(name);}return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({scope:'owned-native-invocation-invalidation-only',...checkInvocationInvalidation(),mutations:await checkInvocationInvalidationMutations(),mutationBaselineChecked:true,actualPolicyClockOrRetentionTruthProved:false,allReaderFamiliesProved:false,implementationRefinement:false,independentCleanroomCertification:false},null,2));
