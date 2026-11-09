import assert from 'node:assert/strict';
import {parentFundedPoolAttachmentBytes} from './owner-attachment-test-helpers.mjs';
import {pathToFileURL} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeRetainedLifecycleFixture} from './native-retained-lifecycle-checks.mjs';
import {nativeAnnotationCaptureProfile as captureProfile} from './native-annotation-capture-checks.mjs';
import {nativeAnnotationInstallationProfile as profile,nativeAnnotationInstallationFacts as annotationFacts} from './native-annotation-installation-checks.mjs';
import {nativeSourceContextFacts as sourceFacts} from './native-source-context-checks.mjs';
import {nativeSourceSearchProfile as searchProfile,nativeSourceSearchCurrent as current} from './native-source-search-checks.mjs';
const facts={...sourceFacts,...annotationFacts};
export function checkCombinedSearchLifecycle(retained=false){let cases=0,protectedDenials=0;
 for(const event of ['dispose','fault']){
  const z=nativeRetainedLifecycleFixture(),domain=z.a.x.z.domain,d=z.root().documents[0],scope={document:d.document,recipient:'reader',purpose:'query',destination:'client',policy:'p1',continuity:'c1',profile:'read'},before=api.invocationUsage(domain).declaredHeldBytes;
  const capture=api.openInvocationAnnotationCapture(domain,captureProfile,facts);assert(capture);
  const acquisition={contract:'read-context/1',operation:'snapshot-handoff',document:d.document,selection:{kind:'capture-current'},data:['source','annotations'],profile:scope.profile},rows=d.records.map(r=>({id:r.id,incarnation:r.incarnation,visible:true,body:true,target:true})),cut=api.prepareInvocationAnnotationCapture(domain,capture,acquisition,scope,rows,facts);assert(cut);
  const contextPool=api.openInvocationAnnotationContexts(domain,profile,facts);assert(contextPool);let acquired=api.installInvocationAnnotationContext(domain,contextPool,capture,cut,scope,facts);assert.equal(acquired.status,'context');
  if(retained){const original=acquired.context;acquired=api.acquireInvocationRetainedAnnotationContext(domain,contextPool,{...acquisition,selection:{kind:'retained',reference:original}},scope,facts);assert.equal(acquired.status,'context');assert.notEqual(acquired.context,original);assert.equal(api.stopInvocationAnnotationContext(domain,contextPool,{contract:'read-context/1',operation:'stop-observation',context:original},scope,facts).status,'stopped');assert.equal(api.retireInvocationAnnotationContext(domain,contextPool,original,facts),true);}
  const search=api.openInvocationSourceSearch(domain,contextPool,searchProfile,facts);assert(search);
  const request={contract:'literal-search/1',operation:'literal-matches',context:acquired.context,ref:{authority:d.document.authority,document:d.document.id,incarnation:d.document.incarnation,revision:d.revision},query:'a',profile:'literal-exact/1',scopes:[{lo:0,hi:d.source.length}],generation:1,navigation:null},q=current(request),result=api.queryInvocationSourceSearch(domain,search,request,scope,q,facts);assert.equal(result.status,'complete');
  assert.equal(api.retireInvocationAnnotationCapture(domain,capture,facts),true);
  assert.equal(api.releaseInvocationSourceSearchResult(domain,search,result,scope,q,facts),result);
  if(event==='dispose'){assert.equal(z.run('dispose').status,'applied');assert.equal(z.root().documents[0].live,false);}
  else{const context=api.invocationAuthorityContext(domain),issued=api.issueInvocation(domain,context,z.a.g());assert(issued);const job={kind:'compound',plan:{contract:'owned-annotation-plan/1',invocation:issued.reference,descriptor:'fault',sources:[],annotations:[{kind:'body',document:null,id:'note',incarnation:1,expectedBody:1,body:'bad'}]}};assert.equal(api.submitInvocation(domain,issued.ticket,job,'typed',context,{publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:context.recipient},z.a.g()).status,'accepted');assert.equal(api.runInvocation(domain,issued.ticket,context,z.a.g(),z.a.x.z.h).status,'faulted');assert.equal(api.invocationUsage(domain).faulted,true);}
  const root=z.root(),parent=api.invocationUsage(domain).declaredHeldBytes,contextHeld=api.invocationAnnotationContextUsage(domain),searchHeld=api.invocationSourceSearchUsage(domain),attachment=parentFundedPoolAttachmentBytes(api,domain,contextPool)+parentFundedPoolAttachmentBytes(api,domain,search);
  if(retained)assert.equal(api.acquireInvocationRetainedAnnotationContext(domain,contextPool,{...acquisition,selection:{kind:'retained',reference:acquired.context}},scope,facts).status,'unavailable');
  assert.equal(api.releaseInvocationSourceSearchResult(domain,search,result,scope,q,facts).status,'unavailable');
  assert.equal(api.queryInvocationSourceSearch(domain,search,request,scope,q,facts).status,'unavailable');
  const stopped=api.invocationAnnotationContextUsage(domain);assert.equal(stopped.holds,1);assert.equal(stopped.issued,contextHeld.issued);assert.equal(stopped.declaredHeldBytes,contextHeld.declaredHeldBytes);
  assert.deepEqual(api.invocationSourceSearchUsage(domain),searchHeld);
  assert.equal(api.retireInvocationAnnotationContext(domain,contextPool,acquired.context,facts),false);
  assert.equal(api.retireInvocationSourceSearch(domain,search,facts),false);
  for(const field of ['ownerControlAllowed','retentionReleased','deliveryDrained'])for(const value of [false,undefined,1]){assert.equal(api.retireInvocationSourceSearchResult(domain,search,result,{...facts,[field]:value}),false);assert.deepEqual(api.invocationAnnotationContextUsage(domain),stopped);assert.deepEqual(api.invocationSourceSearchUsage(domain),searchHeld);protectedDenials++;}
  const cleanup={ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true};
  assert.equal(api.retireInvocationSourceSearchResult(domain,search,structuredClone(result),cleanup),false);
  assert.equal(api.retireInvocationSourceSearchResult(domain,search,result,cleanup),true);assert.equal(api.retireInvocationSourceSearchResult(domain,search,result,cleanup),false);
  assert.equal(api.invocationAnnotationContextUsage(domain).holds,0);assert.equal(api.invocationAnnotationContextUsage(domain).issued,contextHeld.issued);assert.equal(api.invocationSourceSearchUsage(domain).issued,searchHeld.issued);
  assert.equal(api.retireInvocationAnnotationContext(domain,contextPool,acquired.context,cleanup),true);
  assert.equal(api.retireInvocationAnnotationContexts(domain,contextPool,cleanup),true);assert.equal(api.retireInvocationSourceSearch(domain,search,cleanup),true);
  assert.equal(api.invocationUsage(domain).declaredHeldBytes,parent-contextHeld.declaredHeldBytes-searchHeld.declaredHeldBytes-attachment);assert.equal(api.invocationUsage(domain).declaredHeldBytes,before);assert.equal(z.root(),root);cases++;
 }
 return {scope:'isolated-native-combined-search-lifecycle',cases,protectedDenials,retainedContextChecked:retained,actualDisposalAndOwnerFaultChecked:true,onceOnlyLogicalDischargeChecked:true,eagerBulkInvalidationPhysicalConcurrentCrashOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkCombinedSearchLifecycle()));
