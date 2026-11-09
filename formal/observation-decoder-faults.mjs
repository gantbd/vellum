import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as original from './observation-admission.mjs';
import * as native from './invocation-observation.mjs';
import {invocationUsage} from './invocation-publication.mjs';
import {observationDecoderFixture} from './observation-decoder-checks.mjs';
import {invocationObservationContext as context,invocationObservationPolicy as policy} from './invocation-observation-checks.mjs';
import {observationAdmissionGates as gates} from './observation-admission-checks.mjs';
const base=new URL('./',import.meta.url),source=fs.readFileSync(new URL('observation-admission.mjs',base),'utf8'),stages=[['normalize','const n=normalizeInvocationObservationArguments(envelope,c,d.limits,d.limits.maxRequestBytes)??normalizeInvocationObservationArguments(envelope,c,d.limits,d.limits.maxRequestBytes,true);'],['current-packet','const packet=copySettingsData({policy:p,facts:g},d.limits);'],['dispatch','const request=invocationObservationProducerRequest(n);']];
async function load(stage,kind){
 const [name,marker]=stage;assert.equal(source.split(marker).length-1,1,'fault construction target: '+name);
 const probe=kind==='fault'?"if(g?.testDecoderFault===true)throw Error('fixed pre-dispatch decoder fault');":"if(g?.testDecoderReentry===true){const before=observationDecoderUsage(pool,reservation);const nested=queryDecodedPooledObservation(pool,reservation,null,null,null,null);const after=observationDecoderUsage(pool,reservation);const retired=retireObservationReservation(pool,reservation,g);decoderProbe={before,nested,after,retired};}";
 const body=("export let decoderProbe=null;\n"+source.replace(marker,probe+marker)).replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+new URL(p,base).href+q);
 return import('data:text/javascript;base64,'+Buffer.from(body).toString('base64'));
}
function exerciseFault(api,operation,route,faulted){
 const z=observationDecoderFixture(api),after=z.query().observation,w=z.query('bounded-observation-wait','typed',{after,budget:2});assert.equal(w.status,'waiting');
 const parent=api.observationAdmissionUsage(z.pool),local=native.invocationObservationUsage(z.r.context),producer=structuredClone(invocationUsage(z.domain)),root=z.current(),g={...gates,testDecoderFault:true},r=z.query(operation,route,operation==='bounded-observation-wait'?{after,budget:2}:{},context,policy,g);
 assert.deepEqual(invocationUsage(z.domain),producer);assert.equal(z.current(),root);assert.deepEqual(api.observationAdmissionUsage(z.pool),parent);
 if(!faulted){assert.equal(r.status,operation==='bounded-observation-wait'?'waiting':'execution');return;}
 assert.equal(r.status,'unavailable');assert.equal(api.observationDecoderUsage(z.pool,z.r.reservation).busy,false);assert.equal(api.observationDecoderUsage(z.pool,z.r.reservation).faulted,true);const held=native.invocationObservationUsage(z.r.context);assert.equal(held.live,false);assert.equal(held.issued,local.issued);assert.equal(held.waiters,1);assert.equal(held.waiting,0);assert.equal(held.declaredHeldBytes,local.declaredHeldBytes);assert.equal(z.query().status,'unavailable');assert.equal(api.retireObservationReservation(z.pool,z.r.reservation,gates).status,'unavailable');
 assert.equal(native.retireInvocationWait(z.r.context,w.wait,gates).status,'retired');assert.equal(api.retireObservationReservation(z.pool,z.r.reservation,gates).status,'retired');const released=api.observationAdmissionUsage(z.pool);for(const s of ['host','origin']){assert.equal(released.state.used[s].bytes,released.baseBytes);assert.equal(released.state.used[s].slots,0);assert.equal(released.state.used[s].completions,0);assert.equal(released.state.used[s].work,1);assert.equal(released.state.used[s].fanout,1);}assert.equal(z.finish().status,'settled');assert.equal(z.current().publication,1);
}
export async function checkObservationDecoderFaults(){
 let faults=0,reentries=0;for(const stage of stages){
  const failed=await load(stage,'fault');for(const operation of ['status','bounded-observation-wait'])for(const route of ['typed','dynamic']){exerciseFault(original,operation,route,false);exerciseFault(failed,operation,route,true);faults++;}
  const nested=await load(stage,'reentry');for(const operation of ['status','retained-outcome','minimal-control-status','bounded-observation-wait'])for(const route of ['typed','dynamic']){const z=observationDecoderFixture(nested),after=z.query().observation;if(operation==='retained-outcome')z.finish();const parent=nested.observationAdmissionUsage(z.pool),producer=structuredClone(invocationUsage(z.domain)),root=z.current(),r=z.query(operation,route,operation==='bounded-observation-wait'?{after,budget:2}:{},context,policy,{...gates,testDecoderReentry:true});assert.equal(r.status,operation==='retained-outcome'?'outcome':operation==='bounded-observation-wait'?'waiting':'execution');const probe=nested.decoderProbe;assert(probe);assert.equal(probe.before.busy,true);assert.equal(probe.nested.status,'unavailable');assert.equal(probe.after.busy,true);assert.equal(probe.retired.status,'unavailable');assert.deepEqual(nested.observationAdmissionUsage(z.pool),parent);assert.deepEqual(invocationUsage(z.domain),producer);assert.equal(z.current(),root);assert.equal(nested.observationDecoderUsage(z.pool,z.r.reservation).busy,false);assert.equal(nested.observationDecoderUsage(z.pool,z.r.reservation).faulted,false);reentries++;}
 }
 return {scope:'selected-pre-dispatch-F06-decoder-faults',stages:stages.map(s=>s[0]),faults,reentries,coherentBaselinePerFault:true,actualSharedChargeAndProtectedRetirementChecked:true,postDispatchCrashPhysicalGlobalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(await checkObservationDecoderFaults()));
