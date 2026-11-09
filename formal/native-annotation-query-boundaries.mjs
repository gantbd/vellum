import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeAnnotationCaptureFixture as captureFixture,nativeAnnotationCaptureProfile as captureProfile} from './native-annotation-capture-checks.mjs';
import {nativeAnnotationInstallationProfile as profile,nativeAnnotationInstallationFacts as facts} from './native-annotation-installation-checks.mjs';
const size=v=>Buffer.byteLength(JSON.stringify(v));
function fixture(patch={},sourcePatch={},longProfile=false){
 const x=captureFixture({sourceProfile:{...captureProfile.sourceProfile,...sourcePatch}});for(const name of ['first','second']){const plan=x.z.prepare({operation:'create'},{freshName:name});x.z.execute(plan);}
 if(longProfile){x.scope.profile='p'.repeat(1024);x.request.profile=x.scope.profile;}
 const cut=x.prepare({...x.request,data:['source','annotations']}),handle=api.openInvocationAnnotationContexts(x.z.domain,{...profile,...patch},facts);assert(cut&&handle);const installed=api.installInvocationAnnotationContext(x.z.domain,handle,x.handle,cut,x.scope,facts);assert.equal(installed.status,'context');
 const context=installed.context,page={contract:'annotation-query/1',operation:'enumerate',document:x.document,snapshot:context,profile:x.scope.profile,limit:2,continuation:null},source={contract:'source-query/1',operation:'bounded-range',document:x.document,snapshot:context,profile:x.scope.profile,lo:0,hi:1},line={contract:'source-query/1',operation:'line',document:x.document,snapshot:context,profile:x.scope.profile,line:0};
 const query=(r=page)=>api.queryInvocationAnnotationContext(x.z.domain,handle,r,x.scope,facts),read=(r=source)=>api.queryInvocationCombinedSource(x.z.domain,handle,r,x.scope,facts),usage=()=>api.invocationAnnotationContextUsage(x.z.domain);return {x,context,page,source,line,query,read,usage};
}
export function checkNativeAnnotationQueryBoundaries(){let cases=0;const dimensions=[],visit=(name,f)=>{for(const delta of [-1,0,1]){f(delta);cases++;}dimensions.push(name);};
 const seed=fixture(),page=seed.query();assert.equal(page.status,'page');
 for(const field of ['maxSelectors','maxIssued'])visit('whole-page-'+field,delta=>{const z=fixture({[field]:4+delta}),before=z.usage(),out=z.query();assert.equal(out.status,delta<0?'unavailable':'page');assert.equal(z.usage().selectors,before.selectors+(delta>=0?3:0));assert.equal(z.usage().issued,before.issued+(delta>=0?3:0));assert.equal(z.usage().declaredHeldBytes,before.declaredHeldBytes);});
 visit('whole-page-reply-bytes',delta=>{const z=fixture({maxReplyBytes:size(page)+delta}),before=z.usage(),out=z.query();assert.equal(out.status,delta<0?'unavailable':'page');assert.equal(z.usage().issued,before.issued+(delta>=0?3:0));});
 const recordBytes=Math.max(size({kind:'record',context:seed.context,reference:'0'}),size({kind:'continuation',context:seed.context,limit:2,next:2}));assert(recordBytes-1>=size({kind:'context',context:seed.context}));
 visit('whole-page-selector-metadata-bytes',delta=>{const z=fixture({maxSelectorRecordBytes:recordBytes+delta}),before=z.usage(),out=z.query();assert.equal(out.status,delta<0?'unavailable':'page');assert.equal(z.usage().issued,before.issued+(delta>=0?3:0));});
 visit('captured-page-row-limit',delta=>{const z=fixture({annotationProfile:{...profile.annotationProfile,maxPageRows:2+delta}}),before=z.usage(),out=z.query();assert.equal(out.status,delta<0?'unavailable':'page');assert.equal(z.usage().issued,before.issued+(delta>=0?3:0));});
 const long=fixture({}, {},true),pageBytes=size({...long.page,snapshot:null,continuation:null}),sourceBytes=size({...long.source,snapshot:null});
 for(const [name,bytes,kind]of [['annotation-request-bytes',pageBytes,'annotation'],['source-request-bytes',sourceBytes,'source']])visit(name,delta=>{const z=fixture({maxRequestBytes:bytes+delta},{},true),before=z.usage(),out=kind==='annotation'?z.query():z.read();assert.equal(out.status,delta<0?'unavailable':kind==='annotation'?'page':'content');assert.equal(z.usage().issued,before.issued+(delta>=0&&kind==='annotation'?3:0));if(delta<0)assert.deepEqual(z.usage(),before);});
 visit('captured-source-content-bytes',delta=>{const z=fixture({},{maxContentBytes:1+delta}),before=z.usage();assert.equal(z.read().status,delta<0?'unavailable':'content');assert.deepEqual(z.usage(),before);});
 const bodyBytes=seed.x.z.current().documents[0].source.length;
 visit('captured-line-content-bytes',delta=>{const z=fixture({},{maxContentBytes:bodyBytes+delta}),before=z.usage();assert.equal(z.read(z.line).status,delta<0?'unavailable':'line');assert.deepEqual(z.usage(),before);});
 const source=seed.read(),sourceReplyBytes=size(source),empty=seed.read({...seed.source,hi:0});assert(sourceReplyBytes-1>=size(empty));
 visit('captured-source-reply-bytes',delta=>{const z=fixture({},{maxReplyBytes:sourceReplyBytes+delta}),before=z.usage();assert.equal(z.read().status,delta<0?'unavailable':'content');assert.deepEqual(z.usage(),before);});
 visit('outer-source-reply-bytes',delta=>{const z=fixture({maxReplyBytes:sourceReplyBytes+delta}),before=z.usage();assert.equal(z.read().status,delta<0?'unavailable':'content');assert.deepEqual(z.usage(),before);});
 return {scope:'isolated-native-annotation-source-query-boundaries',cases,dimensions,wholePageIssuanceAndReadOnlyFramesChecked:true,allDominatedBoundsPhysicalOrProductionProfilesProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkNativeAnnotationQueryBoundaries()));
