import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as native from './invocation-observation.mjs';
import {invocationUsage} from './invocation-publication.mjs';
import {observationResultFixture} from './invocation-observation-result-checks.mjs';
import {observationAdmissionFixture} from './observation-admission-checks.mjs';
import {observationAdmissionUsage,retireObservationReservation} from './observation-admission.mjs';
import {invocationObservationContext as context,invocationObservationPolicy as policy,invocationObservationGates as gates} from './invocation-observation-checks.mjs';
const base=new URL('./',import.meta.url),source=fs.readFileSync(new URL('invocation-observation.mjs',base),'utf8'),copy=structuredClone,stages=[
 ["stage-phase","frame={...copy(body),observation:token};","frame={...copy(body),observation:token};if(g?.testResultFault===true)frame.phase='surprise';","query",0],
 ["stage-reference","frame={...copy(body),observation:token};","frame={...copy(body),observation:token};if(g?.testResultFault===true)frame.observation=structuredClone(token);","query",0],
 ["stage-outcome","frame={...copy(body),observation:token};","frame={...copy(body),observation:token};if(g?.testResultFault===true)frame.outcome.publication='surprise';","outcome",0],
 ["stage-exception","frame={...copy(body),observation:token};","frame={...copy(body),observation:token};if(g?.testResultFault===true)throw Error('fixed construction fault');","query",0],
 ['start-wait',"),result={status:'waiting',wait};","),result={status:'waiting',wait};if(g?.testResultFault===true)result.wait=structuredClone(wait);",'start',0],
 ['continuing-wait',"const waiting=()=>{const result={status:'waiting',wait};","const waiting=()=>{const result={status:'waiting',wait};if(g?.testResultFault===true)result.status='surprise';",'tick',0],
 ['delivery-before-install',"const result={status:'observed',result:s.frame};","const result={status:'observed',result:s.frame};if(g?.testResultFault===true)result.result.phase='surprise';",'poll',0],
 ['completion-after-install',"const end=result=>{","const end=result=>{if(g?.testResultFault===true&&result.status==='observed')result.status='surprise';",'poll',1]
];
async function load(rule){if(!rule)return native;const [name,from,to]=rule;assert.equal(source.split(from).length-1,1,name+': construction target');const text=source.replace(from,to).replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+new URL(p,base).href+q);return import('data:text/javascript;base64,'+Buffer.from(text).toString('base64'));}
function exercise(api,mode,faulted,extraIssued=0){
 const x=observationResultFixture(api),first=x.query(),w=x.wait(first.observation);assert.equal(w.status,'waiting');if(['outcome','poll'].includes(mode))assert.equal(x.finish().status,'settled');
 const before=api.invocationObservationUsage(x.handle),producer=copy(invocationUsage(x.domain)),root=x.current(),g={...gates,testResultFault:true};
 const result=mode==='query'?x.query('status',policy,g):mode==='outcome'?x.query('retained-outcome',policy,g):mode==='start'?x.wait(first.observation,2,g):x.advance(w.wait,mode==='tick'?'tick':'poll',g);
 assert.deepEqual(invocationUsage(x.domain),producer);assert.equal(x.current(),root);
 if(!faulted){assert.equal(result.status,mode==='query'?'execution':mode==='outcome'?'outcome':mode==='start'||mode==='tick'?'waiting':'observed');return;}
 assert.deepEqual(result,{status:'unavailable'});const after=api.invocationObservationUsage(x.handle);assert.equal(after.live,false);assert.equal(after.ordinaryIssued,before.ordinaryIssued+extraIssued);assert.equal(after.controlIssued,before.controlIssued);assert.equal(after.references,0);assert.equal(after.waiters,1);assert.equal(after.waiting,0);assert.equal(after.declaredHeldBytes,before.declaredHeldBytes);
 assert.equal(x.query().status,'unavailable');assert.equal(x.query('minimal-control-status').status,'unavailable');assert.equal(x.advance(w.wait,'poll').status,'unavailable');assert.equal(api.retireInvocationObservation(x.handle,gates).status,'unavailable');
 for(const key of ['ownerControlAllowed','retentionReleased','deliveryDrained'])assert.equal(api.retireInvocationWait(x.handle,w.wait,{...gates,[key]:false}).status,'unavailable');
 assert.equal(api.retireInvocationWait(x.handle,w.wait,gates).status,'retired');assert.equal(api.retireInvocationObservation(x.handle,gates).status,'retired');assert.equal(api.invocationObservationUsage(x.handle),null);
 if(!['outcome','poll'].includes(mode))assert.equal(x.finish().status,'settled');assert.equal(x.current().publication,1);assert.equal(invocationUsage(x.domain).starts,1);
}
function sharedReservationFault(){
 const z=observationAdmissionFixture(),r=z.reserve();assert.equal(r.status,'context');const first=native.queryInvocationObservation(r.context,z.request(r.context),context,policy,gates),w=native.startInvocationWait(r.context,z.request(r.context,'bounded-observation-wait',{after:first.observation,budget:2}),context,policy,gates);assert.equal(w.status,'waiting');
 const before=observationAdmissionUsage(z.pool),producer=copy(invocationUsage(z.domain)),root=z.current(),bad={...gates};Object.defineProperty(bad,'ordinaryReady',{enumerable:true,get(){throw Error('fixed trusted fact access fault');}});
 assert.equal(native.queryInvocationObservation(r.context,z.request(r.context),context,policy,bad).status,'unavailable');assert.equal(native.invocationObservationUsage(r.context).live,false);assert.equal(native.invocationObservationUsage(r.context).waiters,1);assert.equal(native.invocationObservationUsage(r.context).waiting,0);assert.deepEqual(observationAdmissionUsage(z.pool),before);assert.deepEqual(invocationUsage(z.domain),producer);assert.equal(z.current(),root);
 assert.equal(retireObservationReservation(z.pool,r.reservation,gates).status,'unavailable');assert.equal(native.retireInvocationWait(r.context,w.wait,gates).status,'retired');assert.equal(retireObservationReservation(z.pool,r.reservation,gates).status,'retired');const after=observationAdmissionUsage(z.pool);assert.equal(after.reservations,0);for(const scope of ['host','origin']){assert.equal(after.state.used[scope].bytes,after.baseBytes);assert.equal(after.state.used[scope].slots,0);assert.equal(after.state.used[scope].completions,0);assert.equal(after.state.used[scope].work,1);assert.equal(after.state.used[scope].fanout,1);}assert.equal(z.finish().status,'settled');assert.equal(z.current().publication,1);
}
export async function checkInvocationObservationResultFaults(){const detected=[];for(const rule of stages){exercise(await load(null),rule[3],false);exercise(await load(rule),rule[3],true,rule[4]);detected.push(rule[0]);}sharedReservationFault();return {scope:'selected-native-F06-result-invariant-faults',cases:detected.length,stages:detected,sharedReservationFaultCases:1,coherentBaselinePerStage:true,lateIssuedReferenceRetained:true,waiterCapacityRetainedUntilDrain:true,producerUntouched:true,allFaultCrashGlobalPhysicalOrIndependentProof:false};}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(await checkInvocationObservationResultFaults()));
