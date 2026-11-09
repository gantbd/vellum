// Actual native settings read composition, not real policy or a hostile wire decoder.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {parentFundedPoolAttachmentBytes} from './owner-attachment-test-helpers.mjs';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {engineSettingsFixture,engineSettingsProfile,engineSettingsDefinition} from './engine-settings-checks.mjs';
const copy=structuredClone,id=n=>({authority:'a',id:n,incarnation:1}),key=r=>JSON.stringify([r.authority,r.id,r.incarnation]);
const flags=['authorized','scope','continuous','live','notStopped','ownerControlAllowed','outsidePublicationGuard','resourcesReady','preparationReserved','visibilityAttested','visibilityComplete','targetReadable','retained','transferReady','replyReady','retentionReleased','deliveryDrained'];
const good=Object.fromEntries(flags.map(k=>[k,true]));
const profile={engineProfile:engineSettingsProfile,maxPackets:4,maxContexts:4,maxOrigins:12,maxIssued:128,maxPacketBytes:65536,maxCaptureBytes:32768,maxReplyBytes:32768,maxRequestBytes:4096,maxContextBytes:2048,maxSelectorBytes:128,maxInputNodes:32768,maxInputDepth:256,maxHeldBytes:4194304};
function fixture(api=kernel,options={}){
 const z=engineSettingsFixture(api,options),p=copy({...profile,engineProfile:z.p,...options.readProfile}),c={authority:'a',engine:'settings-engine',recipient:'reader',purpose:'inspect',destination:'local',policy:'p1',continuity:'c1',profile:'native-engine-settings-read/1'};
 const pool=api.openInvocationSettingsReads(z.domain,p,good),cfg=z.root().engineSettings,actual=cfg.resolved[key(id('TA'))];
 const facts={target:id('TA'),capabilities:true,keys:cfg.schemas.map(s=>({key:s.key,visible:true,value:true,origins:true,declaration:true,originIds:Array.isArray(actual[s.key].origins)?actual[s.key].origins:[...new Set(Object.values(actual[s.key].origins).flat())]}))};
 const cut=(f=facts,scope=c,g=good,basis=z.root().engineSettings)=>api.prepareInvocationSettingsRead(z.domain,pool,scope,f,basis,g);
 const acquire=(f=facts)=>api.acquireInvocationSettingsRead(z.domain,pool,c,cut(f),good);
 const envelope=(ctx,operation='effective-configuration',name='scalar',route='typed')=>({route,request:{...(route==='typed'?{family:6}:{name:'inspect-context'}),variant:operation,version:1,arguments:{contract:'configuration-query/1',operation,context:ctx.context,target:ctx.target,profile:c.profile,...(operation==='capabilities'?{}:{key:name})}}});
 const query=(ctx,operation='effective-configuration',name='scalar',route='typed',f=facts,scope=c,g=good)=>api.queryInvocationSettingsRead(z.domain,pool,envelope(ctx,operation,name,route),scope,cut(f,scope),g);
 return {...z,p,c,pool,facts,cut,acquire,envelope,query,settingsEnvelope:z.envelope,usage:()=>api.invocationSettingsReadUsage(z.domain),parentUsage:()=>api.invocationUsage(z.domain)};
}
export {fixture as engineSettingsReadFixture,good as engineSettingsReadGates};
export function checkEngineSettingsReads(api=kernel){
 const controls=[],control=(name,f)=>{f();controls.push(name);};let products=0,gateCases=0,contextCases=0,budgetCases=0,gapCases=0,transientCases=0;
 control('all-three-typed-dynamic-products-use-owned-state-without-jobs',()=>{
  const z=fixture(api),ctx=z.acquire();assert.equal(ctx.status,'context');const root=z.root(),before=z.parentUsage();
  for(const route of ['typed','dynamic'])for(const op of ['effective-configuration','permitted-origin','capabilities']){const r=z.query(ctx,op,'scalar',route);assert.equal(r.status,{ 'effective-configuration':'value','permitted-origin':'origins',capabilities:'capabilities'}[op]);if(r.status==='value')assert.equal(r.value,1);if(r.status==='origins'){assert.equal(r.coverage,'permitted-only');assert.equal(r.origins.length,1);assert.equal(r.origins[0].kind,'engine-settings-origin/1');}if(r.status==='capabilities'){assert.equal(r.capabilities.length,3);assert(!Object.hasOwn(r.capabilities[0],'default'));assert.equal(r.capabilities[0].sensitivity,true);}products++;}
  assert.equal(z.root(),root);assert.deepEqual(z.parentUsage(),before);
 });
 control('context-target-cut-pool-and-owner-lookalikes-carry-no-authority',()=>{
  const a=fixture(api),b=fixture(api),ctx=a.acquire(),e=a.envelope(ctx);
  for(const wrong of [{...e,request:{...e.request,arguments:{...e.request.arguments,context:copy(ctx.context)}}},{...e,request:{...e.request,arguments:{...e.request.arguments,target:copy(ctx.target)}}}])assert.equal(api.queryInvocationSettingsRead(a.domain,a.pool,wrong,a.c,a.cut(),good).status,'unavailable');
  assert.equal(api.queryInvocationSettingsRead(b.domain,b.pool,e,b.c,b.cut(),good).status,'unavailable');
  assert.equal(api.acquireInvocationSettingsRead(a.domain,copy(a.pool),a.c,a.cut(),good).status,'unavailable');assert.equal(api.acquireInvocationSettingsRead(a.domain,a.pool,a.c,copy(a.cut()),good).status,'unavailable');
 });
 control('complete-current-scope-and-strict-grants-are-required',()=>{
  for(const gate of flags.filter(k=>!['retentionReleased','deliveryDrained'].includes(k))){const z=fixture(api),ctx=z.acquire(),g={...good,[gate]:false};const preparation=['ownerControlAllowed','preparationReserved','visibilityAttested','visibilityComplete','targetReadable'].includes(gate);
   if(preparation)assert.equal(z.cut(z.facts,z.c,g),null);else assert.equal(z.query(ctx,'effective-configuration','scalar','typed',z.facts,z.c,g).status,'unavailable');gateCases++;
  }
  for(const k of Object.keys(fixture(api).c)){const z=fixture(api),ctx=z.acquire(),c={...z.c,[k]:'other'};assert.equal(z.query(ctx,'effective-configuration','scalar','typed',z.facts,c).status,'unavailable');contextCases++;}
  const z=fixture(api),ctx=z.acquire();assert.equal(z.query(ctx,'effective-configuration','scalar','typed',z.facts,z.c,{...good,authorized:1}).status,'unavailable');
 });
 control('current-cuts-cannot-be-replaced-by-root-copies-or-reused',()=>{
  const z=fixture(api);assert.equal(z.cut(z.facts,z.c,good,copy(z.root().engineSettings)),null);const cut=z.cut(),ctx=api.acquireInvocationSettingsRead(z.domain,z.pool,z.c,cut,good);assert.equal(ctx.status,'context');assert.equal(api.acquireInvocationSettingsRead(z.domain,z.pool,z.c,cut,good).status,'unavailable');
 });
 control('widening-never-enlarges-a-capture-and-narrowing-cannot-release-it',()=>{
  const z=fixture(api),small=copy(z.facts);small.keys[0].value=false;small.keys[0].origins=false;small.keys[0].originIds=[];small.keys[0].declaration=false;const ctx=z.acquire(small);
  assert.equal(z.query(ctx).status,'unavailable');const caps=z.query(ctx,'capabilities');assert.equal(caps.status,'capabilities');assert.equal(caps.capabilities.length,2);
  const full=z.acquire(),narrow=copy(z.facts);narrow.keys[0].value=false;assert.equal(z.query(full,'effective-configuration','scalar','typed',narrow).status,'unavailable');
 });
 control('sensitivity-metadata-never-grants-value-or-declaration-access',()=>{
  const d=engineSettingsDefinition();d.schemas[0].sensitivity=false;const z=fixture(api,{definition:d}),f=copy(z.facts);f.keys[0]={key:'scalar',visible:false,value:false,origins:false,declaration:false,originIds:[]};const ctx=z.acquire(f);assert.equal(z.query(ctx,'effective-configuration','scalar','typed',f).status,'unavailable');const caps=z.query(ctx,'capabilities','scalar','typed',f);assert.equal(caps.status,'capabilities');assert.equal(caps.capabilities.length,2);
 });
 control('origin-references-are-context-local-and-not-raw-private-identities',()=>{
  const z=fixture(api),a=z.acquire(),b=z.acquire(),x=z.query(a,'permitted-origin'),y=z.query(b,'permitted-origin');assert.notEqual(x.origins[0],y.origins[0]);assert.equal(z.query(a,'permitted-origin').origins[0],x.origins[0]);assert(!JSON.stringify(x).includes('library'));assert(!JSON.stringify(x).includes('TA'));
 });
 control('empty-permitted-origins-do-not-claim-complete-absence',()=>{
  const z=fixture(api),f=copy(z.facts);f.keys.forEach(r=>r.originIds=[]);const ctx=z.acquire(f),r=z.query(ctx,'permitted-origin','scalar','typed',f);assert.deepEqual(r.origins,[]);assert.equal(r.coverage,'permitted-only');
 });
 control('actual-settings-activation-stales-visible-values-without-refresh',()=>{
  const z=fixture(api),f=copy(z.facts);f.keys.forEach(r=>{r.origins=false;r.originIds=[];});const ctx=z.acquire(f),old=z.cut(f);assert.equal(z.submit().status,'accepted');assert.equal(z.run().status,'settled');assert.equal(api.queryInvocationSettingsRead(z.domain,z.pool,z.envelope(ctx),z.c,old,good).status,'unavailable');assert.equal(z.query(ctx,'effective-configuration','scalar','typed',f).status,'unavailable');assert.equal(api.discardInvocationSettingsReadCut(z.domain,z.pool,old,good),true);
  const fresh=z.acquire(f);assert.equal(z.query(fresh,'effective-configuration','scalar','typed',f).value,2);
 });
 control('hidden-world-differences-do-not-enter-permitted-products',()=>{
  const results=[];for(const text of ['secret one','secret two']){const d=engineSettingsDefinition();d.schemas[1].default=[text];const z=fixture(api,{definition:d}),f=copy(z.facts);f.keys[1]={key:'list',visible:false,value:false,origins:false,declaration:false,originIds:[]};const ctx=z.acquire(f);results.push([z.query(ctx,'effective-configuration','scalar','typed',f),z.query(ctx,'permitted-origin','scalar','typed',f),z.query(ctx,'capabilities','scalar','typed',f)]);}
  assert.deepEqual(JSON.parse(JSON.stringify(results[0])),JSON.parse(JSON.stringify(results[1])));
 });
 control('unrelated-target-activation-keeps-a-current-permitted-read',()=>{
  const z=fixture(api,{request:{changes:[{layer:id('w2'),expectedGeneration:1,key:'scalar',operation:'set',value:3}]}}),ctx=z.acquire();assert.equal(z.submit().status,'accepted');assert.equal(z.run().status,'settled');assert.equal(z.root().engineSettings.resolved[key(id('TB'))].scalar.value,3);assert.equal(z.query(ctx).value,1);
 });
 control('reply-values-and-projected-origin-containers-are-not-owned-state-aliases',()=>{
  const z=fixture(api),ctx=z.acquire(),list=z.query(ctx,'effective-configuration','list');assert.equal(list.status,'value');assert.throws(()=>list.value.push('not an edit'),TypeError);
  const origins=z.query(ctx,'permitted-origin','map');assert.equal(origins.status,'origins');origins.origins.left.length=0;const again=z.query(ctx,'permitted-origin','map');assert.equal(again.status,'origins');assert.equal(again.origins.left.length,1);assert.equal(z.root().publication,0);
 });
 control('visibility-row-enumeration-cannot-reorder-accepted-declarations',()=>{
  const z=fixture(api),f=copy(z.facts);f.keys.reverse();const ctx=z.acquire(f),r=z.query(ctx,'capabilities','scalar','typed',z.facts);assert.equal(r.status,'capabilities');assert.equal(r.coverage,'permitted-only');assert.deepEqual(r.capabilities.map(d=>d.key),['scalar','list','map']);
 });
 control('malformed-facts-and-unknown-requests-never-become-default-values',()=>{
  const z=fixture(api),ctx=z.acquire();for(const modify of [f=>f.keys.pop(),f=>f.keys.push(f.keys[0]),f=>f.keys[0].originIds.push('invented'),f=>f.keys[0].value='yes',f=>f.target.id='missing',f=>f.keys[0].extra='value']){const f=copy(z.facts);modify(f);assert.equal(z.cut(f),null);}
  for(const op of ['theme','effective-configuration']){const e=z.envelope(ctx,op,op==='theme'?'scalar':'missing');assert.equal(api.queryInvocationSettingsRead(z.domain,z.pool,e,z.c,z.cut(),good).status,'unavailable');}
  const e=z.envelope(ctx);Object.defineProperty(e.request.arguments,'key',{enumerable:true,get(){assert.fail('getter executed');}});assert.equal(api.queryInvocationSettingsRead(z.domain,z.pool,e,z.c,z.cut(),good).status,'unavailable');
 });
 control('actual-parent-reservation-cannot-be-overcommitted-or-refunded-early',()=>{
  const z=fixture(api),held=z.usage().declaredHeldBytes,attachment=parentFundedPoolAttachmentBytes(api,z.domain,z.pool),before=z.parentUsage().declaredHeldBytes-held-attachment;assert(held>0);const denied=fixture(api,{limits:{maxHeldBytes:before+attachment}});assert.equal(denied.pool,null);assert.equal(denied.usage(),null);
  const ctx=z.acquire();assert.equal(api.retireInvocationSettingsReads(z.domain,z.pool,good),false);assert.equal(api.retireInvocationSettingsRead(z.domain,z.pool,ctx.context,good),false);assert.equal(api.stopInvocationSettingsRead(z.domain,z.pool,ctx.context,z.c,good),true);assert.equal(api.retireInvocationSettingsRead(z.domain,z.pool,ctx.context,{...good,deliveryDrained:false}),false);assert.equal(api.retireInvocationSettingsRead(z.domain,z.pool,ctx.context,good),true);assert.equal(api.retireInvocationSettingsReads(z.domain,z.pool,good),true);assert.equal(z.parentUsage().declaredHeldBytes,before);assert.equal(api.openInvocationSettingsReads(z.domain,z.p,good),null);
 });
 control('finite-capacities-refuse-without-fabricating-a-context',()=>{
  for(const limits of [{maxPackets:0},{maxContexts:0},{maxOrigins:0},{maxIssued:0},{maxPacketBytes:1},{maxCaptureBytes:1},{maxReplyBytes:1},{maxContextBytes:1},{maxSelectorBytes:1},{maxInputNodes:1},{maxInputDepth:0},{maxHeldBytes:0}]){const z=fixture(api,{readProfile:limits}),ctx=z.pool?z.acquire():{status:'unavailable'};assert.notEqual(ctx.status,'context');assert.equal(z.root().publication,0);budgetCases++;}
 });
 control('lifetime-issuance-is-spent-after-release-and-packet-retirement',()=>{
  const z=fixture(api,{readProfile:{maxIssued:4}}),ctx=z.acquire();assert.equal(ctx.status,'context');assert.equal(z.usage().issued,4);assert.equal(api.stopInvocationSettingsRead(z.domain,z.pool,ctx.context,z.c,good),true);assert.equal(api.retireInvocationSettingsRead(z.domain,z.pool,ctx.context,good),true);assert.equal(z.cut(),null);assert.equal(z.usage().issued,4);
 });
 control('fault-fences-disclosure-but-does-not-block-protected-cleanup',()=>{
  const z=fixture(api),ctx=z.acquire(),packet=z.cut();assert.equal(z.submit().status,'accepted');const h=z.host();Object.defineProperty(h,'semanticSettingsProfile',{get(){throw Error('host fault');}});z.run(h);assert.equal(z.parentUsage().faulted,true);assert.equal(z.query(ctx).status,'unavailable');assert.equal(api.discardInvocationSettingsReadCut(z.domain,z.pool,packet,good),true);assert.equal(api.stopInvocationSettingsRead(z.domain,z.pool,ctx.context,z.c,good),true);assert.equal(api.retireInvocationSettingsRead(z.domain,z.pool,ctx.context,good),true);assert.equal(api.retireInvocationSettingsReads(z.domain,z.pool,good),true);
 });
 control('observed-authority-continuity-and-retention-gaps-are-absorbing',()=>{
  for(const gate of ['authorized','scope','continuous','live','retained']){const z=fixture(api),ctx=z.acquire(),packet=z.cut(),e=z.envelope(ctx),before=z.root();assert.equal(api.queryInvocationSettingsRead(z.domain,z.pool,e,z.c,packet,{...good,[gate]:false}).status,'unavailable');assert.equal(api.queryInvocationSettingsRead(z.domain,z.pool,e,z.c,packet,good).status,'unavailable');const fresh=z.acquire();assert.equal(z.query(fresh).status,'value');assert.equal(z.root(),before);assert.equal(z.usage().invalidated,1);assert.equal(api.retireInvocationSettingsRead(z.domain,z.pool,ctx.context,{...good,deliveryDrained:false}),false);assert.equal(api.retireInvocationSettingsRead(z.domain,z.pool,ctx.context,good),true);assert.equal(api.discardInvocationSettingsReadCut(z.domain,z.pool,packet,good),true);gapCases++;}
 });
 control('observed-projection-invalidation-cannot-revive-after-regrant-or-ABA',()=>{
  const z=fixture(api),ctx=z.acquire(),narrow=copy(z.facts);narrow.keys[0].value=false;assert.equal(z.query(ctx,'effective-configuration','scalar','typed',narrow).status,'unavailable');assert.equal(z.query(ctx).status,'unavailable');
  const b=fixture(api),f=copy(b.facts);f.keys.forEach(r=>{r.origins=false;r.originIds=[];r.declaration=false;});const old=b.acquire(f);assert.equal(b.submit().status,'accepted');assert.equal(b.run().status,'settled');assert.equal(b.query(old,'effective-configuration','scalar','typed',f).status,'unavailable');
  const i=api.issueInvocation(b.domain,b.controlContext,b.host());assert(i);const e=b.settingsEnvelope('typed');e.request.arguments={...b.request,invocation:i.reference,changes:[{...b.request.changes[0],expectedGeneration:2,value:1}]};const plan=api.prepareInvocationSemanticSettings(b.domain,i.ticket,e,b.p.engineProfile,b.controlContext,b.host());assert(plan);assert.equal(api.submitInvocation(b.domain,i.ticket,{kind:'settings',plan},'typed',b.controlContext,b.controlPolicy,b.host()).status,'accepted');assert.equal(api.runInvocation(b.domain,i.ticket,b.controlContext,b.host(),b.host()).status,'settled');assert.equal(b.root().engineSettings.resolved[key(id('TA'))].scalar.value,1);assert.equal(b.query(old,'effective-configuration','scalar','typed',f).status,'unavailable');const fresh=b.acquire(f);assert.equal(b.query(fresh,'effective-configuration','scalar','typed',f).value,1);
 });
 control('transient-transfer-and-resource-refusals-do-not-invalidate',()=>{
  for(const gate of ['resourcesReady','transferReady','replyReady','outsidePublicationGuard','notStopped']){const z=fixture(api),ctx=z.acquire();assert.equal(z.query(ctx,'effective-configuration','scalar','typed',z.facts,z.c,{...good,[gate]:false}).status,'unavailable');assert.equal(z.query(ctx).status,'value');transientCases++;}
 });
 control('foreign-or-malformed-gap-claims-cannot-poison-a-context',()=>{
  for(const kind of ['target','recipient','variant','extra']){const z=fixture(api),ctx=z.acquire(),e=z.envelope(ctx),c={...z.c};if(kind==='target')e.request.arguments.target=copy(ctx.target);if(kind==='recipient')c.recipient='other';if(kind==='variant')e.request.variant='theme';if(kind==='extra')e.extra=true;const cut=z.cut(z.facts,c);assert.equal(api.queryInvocationSettingsRead(z.domain,z.pool,e,c,cut,{...good,continuous:false}).status,'unavailable');assert.equal(z.usage().invalidated,0);assert.equal(z.query(ctx).status,'value');}
  const z=fixture(api),ctx=z.acquire();assert.equal(z.query(ctx,'effective-configuration','scalar','typed',z.facts,z.c,{...good,continuous:undefined}).status,'unavailable');assert.equal(z.query(ctx).status,'value');assert.equal(z.query(ctx,'effective-configuration','scalar','typed',z.facts,z.c,{...good,outsidePublicationGuard:false,continuous:false}).status,'unavailable');assert.equal(z.query(ctx).status,'value');
 });
 return {scope:'owned-native-engine-settings-inspection-only',products,gateCases,contextCases,budgetCases,gapCases,transientCases,controls,actualOwnerAndParentChargeChecked:true,capturedCeilingAndCurrentReleaseChecked:true,opaqueOriginsAndFaultCleanupChecked:true,observedInvalidationAbsorbingChecked:true,hostPolicyConsumerOrPhysicalTruthProved:false,allF07ProductsOrWireProved:false,independentRefinementProved:false};
}
const mutations=[
 ['read-permission-ignored','!yes(g,read)','false'],
 ['recipient-scope-unbound','!equal(ctx.scope,c)','false'],
 ['caller-copy-as-current-basis','basis!==owner.root.engineSettings','false'],
 ['stale-read-cut-accepted','v.basis!==owner.root.engineSettings','false'],
 ['value-grant-narrowing-ignored','old.value&&!grant.value','false'],
 ['capture-ceiling-replaced','projection(p.cfg,p.facts,old?.rows??null)','projection(p.cfg,p.facts,null)'],
 ['origin-identities-leaked','ids.map(id=>ctx.refs.get(id))','ids'],
 ['parent-budget-not-checked','charge>owner.limits.maxHeldBytes-owner.charge','false'],
 ['retired-context-refunds-issuance','m.contexts.delete(reference)','(m.issued=0,m.contexts.delete(reference))'],
 ['pool-retires-live-contexts','m.packets.size||m.contexts.size','m.packets.size'],
 ['fault-blocks-cut-cleanup','discardNativeSettingsReadCut(owner,domain,handle,packet,g){const m=retained(owner,domain)','discardNativeSettingsReadCut(owner,domain,handle,packet,g){const m=active(owner,domain)'],
 ['closed-pool-reopens','m.closed=true;m.profile=null;owner.charge-=m.charge;return true;','m.closed=true;m.profile=null;owner.charge-=m.charge;pools.delete(owner);return true;'],
 ['read-cut-can-be-reused','m.packets.delete(packet);return v;','return v;'],
 ['target-handle-unbound','ctx.target!==a.target','false'],
 ['incomplete-facts-accepted','!factsValid(data.value.cfg,data.value.facts)','false'],
 ['accessor-record-accepted',"Object.hasOwn(ds[k],'value')",'true'],
 ['changed-visible-projection-released','!equal(current,ctx.data)','false'],
 ['visibility-facts-select-declaration-order','cfg.schemas.map(s=>f.keys.find(r=>r.key===s.key)).filter(r=>r.visible)','f.keys.filter(r=>r.visible)'],
 ['observed-gap-not-retained',"if(['authorized','scope','continuous','live','retained'].some(k=>g?.[k]===false)){invalidate(ctx);return no();}",''],
 ['observed-projection-invalidation-not-retained','if(!current||!equal(current,ctx.data)){invalidate(ctx);return no();}','if(!current||!equal(current,ctx.data))return no();'],
 ['transient-resource-failure-destroys-context','if(!yes(g,read))return no();','if(!yes(g,read)){invalidate(ctx);return no();}'],
 ['foreign-gap-poisons-other-contexts','const a=request(m,envelope),ctx=a&&m.contexts.get(a.context);','if(g?.continuous===false)for(const held of m.contexts.values())invalidate(held);const a=request(m,envelope),ctx=a&&m.contexts.get(a.context);']
];
function moduleURL(file,overrides={},mutation=null){let source=fs.readFileSync(new URL(file,import.meta.url),'utf8');if(mutation){assert(source.includes(mutation[1]),mutation[0]);source=source.replaceAll(mutation[1],mutation[2]);}source=source.replace(/from '(\.\/[^']+)'/g,(_,p)=>`from ${JSON.stringify(overrides[p]??new URL(p,import.meta.url).href)}`).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>`new URL(${JSON.stringify(new URL(p,import.meta.url).href)},import.meta.url)`);return 'data:text/javascript;base64,'+Buffer.from(source).toString('base64');}
export async function checkEngineSettingsReadMutations(){const load=async mutation=>{const reader=moduleURL('engine-settings-reads.mjs',{},mutation);return import(moduleURL('invocation-publication.mjs',{'./engine-settings-reads.mjs':reader}));};checkEngineSettingsReads(await load(null));const killed=[];for(const mutation of mutations){const api=await load(mutation);assert.throws(()=>checkEngineSettingsReads(api),{code:'ERR_ASSERTION'},mutation[0]);killed.push(mutation[0]);}return killed;}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkEngineSettingsReads(),mutationBaselineChecked:true,mutations:await checkEngineSettingsReadMutations()},null,2));
