// Scoped catalog/dependency boundary checks, not all-native or hostile decoder proof.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {isDeepStrictEqual as equal} from 'node:util';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {compileKeymap,stepChord} from './frontend-keymaps.mjs';
import {openInvocationDomain,invocationAuthorityRoot,invocationUsage} from './invocation-publication.mjs';
import {decodeTyped,planRoute} from './operation-catalog.mjs';
import {validateOwnership,decodeEngineTyped,decodeEngineDynamic,routeEngineTyped,routeEngineDynamic,engineInventory} from './engine-facade.mjs';
import {decodeIntegrationTyped,decodeIntegrationDynamic,routeIntegrationTyped,routeIntegrationDynamic,integrationInventory} from './integration-facade.mjs';
const here=path.dirname(fileURLToPath(import.meta.url));
export function checkEngineUIBoundary(){
const rows=JSON.parse(fs.readFileSync(path.join(here,'operation-families.json')));
const ownership=JSON.parse(fs.readFileSync(path.join(here,'engine-ui-ownership.json')));
// Independently transcribed from specification §1.1, not inferred from the manifest.
const presentation=new Set(['F09/caret-hit-conversion','F09/bounded-layout','F09/visual-navigation-target',
  'F12/supported-visual-movement','F15/scroll','F15/viewport-intent','F25/enabled-provider-measurement',
  'F26/dialog','F26/native-input-accessibility','F26/host-preference','F28/enabled-geometry-result']);
const integration=new Set(['F11/accepted-paste','F11/validated-cut-deletion','F22/plain-or-supported-structured-read','F23/copy','F23/cut-export-stage']);
let engineVariants=0,presentationVariants=0,integrationVariants=0,routingCases=0,integrationRoutingCases=0;
const flags=['enabled','grant','schemaValid','current','ordinaryCapacity','protectedCapacity','trustedProducer','attributed','releaseAllowed'];
for(let i=0;i<rows.length;i++)for(const variant of rows[i].variants){
  const t={family:i,variant,version:1},d={name:rows[i].name,variant,version:1};
  const key=rows[i].id+'/'+variant,integrated=integration.has(key),allowed=!presentation.has(key)&&!integrated;
  assert.equal(decodeEngineTyped(t)!==null,allowed);assert(equal(decodeEngineTyped(t),decodeEngineDynamic(d)));
  assert.equal(decodeIntegrationTyped(t)!==null,integrated);assert(equal(decodeIntegrationTyped(t),decodeIntegrationDynamic(d)));
  if(allowed)engineVariants++;else if(integrated)integrationVariants++;else presentationVariants++;
  for(let mask=0;mask<512;mask++){
    const facts=Object.fromEntries(flags.map((f,bit)=>[f,!!(mask&(1<<bit))]));
    const expected=allowed?planRoute(decodeTyped(rows,t),facts):{status:'unsupported',sink:null};
    assert(equal(routeEngineTyped(t,facts),expected));assert(equal(routeEngineDynamic(d,facts),expected));routingCases++;
    const expectedIntegration=integrated?planRoute(decodeTyped(rows,t),facts):{status:'unsupported',sink:null};
    assert(equal(routeIntegrationTyped(t,facts),expectedIntegration));assert(equal(routeIntegrationDynamic(d,facts),expectedIntegration));integrationRoutingCases++;
  }
}
const controls=[],control=(name,fn)=>{fn();controls.push(name);};
assert(equal(engineInventory().flatMap(f=>f.variants.map(v=>f.id+'/'+v)).toSorted(),
  rows.flatMap(f=>f.variants.map(v=>f.id+'/'+v)).filter(v=>!presentation.has(v)&&!integration.has(v)).toSorted()));
assert(equal(integrationInventory().flatMap(f=>f.variants.map(v=>f.id+'/'+v)).toSorted(),[...integration].toSorted()));
control('frontend-settings-keymap-not-an-engine-verb',()=>assert.equal(decodeEngineDynamic({name:'activate-context',variant:'settings-keymap',version:1}),null));
control('unknown-version-refused',()=>assert.equal(decodeEngineTyped({family:0,variant:'list',version:2}),null));
control('caller-cannot-supply-owner',()=>assert.equal(decodeEngineDynamic({name:'change-viewport',variant:'scroll',version:1,owner:'engine'}),null));
control('stable-family-index-after-presentation-gap',()=>assert.equal(engineInventory().find(f=>f.id==='F16').family,15));
control('metadata-is-owned-copy',()=>{const a=engineInventory();a[0].variants.length=0;assert(engineInventory()[0].variants.length>0);});
control('integration-metadata-is-owned-copy',()=>{const a=integrationInventory();a[0].variants.length=0;assert(integrationInventory()[0].variants.length>0);});
control('integration-family-indices-remain-editor-wide',()=>assert(equal(integrationInventory().map(f=>f.family),[10,21,22])));
control('integration-decoder-descriptor-cannot-be-mutated-into-another-control-profile',()=>{
 const decoded=decodeIntegrationTyped({family:22,variant:'copy',version:1});assert(decoded);
 assert(Object.isFrozen(decoded.descriptor.variants));
 assert.throws(()=>{decoded.descriptor.protectedVariants=['copy'];},{name:'TypeError'});
});
for(const [name,change] of [
  ['missing-family',p=>p.pop()],['duplicate-owner',p=>p[0].presentation.push(p[0].engine[0])],
  ['unknown-variant',p=>p[0].engine.push('surprise')],['omitted-variant',p=>p[0].engine.pop()],
  ['foreign-family',p=>p[0].family='F99'],['undeclared-field',p=>p[0].default='engine'],
  ['integration-double-owner',p=>p[10].engine.push(p[10].integration[0])],
  ['integration-missing-owner',p=>p[21].integration.pop()]
])control(name,()=>{const p=structuredClone(ownership);change(p);assert.throws(()=>validateOwnership(rows,p));});
// Declared static local imports only; no claim about arbitrary dynamic code/Rust linkage.
const seen=new Set(),forbidden=new Set(['frontend-keymaps.mjs','frontend-configuration-reference.mjs','ratatui-keymap-reference.mjs']);
function visit(name){
  if(seen.has(name))return;assert(!forbidden.has(name),`engine imports ${name}`);seen.add(name);
  const code=fs.readFileSync(path.join(here,name),'utf8');
  for(const m of code.matchAll(/(?:from\s*|import\s*)['"]\.\/([^'"]+\.mjs)['"]/g))visit(m[1]);
}
control('engine-reference-import-graph-excludes-frontend',()=>{visit('engine-facade.mjs');visit('invocation-publication.mjs');visit('semantic-settings.mjs');visit('read-search-composition.mjs');visit('native-discovery.mjs');visit('discovery-arguments.mjs');visit('discovery-results.mjs');visit('analysis-work.mjs');visit('analysis-query-descriptor.mjs');visit('source-load-request.mjs');visit('ordinary-save-request.mjs');visit('native-operation-observation.mjs');visit('observation-admission.mjs');visit('observation-stop-facade.mjs');});
let frontendTraces=0,integrationFrames=0;
control('frontend-only-chord-steps-preserve-live-engine',()=>{
  const context={authority:'a',engine:'e',recipient:'r',purpose:'edit',destination:'local',profile:'p',continuity:'c'};
  const gates=Object.fromEntries(['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady','bookkeepingReserved','uniqueDomainAttested'].map(k=>[k,true]));
  const root={authority:'a',publication:0,documents:[{document:{authority:'a',id:'d',incarnation:1},live:true,source:[97],revision:1,annotationSet:1,records:[]}],groups:[],frontiers:{},views:[]};
  const domain=openInvocationDomain(root,context,{maxSlots:2,maxIssued:4,maxIntentBytes:4096,maxResultBytes:4096,maxRootBytes:16384,maxHeldBytes:131072},gates);assert(domain);
  const before=invocationAuthorityRoot(domain),usage=invocationUsage(domain);
  const profile={reference:'keys',keys:['g','x','y'],maxChordLength:3,maxEntries:4,maxDefaults:0,maxMaps:1,maxCells:32};
  const commands=[{reference:'move',family:'F12',variant:'logical-movement',version:1}];
  const input={reference:'input',generation:1,maps:['map'],profile:'keys',target:{document:root.documents[0].document,revision:1},configurationTarget:null};
  const words=[[]];for(let i=0;i<4;i++)for(const w of words.filter(w=>w.length===i))for(const k of profile.keys)words.push([...w,k]);
  for(const word of words){
    const table=compileKeymap([{reference:'map',profile:'keys',rank:0,generation:1,entries:[{sequence:['g','y'],action:{kind:'bind',command:'move'}}],defaults:[]}],profile,commands);assert(table);
    let pending=null;for(const key of word)pending=stepChord(table,input,pending,{kind:'key',key,chord:'c',context:input}).pending;
    assert.equal(invocationAuthorityRoot(domain),before);assert(equal(invocationUsage(domain),usage));frontendTraces++;
  }
  const facts=Object.fromEntries(flags.map(k=>[k,true]));
  for(const row of integrationInventory())for(const variant of row.variants)for(const route of ['typed','dynamic']){
    const request={...(route==='typed'?{family:row.family}:{name:row.name}),variant,version:1};
    assert.equal((route==='typed'?routeIntegrationTyped:routeIntegrationDynamic)(request,facts).status,'routed');
    assert.equal(invocationAuthorityRoot(domain),before);assert(equal(invocationUsage(domain),usage));integrationFrames++;
  }
});
return {status:'engine-ui-reference-boundary-only',engineVariants,presentationVariants,integrationVariants,routingCases,integrationRoutingCases,controls,staticModules:seen.size,frontendTraces,integrationFrames,
  ownershipCatalogTotalAndDisjointChecked:true,integrationExecutionComplete:false,clipboardWorkflowKernelSeparationProved:false,
  allNativeGatewaysSeparated:false,argumentProductsComplete:false,physicalOrHostIsolationProved:false,implementationRefinement:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkEngineUIBoundary(),null,2));
