import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeSourceContextFixture as fixture,nativeSourceContextProfile as profile,nativeSourceContextFacts as facts} from './native-source-context-checks.mjs';
const size=v=>Buffer.byteLength(JSON.stringify(v));
export function checkRetainedSourceBoundaries(){let cases=0;const dimensions=[],visit=(name,f)=>{for(const delta of [-1,0,1]){f(delta);cases++;}dimensions.push(name);};
 for(const field of ['maxContexts','maxIssued'])visit(field,delta=>{const x=fixture(api,{[field]:2+delta}),a=x.acquire().context,before=x.usage(),out=x.acquire({...x.request,selection:{kind:'retained',reference:a}});assert.equal(out.status,delta<0?'unavailable':'context');assert.equal(x.usage().issued,before.issued+(delta>=0?1:0));assert.equal(x.usage().contexts,before.contexts+(delta>=0?1:0));assert.equal(x.usage().declaredHeldBytes,before.declaredHeldBytes);});
 const profileName='source-read-'.repeat(64),seed=fixture(),scope={...seed.c,profile:profileName},currentRequest={...seed.request,profile:profileName},wire={...currentRequest,selection:{kind:'retained',reference:null}},minimum=size(wire);assert(minimum>size(profile));assert(size(currentRequest)<minimum);
 visit('retained-request-envelope',delta=>{const x=fixture(api,{maxRequestBytes:minimum+delta}),c={...x.c,profile:profileName},r={...x.request,profile:profileName},a=x.acquire(r,c).context;assert(a);const before=x.usage(),out=x.acquire({...r,selection:{kind:'retained',reference:a}},c);assert.equal(out.status,delta<0?'unavailable':'context');assert.equal(x.usage().issued,before.issued+(delta>=0?1:0));assert.equal(x.usage().contexts,before.contexts+(delta>=0?1:0));assert.equal(x.usage().declaredHeldBytes,before.declaredHeldBytes);});
 return {scope:'isolated-native-retained-source-exact-boundaries',cases,dimensions,retainedAdmissionAndNonrefundingIssuanceChecked:true,physicalOrAllProfilesProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkRetainedSourceBoundaries()));
