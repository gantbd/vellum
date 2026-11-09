// Working composition probe, outside the frozen formal input inventory.
// Uses actual adopted owners. It does not install a facade or a native clipboard.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as clipboard from '../formal/clipboard-products.mjs';
import * as invocation from '../formal/invocation-publication.mjs';
import {fixture, g as host, ingress, retire} from '../formal/clipboard-product-checks.mjs';
import {clipboardInvocationContext as context, clipboardInvocationPolicy as policy} from '../formal/clipboard-invocation-checks.mjs';

const limits={maxSlots:4,maxIssued:16,maxIntentBytes:65536,maxResultBytes:4096,maxRootBytes:262144,maxHeldBytes:2097152};
const grants={...host,bookkeepingReserved:true,retained:true,transferReady:true,uniqueDomainAttested:true,schedulerOwned:true,reservationHeld:true,ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true};
const copy=structuredClone;

function originalTransfer(operation){
 const z=fixture(),domain=invocation.openInvocationDomain(z.state,context,limits,grants);
 assert(domain);
 const issued=invocation.issueInvocation(domain,context,grants);assert(issued);
 const request=z.request(operation,issued.reference),before=copy(z.state);
 const admitted=clipboard.admitClipboard(z.owner,z.state,request,host);
 assert.equal(admitted.status,'admitted');
 assert.equal(clipboard.prepareClipboardInvocation(z.owner,admitted.task,limits.maxIntentBytes),null);
 assert.equal(clipboard.admitClipboard(z.owner,z.state,copy(request),host).status,'unavailable');
 assert.deepEqual(clipboard.clipboardUsage(z.owner),{tasks:1,issued:1,outstanding:0,heldBytes:clipboard.clipboardUsage(z.owner).heldBytes});
 const dispatch=z.start(admitted.task);
 assert.deepEqual(dispatch.origin,request.origin);
 assert.equal(dispatch.origin.operation,issued.reference);
 assert.equal(clipboard.dispatchClipboard(z.owner,admitted.task,z.state,host).status,'not-dispatched');
 assert.equal(clipboard.clipboardUsage(z.owner).outstanding,1);
 assert.deepEqual(z.state,before);
 return {z,domain,issued,request,task:admitted.task,before};
}

export function checkClipboardLineage(){
 const controls=[];
 for(const operation of ['paste-request','cut-export-stage']){
  const a=originalTransfer(operation),{z,domain,issued,task}=a;
  assert.equal(z.result(task,'accepted',operation==='paste-request'?{format:'plain',fragments:['Z']}:null).status,'recorded');
  assert.equal(z.result(task,'accepted',operation==='paste-request'?{format:'plain',fragments:['Z']}:null).status,'duplicate');
  const plan=clipboard.prepareClipboardInvocation(z.owner,task,limits.maxIntentBytes);assert(plan);
  assert.equal(clipboard.clipboardInvocationIntent(plan,limits.maxIntentBytes).invocation,issued.reference);
  const job={kind:'clipboard',plan};
  assert.equal(invocation.submitInvocation(domain,issued.ticket,job,'typed',context,policy,grants).status,'accepted');
  assert.equal(invocation.submitInvocation(domain,issued.ticket,job,'dynamic',context,policy,grants).status,'pending');
  assert.equal(invocation.runInvocation(domain,issued.ticket,context,grants,host).status,'settled');
  const root=invocation.invocationAuthorityRoot(domain);
  assert.equal(root.publication,1);
  assert.equal(z.inspect(task).localDecision,'applied');
  assert.equal(invocation.observeInvocation(domain,issued.ticket,context,policy,grants).outcome.effect,'confirmed');
  assert.equal(invocation.submitInvocation(domain,issued.ticket,job,'dynamic',context,policy,grants).status,'observe-existing');
  assert.equal(invocation.runInvocation(domain,issued.ticket,context,grants,host).status,'unavailable');
  assert.equal(invocation.invocationAuthorityRoot(domain),root);
  assert.equal(invocation.invocationUsage(domain).starts,1);
  assert.equal(clipboard.clipboardUsage(z.owner).issued,1);
  assert.equal(clipboard.clipboardUsage(z.owner).outstanding,1);
  assert.equal(clipboard.retireClipboard(z.owner,task,retire).status,'unavailable');
  controls.push(operation+'-original-request-through-once-only-local-publication');
 }

 {
  const {z,domain,issued,task,before}=originalTransfer('copy');
  const row=z.state.clipboardRegistrations[0];
  const exportGrants={...host,provider:row.provider,capability:row.capability};
  assert.equal(clipboard.readClipboardExport(z.owner,task,exportGrants).status,'clipboard-payload');
  assert.equal(z.result(task,'accepted',null).status,'recorded');
  assert.equal(clipboard.prepareClipboardInvocation(z.owner,task,limits.maxIntentBytes),null);
  assert.equal(z.inspect(task).outcome,'accepted');
  assert.deepEqual(invocation.invocationAuthorityRoot(domain),before);
  assert.equal(invocation.invocationUsage(domain).starts,0);
  // Copy's provider acceptance is not a terminal shared invocation outcome.
  assert.equal(invocation.observeInvocation(domain,issued.ticket,context,policy,grants).outcome.observation,'unavailable');
  assert.equal(clipboard.clipboardUsage(z.owner).outstanding,1);
  assert.equal(clipboard.retireClipboard(z.owner,task,retire).status,'unavailable');
  assert.equal(z.quiet(task).status,'recorded');
  assert.equal(clipboard.retireClipboard(z.owner,task,retire).status,'retired');
  controls.push('copy-provider-acceptance-does-not-manufacture-local-terminal-outcome');
 }

 {
  const {z,request,task}=originalTransfer('copy');
  // A fresh attempt is a different native request, not matching-retry lookup.
  const changed=copy(request);changed.origin.attempt='2';
  const other=clipboard.admitClipboard(z.owner,z.state,changed,host);
  assert.equal(other.status,'admitted');assert.notEqual(other.task,task);
  assert.equal(clipboard.clipboardUsage(z.owner).issued,2);
  assert.equal(clipboard.clipboardUsage(z.owner).tasks,2);
  controls.push('minting-a-new-attempt-is-not-an-idempotent-facade-retry');
 }

 {
  const {z,task}=originalTransfer('paste-request');
  const authentic=z.event(task,'result','accepted',{format:'plain',fragments:['Z']});
  for(const patch of [{task:{...task}},{origin:{...authentic.origin,attempt:'foreign'}},{capture:{...authentic.capture,sourceRevision:99}}]){
   assert.equal(clipboard.observeClipboard(z.owner,{...authentic,...patch},ingress).status,'unavailable');
   assert.equal(z.inspect(task).contested,false);
   assert.equal(z.inspect(task).outcome,'unknown');
  }
  assert.equal(clipboard.observeClipboard(z.owner,authentic,ingress).status,'recorded');
  controls.push('foreign-evidence-cannot-contest-the-original-task');
 }
 return {scope:'actual-native-clipboard-lineage-working-reference',controls,
  facadeInstalled:false,publicRequestReplayClosed:false,copySharedTerminalOutcomeClosed:false,
  providerTruthProved:false,newFormalQualification:false};
}

if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
 console.log(JSON.stringify(checkClipboardLineage(),null,2));
}
