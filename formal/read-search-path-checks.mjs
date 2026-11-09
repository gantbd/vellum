import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as observer from './publication-observation.mjs';
import {acquireObservationRead} from './observation-reads.mjs';
import * as reads from './read-contexts.mjs';
import * as versions from './version-set-contexts.mjs';
import {searchLiteral,replacementResultFootprint} from './literal-search.mjs';
import {copySettingsData} from './semantic-settings.mjs';
import {searchCapturedReadContext} from './read-search-composition.mjs';
const copy=structuredClone;
const catalog=JSON.parse(fs.readFileSync(new URL('./observation-reads.json',import.meta.url)));
const profile=JSON.parse(fs.readFileSync(new URL('./publication-observation.json',import.meta.url))).profile;
const observerLimits={maxRootBytes:131072,maxDocuments:8,maxRows:32,maxViews:8,maxProjectionBytes:16384,maxReplyBytes:32768,maxQueue:2,maxOutcomes:8,maxOutcomeBytes:4096,maxIssued:128,maxHeldBytes:1048576,maxBindings:32,maxBindingBytes:32768};
const readLimits={contexts:4,units:128,selectors:32,holds:4,issuance:64,sourceBytes:32,source:{maxRootBytes:32,maxInputBytes:32,maxContentBytes:32,maxReplyBytes:4096},annotation:{maxInputRows:32,maxCaptureBytes:16384,maxRowBytes:2048,maxPageRows:8}};
const queryLimits={maxRootBytes:1024,maxScopes:8,maxInputBytes:1024,maxQueryBytes:128,maxComparisons:65536,maxMatches:64,maxReplyBytes:32768,maxReplacementBytes:1024,maxEditBytes:8192,chunkBytes:1};
const input={maxRequestBytes:16384,maxInputNodes:4096,maxInputDepth:64};
function fixture(sourceAllowed=true){
 const document={authority:'a',id:'d',incarnation:1};
 const root={authority:'a',publication:0,documents:[{document,live:true,source:[97,97,32,97,97],revision:1,annotationSet:1,records:[]}],views:[],groups:[],frontiers:{}};
 const context={authority:'a',recipient:'reader',purpose:'query',destination:'client',policy:'p1',continuity:'c1',profile,documents:[{document,source:sourceAllowed,annotations:[],views:[],collection:{body:true,target:true}}],outcomePolicy:{publication:false,control:true,effect:false,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:'reader'}};
 const gates=Object.fromEntries([...catalog.releaseGates,'preparationReserved','visibilityAttested','visibilityComplete','projectionOrderAttested','baselineAllowed'].map(k=>[k,true]));
 const packet=observer.prepareCollectionVisibility(root,context,[],observerLimits,gates);
 const opened=observer.openPublicationObserver(root,context,observerLimits,gates,packet);assert(opened);
 const current={document,recipient:context.recipient,purpose:context.purpose,destination:context.destination,policy:context.policy,continuity:context.continuity,profile,...Object.fromEntries([...catalog.readGates,'replyReady'].map(k=>[k,true]))};
 const registry=reads.newReadRegistry('read-issuer',readLimits);
 const acquire=(data)=>acquireObservationRead(registry,opened.owner,root,{contract:'read-context/1',operation:'snapshot-handoff',document,selection:{kind:'capture-current'},data,profile},context,current,readLimits,gates,observer.prepareCollectionVisibility(root,context,[],observerLimits,gates));
 return {document,root,context,current,registry,gates,owner:opened.owner,acquire};
}
function query(z,token,operation='literal-matches'){
 const request={contract:'literal-search/1',operation,context:token,ref:{authority:'a',document:'d',incarnation:1,revision:1},query:'aa',profile:'literal-exact/1',scopes:[{lo:0,hi:5}],generation:1,navigation:operation==='literal-matches'?null:{start:operation==='next-match'?0:5,current:null,wrap:false}};
 const searchCurrent={request:copy(request),grant:true,scope:true,continuity:true,capacity:true,retained:true,notStopped:true};
 return {request,current:z.current,searchCurrent,searchLimits:queryLimits};
}
const depth=v=>v&&typeof v==='object'&&Object.keys(v).length?1+Math.max(...Object.values(v).map(depth)):0;
export function checkReadSearchPaths(run=searchCapturedReadContext){
 let nativeCases=0,boundaryCases=0;const controls=[],check=(name,f)=>{f();controls.push(name);};
 check('observer-read-context-native-search-uses-one-captured-source',()=>{
  for(const data of [['source'],['source','annotations']])for(const operation of ['literal-matches','next-match','previous-match']){
   const z=fixture(),usage=observer.publicationObserverUsage(z.owner),acquired=z.acquire(data);assert.equal(acquired.result.status,'context');
   const p=query(z,acquired.result.context,operation),before=copy(acquired.state),source=copy(z.root.documents[0].source);
   z.root.documents[0].source.fill(88);z.root.documents[0].revision++;
   const out=run(acquired.state,p.request,p.current,p.searchCurrent,p.searchLimits,input);
   const expected=searchLiteral({ref:p.request.ref,source},p.request,p.searchCurrent,{...queryLimits,maxRootBytes:32,maxInputBytes:32,maxReplyBytes:4096});
   assert.deepEqual(out.result,expected);assert(replacementResultFootprint(out.result));assert.equal(out.state,acquired.state);assert.deepEqual(acquired.state,before);assert.deepEqual(observer.publicationObserverUsage(z.owner),usage);nativeCases++;
  }
 });
 check('observer-annotation-grant-does-not-become-source-or-search-authority',()=>{
  const z=fixture(false);assert.equal(z.acquire(['source']).result.status,'unavailable');
  const acquired=z.acquire(['annotations']);assert.equal(acquired.result.status,'context');const p=query(z,acquired.result.context);
  assert.equal(run(acquired.state,p.request,p.current,p.searchCurrent,p.searchLimits,input).result.status,'unavailable');
 });
 check('complete-packet-byte-node-depth-exact-boundaries',()=>{
  const z=fixture(),acquired=z.acquire(['source']),p=query(z,acquired.result.context),measured=copySettingsData(p,input);assert(measured);
  const minima={maxRequestBytes:measured.bytes,maxInputNodes:measured.nodes,maxInputDepth:depth(p)};
  for(const field of Object.keys(minima))for(const delta of [-1,0,1]){
   const bounds={...input,[field]:minima[field]+delta},before=copy(acquired.state),out=run(acquired.state,p.request,p.current,p.searchCurrent,p.searchLimits,bounds);
   assert.equal(out.result.status,delta<0?'unavailable':'complete',`${field}/${delta}`);assert.equal(out.state,acquired.state);assert.deepEqual(acquired.state,before);boundaryCases++;
  }
 });
 check('stopping-captured-context-denies-later-search-without-refunding-issuance',()=>{
  const z=fixture(),acquired=z.acquire(['source']),p=query(z,acquired.result.context),held=reads.holdReadContext(acquired.state,acquired.result.context);assert(held.ticket);
  const prior=run(held.state,p.request,p.current,p.searchCurrent,p.searchLimits,input);assert.equal(prior.result.status,'complete');
  const stopped=reads.closeReadContext(held.state,{contract:'read-context/1',operation:'stop-observation',context:acquired.result.context},z.current.recipient);assert.equal(stopped.result.status,'stopped');
  assert.equal(run(stopped.state,p.request,p.current,p.searchCurrent,p.searchLimits,input).result.status,'unavailable');
  assert.equal(stopped.state.contexts.length,1);assert.equal(stopped.state.issued,held.state.issued);
  const released=reads.releaseReadHold(stopped.state,held.ticket);assert.equal(released.contexts.length,0);assert.equal(released.issued,held.state.issued);
  // Prior result memory still exists. This is not proof of F11/F12 re-use authorization.
  assert(replacementResultFootprint(prior.result));
 });
 check('version-set-member-is-not-an-independent-source-context',()=>{
  const z=fixture(),limits={...readLimits,groups:2,participants:2,retainedCuts:2,inputRoots:2},member={document:z.document,sourceRevision:1,source:[97,97,32,97,97],records:[]};
  const authority={id:'a',current:{identity:'cut',roots:[member]},retained:[]},host={...z.current,members:[{...z.current,stamp:{source:1}}]};
  const candidate=versions.prepareVersionSet(authority,{contract:'read-version-set/1',operation:'snapshot-handoff',authority:'a',participants:[{document:z.document,data:['source']}],selection:{kind:'capture-current'},profile},host,limits);assert(candidate);
  const acquired=versions.acquireVersionSet(versions.newVersionRegistry('set-issuer',limits),candidate,host);assert.equal(acquired.result.status,'version-set');
  const token=acquired.result.participants[0].snapshot,p=query(z,token);
  const direct=versions.queryVersionMember(acquired.state,{contract:'source-query/1',operation:'bounded-range',document:z.document,snapshot:token,profile,lo:0,hi:5},host);assert.equal(direct.result.status,'content');
  for(const current of [p.current,{...p.current,versionSetPermit:'foreign'}])assert.equal(run(acquired.state.registry,p.request,current,p.searchCurrent,p.searchLimits,input).result.status,'unavailable');
  const stopped=versions.stopVersionSet(acquired.state,{contract:'read-context/1',operation:'stop-observation',context:acquired.result.context},z.current.recipient);assert.equal(stopped.result.status,'stopped');
  assert.equal(run(stopped.state.registry,p.request,{...p.current,versionSetPermit:acquired.result.context},p.searchCurrent,p.searchLimits,input).result.status,'unavailable');
 });
 return {scope:'serialized-observer-read-context-native-search-path',nativeCases,boundaryCases,controls,nativeIssuerResultLifetimeActualPolicyOrPhysicalAccountingProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkReadSearchPaths()));
