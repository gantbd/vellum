// Native resource-reader invalidation; not all F02/F07/provider policy truth.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {registrationReadFixture as fixture,registrationReadReferenceApi as reference,checkRegistrationReads} from './registration-read-checks.mjs';
import {resourceRegistrationInstall as install} from './resource-registration-checks.mjs';
const copy=structuredClone,list={operation:'list',limit:1,continuation:null};
export function checkReadInvalidation(supplied=reference){
 const api={...reference,...supplied},legacy=checkRegistrationReads(api,true),controls=[],control=(n,f)=>{f();controls.push(n);};let gapCases=0,transientCases=0,unknownCases=0;
 control('bound-observed-loss-is-absorbing-without-authoring-or-refunding',()=>{
  for(const gate of ['authorized','scope','continuous','live','retained']){const x=fixture(api),c=x.acquire().context,first=x.query(c,list);assert.equal(first.status,'page');const root=x.z.root(),native=copy(x.z.usage()),before=copy(x.usage());assert.equal(x.query(c,list,x.scope,x.g({[gate]:false,ownerControlAllowed:false})).status,'unavailable');assert.equal(x.usage().invalidated,1);assert.equal(x.usage().selectors,0);assert.equal(x.usage().issued,before.issued);assert.equal(x.usage().declaredHeldBytes,before.declaredHeldBytes);assert.equal(x.query(c,list).status,'unavailable');assert.equal(x.query(c,{operation:'inspect',entry:first.entries[0].reference}).status,'unavailable');assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),native);const fresh=x.acquire();assert.equal(fresh.status,'context');assert.equal(x.query(fresh.context,list).status,'page');gapCases++;}
 });
 control('transient-pressure-and-query-cancel-do-not-destroy-frontiers',()=>{
  for(const gate of ['resourcesReady','transferReady','replyReady','notStopped']){const x=fixture(api),c=x.acquire().context;assert.equal(x.query(c,list,x.scope,x.g({[gate]:false})).status,'unavailable');assert.equal(x.usage().invalidated,0);assert.equal(x.query(c,list).status,'page');transientCases++;}
 });
 control('unknown-host-facts-refuse-without-asserting-an-observed-loss',()=>{
  for(const gate of ['authorized','scope','continuous','live','retained'])for(const value of [1,undefined]){const x=fixture(api),c=x.acquire().context;assert.equal(x.query(c,list,x.scope,x.g({[gate]:value})).status,'unavailable');assert.equal(x.usage().invalidated,0);assert.equal(x.query(c,list).status,'page');unknownCases++;}
 });
 control('permission-regrant-and-real-native-metadata-ABA-never-revive',()=>{
  for(const cause of ['visibility','descriptor','metadata']){const x=fixture(api),c=x.acquire().context;assert.equal(x.query(c,list).status,'page');const f=x.facts(),original=x.z.source().atomicRead;
   if(cause==='visibility'){f[0].visible=false;f[0].descriptor=false;f[0].order=null;f.filter(r=>r.visible).forEach((r,i)=>r.order=i);}else if(cause==='descriptor')f[0].descriptor=false;else{const change=install('read',x.z.source());change.configuration.atomicRead=!original;assert.equal(x.z.run([change]).status,'applied');}
   assert.equal(x.query(c,list,x.scope,x.g(),f).status,'unavailable');assert.equal(x.usage().invalidated,1);
   if(cause==='metadata'){const change=install('read',x.z.source());change.configuration.atomicRead=original;assert.equal(x.z.run([change]).status,'applied');assert.equal(x.z.source().atomicRead,original);}
   assert.equal(x.query(c,list).status,'unavailable');const fresh=x.acquire();assert.equal(fresh.status,'context');assert.equal(x.query(fresh.context,list).status,'page');
  }
 });
 control('foreign-and-malformed-claims-cannot-poison-bound-contexts',()=>{
  for(const kind of ['copy','recipient','profile','extra','operation']){const x=fixture(api),c=x.acquire().context,p=x.prepare(),scope={...x.scope},request={contract:'resource-catalog-query/1',operation:'capabilities',context:c,profile:x.scope.profile,request:list};if(kind==='copy')request.context=copy(c);if(kind==='recipient')scope.recipient='other';if(kind==='profile')scope.profile='other';if(kind==='extra')request.extra=true;if(kind==='operation')request.operation='execute';assert.equal(api.queryRegistrationRead(x.owner,request,scope,p,x.g({continuous:false})).status,'unavailable');assert.equal(api.retireRegistrationVisibility(x.owner,p,x.g()).status,'retired');assert.equal(x.usage().invalidated,0);assert.equal(x.query(c,list).status,'page');}
 });
 control('retained-invalidated-slots-need-explicit-discharge',()=>{
  const x=fixture(api,undefined,{limits:{maxContexts:1}}),c=x.acquire().context;assert.equal(x.query(c,list,x.scope,x.g({continuous:false})).status,'unavailable');const before=copy(x.usage());assert.equal(x.acquire().status,'unavailable');assert.equal(x.usage().contexts,1);assert.equal(api.retireRegistrationRead(x.owner,c,x.g({retentionReleased:false})).status,'unavailable');assert.equal(api.retireRegistrationRead(x.owner,c,x.g({deliveryDrained:false})).status,'unavailable');assert.equal(x.usage().declaredHeldBytes,before.declaredHeldBytes);assert.equal(api.retireRegistrationRead(x.owner,c,x.g()).status,'retired');assert.equal(x.usage().issued,before.issued);assert.equal(x.usage().invalidated,0);assert.equal(x.acquire().status,'context');
 });
 control('invalidation-and-retirement-do-not-refund-lifetime-issuance',()=>{
  const x=fixture(api,undefined,{limits:{maxIssued:3}}),c=x.acquire().context;assert.equal(x.query(c,list).status,'page');assert.equal(x.usage().issued,3);assert.equal(x.query(c,list,x.scope,x.g({continuous:false})).status,'unavailable');assert.equal(api.retireRegistrationRead(x.owner,c,x.g()).status,'retired');assert.equal(x.usage().issued,3);assert.equal(x.acquire().status,'unavailable');
 });
 control('one-context-retention-loss-does-not-invalidate-another-hold',()=>{
  const x=fixture(api),a=x.acquire().context,b=x.acquire().context;assert.equal(x.query(a,list,x.scope,x.g({retained:false})).status,'unavailable');assert.equal(x.query(b,list).status,'page');assert.equal(x.usage().invalidated,1);assert.equal(x.usage().contexts,2);
 });
 return {scope:'owned-resource-read-invalidation-only',gapCases,transientCases,unknownCases,legacyControls:legacy.controls.length,controls,actualNativeOwnerAndMetadataABAChecked:true,absorbingBoundInvalidationAndDischargeChecked:true,allReaderOrHostPolicyTruthProved:false,physicalConcurrentOrIndependentRefinementProved:false};
}
const mutations=[
 ['observed-loss-not-retained','if(["authorized","scope","continuous","live","retained"].some(k=>g?.[k]===false)){closeContext(m,request.context,ctx,true);return no();}',''],
 ['projection-invalidation-not-retained','if(!captureFits(m,current)||!equal(current,ctx.rows)){closeContext(m,request.context,ctx,true);return no();}','if(!captureFits(m,current)||!equal(current,ctx.rows))return no();'],
 ['closed-context-can-read-again','||ctx.stopped||','||false||'],
 ['invalidation-retires-before-discharge','if(invalidated)ctx.invalidated=true;','if(invalidated){ctx.invalidated=true;m.contexts.delete(context);}'],
 ['transient-refusal-invalidates','if(!gates(g,catalog.releaseGates))return no();','if(!gates(g,catalog.releaseGates)){closeContext(m,request.context,ctx,true);return no();}'],
 ['foreign-gap-claim-poisons-context','ctx=m?.contexts.get(request?.context);if(!m','ctx=m?.contexts.get(request?.context);if(m&&ctx&&g?.continuous===false)closeContext(m,request.context,ctx,true);if(!m'],
 ['unknown-fact-becomes-loss','some(k=>g?.[k]===false)','some(k=>g?.[k]!==true)'],
 ['invalidated-holds-hidden-from-supervision','if(invalidated)ctx.invalidated=true;','if(invalidated)ctx.invalidated=false;']
];
function url(mutation){let source=fs.readFileSync(new URL('./registration-reads.mjs',import.meta.url),'utf8');if(mutation){assert(source.includes(mutation[1]),mutation[0]);source=source.replaceAll(mutation[1],mutation[2]);}return 'data:text/javascript;base64,'+Buffer.from(source.replace(/from '(\.\/[^']+)'/g,(_,p)=>`from ${JSON.stringify(new URL(p,import.meta.url).href)}`).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>`new URL(${JSON.stringify(new URL(p,import.meta.url).href)},import.meta.url)`)).toString('base64');}
export async function checkReadInvalidationMutations(){checkReadInvalidation(await import(url(null)));const killed=[];for(const mutation of mutations){const api=await import(url(mutation));assert.throws(()=>checkReadInvalidation(api),{code:'ERR_ASSERTION'},mutation[0]);killed.push(mutation[0]);}return killed;}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkReadInvalidation(),mutationBaselineChecked:true,mutations:await checkReadInvalidationMutations()},null,2));
