import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeRetainedLifecycleFixture} from './native-retained-lifecycle-checks.mjs';
import {nativeSourceContextProfile as sourceProfile,nativeSourceContextFacts as facts} from './native-source-context-checks.mjs';
import {nativeSourceSearchProfile as searchProfile,nativeSourceSearchCurrent as current} from './native-source-search-checks.mjs';
export function checkNativeSourceSearchDisposal(){
 const z=nativeRetainedLifecycleFixture(),domain=z.a.x.z.domain,d=z.root().documents[0],document=d.document;
 const scope={document,recipient:'reader',purpose:'query',destination:'client',policy:'policy-1',continuity:'c1',profile:'source-read'},before=api.invocationUsage(domain).declaredHeldBytes;
 const source=api.openInvocationSourceContexts(domain,sourceProfile,facts);assert(source);
 const acquired=api.acquireInvocationSourceContext(domain,source,{contract:'read-context/1',operation:'snapshot-handoff',document,selection:{kind:'capture-current'},data:['source'],profile:scope.profile},scope,facts);assert.equal(acquired.status,'context');
 const search=api.openInvocationSourceSearch(domain,source,searchProfile,facts);assert(search);
 const request={contract:'literal-search/1',operation:'literal-matches',context:acquired.context,ref:{authority:document.authority,document:document.id,incarnation:document.incarnation,revision:d.revision},query:'a',profile:'literal-exact/1',scopes:[{lo:0,hi:d.source.length}],generation:1,navigation:null},q=current(request);
 const result=api.queryInvocationSourceSearch(domain,search,request,scope,q,facts);assert.equal(result.status,'complete');assert.equal(api.releaseInvocationSourceSearchResult(domain,search,result,scope,q,facts),result);
 assert.equal(z.run('dispose').status,'applied');assert.equal(z.root().documents[0].live,false);const disposed=z.root(),charged=api.invocationSourceContextUsage(domain),searchCharged=api.invocationSourceSearchUsage(domain);
 assert.equal(charged.invalidated,0);assert.equal(charged.holds,1);
 // Native disposal, not caller-supplied false facts, fences an existing child.
 assert.deepEqual(api.releaseInvocationSourceSearchResult(domain,search,result,scope,q,facts),{status:'unavailable'});
 assert.deepEqual(api.queryInvocationSourceSearch(domain,search,request,scope,q,facts),{status:'unavailable'});
 const invalid=api.invocationSourceContextUsage(domain);assert.equal(invalid.invalidated,1);assert.equal(invalid.retainedRoots,1);assert.equal(invalid.holds,1);assert.equal(invalid.issued,charged.issued);assert.equal(invalid.declaredHeldBytes,charged.declaredHeldBytes);assert.deepEqual(api.invocationSourceSearchUsage(domain),searchCharged);
 assert.equal(api.retireInvocationSourceContext(domain,source,acquired.context,facts),false);assert.equal(api.retireInvocationSourceSearch(domain,search,facts),false);
 let denials=0;for(const field of ['ownerControlAllowed','retentionReleased','deliveryDrained'])for(const value of [false,undefined,1]){assert.equal(api.retireInvocationSourceSearchResult(domain,search,result,{...facts,[field]:value}),false);assert.deepEqual(api.invocationSourceContextUsage(domain),invalid);assert.deepEqual(api.invocationSourceSearchUsage(domain),searchCharged);denials++;}
 assert.equal(api.retireInvocationSourceSearchResult(domain,search,structuredClone(result),facts),false);
 // Cleanup needs protected discharge, not a renewed read or query grant.
 const cleanup={ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true};
 assert.equal(api.retireInvocationSourceSearchResult(domain,search,result,cleanup),true);assert.equal(api.retireInvocationSourceSearchResult(domain,search,result,cleanup),false);assert.equal(api.invocationSourceContextUsage(domain).retainedRoots,0);assert.equal(api.invocationSourceContextUsage(domain).holds,0);assert.equal(api.invocationSourceContextUsage(domain).issued,charged.issued);assert.equal(api.invocationSourceSearchUsage(domain).issued,searchCharged.issued);
 assert.equal(api.retireInvocationSourceContext(domain,source,acquired.context,cleanup),true);assert.equal(api.retireInvocationSourceContexts(domain,source,cleanup),true);assert.equal(api.retireInvocationSourceSearch(domain,search,cleanup),true);assert.equal(api.retireInvocationSourceSearch(domain,search,cleanup),false);assert.equal(api.invocationUsage(domain).declaredHeldBytes,before);assert.equal(z.root(),disposed);
 return {scope:'isolated-native-search-disposal-composition',nativeLifecycleCases:1,protectedDischargeDenials:denials,actualDisposalAndOnceOnlyLogicalDischargeChecked:true,eagerBulkInvalidationPhysicalReclamationF11F12AsyncOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkNativeSourceSearchDisposal()));
