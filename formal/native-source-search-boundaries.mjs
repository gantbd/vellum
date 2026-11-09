import assert from 'node:assert/strict';
import {parentFundedPoolAttachmentBytes} from './owner-attachment-test-helpers.mjs';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {copySettingsData} from './semantic-settings.mjs';
import {nativeSourceContextFixture,nativeSourceContextFacts as facts} from './native-source-context-checks.mjs';
import {nativeSourceSearchFixture as fixture,nativeSourceSearchProfile as profile,nativeSourceSearchCurrent as current} from './native-source-search-checks.mjs';
const size=v=>Buffer.byteLength(JSON.stringify(v));
const charge=p=>copySettingsData(p,p).bytes+2*p.maxRequestBytes+2*p.maxCaptureBytes+3*p.maxResults*(p.maxCaptureBytes+p.maxRequestBytes+p.limits.maxReplyBytes+p.limits.maxMatches*p.maxSelectorBytes);
export function checkNativeSourceSearchBoundaries(api=kernel){
 let cases=0;const dimensions=[],visit=(name,f)=>{for(const delta of [-1,0,1]){f(delta);cases++;}dimensions.push(name);};
 let fixed={...profile};for(let n=0;n<4;n++)fixed={...fixed,maxHeldBytes:charge(fixed)};assert.equal(fixed.maxHeldBytes,charge(fixed));
 visit('local-reservation-charge',delta=>{const x=nativeSourceContextFixture(api),before=x.z.usage(),p={...fixed,maxHeldBytes:fixed.maxHeldBytes+delta},h=api.openInvocationSourceSearch(x.z.domain,x.handle,p,facts);assert.equal(h!==null,delta>=0);assert.equal(x.z.usage().declaredHeldBytes,before.declaredHeldBytes+(delta>=0?charge(p)+parentFundedPoolAttachmentBytes(api,x.z.domain,h):0));assert.deepEqual(x.usage().holds,0);});
 for(const field of ['maxResults','maxIssued'])visit(field,delta=>{const z=fixture(api,{[field]:1+delta}),r=z.request(),q=current(r),before=z.x.usage(),out=z.query(r,q);assert.equal(out.status,delta<0?'unavailable':'complete');assert.equal(z.usage().issued,delta<0?0:1);assert.equal(z.x.usage().issued,before.issued+(delta<0?0:1));assert.equal(z.usage().results,delta<0?0:1);if(delta>=0){assert.equal(z.retire(out),true);assert.equal(z.usage().issued,1);assert.equal(z.x.usage().holds,0);assert.equal(z.query(r,q).status,field==='maxIssued'&&delta===0?'unavailable':'complete');}});
 const seed=fixture(api),r=seed.request(),q=current(r),out=seed.query(r,q);
 const selector=Math.max(size(r.context),...out.matches.map(m=>size(m.reference)));
 visit('opaque-selector-bytes',delta=>{const z=fixture(api,{maxSelectorBytes:selector+delta}),request=z.request(),before=z.x.usage(),answer=z.query(request,current(request));assert.equal(answer.status,delta<0?'unavailable':'complete');assert.equal(z.usage().results,delta<0?0:1);assert.equal(z.x.usage().holds,delta<0?0:1);assert.equal(z.x.usage().issued,before.issued+(delta<0?0:1));});
 const packet={request:{...r,context:null},current:{...q,request:{...q.request,context:null}}};
 visit('copied-request-and-current-packet-bytes',delta=>{const z=fixture(api,{maxRequestBytes:size(packet)+delta}),request=z.request(),before=z.x.usage(),answer=z.query(request,current(request));assert.equal(answer.status,delta<0?'unavailable':'complete');assert.equal(z.usage().issued,delta<0?0:1);assert.equal(z.x.usage().issued,before.issued+(delta<0?0:1));});
 const d=seed.x.z.root().documents[0],snapshot={ref:r.ref,source:d.source,profile:seed.x.c.profile,selector:r.context},capture=Math.max(size(seed.x.c),size(snapshot));
 visit('scope-and-source-snapshot-capture-bytes',delta=>{const z=fixture(api,{maxCaptureBytes:capture+delta}),request=z.request(),before=z.x.usage(),answer=z.query(request,current(request));assert.equal(answer.status,delta<0?'unavailable':'complete');assert.equal(z.usage().results,delta<0?0:1);assert.equal(z.x.usage().holds,delta<0?0:1);assert.equal(z.usage().issued,delta<0?0:1);assert.equal(z.x.usage().issued,before.issued+(delta<0&&size(seed.x.c)>size(snapshot)?0:1));});
 return {scope:'isolated-native-source-search-exact-boundaries',cases,dimensions,physicalAllocationAsyncOrF11F12UseProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkNativeSourceSearchBoundaries()));
