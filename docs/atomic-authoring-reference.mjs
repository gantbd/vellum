// Working native composition evidence, not a public mixed facade or a qualified input.
import assert from 'node:assert/strict';
import * as kernel from '../formal/invocation-publication.mjs';
import * as editing from '../formal/annotation-editing.mjs';
import * as reversal from '../formal/history-reversal.mjs';
import {annotationMembershipEditingFixture} from '../formal/annotation-membership-editing-checks.mjs';
import {prepareAnnotationResultAdmission,captureAnnotationResult,releaseAnnotationResult}
  from '../formal/annotation-results.mjs';

const api={...kernel,...editing,...reversal},copy=structuredClone;
const id=n=>({authority:'a',id:n,incarnation:1});
const fixture=()=>{
  const seed=annotationMembershipEditingFixture(api),state=copy(seed.root());
  const d=state.documents[0],other={...copy(d.records[0]),id:'other'};
  d.records.push(other);
  d.annotationLifecycle.entries.push({...copy(d.annotationLifecycle.entries[0]),id:'other'});
  const z=annotationMembershipEditingFixture(api,'create',{state});
  const target={basis:'candidate',descriptor:z.reference,
    target:{kind:'range',start:8,end:9}};
  const plan={contract:'owned-annotation-plan/1',invocation:z.reference,descriptor:z.reference,
    sources:[{document:id('A'),expectedSource:1,expectedAnnotationSet:1,
      edits:[{lo:8,hi:8,expected:[],ins:[88]}]}],
    annotations:[
      {kind:'create',document:id('A'),id:'fresh',incarnation:1,body:'created',target:copy(target)},
      {kind:'body',document:id('A'),id:'note',incarnation:1,expectedBody:1,body:'changed'},
      {kind:'reattach',document:id('A'),id:'note',incarnation:1,expectedTarget:1,target:copy(target)},
      {kind:'delete',document:id('A'),id:'other',incarnation:1,expectedBody:1,expectedTarget:1}
    ]};
  const h={...z.h,sourceWritable:true,
    membershipAllocationPlan:copy(plan),
    membershipAllocations:[{document:id('A'),id:'fresh',incarnation:1}]};
  return {z,plan,h};
};

const controls=[];
const control=(name,test)=>{test();controls.push(name);};

control('all-four-authoring-operations-and-source-share-one-native-publication',()=>{
  const {z,plan,h}=fixture();
  const receiptInputs=plan.annotations.map(component=>{
    const operation=component.kind==='body'?'update-body':component.kind;
    const annotation={document:copy(component.document),id:component.id,incarnation:component.incarnation};
    const value={request:{operation,invocation:z.reference,document:copy(component.document),annotation},
      allocation:copy(h.annotationAllocation),recipient:h.recipient,profile:h.profile};
    const grant={annotationResultGrant:{invocation:z.reference,annotation,
      identity:true,contentVersion:true,targetGeneration:true}};
    const ceiling=prepareAnnotationResultAdmission(value,{authority:'a',recipient:'reader',profile:'execution'},grant,4096);
    assert(ceiling);
    return {operation,name:component.id,value,grant,ceiling};
  });
  assert.equal(z.submit({kind:'compound',plan}).status,'accepted');
  assert.equal(z.run(h).status,'settled');
  assert.equal(z.read().outcome.publication,'applied');
  const root=z.root(),d=root.documents[0];
  assert.equal(Buffer.from(d.source).toString(),'aé🙂zX');
  assert.equal(root.publication,1);
  assert.equal(root.groups.length,1);
  assert.equal(root.groups[0].changes.length,5);
  assert.equal(root.groups[0].scopes.length,4);
  assert.equal(d.annotationSet,2);
  const note=d.records.find(r=>r.id==='note'),fresh=d.records.find(r=>r.id==='fresh');
  assert.equal(note.body,'changed');
  assert.equal(note.bodyVersion,2);
  assert.equal(note.targetGeneration,2);
  assert.deepEqual(note.target.target,{kind:'range',start:8,end:9});
  assert.equal(fresh.body,'created');
  assert.deepEqual(fresh.target.target,{kind:'range',start:8,end:9});
  assert.equal(d.records.some(r=>r.id==='other'),false);
  assert.equal(d.annotationLifecycle.entries.find(e=>e.id==='other').phase,'retained-absent');

  // Policy helpers are exercised against this exact publication; the coordinator
  // does not yet own/charge a mixed receipt vector or expose its public retrieval.
  for(const {operation,name,value,grant,ceiling} of receiptInputs){
    const receipt=captureAnnotationResult(value,{status:'applied',state:root});
    assert(Object.isFrozen(receipt));
    const out=releaseAnnotationResult(receipt,ceiling,grant,4096);
    assert.equal(out.status,'annotation-result');
    assert.equal(out.result.membership,operation==='delete'?'retained-absent':'present');
    assert.equal(out.result.contentVersion,name==='note'?2:1);
    assert.equal(out.result.targetGeneration,name==='note'?2:1);
    assert.equal(releaseAnnotationResult(receipt,ceiling,
      {annotationResultGrant:{...grant.annotationResultGrant,identity:false,contentVersion:false,targetGeneration:false}},4096).status,'unavailable');
    assert.equal(releaseAnnotationResult(receipt,ceiling,grant,1).status,'unavailable');
  }
});

for(const [name,change] of [
  ['invalid-last-component-preserves-source-records-history-and-publication',plan=>{plan.annotations.at(-1).expectedTarget=2;}],
  ['duplicate-body-components-are-not-sequential-or-last-wins',plan=>{plan.annotations.push({...copy(plan.annotations[1]),body:'last'});}],
  ['create-and-body-for-same-identity-refuse',plan=>{plan.annotations.push({...copy(plan.annotations[1]),id:'fresh'});}],
  ['delete-and-reattach-for-same-identity-refuse',plan=>{plan.annotations.push({...copy(plan.annotations[2]),id:'other'});}],
  ['foreign-candidate-descriptor-refuses-whole-plan',plan=>{plan.annotations[2].target.descriptor='foreign';}]
])control(name,()=>{
  const {z,plan,h}=fixture(),before=z.root();change(plan);
  // Keep actual whole-plan allocation attestation current, so it cannot mask the tested fault.
  h.membershipAllocationPlan=copy(plan);
  assert.equal(z.submit({kind:'compound',plan}).status,'accepted');
  assert.equal(z.run(h).status,'settled');
  assert.equal(z.read().outcome.publication,'notApplied');
  assert.equal(z.root(),before);
});

control('unexpected-host-protocol-fault-is-unknown-not-ordinary-refusal',()=>{
  const {z,plan,h}=fixture(),before=z.root();
  assert.equal(z.submit({kind:'compound',plan}).status,'accepted');
  Object.defineProperty(h,'authorized',{get(){throw Error('injected host protocol fault');}});
  assert.equal(z.run(h).status,'faulted');
  assert.equal(z.read().outcome.publication,'unknown');
  assert.equal(z.read().outcome.control,'faulted');
  assert.equal(z.root(),before);
});

console.log(JSON.stringify({scope:'working-native-atomic-authoring-composition-only',
  controls,wholePublicationParticipants:5,distinctHistoryScopes:4,historicalReceipts:4,
  publicMixedFacadeProved:false,coordinatorRetainedMixedReceiptsProved:false,
  completeReceiptVectorFundingProved:false,
  actualAllocationIssuerTruthProved:false,fullQualificationOrIndependentAssurance:false}));
