import assert from 'node:assert/strict';import {pathToFileURL} from 'node:url';
import * as discovery from './native-discovery.mjs';import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';import {documentReadResultEngineDescriptor,annotationReadResultEngineDescriptor} from './read-engine-descriptor.mjs';import {annotationQueryGatewayFixture} from './annotation-query-gateway-checks.mjs';
const profile='owned-contract-discovery/6',copy=structuredClone,no={status:'unavailable'};
function setup(options={},api=discovery,factory=nativeDiscoveryFixture){
 const x=factory(api,undefined,{...options,limits:{maxPageRows:8,maxMetadataDepth:32,...options.limits}}),c={...x.scope,profile},rows=(scope=c)=>api.nativeBuiltinDiscoveryRows(x.owner,scope,x.g()),packet=(scope=c,visibility=x.visibility(rows(scope)??[]),target=null,advice=null)=>api.prepareNativeBuiltinDiscoveryCut(x.owner,scope,visibility,target,advice,x.g()),open=(scope=c,visibility)=>{const p=packet(scope,visibility);assert(p);const out=api.acquireNativeDiscovery(x.owner,scope,p,x.g());api.retireNativeDiscoveryCut(x.owner,p,x.g());return out;},query=(context,r,scope=c,visibility,g=x.g())=>{const p=packet(scope,visibility);assert(p);const out=api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile:scope.profile,context,...r},scope,p,g);api.retireNativeDiscoveryCut(x.owner,p,x.g());return out;};return{...x,c,rows,packet,open,query};
}
export function checkReadResultDiscovery(api=discovery,factory=nativeDiscoveryFixture,annotationFactory=annotationQueryGatewayFixture){
 const controls=[],check=(name,f)=>{f();controls.push(name);},make=options=>setup(options,api,factory);let schemaDenials=0,lossCases=0,descriptors=0;
 check('five-older-builtins-unchanged-only-existing-read-rows-enriched',()=>{
  const x=make(),all=[1,2,3,4,5,6].map(n=>x.rows({...x.c,profile:'owned-contract-discovery/'+n}));assert(all.every(Array.isArray),'all supported discovery profiles need their catalogs');assert.deepEqual(all.map(r=>r.length),[2,3,4,5,7,7]);for(let i=1;i<5;i++)assert.deepEqual(all[i].slice(0,all[i-1].length),all[i-1]);assert.deepEqual(all[5].slice(0,5),all[4].slice(0,5));assert.deepEqual(all[5].slice(5).map(row=>({...row,variants:row.variants.map(v=>{const{result,...outcomes}=v.outcomes;assert(result);return{...v,outcomes};})})),all[4].slice(5));assert(all[4].slice(5).every(r=>r.variants.every(v=>!Object.hasOwn(v.outcomes,'result'))));
 });
 check('inspection-releases-six-exact-native-result-schemas',()=>{
  const x=make(),root=x.z.root(),parent=x.z.usage(),a=x.open(),page=x.query(a.context,{operation:'list',limit:7,continuation:null});assert.equal(page.status,'page');for(const[i,d]of [documentReadResultEngineDescriptor(),annotationReadResultEngineDescriptor()].entries()){const out=x.query(a.context,{operation:'inspect',entry:page.entries[i+5].reference});assert.equal(out.status,'descriptor');assert.deepEqual(out.descriptor.variants.map(({reference,...v})=>v),d.variants.map(({id,...v})=>v));assert(out.descriptor.variants.every(v=>v.outcomes.result.coverage==='native-value-structure-only'));descriptors++;}assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),parent);
 });
 check('captured-schema-ceiling-cannot-expand',()=>{
  const x=make({schema:false}),a=x.open(),visibility=x.visibility(x.rows());visibility.forEach(v=>v.schema=true);const page=x.query(a.context,{operation:'list',limit:7,continuation:null},x.c,visibility);assert.equal(page.status,'page');assert.equal(x.usage().stopped,0);for(const i of [5,6]){const d=x.query(a.context,{operation:'inspect',entry:page.entries[i].reference},x.c,visibility);assert.equal(d.status,'descriptor');assert(d.descriptor.variants.every(v=>Object.keys(v).length===1&&Object.hasOwn(v,'reference')));}
 });
 check('foreign-authority-engine-and-unknown-profile-refuse-without-poisoning',()=>{
  const x=make(),a=x.open(),before=x.usage();for(const c of [{...x.c,authority:'foreign'},{...x.c,engine:'foreign'},{...x.c,profile:'owned-contract-discovery/14'}]){assert.equal(x.rows(c),null);assert.equal(x.packet(c),null);assert.deepEqual(x.usage(),before);}assert.equal(x.query(a.context,{operation:'list',limit:7,continuation:null}).status,'page');
 });
 check('known-schema-withdrawal-is-absorbing',()=>{
  for(const i of [5,6]){const x=make(),a=x.open(),visibility=x.visibility(x.rows());visibility[i].schema=false;assert.deepEqual(x.query(a.context,{operation:'list',limit:7,continuation:null},x.c,visibility),no);assert.equal(x.usage().stopped,1);assert.deepEqual(x.query(a.context,{operation:'list',limit:7,continuation:null}),no);lossCases++;}
 });
 check('older-profile-mismatch-does-not-reinterpret-or-poison-new-context',()=>{
  for(const n of [1,2,3,4,5]){const x=make(),a=x.open(),scope={...x.c,profile:'owned-contract-discovery/'+n};assert.deepEqual(x.query(a.context,{operation:'list',limit:7,continuation:null},scope),no);assert.equal(x.usage().stopped,0);assert.equal(x.query(a.context,{operation:'list',limit:7,continuation:null}).status,'page');}
 });
 check('result-recognition-refuses-missing-widened-foreign-or-executable-metadata',()=>{
  const x=make(),before=x.usage(),base=x.rows();let called=0;
  for(const i of [5,6])for(let v=0;v<3;v++)for(const change of [m=>m.schema.oneOf[0].required.pop(),m=>m.schema.oneOf[0].additionalProperties=true,m=>m.schema.$ref='https://invalid.example/result',m=>m.coverage='authenticity-proved',m=>m.runtimeBindings=[],m=>m.execute=()=>{called++;}]){const rows=copy(base);change(rows[i].variants[v].outcomes.result);assert.equal(x.prepare({scope:x.c,rows,visibility:x.visibility(rows)}),null);assert.deepEqual(x.usage(),before);schemaDenials++;}
  for(const i of [5,6])for(let v=0;v<3;v++){const rows=copy(base);delete rows[i].variants[v].outcomes.result;assert.equal(x.prepare({scope:x.c,rows,visibility:x.visibility(rows)}),null);assert.deepEqual(x.usage(),before);schemaDenials++;}assert.equal(called,0);
 });
 check('bounded-new-profile-refusal-does-not-disable-older-profiles',()=>{
  const x=make({limits:{maxMetadataDepth:16}});assert.equal(x.rows(),null);const old={...x.c,profile:'owned-contract-discovery/5'};assert.equal(x.rows(old).length,7);assert.equal(x.open(old).status,'context');const y=make({limits:{maxEntries:6}});assert.equal(y.rows(),null);const prior={...y.c,profile:'owned-contract-discovery/4'};assert.equal(y.rows(prior).length,5);assert.equal(y.open(prior).status,'context');
 });
 check('generic-equal-metadata-cannot-borrow-builtin-origin',()=>{
  const x=make(),a=x.open(),rows=x.rows(),p=x.prepare({scope:x.c,rows,visibility:x.visibility(rows)});assert(p);assert.deepEqual(api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile,context:a.context,operation:'list',limit:7,continuation:null},x.c,p,x.g()),no);assert.equal(x.query(a.context,{operation:'list',limit:7,continuation:null}).status,'page');
 });
 check('discovery-selectors-do-not-grant-native-annotation-access',()=>{
  const x=make(),a=x.open(),page=x.query(a.context,{operation:'list',limit:7,continuation:null}),d=x.query(a.context,{operation:'inspect',entry:page.entries[6].reference}),z=annotationFactory(),before=z.usage();for(const snapshot of [a.context,page.entries[6].reference,...d.descriptor.variants.map(v=>v.reference)]){assert.equal(z.query(z.envelope(z.request('enumerate',{snapshot}))).status,'unavailable');assert.deepEqual(z.usage(),before);}assert.equal(z.query().status,'page');
 });
 check('known-authority-loss-absorbs-unknown-only-withholds',()=>{
  for(const value of [false,undefined,1]){const x=make(),a=x.open(),r={operation:'list',limit:7,continuation:null};assert.deepEqual(x.query(a.context,r,x.c,undefined,x.g({discoveryAllowed:value})),no);assert.equal(x.usage().stopped,value===false?1:0);assert.equal(x.query(a.context,r).status,value===false?'unavailable':'page');lossCases++;}
 });
 check('protected-retirement-preserves-spent-issuance',()=>{
  const x=make(),a=x.open();x.query(a.context,{operation:'list',limit:7,continuation:null});const issued=x.usage().issued;assert.equal(api.stopNativeDiscovery(x.owner,a.context,x.c,x.g()).status,'stopped');for(const k of ['retentionReleased','deliveryDrained'])assert.deepEqual(api.retireNativeDiscovery(x.owner,a.context,x.g({[k]:false})),no);assert.equal(api.retireNativeDiscovery(x.owner,a.context,x.g()).status,'retired');assert.equal(x.usage().issued,issued);
 });
 check('target-bound-advice-does-not-override-native-authority',()=>{
  const x=make(),rows=x.rows(),cut=()=>api.prepareNativeBuiltinDiscoveryCut(x.owner,x.c,x.visibility(rows),x.target(),x.advice(rows),x.g()),p=cut(),a=api.acquireNativeDiscovery(x.owner,x.c,p,x.g());assert.equal(a.status,'context');assert(a.target);api.retireNativeDiscoveryCut(x.owner,p,x.g());const query=(r,g=x.g())=>{const p=cut();assert(p);const out=api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile,context:a.context,...r},x.c,p,g);api.retireNativeDiscoveryCut(x.owner,p,x.g());return out;},page=query({operation:'list',limit:7,continuation:null});for(const i of [5,6]){const entry=page.entries[i].reference,d=query({operation:'inspect',entry});for(const v of d.descriptor.variants){const r={operation:'advisory-enablement',entry,variant:v.reference,target:a.target};assert.equal(query(r).enabled,true);assert.deepEqual(query(r,x.g({advisoryAdviceAttested:false})),no);assert.deepEqual(query({...r,target:copy(a.target)}),no);}}const z=annotationFactory();assert.equal(z.query(z.envelope(),undefined,z.authority,z.current(),{...z.g,projectionCurrent:false}).status,'unavailable');assert.equal(z.usage().stopped,1);
 });
 return{scope:'owned-native-read-result-discovery',controls,schemaDenials,lossCases,descriptors,olderFiveProfilesUnchanged:true,resultMetadataIsNotAuthorityOrWireCodec:true,physicalGlobalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkReadResultDiscovery()));
export{setup as readResultDiscoveryFixture};
