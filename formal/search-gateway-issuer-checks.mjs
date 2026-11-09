import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeSearchConsumerWitness} from './native-source-search-consumer-witness.mjs';
import {nativeCombinedSearchFixture as combined} from './native-combined-search-checks.mjs';
import {nativeVersionSetSearchFixture as coherent} from './native-version-set-search-checks.mjs';
import {nativeSourceSearchCurrent as current} from './native-source-search-checks.mjs';
import {nativeSourceContextFacts as sourceFacts} from './native-source-context-checks.mjs';
import {nativeAnnotationInstallationFacts as annotationFacts} from './native-annotation-installation-checks.mjs';
const limits={maxRequestBytes:8192,maxInputNodes:16384,maxInputDepth:128},facts={...sourceFacts,...annotationFacts};
function query(route,domain,handle,r,c,q,g){const envelope={route,request:{...(route==='typed'?{family:4}:{name:'search-matches'}),variant:r.operation,version:1,arguments:r}},before=api.invocationUsage(domain).declaredHeldBytes,out=api.queryInvocationDecodedSearch(domain,handle,envelope,limits,api.invocationAuthorityContext(domain),c,q,g);assert.equal(api.invocationUsage(domain).declaredHeldBytes,before);assert.deepEqual(api.invocationSearchDecodeUsage(domain),{busy:false,declaredWorkspaceBytes:0});return out;}
export function checkSearchGatewayIssuers(){const issuers=[],controls=[];let variantCases=0,consumerCases=0;
 for(const route of ['typed','dynamic']){
  for(const issuer of ['source','combined','retained-source','retained-combined','version-set','retained-version-set']){const evidence=nativeSearchConsumerWitness(issuer,(...args)=>query(route,...args));assert.equal(evidence.selectedDependentCurrentnessChecked,true);consumerCases+=evidence.witnesses.length;issuers.push({route,issuer,cases:evidence.witnesses.length});}
  for(const kind of ['combined','version-set'])for(const operation of ['literal-matches','next-match','previous-match']){const z=kind==='combined'?combined():coherent(),r=kind==='combined'?z.request(z.context,operation):z.request(0,operation),c=kind==='combined'?z.z.x.scope:z.scope(),g=kind==='combined'?facts:z.grants(),out=query(route,z.domain,z.handle,r,c,current(r),g);assert.equal(out.status,'complete');assert.equal(out.total,1);assert.equal(out.context,r.context);assert.equal(z.release(out,current(r)),out);assert.equal(z.retire(out),true);variantCases++;}
 }
 controls.push('same-three-issuer-native-owner-for-both-routes','original-and-retained-search-results-remain-dependent-F11-F12-inputs','peer-loss-stop-retirement-and-query-loss-block-consumer-publication','canonical-workspace-is-refunded-without-refunding-native-holds-or-issuance');
 return {scope:'isolated-decoded-search-native-issuer-and-dependent-consumer-composition',issuers,variantCases,consumerCases,controls,allProfilesAsyncWirePhysicalConcurrentOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkSearchGatewayIssuers()));
