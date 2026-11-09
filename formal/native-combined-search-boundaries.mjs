import assert from 'node:assert/strict';
import {parentFundedPoolAttachmentBytes} from './owner-attachment-test-helpers.mjs';
import {pathToFileURL} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeCombinedSearchFixture as fixture} from './native-combined-search-checks.mjs';
import {nativeAnnotationCaptureFixture as captureFixture} from './native-annotation-capture-checks.mjs';
import {nativeAnnotationInstallationProfile as profile,nativeAnnotationInstallationFacts as facts} from './native-annotation-installation-checks.mjs';
import {annotationContextCharge as charge} from './native-annotation-installation-boundaries.mjs';
const size=v=>Buffer.byteLength(JSON.stringify(v));
export function checkCombinedSearchBoundaries(){let cases=0;const dimensions=[],visit=(name,f)=>{for(const delta of [-1,0,1]){f(delta);cases++;}dimensions.push(name);};
 for(const [field,min]of [['maxHolds',1],['maxIssued',2]])visit(field,delta=>{const x=fixture({[field]:min+delta}),before=x.z.usage(),out=x.query();assert.equal(out.status,delta<0?'unavailable':'complete');assert.equal(x.z.usage().holds,delta<0?0:1);assert.equal(x.z.usage().issued,before.issued+(delta<0?0:1));assert.equal(x.usage().issued,delta<0?0:1);if(delta>=0){assert.equal(x.retire(out),true);assert.equal(x.z.usage().holds,0);assert.equal(x.z.usage().issued,before.issued+1);}});
 const context={kind:'native-annotation-read-context/1'},metadata=size({context,published:false});
 // Hold metadata has a higher byte floor than the installation row, so its
 // boundary is observable after a successful context installation.
 assert(metadata>size({kind:'context',context}));
 visit('combined-hold-record-bytes',delta=>{const x=fixture({maxSelectorRecordBytes:metadata+delta}),before=x.z.usage(),out=x.query();assert.equal(out.status,delta<0?'unavailable':'complete');assert.equal(x.z.usage().holds,delta<0?0:1);assert.equal(x.z.usage().issued,before.issued+(delta<0?0:1));assert.equal(x.usage().issued,delta<0?0:1);});
 for(const maxHolds of [0,1,2]){
  let fixed={...profile,maxHolds};for(let n=0;n<4;n++)fixed={...fixed,maxHeldBytes:charge(fixed)};assert.equal(fixed.maxHeldBytes,charge(fixed));
  visit(`hold-reservation-envelope-${maxHolds}`,delta=>{const x=captureFixture(),p={...fixed,maxHeldBytes:fixed.maxHeldBytes+delta},before=api.invocationUsage(x.z.domain).declaredHeldBytes,h=api.openInvocationAnnotationContexts(x.z.domain,p,facts);assert.equal(h!==null,delta>=0);assert.equal(api.invocationUsage(x.z.domain).declaredHeldBytes,before+(delta>=0?charge(p)+parentFundedPoolAttachmentBytes(api,x.z.domain,h):0));});
 }
 return {scope:'isolated-native-combined-search-exact-boundaries',cases,dimensions,sharedIssuanceAndRetainedHoldReservationChecked:true,logicalBytesOnly:true,physicalGlobalOrAllProfilesProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkCombinedSearchBoundaries()));
