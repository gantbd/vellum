import assert from 'node:assert/strict';
import {parentFundedPoolAttachmentBytes} from './owner-attachment-test-helpers.mjs';
import {pathToFileURL} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeRetainedLifecycleFixture} from './native-retained-lifecycle-checks.mjs';
import {nativeSourceContextProfile as profile,nativeSourceContextFacts as facts} from './native-source-context-checks.mjs';
export function checkRetainedSourceLifecycle(){let cases=0,protectedDenials=0;
 for(const event of ['dispose','fault']){
  const z=nativeRetainedLifecycleFixture(),domain=z.a.x.z.domain,d=z.root().documents[0],scope={document:d.document,recipient:'reader',purpose:'query',destination:'client',policy:'p1',continuity:'c1',profile:'source-read'},handle=api.openInvocationSourceContexts(domain,profile,facts);assert(handle);
  const request={contract:'read-context/1',operation:'snapshot-handoff',document:d.document,selection:{kind:'capture-current'},data:['source'],profile:scope.profile},acquire=r=>api.acquireInvocationSourceContext(domain,handle,r,scope,facts),original=acquire(request);assert.equal(original.status,'context');
  const copied=acquire({...request,selection:{kind:'retained',reference:original.context}});assert.equal(copied.status,'context');assert.notEqual(copied.context,original.context);
  assert.equal(api.stopInvocationSourceContext(domain,handle,{contract:'read-context/1',operation:'stop-observation',context:original.context},scope,facts).status,'stopped');assert.equal(api.retireInvocationSourceContext(domain,handle,original.context,facts),true);
  const hold=api.holdInvocationSourceContext(domain,handle,copied.context,facts);assert(hold);
  if(event==='dispose'){assert.equal(z.run('dispose').status,'applied');assert.equal(z.root().documents[0].live,false);}
  else{const context=api.invocationAuthorityContext(domain),issued=api.issueInvocation(domain,context,z.a.g());assert(issued);const job={kind:'compound',plan:{contract:'owned-annotation-plan/1',invocation:issued.reference,descriptor:'fault',sources:[],annotations:[{kind:'body',document:null,id:'note',incarnation:1,expectedBody:1,body:'bad'}]}};assert.equal(api.submitInvocation(domain,issued.ticket,job,'typed',context,{publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:context.recipient},z.a.g()).status,'accepted');assert.equal(api.runInvocation(domain,issued.ticket,context,z.a.g(),z.a.x.z.h).status,'faulted');}
  const root=z.root(),charged=api.invocationUsage(domain).declaredHeldBytes,before=api.invocationSourceContextUsage(domain),attachment=parentFundedPoolAttachmentBytes(api,domain,handle);
  assert.equal(acquire({...request,selection:{kind:'retained',reference:copied.context}}).status,'unavailable');
  // Refusing new admission is not an observation on an existing context. Observe
  // the actual retained consumer before checking its absorbing loss state.
  assert.equal(api.queryInvocationSourceContext(domain,handle,{contract:'source-query/1',operation:'bounded-range',document:d.document,snapshot:copied.context,profile:scope.profile,lo:0,hi:d.source.length},scope,facts).status,'unavailable');
  const invalid=api.invocationSourceContextUsage(domain);assert.equal(invalid.invalidated,1);assert.equal(invalid.holds,1);assert.equal(invalid.retainedRoots,1);assert.equal(invalid.issued,before.issued);assert.equal(invalid.declaredHeldBytes,before.declaredHeldBytes);
  assert.equal(api.queryInvocationSourceContext(domain,handle,{contract:'source-query/1',operation:'bounded-range',document:d.document,snapshot:copied.context,profile:scope.profile,lo:0,hi:d.source.length},scope,facts).status,'unavailable');
  assert.equal(api.retireInvocationSourceContext(domain,handle,copied.context,facts),false);
  for(const field of ['ownerControlAllowed','retentionReleased','deliveryDrained'])for(const value of [false,undefined,1]){assert.equal(api.releaseInvocationSourceContextHold(domain,handle,hold,{...facts,[field]:value}),false);assert.deepEqual(api.invocationSourceContextUsage(domain),invalid);protectedDenials++;}
  const cleanup={ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true};assert.equal(api.releaseInvocationSourceContextHold(domain,handle,hold,cleanup),true);assert.equal(api.releaseInvocationSourceContextHold(domain,handle,hold,cleanup),false);assert.equal(api.invocationSourceContextUsage(domain).retainedRoots,0);assert.equal(api.retireInvocationSourceContext(domain,handle,copied.context,cleanup),true);assert.equal(api.retireInvocationSourceContexts(domain,handle,cleanup),true);assert.equal(api.invocationUsage(domain).declaredHeldBytes,charged-before.declaredHeldBytes-attachment);assert.equal(z.root(),root);cases++;
 }
 return {scope:'isolated-native-retained-source-lifecycle',cases,protectedDenials,actualDisposalFaultAndProtectedDischargeChecked:true,allHostPhysicalConcurrentCrashOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkRetainedSourceLifecycle()));
