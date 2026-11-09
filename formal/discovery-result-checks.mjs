import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as api from './discovery-results.mjs';
import * as native from './native-discovery.mjs';
import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';
import {discoveryArgumentFixture} from './discovery-argument-checks.mjs';
import {discoveryProducerRequest} from './discovery-arguments.mjs';
const copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v)),nodes=v=>1+(v&&typeof v==='object'?Object.values(v).reduce((n,x)=>n+nodes(x),0):0),depth=v=>v&&typeof v==='object'&&Object.keys(v).length?1+Math.max(...Object.values(v).map(depth)):0;
const clone=r=>r.status==='page'?{...r,entries:r.entries.map(e=>({...e}))}:r.status==='descriptor'?{...r,descriptor:{...r.descriptor,...(Object.hasOwn(r.descriptor,'presentation')?{presentation:copy(r.descriptor.presentation)}:{}),variants:r.descriptor.variants.map(({reference,...f})=>({reference,...copy(f)}))}}:{...r};
export function discoveryResultFixture(profile=10,options={}){
 const x=nativeDiscoveryFixture(native,undefined,{...options,limits:{maxPageRows:11,maxMetadataDepth:64,...options.limits}}),c={...x.scope,profile:'owned-contract-discovery/'+profile},rows=native.nativeBuiltinDiscoveryRows(x.owner,c,x.g());assert(Array.isArray(rows));
 const cut=()=>native.prepareNativeBuiltinDiscoveryCut(x.owner,c,x.visibility(rows),options.noTarget?null:x.target(),options.noTarget?null:x.advice(rows),x.g()),p=cut();assert(p);const a=native.acquireNativeDiscovery(x.owner,c,p,x.g());assert.equal(a.status,'context');native.retireNativeDiscoveryCut(x.owner,p,x.g());
 const base={contract:'contract-discovery/1',profile:c.profile,context:a.context},query=(request,g=x.g())=>{const p=cut();assert(p);const r=native.queryNativeDiscovery(x.owner,request,c,p,g);native.retireNativeDiscoveryCut(x.owner,p,x.g());return r;},request={...base,operation:'list',limit:11,continuation:null},page=query(request);assert.equal(page.status,'page');
 return {x,c,rows,a,base,query,request,page};
}
export function checkDiscoveryResults(guard=api){
 const controls=[],check=(name,f)=>{f();controls.push(name);};let nativeCases=0,denials=0,boundaries=0,correlations=0,shapeNonAuthority=0,metadataCases=0;
 const valid=(r,q,l)=>guard.nativeDiscoveryResultValid(r,q,l),deny=(r,q,l)=>{assert.equal(valid(r,q,l),false);denials++;};
 check('four-existing-shapes-fixed-fields-and-inert-metadata-declarations',()=>{
  const d=guard.nativeDiscoveryResultDeclarations();assert.deepEqual(Object.keys(d.forms),['page','descriptor','advice','unavailable']);assert.deepEqual(d.forms.page,['status','context','coverage','entries','continuation']);assert.deepEqual(d.forms.descriptor,['status','context','descriptor']);assert.deepEqual(d.forms.advice,['status','context','target','entry','variant','enabled']);assert.deepEqual(d.forms.unavailable,['status']);assert.deepEqual(d.facets,['input','outcomes','requirements','limits','cancellation']);assert.equal(d.coverage,'captured-permitted');assert(Object.isFrozen(d)&&Object.isFrozen(d.forms.page)&&Object.isFrozen(d.facets));
 });
 check('ten-profiles-all-current-rows-and-advice-have-closed-native-values',()=>{
  for(let profile=1;profile<=10;profile++){
   const z=discoveryResultFixture(profile),root=z.x.z.root(),parent=z.x.z.usage();assert(valid(z.page,z.request,z.x.l));nativeCases++;
   for(const e of z.page.entries){
    const q={...z.base,operation:'inspect',entry:e.reference},r=z.query(q);assert.equal(r.status,'descriptor');assert(valid(r,q,z.x.l));assert(valid(clone(r),q,z.x.l));assert(valid(Object.assign(Object.create(null),clone(r)),q,z.x.l));nativeCases++;
    const advice={...z.base,operation:'advisory-enablement',entry:e.reference,variant:r.descriptor.variants[0].reference,target:z.a.target},out=z.query(advice);assert.equal(out.status,'advice');assert(valid(out,advice,z.x.l));nativeCases++;
   }assert.equal(z.x.z.root(),root);assert.deepEqual(z.x.z.usage(),parent);
  }
 });
 check('both-normalized-routes-preserve-native-result-request-roles',()=>{
  for(let profile=1;profile<=10;profile++)for(const operation of ['list','inspect','advisory-enablement']){const z=discoveryArgumentFixture(operation,profile);for(const route of ['typed','dynamic']){const n=z.n(z.envelope(route));assert(n);const q=discoveryProducerRequest(n),r=z.query(q);assert(valid(r,q,z.x.l));nativeCases++;}}
 });
 check('empty-permitted-rows-hidden-facets-and-variants-are-not-absence-or-authority',()=>{
  for(const options of [{visible:()=>false},{schema:false,presentation:false},{variantVisible:()=>false},{noTarget:true}]){
   const z=discoveryResultFixture(10,options);assert(valid(z.page,z.request,z.x.l));nativeCases++;
   for(const e of z.page.entries){const q={...z.base,operation:'inspect',entry:e.reference},r=z.query(q);assert.equal(r.status,'descriptor');assert(valid(r,q,z.x.l));nativeCases++;}
   if(options.noTarget){const q={...z.base,operation:'advisory-enablement',entry:z.page.entries[0].reference,variant:{},target:null},r=z.query(q);assert.deepEqual(r,{status:'unavailable'});assert(valid(r,q,z.x.l));nativeCases++;}
  }
 });
 check('closed-outer-and-nested-fields-deny-surprises-and-cross-operation-results',()=>{
  for(const operation of ['list','inspect','advisory-enablement']){const z=discoveryArgumentFixture(operation),q=z.r,r=z.query(q),l=z.x.l;assert(valid(r,q,l));assert(valid({status:'unavailable'},q,l));
   for(const field of Object.keys(r)){const bad=clone(r);delete bad[field];deny(bad,q,l);}for(const bad of [{...clone(r),extra:true},{...clone(r),status:'surprise'},{status:'unavailable',result:r},{status:'unavailable',reason:'hidden'},null,[]])deny(bad,q,l);
   for(const other of ['list','inspect','advisory-enablement'].filter(x=>x!==operation)){
    const base={contract:q.contract,profile:q.profile,context:q.context},d=z.query({...base,operation:'inspect',entry:z.entry}),request={...base,operation:other,...(other==='list'?{limit:1,continuation:null}:other==='inspect'?{entry:z.entry}:{entry:z.entry,variant:d.descriptor.variants[0].reference,target:z.a.target})};deny(r,request,l);
   }
   if(operation==='list'){
    for(const field of Object.keys(r.entries[0])){const bad=clone(r);delete bad.entries[0][field];deny(bad,q,l);}
    for(const change of [v=>v.coverage='complete',v=>v.entries[0].extra=true,v=>v.entries[0].version=0,v=>v.entries[0].version='1',v=>v.entries[0].version=1.5,v=>v.entries[0].version=Number.MAX_SAFE_INTEGER+1,v=>v.entries[0].kind='execute',v=>v.entries[0].reference=null,v=>v.entries[0].reference=[],v=>v.continuation=1,v=>v.continuation=[],v=>v.entries.push({...v.entries[0]})]){const bad=clone(r);change(bad);deny(bad,q,l);}
    const all=discoveryResultFixture(1),duplicate=clone(all.page);assert.equal(duplicate.entries.length,2);duplicate.entries[1].reference=duplicate.entries[0].reference;deny(duplicate,all.request,all.x.l);deny(all.page,{...all.request,limit:1},all.x.l);
   }
   if(operation==='inspect'){
    for(const field of Object.keys(r.descriptor)){const bad=clone(r);delete bad.descriptor[field];if(field==='presentation')assert(valid(bad,q,l));else deny(bad,q,l);}
    for(const change of [v=>v.descriptor.extra=true,v=>v.descriptor.version=0,v=>v.descriptor.kind='execute',v=>v.descriptor.variants[0].extra=true,v=>v.descriptor.variants[0].reference=null,v=>v.descriptor.presentation.loader='fetch',v=>v.descriptor.variants.push({...v.descriptor.variants[0]})]){const bad=clone(r);change(bad);deny(bad,q,l);}
    for(const field of ['input','outcomes','requirements','limits','cancellation']){const bad=clone(r);delete bad.descriptor.variants[0][field];deny(bad,q,l);}
    const hidden=clone(r);hidden.descriptor.variants=hidden.descriptor.variants.map(v=>({reference:v.reference}));assert(valid(hidden,q,l));hidden.descriptor.variants[0].outcomes={};deny(hidden,q,l);
   }
   if(operation==='advisory-enablement'){deny({...r,enabled:1},q,l);assert(valid({...r,enabled:false},q,l));}
  }
 });
 check('request-correlations-require-original-context-entry-variant-and-target',()=>{
  for(const operation of ['list','inspect','advisory-enablement']){
   const z=discoveryArgumentFixture(operation),q=z.r,r=z.query(q),l=z.x.l;for(const field of ['context',...(operation==='inspect'?['entry']:operation==='advisory-enablement'?['entry','variant','target']:[])]){const shadow={...q,[field]:copy(q[field])};deny(r,shadow,l);assert.equal(z.query(shadow).status,'unavailable');correlations++;}
   deny(r,{...q,extra:true},l);deny(r,{...q,operation:'execute'},l);deny(r,{...q,profile:'\ud800'},l);
  }
 });
 check('opaque-shape-is-not-native-identity-and-no-reference-content-is-traversed',()=>{
  const z=discoveryArgumentFixture('advisory-enablement'),r=z.query(z.r),l=z.x.l;let calls=0;const opaque={};Object.defineProperty(opaque,'kind',{enumerable:true,get(){calls++;throw Error('opaque reference getter');}});
  const q={...z.r,context:opaque,entry:opaque,variant:opaque,target:opaque},v={...r,context:opaque,entry:opaque,variant:opaque,target:opaque};assert(valid(v,q,l));assert.equal(calls,0);assert.equal(z.query(q).status,'unavailable');shapeNonAuthority++;
  const p=discoveryArgumentFixture(),page=p.query(p.r),fake=clone(page);fake.entries[0].reference=opaque;assert(valid(fake,p.r,p.x.l));assert.equal(calls,0);shapeNonAuthority++;
 });
 check('metadata-json-costs-utf8-density-and-depth-are-bounded-without-loaders',()=>{
  const examples=[null,true,false,0,-0,1.25,'English Deutsch 漢字 🙂','a\n\t"\\\u0000',[],{},[1,{nested:['text',null]}],{schema:{$ref:'https://not-loaded.invalid/schema'},source:'literal prose'}];
  for(const value of examples){const l={maxMetadataBytes:size(value),maxMetadataNodes:nodes(value),maxMetadataDepth:depth(value)};assert(guard.discoveryResultMetadataValid(value,l));metadataCases++;for(const field of ['maxMetadataBytes','maxMetadataNodes','maxMetadataDepth'])if(l[field]>0){assert.equal(guard.discoveryResultMetadataValid(value,{...l,[field]:l[field]-1}),false);metadataCases++;}}
  const l={maxMetadataBytes:65536,maxMetadataNodes:4096,maxMetadataDepth:128},cycle={};cycle.self=cycle;const sparse=[];sparse.length=1;
  const symbol={x:1,[Symbol('extra')]:true},hidden={x:1};Object.defineProperty(hidden,'extra',{value:1,enumerable:false});const extraArray=[1];extraArray.extra=true;
  for(const value of [undefined,()=>{},NaN,Infinity,'\ud800',{['\ud800']:1},cycle,sparse,new Date(),{x:1n},symbol,hidden,extraArray]){assert.equal(guard.discoveryResultMetadataValid(value,l),false);metadataCases++;}
  let calls=0;const getter={};Object.defineProperty(getter,'x',{enumerable:true,get(){calls++;throw Error('metadata getter');}});assert.equal(guard.discoveryResultMetadataValid(getter,l),false);assert.equal(calls,0);metadataCases++;
 });
 check('result-data-getters-symbols-and-hidden-fields-refuse-without-side-effects',()=>{
  const z=discoveryArgumentFixture('inspect'),r=z.query(z.r),root=z.x.z.root(),usage=z.x.usage(),parent=z.x.z.usage();let calls=0;
  for(const target of ['outer','descriptor','variant','request']){const bad=clone(r),q={...z.r},value={outer:bad,descriptor:bad.descriptor,variant:bad.descriptor.variants[0],request:q}[target],key={outer:'status',descriptor:'version',variant:'input',request:'profile'}[target];Object.defineProperty(value,key,{enumerable:true,get(){calls++;throw Error('result getter');}});deny(bad,q,z.x.l);}
  for(const modify of [v=>v[Symbol('extra')]=1,v=>Object.defineProperty(v,'status',{value:'descriptor',enumerable:false}),v=>Object.setPrototypeOf(v,{inherited:true}),v=>v.descriptor.variants[0].outcomes=()=>{}]){const bad=clone(r);modify(bad);deny(bad,z.r,z.x.l);}assert.equal(calls,0);assert.equal(z.x.z.root(),root);assert.deepEqual(z.x.usage(),usage);assert.deepEqual(z.x.z.usage(),parent);
 });
 check('page-variant-and-metadata-limits-have-independent-exact-boundaries',()=>{
  const z=discoveryArgumentFixture('inspect',10),r=z.query(z.r),facets=r.descriptor.variants.flatMap(v=>['input','outcomes','requirements','limits','cancellation'].map(k=>v[k]));if(r.descriptor.presentation)facets.push(r.descriptor.presentation);
  const bounds={maxVariants:r.descriptor.variants.length,maxMetadataBytes:Math.max(...facets.map(size)),maxMetadataNodes:Math.max(...facets.map(nodes)),maxMetadataDepth:Math.max(...facets.map(depth))};
  for(const [field,min]of Object.entries(bounds))for(const delta of [-1,0,1]){assert.equal(valid(r,z.r,{...z.x.l,[field]:min+delta}),delta>=0,field);boundaries++;}
  const p=discoveryArgumentFixture(),page=p.query(p.r);for(const delta of [-1,0,1]){assert.equal(valid(page,p.r,{...p.x.l,maxPageRows:page.entries.length+delta}),delta>=0);boundaries++;}
 });
 return {scope:'closed-existing-F01-native-result-products',controls,nativeCases,denials,boundaries,correlations,shapeNonAuthority,metadataCases,shapes:4,noNewNativeResultOrQueryOrWire:true,nativeSelectorInstallationOrFacadeFaultIntegrationProved:false,physicalGlobalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkDiscoveryResults()));
