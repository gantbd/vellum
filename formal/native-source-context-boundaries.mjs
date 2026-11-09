import assert from 'node:assert/strict';
import {parentFundedPoolAttachmentBytes} from './owner-attachment-test-helpers.mjs';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {copySettingsData} from './semantic-settings.mjs';
import {invocationPublicationFixture} from './invocation-publication-checks.mjs';
import {nativeSourceContextFixture as fixture,nativeSourceContextProfile as profile,nativeSourceContextFacts as facts} from './native-source-context-checks.mjs';
const size=v=>Buffer.byteLength(JSON.stringify(v));
const charge=p=>copySettingsData(p,{...p,maxRequestBytes:p.maxRequestBytes}).bytes+2*p.maxRequestBytes+p.maxCaptureBytes+2*p.maxContextBytes+2*p.maxReplyBytes+p.maxContexts*(2*p.maxCaptureBytes+p.maxContextBytes+p.maxReplyBytes)+(p.maxContexts+p.maxHolds)*p.maxSelectorBytes;
export function checkSourceContextBoundaries(api=kernel){
 let cases=0;const dimensions=[];
 const visit=(name,f)=>{for(const delta of [-1,0,1]){f(delta);cases++;}dimensions.push(name);};
 let fixed={...profile};for(let n=0;n<4;n++)fixed={...fixed,maxHeldBytes:charge(fixed)};assert.equal(fixed.maxHeldBytes,charge(fixed));
 visit('local-pool-charge',delta=>{const z=invocationPublicationFixture(api),before=z.usage().declaredHeldBytes,p={...fixed,maxHeldBytes:fixed.maxHeldBytes+delta},handle=api.openInvocationSourceContexts(z.domain,p,facts);assert.equal(handle!==null,delta>=0);assert.equal(z.usage().declaredHeldBytes,before+(delta>=0?charge(p)+parentFundedPoolAttachmentBytes(api,z.domain,handle):0));});
 const parentProbe=fixture(api),attachment=parentFundedPoolAttachmentBytes(api,parentProbe.z.domain,parentProbe.handle);
 visit('actual-parent-headroom',delta=>{const seed=invocationPublicationFixture(api),base=seed.usage().declaredHeldBytes,cost=charge(profile)+attachment,limits={maxSlots:4,maxIssued:16,maxIntentBytes:65536,maxResultBytes:4096,maxRootBytes:262144,maxHeldBytes:base+cost+delta},z=invocationPublicationFixture(api,undefined,limits),before=z.usage().declaredHeldBytes;assert.equal(before,base);const h=api.openInvocationSourceContexts(z.domain,profile,facts);assert.equal(h!==null,delta>=0);assert.equal(z.usage().declaredHeldBytes,before+(delta>=0?cost:0));});
 const seed=fixture(api),d=seed.z.root().documents[0],payload={ref:{authority:d.document.authority,document:d.document.id,incarnation:d.document.incarnation,revision:d.revision},source:d.source,profile:seed.c.profile};
 const minima={maxRootBytes:size(seed.z.root()),maxCaptureBytes:size(payload),maxContextBytes:size(seed.c),maxSelectorBytes:size({kind:'native-source-read-context/1'}),maxReplyBytes:size({status:'context',context:{kind:'native-source-read-context/1'},data:['source']})};
 for(const [field,min]of Object.entries(minima))visit(field,delta=>{const x=fixture(api,{[field]:min+delta}),before=x.usage(),out=x.acquire();assert.equal(out.status,delta<0?'unavailable':'context',field);assert.equal(x.usage().issued,before.issued+(delta>=0?1:0));assert.equal(x.usage().contexts,before.contexts+(delta>=0?1:0));assert.equal(x.usage().declaredHeldBytes,before.declaredHeldBytes);});
 for(const field of ['maxRootBytes','maxInputBytes'])visit('captured-source-'+field,delta=>{const x=fixture(api,{sourceProfile:{...profile.sourceProfile,[field]:3+delta}}),before=x.usage(),out=x.acquire();assert.equal(out.status,delta<0?'unavailable':'context');assert.equal(x.usage().issued,before.issued+(delta>=0?1:0));});
 visit('captured-source-content',delta=>{const x=fixture(api,{sourceProfile:{...profile.sourceProfile,maxContentBytes:3+delta}}),a=x.acquire();assert.equal(a.status,'context');const before=x.usage(),out=x.query(a.context);assert.equal(out.status,delta<0?'unavailable':'content');assert.deepEqual(x.usage(),before);});
 const probe={status:'content',ref:payload.ref,range:[0,0],source:[],unit:'utf8-byte',context:{kind:'native-source-read-context/1'}};
 visit('captured-source-reply',delta=>{const x=fixture(api,{sourceProfile:{...profile.sourceProfile,maxReplyBytes:size(probe)+delta}}),before=x.usage(),out=x.acquire();assert.equal(out.status,delta<0?'unavailable':'context');assert.equal(x.usage().issued,before.issued+(delta>=0?1:0));});
 return {scope:'isolated-native-source-context-exact-boundaries',cases,dimensions,actualParentComparedSeparately:true,physicalAllocationOrAllProfilesProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkSourceContextBoundaries()));
