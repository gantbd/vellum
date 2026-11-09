// Actual-parent temporary frame funding for the existing fourteen F17 forms.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {annotationInvocationLimits as limits} from './annotation-observation-checks.mjs';
import {lifecycleEnvelopeFixture} from './lifecycle-envelope-checks.mjs';
import {activationEnvelopeFixture,activationEnvelopeIds} from './lifecycle-activation-envelope-checks.mjs';
const forms=[...['create-scratch','attach-view','detach-view','membership','dispose'].map(id=>({id,activation:false})),...activationEnvelopeIds.map(id=>({id,activation:true}))];
const fixture=(form,route='typed')=>form.activation?activationEnvelopeFixture(form.id,route):lifecycleEnvelopeFixture(form.id,route);
const prepare=(x,form,e=x.envelope(),g=x.facts())=>form.activation?x.prepare(e,x.ports,x.controlContext,g):x.prepare(e,undefined,x.controlContext,g);
export function checkLifecycleWorkspace(){
 const units=4*limits.maxIntentBytes,controls=[],control=(name,f)=>{f();controls.push(name);};let nativeCases=0,boundaries=0,refusals=0,faults=0;
 control('both-routes-reserve-the-four-capture-frames-on-the-actual-parent',()=>{
  for(const form of forms)for(const route of ['typed','dynamic']){const x=fixture(form,route),before=x.usage(),root=x.root(),g=x.facts();let observed=0;
   Object.defineProperty(g,'authorized',{get(){observed++;assert.equal(x.usage().declaredHeldBytes,before.declaredHeldBytes+units);assert.equal(prepare(x,form),null);assert.equal(x.retire().status,'unavailable');return true;}});
   assert(prepare(x,form,x.envelope(),g),form.id);assert(observed>0);assert.equal(x.root(),root);assert.deepEqual(x.usage(),before);nativeCases++;
  }
 });
 control('exact-parent-headroom-boundaries-refuse-before-native-preparation',()=>{
  for(const form of forms)for(const delta of [-1,0,1]){const x=fixture(form),before=x.usage(),occupied=limits.maxHeldBytes-before.declaredHeldBytes-units-delta;assert(Number.isSafeInteger(occupied)&&occupied>=0);
   const lease=kernel.reserveInvocationRegistrationReadCharge(x.domain,occupied,x.facts());assert(lease);const filled=x.usage(),root=x.root(),g=x.facts();let nativeCalls=0;Object.defineProperty(g,'authorized',{get(){nativeCalls++;return true;}});
   assert.equal(Boolean(prepare(x,form,x.envelope(),g)),delta>=0,form.id+'/'+delta);assert.deepEqual(x.usage(),filled);assert.equal(x.root(),root);if(delta<0)assert.equal(nativeCalls,0);assert.equal(x.usage().faulted,false);
   assert.equal(kernel.releaseInvocationRegistrationReadCharge(x.domain,lease,{...x.facts(),ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true}),true);assert.deepEqual(x.usage(),before);boundaries++;
  }
 });
 control('ordinary-refusal-and-native-fault-refund-only-temporary-workspace',()=>{
  for(const form of forms){const x=fixture(form),before=x.usage(),root=x.root(),e=x.envelope();e.extra=true;assert.equal(prepare(x,form,e),null);assert.deepEqual(x.usage(),before);assert.equal(x.root(),root);assert(prepare(x,form));refusals++;
   const y=fixture(form),usage=y.usage(),prior=y.root(),g=y.facts();Object.defineProperty(g,'authorized',{get(){assert.equal(y.usage().declaredHeldBytes,usage.declaredHeldBytes+units);throw Error('fixed native fault');}});
   assert.equal(prepare(y,form,y.envelope(),g),null);assert.equal(y.root(),prior);assert.deepEqual(y.usage(),{...usage,faulted:true});assert.equal(y.retire().status,'retired');assert.equal(y.usage().declaredHeldBytes,usage.declaredHeldBytes);assert.equal(y.usage().issued,usage.issued);faults++;
  }
 });
 return {scope:'actual-parent-F17-envelope-workspace-only',controls,forms:forms.length,nativeCases,boundaries,refusals,faults,frames:4,
  actualParentDebitHeadroomAndRefundChecked:true,nativePreparedAndSpentIssuanceRetained:true,
  publicF18FenceOrCompleteF17EventsProved:false,physicalHostileInputHostOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkLifecycleWorkspace()));
