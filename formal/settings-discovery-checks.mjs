import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import * as discovery from './native-discovery.mjs';
import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';
import {settingsQueryEngineDescriptor} from './settings-query-descriptor.mjs';
import {settingsGatewayFixture} from './settings-query-gateway-checks.mjs';
const profile='owned-contract-discovery/9',copy=structuredClone,no={status:'unavailable'};
// Pinned from the audited and adopted specification 0.174, not this candidate.
const olderProfileGolden=['3107d5c1148abb0743cadddfb89ed1e649275fa1e4209e73efd51c3360a809fb','fd988802398c328878e6da6e0a224781477bcd8f09c38a6df05c9a214a7ca7f0','fede183bbf34068421706596e77b342d20a0b625d276ac2a1e61eb32e0990bce','3ceed9660c483e92747834c85a91370c5d8017b294110bdce2d9503b4fbeeaa5','a40ee43d0b13986d14737cc20ac686a9d864e1956ace2b700003924acefed6d0','ef1e1e7a3d3a6b56a916f297c3d5d13d69c0f8ac4d4d57da4dc0a227092e59aa','b94f6a1b69cc131796f4fd26078cc0a6b64ddfc40c84671856b73837157bbaf4','0b7ad19fc82d8bae95637f07242e1e9c65b9bd58b6ef59b18ff6fe1b4dfeb865'];
const canonical=v=>Array.isArray(v)?v.map(canonical):v&&typeof v==='object'?Object.fromEntries(Object.keys(v).sort().map(k=>[k,canonical(v[k])])):v;
const fingerprint=v=>createHash('sha256').update(JSON.stringify(canonical(v))).digest('hex');
export function settingsDiscoveryFixture(options={},api=discovery,factory=nativeDiscoveryFixture){
 const x=factory(api,undefined,{...options,limits:{maxPageRows:10,maxMetadataDepth:64,...options.limits}}),c={...x.scope,profile},rows=(scope=c)=>api.nativeBuiltinDiscoveryRows(x.owner,scope,x.g()),packet=(scope=c,visibility=x.visibility(rows(scope)??[]),target=null,advice=null)=>api.prepareNativeBuiltinDiscoveryCut(x.owner,scope,visibility,target,advice,x.g()),open=(scope=c,visibility)=>{const p=packet(scope,visibility);assert(p);const out=api.acquireNativeDiscovery(x.owner,scope,p,x.g());api.retireNativeDiscoveryCut(x.owner,p,x.g());return out;},query=(context,r,scope=c,visibility,g=x.g())=>{const p=packet(scope,visibility);assert(p);const out=api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile:scope.profile,context,...r},scope,p,g);api.retireNativeDiscoveryCut(x.owner,p,x.g());return out;};
 return {...x,c,rows,packet,open,query};
}
export function checkSettingsDiscovery(api=discovery,factory=nativeDiscoveryFixture,readerFactory=settingsGatewayFixture){
 const controls=[],check=(name,f)=>{f();controls.push(name);},make=options=>settingsDiscoveryFixture(options,api,factory);let schemaDenials=0,lossCases=0,descriptors=0,nativeAuthorityRefusals=0;
 check('eight-older-profiles-unchanged-nine-appends-only-F07',()=>{
  const x=make(),all=Array.from({length:9},(_,n)=>x.rows({...x.c,profile:'owned-contract-discovery/'+(n+1)}));assert(all.every(Array.isArray));assert.deepEqual(all.map(r=>r.length),[2,3,4,5,7,7,8,9,10]);assert.deepEqual(all.slice(0,8).map(fingerprint),olderProfileGolden);assert.deepEqual(all[8].slice(0,9),all[7]);const row=all[8][9],d=settingsQueryEngineDescriptor();assert.equal(row.contract,d.contract);assert.equal(row.kind,'query');assert.deepEqual(row.variants,d.variants);assert.equal(row.command.id,JSON.stringify([x.c.engine,'builtin','F07']));assert.equal(row.command.authority,x.c.authority);assert.equal(row.command.incarnation,1);
 });
 check('owned-inspection-releases-three-synchronous-products-and-fixed-local-type-references',()=>{
  const x=make(),root=x.z.root(),parent=x.z.usage(),a=x.open(),page=x.query(a.context,{operation:'list',limit:10,continuation:null});assert.equal(page.status,'page');assert.equal(page.entries.length,10);const r=x.query(a.context,{operation:'inspect',entry:page.entries[9].reference});assert.equal(r.status,'descriptor');assert.deepEqual(r.descriptor.variants.map(({reference,...v})=>v),settingsQueryEngineDescriptor().variants.map(({id,...v})=>v));assert(r.descriptor.variants.every(v=>v.outcomes.events==='no-asynchronous-events-in-this-synchronous-profile'));assert.deepEqual(Object.keys(r.descriptor.variants[2].outcomes.result.schema.$defs),['settingType']);assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),parent);descriptors++;
 });
 check('captured-schema-ceiling-does-not-expand',()=>{
  const x=make({schema:false}),a=x.open(),visibility=x.visibility(x.rows());visibility.forEach(v=>v.schema=true);const page=x.query(a.context,{operation:'list',limit:10,continuation:null},x.c,visibility);assert.equal(page.status,'page');const r=x.query(a.context,{operation:'inspect',entry:page.entries[9].reference},x.c,visibility);assert.equal(r.status,'descriptor');assert(r.descriptor.variants.every(v=>Object.keys(v).length===1&&Object.hasOwn(v,'reference')));assert.equal(x.usage().stopped,0);
 });
 check('recognized-metadata-withdrawal-stops-without-regrant-revival',()=>{
  const x=make(),a=x.open(),visibility=x.visibility(x.rows());visibility[9].schema=false;assert.deepEqual(x.query(a.context,{operation:'list',limit:10,continuation:null},x.c,visibility),no);assert.equal(x.usage().stopped,1);assert.deepEqual(x.query(a.context,{operation:'list',limit:10,continuation:null}),no);lossCases++;
 });
 check('foreign-authority-engine-unknown-twelve-and-cross-profile-cuts-refuse',()=>{
  const x=make(),a=x.open(),before=x.usage();for(const c of [{...x.c,authority:'foreign'},{...x.c,engine:'foreign'},{...x.c,profile:'owned-contract-discovery/14'}]){assert.equal(x.rows(c),null);assert.equal(x.packet(c),null);assert.deepEqual(x.usage(),before);}for(let n=1;n<=8;n++)assert.deepEqual(x.query(a.context,{operation:'list',limit:10,continuation:null},{...x.c,profile:'owned-contract-discovery/'+n}),no);assert.equal(x.usage().stopped,0);assert.equal(x.query(a.context,{operation:'list',limit:10,continuation:null}).status,'page');
 });
 check('settings-schemas-cannot-enter-older-profiles',()=>{
  const x=make(),row=x.rows()[9];for(let n=1;n<=8;n++){const c={...x.c,profile:'owned-contract-discovery/'+n},rows=[...x.rows(c),row];assert.equal(api.prepareNativeDiscoveryCut(x.owner,c,rows,x.visibility(rows),null,null,x.g()),null);schemaDenials++;}
 });
 check('exact-input-result-and-type-recognition-refuses-widening-and-loaders',()=>{
  const x=make();let calls=0;for(let index=0;index<3;index++)for(const change of [v=>v.input.schema.additionalProperties=true,v=>v.input.schema.required.pop(),v=>v.input.schema.$ref='https://invalid.example/input',v=>v.input.schema.execute=()=>{calls++;},v=>v.outcomes.result.runtimeBindings=[],v=>v.outcomes.result.notEstablished=[],v=>v.outcomes.result.schema.oneOf[0].additionalProperties=true,v=>v.outcomes.events='public-event-stream']){const rows=copy(x.rows());change(rows[9].variants[index]);assert.equal(api.prepareNativeDiscoveryCut(x.owner,x.c,rows,x.visibility(rows),null,null,x.g()),null);schemaDenials++;}
  for(const change of [v=>v.outcomes.result.schema.$defs.settingType.oneOf.pop(),v=>v.outcomes.result.schema.$defs.settingType.oneOf[4].properties.element.$ref='https://invalid.example/type',v=>v.outcomes.result.schema.$defs.settingType.oneOf[5].properties.fields.additionalProperties.$ref='#/$defs/other']){const rows=copy(x.rows());change(rows[9].variants[2]);assert.equal(api.prepareNativeDiscoveryCut(x.owner,x.c,rows,x.visibility(rows),null,null,x.g()),null);schemaDenials++;}assert.equal(calls,0);assert.equal(x.open().status,'context');
 });
 check('nine-retains-read-history-and-observation-recognizers',()=>{
  const x=make();for(const row of [5,6,7,8])for(let index=0;index<3;index++)for(const change of [v=>v.input.schema.additionalProperties=true,v=>v.outcomes.result.runtimeBindings=[],v=>v.outcomes.result.schema.oneOf.pop()]){const rows=copy(x.rows());change(rows[row].variants[index]);assert.equal(api.prepareNativeDiscoveryCut(x.owner,x.c,rows,x.visibility(rows),null,null,x.g()),null);schemaDenials++;}
 });
 check('publication-exclusion-and-malformed-selectors-preserve-owner',()=>{
  const x=make(),a=x.open(),page=x.query(a.context,{operation:'list',limit:10,continuation:null}),before=x.usage(),root=x.z.root(),parent=x.z.usage();for(const [r,g]of [[{operation:'list',limit:10,continuation:null},x.g({outsidePublicationGuard:false})],[{operation:'list',limit:10,continuation:null,extra:true},x.g()],[{operation:'inspect',entry:{}},x.g()],[{operation:'inspect',entry:copy(page.entries[9].reference)},x.g()],[{operation:'list',limit:0,continuation:null},x.g()],[{operation:'execute'},x.g()]]){assert.deepEqual(x.query(a.context,r,x.c,undefined,g),no);assert.deepEqual(x.usage(),before);assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),parent);}
 });
 check('equal-attested-metadata-cannot-borrow-fixed-origin',()=>{
  const x=make(),rows=x.rows(),generic=()=>api.prepareNativeDiscoveryCut(x.owner,x.c,rows,x.visibility(rows),null,null,x.g()),a=x.open(),p=generic();assert(p);assert.deepEqual(api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile,context:a.context,operation:'list',limit:10,continuation:null},x.c,p,x.g()),no);assert.equal(x.query(a.context,{operation:'list',limit:10,continuation:null}).status,'page');const q=generic(),b=api.acquireNativeDiscovery(x.owner,x.c,q,x.g());assert.equal(b.status,'context');assert.deepEqual(x.query(b.context,{operation:'list',limit:10,continuation:null}),no);assert.equal(x.usage().stopped,0);
 });
 check('discovery-selectors-are-not-settings-contexts-or-targets',()=>{
  const x=make(),a=x.open(),page=x.query(a.context,{operation:'list',limit:10,continuation:null}),entry=page.entries[9].reference,d=x.query(a.context,{operation:'inspect',entry}),z=readerFactory();for(const reference of [a.context,entry,...d.descriptor.variants.map(v=>v.reference)])for(const role of ['context','target']){const e=z.envelope();e.request.arguments[role]=reference;assert.equal(z.query(e).status,'unavailable');nativeAuthorityRefusals++;}assert.equal(z.query().status,'value');
 });
 check('known-loss-absorbs-unknown-withholds-and-retirement-preserves-issuance',()=>{
  for(const value of [false,undefined,1]){const x=make(),a=x.open(),r={operation:'list',limit:10,continuation:null};assert.deepEqual(x.query(a.context,r,x.c,undefined,x.g({discoveryAllowed:value})),no);assert.equal(x.usage().stopped,value===false?1:0);assert.equal(x.query(a.context,r).status,value===false?'unavailable':'page');lossCases++;}
  const x=make(),a=x.open();x.query(a.context,{operation:'list',limit:10,continuation:null});const issued=x.usage().issued;assert.equal(api.stopNativeDiscovery(x.owner,a.context,x.c,x.g()).status,'stopped');for(const field of ['retentionReleased','deliveryDrained'])assert.deepEqual(api.retireNativeDiscovery(x.owner,a.context,x.g({[field]:false})),no);assert.equal(api.retireNativeDiscovery(x.owner,a.context,x.g()).status,'retired');assert.equal(x.usage().issued,issued);
 });
 check('new-profile-capacity-refusal-keeps-older-profile-usable',()=>{
  const x=make({limits:{maxEntries:9}});assert.equal(x.rows(),null);assert.equal(x.packet(),null);const c={...x.c,profile:'owned-contract-discovery/8'};assert.equal(x.rows(c).length,9);assert.equal(x.open(c).status,'context');const y=make({limits:{maxMetadataDepth:16}});assert.equal(y.rows(),null);assert.equal(y.rows({...y.c,profile:'owned-contract-discovery/4'}).length,5);
 });
 check('target-advice-does-not-grant-settings-authority',()=>{
  const x=make(),rows=x.rows(),cut=()=>api.prepareNativeBuiltinDiscoveryCut(x.owner,x.c,x.visibility(rows),x.target(),x.advice(rows),x.g()),p=cut(),a=api.acquireNativeDiscovery(x.owner,x.c,p,x.g());assert.equal(a.status,'context');assert(a.target);const query=(r,g=x.g())=>{const p=cut();assert(p);const out=api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile,context:a.context,...r},x.c,p,g);api.retireNativeDiscoveryCut(x.owner,p,x.g());return out;},page=query({operation:'list',limit:10,continuation:null}),entry=page.entries[9].reference,d=query({operation:'inspect',entry});for(const v of d.descriptor.variants){const r={operation:'advisory-enablement',entry,variant:v.reference,target:a.target};assert.equal(query(r).enabled,true);assert.deepEqual(query(r,x.g({advisoryAdviceAttested:false})),no);assert.deepEqual(query({...r,target:copy(a.target)}),no);}const z=readerFactory(),e=z.envelope();e.request.arguments.target=a.target;assert.equal(z.query(e).status,'unavailable');nativeAuthorityRefusals++;
 });
 return {scope:'owned-native-F07-handler-discovery',controls,schemaDenials,lossCases,descriptors,nativeAuthorityRefusals,olderEightProfilesUnchanged:true,metadataDoesNotGrantSettingsAuthority:true,physicalGlobalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkSettingsDiscovery()));
