import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import * as discovery from './native-discovery.mjs';
import * as kernel from './invocation-publication.mjs';
import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';
import {lifecycleEngineDescriptor} from './lifecycle-engine-descriptor.mjs';
import {lifecycleMetadataFixture} from './lifecycle-discovery-product-checks.mjs';
const profile='owned-contract-discovery/10',copy=structuredClone,no={status:'unavailable'};
// Pinned from adopted 0.187, not generated from the candidate.
const goldens=['3107d5c1148abb0743cadddfb89ed1e649275fa1e4209e73efd51c3360a809fb','fd988802398c328878e6da6e0a224781477bcd8f09c38a6df05c9a214a7ca7f0','fede183bbf34068421706596e77b342d20a0b625d276ac2a1e61eb32e0990bce','3ceed9660c483e92747834c85a91370c5d8017b294110bdce2d9503b4fbeeaa5','a40ee43d0b13986d14737cc20ac686a9d864e1956ace2b700003924acefed6d0','ef1e1e7a3d3a6b56a916f297c3d5d13d69c0f8ac4d4d57da4dc0a227092e59aa','b94f6a1b69cc131796f4fd26078cc0a6b64ddfc40c84671856b73837157bbaf4','0b7ad19fc82d8bae95637f07242e1e9c65b9bd58b6ef59b18ff6fe1b4dfeb865','bbed82676adfaeeaa613c177e5c0a61fe5e32e9f6819d4b95914c2d24c6a08bd'];
const canonical=v=>Array.isArray(v)?v.map(canonical):v&&typeof v==='object'?Object.fromEntries(Object.keys(v).sort().map(k=>[k,canonical(v[k])])):v;
const fingerprint=v=>createHash('sha256').update(JSON.stringify(canonical(v))).digest('hex');
export function lifecycleDiscoveryFixture(options={},api=discovery,factory=nativeDiscoveryFixture){
 const x=factory(api,undefined,{...options,limits:{maxPageRows:11,maxMetadataDepth:64,...options.limits}}),c={...x.scope,profile},rows=(s=c)=>api.nativeBuiltinDiscoveryRows(x.owner,s,x.g()),packet=(s=c,v=x.visibility(rows(s)??[]),target=null,advice=null)=>api.prepareNativeBuiltinDiscoveryCut(x.owner,s,v,target,advice,x.g()),open=(s=c,v)=>{const p=packet(s,v);assert(p);const r=api.acquireNativeDiscovery(x.owner,s,p,x.g());api.retireNativeDiscoveryCut(x.owner,p,x.g());return r;},query=(context,r,s=c,v,g=x.g())=>{const p=packet(s,v);assert(p);const out=api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile:s.profile,context,...r},s,p,g);api.retireNativeDiscoveryCut(x.owner,p,x.g());return out;};
 return {...x,c,rows,packet,open,query};
}
export function checkLifecycleDiscovery(api=discovery,factory=nativeDiscoveryFixture,lifecycleFactory=lifecycleMetadataFixture){
 const controls=[],check=(name,f)=>{f();controls.push(name);},make=options=>lifecycleDiscoveryFixture(options,api,factory),list={operation:'list',limit:11,continuation:null};let schemaDenials=0,authorityDenials=0,lossCases=0;
 check('nine-profile-goldens-preserved-ten-only-appends-eight-F17-variants',()=>{
  const x=make(),all=Array.from({length:10},(_,i)=>x.rows({...x.c,profile:'owned-contract-discovery/'+(i+1)}));assert(all.every(Array.isArray));assert.deepEqual(all.map(r=>r.length),[2,3,4,5,7,7,8,9,10,11]);assert.deepEqual(all.slice(0,9).map(fingerprint),goldens);assert.deepEqual(all[9].slice(0,10),all[8]);const r=all[9][10],d=lifecycleEngineDescriptor();assert.equal(r.contract,d.contract);assert.equal(r.kind,'localCommand');assert.deepEqual(r.variants,d.variants);assert.equal(r.command.id,JSON.stringify([x.c.engine,'builtin','F17']));assert.equal(r.command.authority,x.c.authority);assert.equal(r.command.incarnation,1);
 });
 check('actual-owned-inspection-preserves-mutation-root-and-parent-ledger',()=>{
  const x=make(),root=x.z.root(),parent=x.z.usage(),a=x.open(),page=x.query(a.context,list);assert.equal(page.status,'page');assert.equal(page.entries.length,11);const d=x.query(a.context,{operation:'inspect',entry:page.entries[10].reference});assert.equal(d.status,'descriptor');assert.deepEqual(d.descriptor.variants.map(({reference,...v})=>v),lifecycleEngineDescriptor().variants.map(({id,...v})=>v));assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),parent);
 });
 check('captured-schema-ceiling-and-current-withdrawal-remain-distinct',()=>{
  const x=make({schema:false}),a=x.open(),v=x.visibility(x.rows());v.forEach(r=>r.schema=true);const page=x.query(a.context,list,x.c,v);assert.equal(page.status,'page');const d=x.query(a.context,{operation:'inspect',entry:page.entries[10].reference},x.c,v);assert.equal(d.status,'descriptor');assert(d.descriptor.variants.every(r=>Object.keys(r).length===1&&Object.hasOwn(r,'reference')));
  const y=make(),b=y.open(),w=y.visibility(y.rows());w[10].schema=false;assert.deepEqual(y.query(b.context,list,y.c,w),no);assert.equal(y.usage().stopped,1);assert.deepEqual(y.query(b.context,list),no);lossCases++;
 });
 check('foreign-scopes-unknown-twelve-and-cross-profile-cuts-refuse',()=>{
  const x=make(),a=x.open(),before=x.usage();for(const c of [{...x.c,authority:'foreign'},{...x.c,engine:'foreign'},{...x.c,profile:'owned-contract-discovery/14'}]){assert.equal(x.rows(c),null);assert.equal(x.packet(c),null);assert.deepEqual(x.usage(),before);}for(let i=1;i<10;i++)assert.deepEqual(x.query(a.context,list,{...x.c,profile:'owned-contract-discovery/'+i}),no);assert.equal(x.usage().stopped,0);assert.equal(x.query(a.context,list).status,'page');
 });
 check('lifecycle-schemas-cannot-be-smuggled-into-older-profiles',()=>{
  const x=make(),row=x.rows()[10];for(let i=1;i<10;i++){const c={...x.c,profile:'owned-contract-discovery/'+i},rows=[...x.rows(c),row];assert.equal(api.prepareNativeDiscoveryCut(x.owner,c,rows,x.visibility(rows),null,null,x.g()),null);schemaDenials++;}
 });
 check('exact-lifecycle-schema-outcomes-and-inherited-recognizers-refuse-corruption',()=>{
  const x=make();let calls=0;
  for(let index=0;index<8;index++)for(const change of [v=>v.input.schema.oneOf=[],v=>v.input.schema.$ref='https://not-loaded.invalid/schema',v=>v.input.schema.execute=()=>{calls++;},v=>v.outcomes.result.schema.oneOf.pop(),v=>v.outcomes.result.runtimeBindings=[],v=>v.outcomes.result.notEstablished=[],v=>v.outcomes.events='complete-public-event-stream',v=>v.outcomes.result.scope='new-public-query']){const rows=copy(x.rows());change(rows[10].variants[index]);assert.equal(api.prepareNativeDiscoveryCut(x.owner,x.c,rows,x.visibility(rows),null,null,x.g()),null);schemaDenials++;}
  for(const index of [5,6,7,8,9])for(const change of [v=>v.input.schema.oneOf=[],v=>v.outcomes.result.runtimeBindings=[],v=>v.outcomes.result.schema.oneOf.pop()]){const rows=copy(x.rows());change(rows[index].variants[0]);assert.equal(api.prepareNativeDiscoveryCut(x.owner,x.c,rows,x.visibility(rows),null,null,x.g()),null);schemaDenials++;}assert.equal(calls,0);assert.equal(x.open().status,'context');
 });
 check('fixed-and-equal-generic-origin-contexts-cannot-substitute',()=>{
  const x=make(),rows=x.rows(),generic=()=>api.prepareNativeDiscoveryCut(x.owner,x.c,rows,x.visibility(rows),null,null,x.g()),a=x.open(),p=generic();assert(p);assert.deepEqual(api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile,context:a.context,...list},x.c,p,x.g()),no);api.retireNativeDiscoveryCut(x.owner,p,x.g());const q=generic(),b=api.acquireNativeDiscovery(x.owner,x.c,q,x.g());assert.equal(b.status,'context');api.retireNativeDiscoveryCut(x.owner,q,x.g());assert.deepEqual(x.query(b.context,list),no);assert.equal(x.query(a.context,list).status,'page');assert.equal(x.usage().stopped,0);
 });
 check('discovery-context-entry-and-eight-variant-selectors-are-not-native-capabilities',()=>{
  const x=make(),a=x.open(),page=x.query(a.context,list),entry=page.entries[10].reference,d=x.query(a.context,{operation:'inspect',entry}),z=lifecycleFactory(),before=z.x.root(),u=z.x.usage();
  for(const ref of [a.context,entry,...d.descriptor.variants.map(v=>v.reference)]){
   const e=z.x.envelope();assert.equal(z.x.prepare(e,{authority:ref}),null);assert.equal(z.x.prepare(e,z.x.ports,z.x.controlContext,z.x.facts(),undefined,z.x.domain,ref),null);assert.deepEqual(kernel.observeLifecycleResult(z.x.domain,ref,z.x.controlContext,z.x.controlPolicy,z.x.facts()),no);authorityDenials+=3;
  }assert.equal(z.x.root(),before);assert.deepEqual(z.x.usage(),u);assert(z.x.prepare());
 });
 check('publication-exclusion-and-malformed-query-preserve-all-owner-state',()=>{
  const x=make(),a=x.open(),page=x.query(a.context,list),before=x.usage(),root=x.z.root(),parent=x.z.usage();for(const [r,g]of [[list,x.g({outsidePublicationGuard:false})],[{...list,extra:true},x.g()],[{operation:'inspect',entry:copy(page.entries[10].reference)},x.g()],[{operation:'execute'},x.g()]]){assert.deepEqual(x.query(a.context,r,x.c,undefined,g),no);assert.deepEqual(x.usage(),before);assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),parent);}
 });
 check('known-loss-stops-unknown-withholds-and-protected-retirement-keeps-spent-issuance',()=>{
  for(const value of [false,undefined,1]){const x=make(),a=x.open();assert.deepEqual(x.query(a.context,list,x.c,undefined,x.g({discoveryAllowed:value})),no);assert.equal(x.usage().stopped,value===false?1:0);assert.equal(x.query(a.context,list).status,value===false?'unavailable':'page');lossCases++;}
  const x=make(),a=x.open();x.query(a.context,list);const issued=x.usage().issued;assert.equal(api.stopNativeDiscovery(x.owner,a.context,x.c,x.g()).status,'stopped');for(const k of ['retentionReleased','deliveryDrained'])assert.deepEqual(api.retireNativeDiscovery(x.owner,a.context,x.g({[k]:false})),no);assert.equal(api.retireNativeDiscovery(x.owner,a.context,x.g()).status,'retired');assert.equal(x.usage().issued,issued);
 });
 check('new-profile-capacity-does-not-disable-previous-profile',()=>{
  for(const limits of [{maxEntries:10},{maxVariants:7}]){const x=make({limits});assert.equal(x.rows(),null);assert.equal(x.packet(),null);const c={...x.c,profile:'owned-contract-discovery/9'};assert.equal(x.rows(c).length,10);assert.equal(x.open(c).status,'context');}
 });
 check('positive-target-advice-is-not-native-authority-or-execution',()=>{
  const x=make(),rows=x.rows(),cut=()=>api.prepareNativeBuiltinDiscoveryCut(x.owner,x.c,x.visibility(rows),x.target(),x.advice(rows),x.g()),p=cut(),a=api.acquireNativeDiscovery(x.owner,x.c,p,x.g());assert.equal(a.status,'context');api.retireNativeDiscoveryCut(x.owner,p,x.g());const query=(r,g=x.g())=>{const p=cut();assert(p);const out=api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile,context:a.context,...r},x.c,p,g);api.retireNativeDiscoveryCut(x.owner,p,x.g());return out;},page=query(list),entry=page.entries[10].reference,d=query({operation:'inspect',entry});
  for(const v of d.descriptor.variants){const r={operation:'advisory-enablement',entry,variant:v.reference,target:a.target};assert.equal(query(r).enabled,true);assert.deepEqual(query(r,x.g({advisoryAdviceAttested:false})),no);assert.deepEqual(query({...r,target:copy(a.target)}),no);}const z=lifecycleFactory();assert.equal(z.x.prepare(z.x.envelope(),{authority:a.target}),null);assert(z.x.prepare());authorityDenials++;
 });
 return {scope:'owned-native-F17-handler-discovery',controls,schemaDenials,authorityDenials,lossCases,olderNineProfilesUnchanged:true,metadataDoesNotGrantLifecycleAuthority:true,newPublicQueryOrEventSurfaceSelected:false,hostPhysicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkLifecycleDiscovery()));
