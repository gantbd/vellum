// Fixed verifier injections, not callbacks in the engine interface.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {discoveryGatewayFixture,discoveryGatewayLimits as limits} from './discovery-gateway-checks.mjs';
import {discoveryArgumentClassificationBytes as classification} from './discovery-arguments.mjs';
const raw=fs.readFileSync(new URL('./native-discovery.mjs',import.meta.url),'utf8'),operations=['list','inspect','advisory-enablement'],points=[
 ['classified','  const p=captureDiscoveryArgumentLimits(inputLimits);'],
 ['limits','  const maximum=p.maxRequestBytes,workspace=7*maximum;'],
 ['workspace','  const n=normalizeDiscoveryArguments(envelope,c,p,maximum);'],
 ['facts','  const packet=captureNativeDiscoveryQueryCurrentFacts(g,p,maximum);'],
 ['query','  return queryNativeDiscoveryCore(owner,discoveryProducerRequest(n),n.record.context,cut,packet.value.facts);']
];
async function load(marker,injection,name,input=raw){if(input.split(marker).length-1!==1)throw Error('DISCOVERY_FAULT_CONSTRUCTION_TARGET: '+name);const source=input.replace(marker,injection+marker).replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(new URL(p,import.meta.url).href)).replace(/new URL\('(\.\/[^']+)',import.meta.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,import.meta.url).href)+',import.meta.url)');return import('data:text/javascript;base64,'+Buffer.from(source).toString('base64')+'#'+name);}
export async function checkDiscoveryResultGuard(input=raw){
 const marker=' // Check the actual bounded result before publishing tentative selectors.',modes={closedStatus:"reply={...reply,status:'unexpected'};",wrongContext:'reply={...reply,context:{}};',validatorException:"reply=new Proxy(reply,{getPrototypeOf(){throw Error('selected result validator exception');}});"};let cases=0,freshSelectorCases=0;
 for(const [mode,corruption]of Object.entries(modes)){
  const api=await load(marker,'if(m.discoveryDecoding===true||g.testOnlyCorruptResult===true){'+corruption+'}\n','result-guard-'+mode,input);
  for(const op of operations)for(const route of ['typed','dynamic','native']){
   const x=discoveryGatewayFixture(op,route==='native'?'typed':route,10,{},api);let request=x.request;
   if(op==='list')request={...request,limit:2};
   if(op==='inspect'){const p=x.direct({contract:request.contract,profile:request.profile,context:request.context,operation:'list',limit:1,continuation:x.page.continuation});assert.equal(p.status,'page');request={...request,entry:p.entries[0].reference};}
   const root=x.root(),parent=x.x.z.usage(),before=x.usage();
   const result=route==='native'?x.direct(request,x.x.g({testOnlyCorruptResult:true})):x.query(x.envelope(request));
   assert.equal(result.status,'unavailable');assert.equal(x.root(),root);assert.deepEqual(x.x.z.usage(),parent);assert.equal(kernel.invocationUsage(x.x.domain).faulted,false);
   assert.deepEqual(x.decodeUsage(),{busy:false,declaredWorkspaceBytes:0,faulted:true});assert.deepEqual(x.usage(),before);
   if(op!=='advisory-enablement')freshSelectorCases++;
   assert.equal(api.queryNativeDiscovery(x.x.owner,request,x.c,null,x.x.g()).status,'unavailable');assert.equal(x.retire().status,'retired');assert.equal(x.usage().issued,before.issued);assert.equal(api.retireNativeDiscoveryRegistry(x.x.owner,x.x.g()).status,'retired');cases++;
  }
 }
 return {cases,freshSelectorCases,threeNativeFormsAndBothDecodedRoutes:true,guardBeforeSelectorPublication:true,localFaultProtectedDischargeAndNoDomainFault:true,globalPhysicalOrIndependentProof:false};
}
export async function checkDiscoveryResultGuardMutations(){
 const baseline=await checkDiscoveryResultGuard(),guard='if(!nativeDiscoveryResultValid(reply,request,m.limits))',commit='s.entries=entries;s.variants=variants;s.cursors=cursors;m.selectors=selectors;m.issued=issued;',mutations=[];
 const rules=[['guard-bypassed',guard,'if(false)'],['invalid-result-does-not-fence',guard+'{m.discoveryFaulted=true;',guard+'{m.discoveryFaulted=false;'],['selectors-published-before-guard',' try{'+guard,' '+commit+'\n try{'+guard]];
 for(const [name,from,to]of rules){assert.equal(raw.split(from).length-1,1,name);const input=raw.replace(from,to);let failed=false;try{await checkDiscoveryResultGuard(input);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name+': '+String(e.message).slice(0,100));failed=true;}assert(failed,name);mutations.push(name);}
 return {baseline,mutations,baselineBeforeMutationChecked:true,constructionFailuresCounted:false};
}
export async function checkDiscoveryGatewayFaults(){
 let earlyFaults=0,lateFaults=0,lateInstalled=0,reentries=0;
 const discharge=(x,api,before)=>{
  assert.equal(x.root(),before.root);assert.deepEqual(x.x.z.usage(),before.parent);assert.equal(kernel.invocationUsage(x.x.domain).faulted,false);
  assert.deepEqual(x.decodeUsage(),{busy:false,declaredWorkspaceBytes:0,faulted:true});assert.equal(x.usage().declaredHeldBytes,before.usage.declaredHeldBytes);
  assert.equal(api.nativeBuiltinDiscoveryRows(x.x.owner,x.c,x.x.g()),null);assert.equal(api.queryDecodedNativeDiscovery(x.x.owner,x.envelope(),limits,x.c,null,x.x.g()).status,'unavailable');assert.equal(api.queryNativeDiscovery(x.x.owner,x.request,x.c,null,x.x.g()).status,'unavailable');
  const issued=x.usage().issued;assert.equal(x.retire().status,'retired');assert.equal(x.usage().issued,issued);assert.equal(api.retireNativeDiscoveryRegistry(x.x.owner,x.x.g()).status,'retired');assert.equal(api.openNativeDiscoveryRegistry(x.x.domain,x.x.l,x.x.g()),null);
 };
 for(const [name,marker]of points){
  const api=await load(marker,"throw Error('selected pre-query fault');\n",name);
  for(const op of operations)for(const route of ['typed','dynamic']){
   const x=discoveryGatewayFixture(op,route,10,{},api);assert.equal(x.query().status,'unavailable');const b=x.last();assert.equal(b.afterQuery.preparations,b.usage.preparations);assert.equal(b.afterQuery.issued,b.usage.issued);assert.equal(b.afterQuery.selectors,b.usage.selectors);assert.equal(x.usage().contexts,b.usage.contexts);assert.equal(x.usage().stopped,b.usage.stopped);discharge(x,api,b);earlyFaults++;
  }
 }
 const late=await load('return reply;\n}',"if(m.discoveryDecoding===true)throw Error('selected late query fault');",'late-installed-query');
 for(const op of operations)for(const route of ['typed','dynamic']){
  const x=discoveryGatewayFixture(op,route,10,{},late);let request=x.request;
  if(op==='list')request={...request,limit:2};
  if(op==='inspect'){const next=x.direct({contract:x.request.contract,profile:x.request.profile,context:x.request.context,operation:'list',limit:1,continuation:x.page.continuation});assert.equal(next.status,'page');request={...request,entry:next.entries[0].reference};}
  assert.equal(x.query(x.envelope(request)).status,'unavailable');const b=x.last();assert.equal(b.afterQuery.preparations,b.usage.preparations-1);assert(b.afterQuery.issued>=b.usage.issued);
  if(op!=='advisory-enablement'){assert(b.afterQuery.issued>b.usage.issued);assert(b.afterQuery.selectors>b.usage.selectors);lateInstalled++;}
  discharge(x,late,b);lateFaults++;
 }
 const reentryApi=await load(points[3][1],"g.testOnlyProbe?.();\n",'reentry');
 for(const op of operations)for(const route of ['typed','dynamic']){
  const x=discoveryGatewayFixture(op,route,10,{},reentryApi),rows=reentryApi.nativeBuiltinDiscoveryRows(x.x.owner,x.c,x.x.g()),p=x.cut(),sibling=reentryApi.acquireNativeDiscovery(x.x.owner,x.c,p,x.x.g());assert.equal(sibling.status,'context');reentryApi.retireNativeDiscoveryCut(x.x.owner,p,x.x.g());assert.equal(reentryApi.stopNativeDiscovery(x.x.owner,sibling.context,x.c,x.x.g()).status,'stopped');
  const base=x.usage(),g=x.x.g();let calls=0;g.testOnlyProbe=()=>{
   calls++;assert.deepEqual(x.decodeUsage(),{busy:true,declaredWorkspaceBytes:classification+7*limits.maxRequestBytes,faulted:false});assert.equal(x.usage().declaredHeldBytes,base.declaredHeldBytes+classification+7*limits.maxRequestBytes);
   assert.equal(reentryApi.queryDecodedNativeDiscovery(x.x.owner,x.envelope(),limits,x.c,x.last().packet,x.x.g()).status,'unavailable');
   assert.equal(reentryApi.queryNativeDiscovery(x.x.owner,x.request,x.c,x.last().packet,x.x.g()).status,'unavailable');
   assert.equal(reentryApi.nativeBuiltinDiscoveryRows(x.x.owner,x.c,x.x.g()),null);
   assert.equal(reentryApi.prepareNativeBuiltinDiscoveryCut(x.x.owner,x.c,x.x.visibility(rows),x.x.target(),x.x.advice(rows),x.x.g()),null);
   assert.equal(reentryApi.prepareNativeDiscoveryCut(x.x.owner,x.c,rows,x.x.visibility(rows),x.x.target(),x.x.advice(rows),x.x.g()),null);
   assert.equal(reentryApi.acquireNativeDiscovery(x.x.owner,x.c,x.last().packet,x.x.g()).status,'unavailable');
   assert.equal(reentryApi.stopNativeDiscovery(x.x.owner,x.a.context,x.c,x.x.g()).status,'unavailable');
   assert.equal(reentryApi.retireNativeDiscovery(x.x.owner,sibling.context,x.x.g()).status,'unavailable');
   assert.equal(reentryApi.retireNativeDiscoveryCut(x.x.owner,x.last().packet,x.x.g()).status,'unavailable');
   assert.equal(reentryApi.retireNativeDiscoveryRegistry(x.x.owner,x.x.g()).status,'unavailable');reentries+=10;
  };
  assert.equal(x.query(x.envelope(),limits,x.c,g).status,{list:'page',inspect:'descriptor','advisory-enablement':'advice'}[op]);assert.equal(calls,1);assert.deepEqual(x.decodeUsage(),{busy:false,declaredWorkspaceBytes:0,faulted:false});assert.equal(x.usage().declaredHeldBytes,base.declaredHeldBytes);assert.equal(reentryApi.retireNativeDiscovery(x.x.owner,sibling.context,x.x.g()).status,'retired');
 }
 return {scope:'actual-F01-owner-gateway-fixed-fault-and-reentry',earlyFaults,lateFaults,lateInstalled,reentries,faultedOwnerNotEditingDomain:true,originalHandleCleanupAndOnlyTemporaryRefundChecked:true,resultValidationWorkspaceOrPhysicalOrGeneralFaultContainmentProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(await checkDiscoveryGatewayFaults()));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'[private module]'));process.exitCode=1;}
