import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import * as native from './native-discovery.mjs';
import {advisoryDiscoveryFixture} from './advisory-discovery-checks.mjs';
import {qualifiedSelectionDiscoveryFingerprintsForChecks} from './selection-discovery-checks.mjs';
import {discoverySelfEngineDescriptor} from './discovery-self-descriptor.mjs';
const profile='owned-contract-discovery/13',no={status:'unavailable'},copy=structuredClone;
// /12 reproduced from frozen 0.197; full qualification is still required.
const twelveFingerprint='77c33ea62df337df28cc709c22beb303122c9019cd107fd56ca5f22e1ab73c52';
const canonical=v=>Array.isArray(v)?v.map(canonical):v&&typeof v==='object'?Object.fromEntries(Object.keys(v).sort().map(k=>[k,canonical(v[k])])):v;
const fingerprint=v=>createHash('sha256').update(JSON.stringify(canonical(v))).digest('hex');
export function discoverySelfFixture(options={},api=native){
 const x=advisoryDiscoveryFixture({...options,limits:{maxPageRows:14,...options.limits}},api),c={...x.c,profile};
 return {...x,c,rows:(scope=c)=>x.rows(scope),packet:(scope=c,visibility)=>x.packet(scope,visibility),open:(scope=c,visibility)=>x.open(scope,visibility),query:(context,q,g=x.g(),visibility,route=null,scope=c)=>x.query(context,q,g,visibility,route,scope)};
}
export function independentDiscoverySelfBounds(x){
 const size=v=>Buffer.byteLength(JSON.stringify(v)),stats=(v,d=0)=>{const children=v&&typeof v==='object'?Object.values(v).map(c=>stats(c,d+1)):[];return {nodes:1+children.reduce((n,c)=>n+c.nodes,0),depth:Math.max(d,...children.map(c=>c.depth))};};
 const rows=x.rows(),metadata=rows.flatMap(r=>[...r.variants,r.presentation]),root=x.z.root(),visibility=x.visibility(rows),target=x.target(),advice=x.advice(rows),a=x.open(),page=x.query(a.context,{operation:'list',limit:14,continuation:null}),reply=x.query(a.context,{operation:'inspect',entry:page.entries[13].reference});
 assert.equal(reply.status,'descriptor');
 const projected=rows.map(r=>({key:JSON.stringify([r.command.authority,r.command.id,r.command.incarnation]),schema:true,presentation:true,advice:true,variants:r.variants.map(v=>v.id),value:{version:r.version,kind:r.kind,variants:copy(r.variants),presentation:copy(r.presentation)}}));
 return {maxEntries:14,maxRootBytes:size(root),maxRowBytes:Math.max(...rows.map(size)),maxMetadataBytes:Math.max(...metadata.map(size)),maxMetadataNodes:Math.max(...metadata.map(v=>stats(v).nodes)),maxMetadataDepth:Math.max(...metadata.map(v=>stats(v).depth)),maxPacketBytes:size({root,scope:x.c,registry:rows,visibility,target,advice}),maxCaptureBytes:size({rows:projected,target:{reference:a.target,data:target},builtin:true}),maxReplyBytes:Math.max(size(a),size(page),size(reply))};
}
export function checkDiscoverySelfIntegration(api=native){
 const controls=[],check=(name,fn)=>{fn();controls.push(name);},make=o=>discoverySelfFixture(o,api),list={operation:'list',limit:14,continuation:null};let nativeCases=0,schemaDenials=0,authorityDenials=0,boundaries=0;
 check('twelve-existing-catalogs-preserved-thirteen-appends-only-F01',()=>{
  const x=make(),all=Array.from({length:13},(_,i)=>x.rows({...x.c,profile:'owned-contract-discovery/'+(i+1)}));assert(all.every(Array.isArray));
  assert.deepEqual(all.map(r=>r.length),[2,3,4,5,7,7,8,9,10,11,12,13,14]);assert.deepEqual(all.slice(0,12).map(fingerprint),[...qualifiedSelectionDiscoveryFingerprintsForChecks,twelveFingerprint]);assert.deepEqual(all[12].slice(0,13),all[11]);
  const row=all[12][13];assert.deepEqual(row.variants,discoverySelfEngineDescriptor().variants);assert.equal(row.contract,'contract-discovery/1');assert.equal(row.kind,'query');assert.equal(JSON.parse(row.command.id)[2],'F01');assert.equal(all[12].reduce((n,r)=>n+r.variants.length,0),43);
 });
 check('typed-and-dynamic-list-inspect-and-three-advice-products',()=>{
  for(const route of ['typed','dynamic']){const x=make(),root=x.z.root(),usage=x.z.usage(),a=x.open();assert.equal(a.status,'context');const p=x.query(a.context,list,x.g(),undefined,route);assert.equal(p.status,'page');assert.equal(p.entries.length,14);nativeCases++;
   const entry=p.entries[13].reference,d=x.query(a.context,{operation:'inspect',entry},x.g(),undefined,route);assert.equal(d.status,'descriptor');nativeCases++;assert.deepEqual(d.descriptor.variants.map(({reference,...v})=>v),discoverySelfEngineDescriptor().variants.map(({id,...v})=>v));
   for(const v of d.descriptor.variants){assert.equal(x.query(a.context,{operation:'advisory-enablement',entry,variant:v.reference,target:a.target},x.g(),undefined,route).enabled,true);nativeCases++;}assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),usage);assert.deepEqual(api.nativeDiscoveryDecodeUsage(x.owner),{busy:false,declaredWorkspaceBytes:0,faulted:false});
  }
 });
 check('new-and-inherited-metadata-guards-and-no-backport',()=>{
  const x=make();let invoked=0;
  for(let variant=0;variant<3;variant++)for(const change of [v=>v.input.schema.required=[],v=>v.outcomes.effect='publish',v=>v.outcomes.result.schema.oneOf=[],v=>v.outcomes.notEstablished=[],v=>v.input.schema.execute=()=>{invoked++;}]){const rows=copy(x.rows());change(rows[13].variants[variant]);assert.equal(api.prepareNativeDiscoveryCut(x.owner,x.c,rows,x.visibility(rows),null,null,x.g()),null);schemaDenials++;}
  for(const index of [5,6,7,8,9,10,11,12])for(const mode of ['input','outcomes']){const rows=copy(x.rows()),v=rows[index].variants[0];if(mode==='input')v.input.schema.oneOf=[];else if(index<8)v.outcomes.result.runtimeBindings=[];else v.outcomes.extra=true;assert.equal(api.prepareNativeDiscoveryCut(x.owner,x.c,rows,x.visibility(rows),null,null,x.g()),null);schemaDenials++;}
  const row=x.rows()[13];for(let i=1;i<=12;i++){const c={...x.c,profile:'owned-contract-discovery/'+i},rows=[...x.rows(c),row];assert.equal(api.prepareNativeDiscoveryCut(x.owner,c,rows,x.visibility(rows),null,null,x.g()),null);schemaDenials++;}assert.equal(invoked,0);assert.equal(x.open().status,'context');
 });
 check('captured-schema-ceiling-and-absorbing-current-loss',()=>{
  const x=make({schema:false}),a=x.open(),visibility=x.visibility(x.rows());visibility.forEach(v=>v.schema=true);const p=x.query(a.context,list,x.g(),visibility);assert.equal(p.status,'page');const d=x.query(a.context,{operation:'inspect',entry:p.entries[13].reference},x.g(),visibility);assert.equal(d.status,'descriptor');assert(d.descriptor.variants.every(v=>Object.keys(v).length===1));
  const y=make(),b=y.open(),v=y.visibility(y.rows());v[13].schema=false;assert.deepEqual(y.query(b.context,list,y.g(),v),no);assert.equal(y.usage().stopped,1);assert.deepEqual(y.query(b.context,list),no);
 });
 check('new-profile-scopes-bind-without-changing-older-profiles',()=>{
  const x=make(),a=x.open(),before=x.usage();for(const c of [{...x.c,authority:'foreign'},{...x.c,engine:'foreign'},{...x.c,profile:'owned-contract-discovery/14'}]){assert.equal(x.rows(c),null);assert.equal(x.packet(c),null);assert.deepEqual(x.usage(),before);}
  for(let i=1;i<=12;i++)assert.deepEqual(x.query(a.context,list,x.g(),undefined,null,{...x.c,profile:'owned-contract-discovery/'+i}),no);assert.equal(x.usage().stopped,0);assert.equal(x.query(a.context,list).status,'page');
 });
 check('copied-roles-and-publication-exclusion-preserve-state',()=>{
  const x=make(),a=x.open(),p=x.query(a.context,list),before=x.usage(),root=x.z.root(),parent=x.z.usage();
  for(const [q,g]of [[list,x.g({outsidePublicationGuard:false})],[{...list,extra:true},x.g()],[{operation:'inspect',entry:copy(p.entries[13].reference)},x.g()],[{operation:'execute'},x.g()]]){const cut=x.packet();assert(cut);assert.deepEqual(api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile,context:a.context,...q},x.c,cut,g),no);api.retireNativeDiscoveryCut(x.owner,cut,x.g());authorityDenials++;assert.deepEqual(x.usage(),before);assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),parent);}
 });
 check('positive-advice-is-not-execution-and-needs-original-target',()=>{
  const x=make(),root=x.z.root(),usage=x.z.usage(),a=x.open(),p=x.query(a.context,list),entry=p.entries[13].reference,d=x.query(a.context,{operation:'inspect',entry});
  for(const v of d.descriptor.variants){const q={operation:'advisory-enablement',entry,variant:v.reference,target:a.target};assert.equal(x.query(a.context,q).enabled,true);for(const field of ['advisoryAdviceAttested','advisoryTargetCurrent','advisoryTargetEqualityObservable']){assert.deepEqual(x.query(a.context,q,x.g({[field]:false})),no);authorityDenials++;}assert.deepEqual(x.query(a.context,{...q,target:copy(a.target)}),no);authorityDenials++;}assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),usage);
 });
 const dimensions=['maxEntries','maxRootBytes','maxRowBytes','maxMetadataBytes','maxMetadataNodes','maxMetadataDepth','maxPacketBytes','maxCaptureBytes','maxReplyBytes'];
 check('nine-independent-boundaries-refuse-without-hidden-installation',()=>{
  const expected=independentDiscoverySelfBounds(make());for(const field of dimensions){const attempt=value=>{const x=make({limits:{[field]:value}}),root=x.z.root(),parent=x.z.usage();let success=false;const rows=x.rows();if(rows){if(['maxEntries','maxRootBytes','maxRowBytes','maxMetadataBytes','maxMetadataNodes','maxMetadataDepth'].includes(field))success=true;else{const p=x.packet();if(p){if(field==='maxPacketBytes')success=true;else{const a=api.acquireNativeDiscovery(x.owner,x.c,p,x.g());if(a.status==='context'){if(field==='maxCaptureBytes')success=true;else{const page=x.query(a.context,list);if(page.status==='page')success=x.query(a.context,{operation:'inspect',entry:page.entries[13].reference}).status==='descriptor';}api.stopNativeDiscovery(x.owner,a.context,x.c,x.g());api.retireNativeDiscovery(x.owner,a.context,x.g());}}api.retireNativeDiscoveryCut(x.owner,p,x.g());}}}assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),parent);assert.equal(x.usage().preparations,0);assert.equal(x.usage().contexts,0);return success;};assert.equal(attempt(expected[field]-1),false,field+': minus');assert.equal(attempt(expected[field]),true,field+': exact');assert.equal(attempt(expected[field]+1),true,field+': plus');boundaries+=3;}
 });
 check('new-capacity-pressure-does-not-force-growth-or-disable-twelve',()=>{
  for(const limits of [{maxEntries:13},{maxCaptureBytes:independentDiscoverySelfBounds(make()).maxCaptureBytes-1}]){const x=make({limits});if(x.rows())assert.deepEqual(x.open(),no);else assert.equal(x.packet(),null);assert.equal(x.usage().contexts,0);assert.equal(x.usage().issued,0);assert.equal(x.open({...x.c,profile:'owned-contract-discovery/12'}).status,'context');}
 });
 check('protected-cleanup-and-fixed-origin-remain-native',()=>{
  const x=make(),a=x.open(),rows=x.rows(),p=api.prepareNativeDiscoveryCut(x.owner,x.c,rows,x.visibility(rows),x.target(),x.advice(rows),x.g());assert(p);assert.deepEqual(api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile,context:a.context,...list},x.c,p,x.g()),no);api.retireNativeDiscoveryCut(x.owner,p,x.g());assert.equal(x.query(a.context,list).status,'page');
  const issued=x.usage().issued;assert.equal(api.stopNativeDiscovery(x.owner,a.context,x.c,x.g()).status,'stopped');for(const field of ['retentionReleased','deliveryDrained'])assert.deepEqual(api.retireNativeDiscovery(x.owner,a.context,x.g({[field]:false})),no);assert.equal(api.retireNativeDiscovery(x.owner,a.context,x.g()).status,'retired');assert.equal(x.usage().issued,issued);assert.equal(x.usage().contexts,0);
 });
 return {scope:'owned-existing-F01-self-discovery',controls,nativeCases,schemaDenials,authorityDenials,boundaries,dimensions,olderTwelveCatalogFingerprintsUnchanged:true,twelveFullQualificationRequired:true,metadataDoesNotIssueHandlesOrGrantAuthority:true,newVerbWireOrEvaluatorSelected:false,physicalHostOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(checkDiscoverySelfIntegration()));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'[audit module]'));process.exitCode=1;}
