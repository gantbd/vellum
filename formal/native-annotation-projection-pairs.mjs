import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeAnnotationCaptureFixture as captureFixture} from './native-annotation-capture-checks.mjs';
import {nativeAnnotationInstallationProfile as profile,nativeAnnotationInstallationFacts as facts} from './native-annotation-installation-checks.mjs';
function world(mask,secret=null){const x=captureFixture();if(secret!==null){const plan=x.z.prepare({operation:'create'},{freshName:'private-'+secret,body:'secret'.repeat(secret+1)});x.z.execute(plan);}
 const rows=x.grants().map(row=>({...row,visible:!row.id.startsWith('private-'),body:!row.id.startsWith('private-')&&!!(mask&1),target:!row.id.startsWith('private-')&&!!(mask&2)})),cut=x.prepare(x.request,{...facts,sourceReadable:false},rows);assert(cut);const handle=api.openInvocationAnnotationContexts(x.z.domain,profile,facts);assert(handle);const installed=api.installInvocationAnnotationContext(x.z.domain,handle,x.handle,cut,x.scope,{...facts,sourceReadable:false});assert.equal(installed.status,'context');
 const common={contract:'annotation-query/1',document:x.document,snapshot:installed.context,profile:x.scope.profile},query=r=>api.queryInvocationAnnotationContext(x.z.domain,handle,r,x.scope,{...facts,sourceReadable:false}),page=query({...common,operation:'enumerate',limit:2,continuation:null});assert.equal(page.status,'page');return {x,handle,common,query,page,usage:api.invocationAnnotationContextUsage(x.z.domain)};
}
export function checkNativeAnnotationProjectionPairs(){let pairedCases=0,productPairs=0;
 for(let mask=0;mask<4;mask++)for(let secret=0;secret<3;secret++){
  const left=world(mask),right=world(mask,secret);assert.deepEqual(left.page,right.page);assert.deepEqual(left.usage,right.usage);assert.equal(left.page.records.length,1);assert.equal(left.page.continuation,null);assert.equal(JSON.stringify(right.page).includes('secret'),false);assert.equal(JSON.stringify(right.page).includes('private-'),false);
  for(const operation of ['inspect','target-location']){const a=left.query({...left.common,operation,annotation:left.page.records[0].reference}),b=right.query({...right.common,operation,annotation:right.page.records[0].reference});assert.deepEqual(a,b);productPairs++;}
  assert.equal(Object.hasOwn(right.page.records[0],'content'),!!(mask&1));assert.equal(Object.hasOwn(right.page.records[0],'location'),!!(mask&2));pairedCases++;
 }
 return {scope:'isolated-native-annotation-projected-payload-pairs',pairedCases,productPairs,actualHiddenAuthoringAndFixedReservationCompared:true,timingCapacityAllWorldOrWireNoninterferenceProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkNativeAnnotationProjectionPairs()));
