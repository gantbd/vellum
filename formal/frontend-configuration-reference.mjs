// Earlier combined frontend settings/keymap fixture, not an engine publication API.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {resolvePreference} from './baseline-contracts.mjs';
import {mergeDependencies} from './annotation-operations.mjs';
import {prepareKeymapPart,keymapDependencies,keymapChanged,publishKeymapPart} from './frontend-keymaps.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./frontend-configuration-reference.json',import.meta.url),'utf8'));
const copy=structuredClone;
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const bytes=v=>Buffer.byteLength(JSON.stringify(v),'utf8');
const put=(object,key,value)=>Object.defineProperty(object,key,{value,writable:true,enumerable:true,configurable:true});
const canonical=v=>JSON.stringify(Array.isArray(v)?v.map(x=>JSON.parse(canonical(x))):
  v&&typeof v==='object'?Object.fromEntries(Object.keys(v).sort().map(k=>[k,JSON.parse(canonical(v[k]))])):v);
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const plans=new WeakMap();
const dependency=d=>exact(d,catalog.dependencyFields)&&typeof d.domain==='string'&&typeof d.identity==='string'&&Number.isSafeInteger(d.generation)&&d.generation>0;
const normalizedDependencies=ds=>[...new Map(ds.map(d=>[canonical([d.domain,d.identity]),d])).values()].sort((a,b)=>canonical(a).localeCompare(canonical(b)));
function typed(type,value,partial=false){
  if(type.kind==='boolean')return typeof value==='boolean';
  if(type.kind==='integer')return Number.isSafeInteger(value)&&value>=type.min&&value<=type.max;
  if(type.kind==='text')return typeof value==='string'&&[...value].every(c=>{const n=c.codePointAt(0);return n<0xd800||n>0xdfff;});
  if(type.kind==='enum')return type.values.some(v=>equal(v,value));
  if(type.kind==='list')return Array.isArray(value)&&value.every(v=>typed(type.element,v));
  return type.kind==='map'&&value&&typeof value==='object'&&!Array.isArray(value)&&
    Object.keys(value).every(k=>Object.hasOwn(type.fields,k)&&typed(type.fields[k],value[k]))&&
    (partial||Object.keys(type.fields).every(k=>Object.hasOwn(value,k)));
}
function replace(rows){
  const values=[...new Set(rows.map(r=>canonical(r.value)))].sort();
  for(const rank of new Set(rows.map(r=>r.rank)))
    if(resolvePreference(rows.filter(r=>r.rank===rank).map(r=>({rank,value:values.indexOf(canonical(r.value))})),values.map((_,i)=>i)).kind==='conflict')return {kind:'conflict'};
  const result=resolvePreference(rows.map(r=>({rank:r.rank,value:values.indexOf(canonical(r.value))})),values.map((_,i)=>i));
  return result.kind==='value'?{kind:'value',value:JSON.parse(values[result.value])}:{kind:result.kind};
}
export function resolveSetting(schema,layers,restriction){
  if(!catalog.domains.includes(schema.domain)||!typed(schema.type,schema.default)||!['replace','merge-map'].includes(schema.composition)||
      (schema.composition==='merge-map'&&schema.type.kind!=='map'))return null;
  const rows=[{rank:-1,value:copy(schema.default),origin:'library'}];
  for(const layer of layers){
    if(!Object.hasOwn(layer.values,schema.key))continue;
    if(!schema.scopes.includes(layer.kind)||!catalog.scopeOrder.includes(layer.kind))return null;
    const entry=layer.values[schema.key];
    if(!exact(entry,entry.kind==='reset'?['kind']:['kind','value'])||!['set','reset'].includes(entry.kind))return null;
    const value=entry.kind==='reset'?schema.default:entry.value;
    if(!typed(schema.type,value,schema.composition==='merge-map'))return null;
    rows.push({rank:catalog.scopeOrder.indexOf(layer.kind),value:copy(value),origin:layer.reference});
  }
  let value,origins;
  if(schema.composition==='merge-map'){
    value={};origins={};
    for(const key of Object.keys(schema.type.fields)){
      const contributors=rows.filter(r=>Object.hasOwn(r.value,key)),result=replace(contributors.map(r=>({...r,value:r.value[key]})));
      if(result.kind!=='value')return null;put(value,key,result.value);
      const rank=Math.max(...contributors.map(r=>r.rank));put(origins,key,contributors.filter(r=>r.rank===rank).map(r=>r.origin).sort());
    }
  }else{
    const result=replace(rows);if(result.kind!=='value')return null;value=result.value;
    const rank=Math.max(...rows.map(r=>r.rank));origins=rows.filter(r=>r.rank===rank).map(r=>r.origin).sort();
  }
  if(!typed(schema.type,value)||(restriction&&!restriction.some(v=>equal(v,value))))return null;
  return {value,origins};
}
export function resolveConfiguration(state){
  const resolved={};
  if(new Set(state.schemas.map(s=>s.key)).size!==state.schemas.length||new Set(state.layers.map(l=>l.reference)).size!==state.layers.length)return null;
  for(const layer of state.layers)if(Object.keys(layer.values).some(key=>!state.schemas.some(s=>s.key===key)))return null;
  for(const target of state.targets){
    if(new Set(target.layers).size!==target.layers.length||target.layers.some(ref=>!state.layers.some(l=>l.reference===ref)))return null;
    put(resolved,target.reference,{});
    for(const schema of state.schemas){
      const out=resolveSetting(schema,state.layers.filter(l=>target.layers.includes(l.reference)),Object.hasOwn(state.restrictions,schema.key)?state.restrictions[schema.key]:null);
      if(!out)return null;put(resolved[target.reference],schema.key,out);
    }
  }
  return resolved;
}
const gates=['authorized','bounded','notStopped','interpretationCurrent','companionsReady','retentionReady','invocationFresh'];
function activationDependencies(state,changes){
  const written=new Set(changes.map(c=>c.layer));
  const targets=state.targets.filter(t=>t.layers.some(ref=>written.has(ref)));
  const needed=new Set([...written,...targets.flatMap(t=>t.layers)]);
  return {schemaGeneration:state.schemaGeneration,restrictionGeneration:state.restrictionGeneration,membershipGeneration:state.membershipGeneration,
    schemas:copy(state.schemas),restrictions:copy(state.restrictions),targets:copy(targets).sort((a,b)=>a.reference.localeCompare(b.reference)),
    layers:copy(state.layers.filter(l=>needed.has(l.reference))).sort((a,b)=>a.reference.localeCompare(b.reference))};
}
export function prepareSettings(state,request,host){
  if(!exact(request,catalog.activationFields)||request.contract!==catalog.activationContract||request.operation!=='settings-keymap'||request.profile!==host.profile||
      typeof request.invocation!=='string'||!request.invocation||!Array.isArray(request.changes)||!Array.isArray(request.keymaps)||
      request.changes.length>host.maxChanges||bytes(request)>host.maxPlanBytes||bytes(state)>host.maxStateBytes||!gates.every(k=>host[k]))return null;
  if(!Array.isArray(request.dependencies)||request.dependencies.length>host.maxDependencies||!request.dependencies.every(dependency)||
      !mergeDependencies(host.requiredDependencies,request.dependencies,state.contexts,host.maxDependencies))return null;
  const n=copy(state),seen=new Set();
  for(const change of request.changes){
    if(!exact(change,catalog.changeFields)||!['set','remove','reset'].includes(change.operation)||(change.operation!=='set'&&change.value!==null))return null;
    const key=JSON.stringify([change.layer,change.key]);if(seen.has(key))return null;seen.add(key);
    const layer=n.layers.find(l=>l.reference===change.layer),schema=n.schemas.find(s=>s.key===change.key);
    if(!layer||!schema||change.expectedGeneration!==layer.generation||!host.writableLayers.includes(layer.reference)||!schema.scopes.includes(layer.kind))return null;
    if(change.operation==='remove')delete layer.values[change.key];
    else put(layer.values,change.key,change.operation==='reset'?{kind:'reset'}:{kind:'set',value:copy(change.value)});
  }
  const resolved=resolveConfiguration(n),keymapPart=prepareKeymapPart(state,request.keymaps,host);if(!resolved||!keymapPart)return null;
  const plan=freeze({request:copy(request),dependencies:activationDependencies(state,request.changes),candidate:n,resolved,keymapPart,recipient:host.recipient,requiredDependencies:copy(normalizedDependencies(host.requiredDependencies))});
  if(bytes(plan)>host.maxPlanBytes)return null;plans.set(plan,host.owner);return plan;
}
export function activateSettings(state,plan,host){
  const refuse=()=>({status:'not-published',state});
  if(!plan||!plans.has(plan)||plans.get(plan)!==host.owner||plan.recipient!==host.recipient||plan.request.profile!==host.profile||
      !gates.every(k=>host[k])||!plan.request.changes.every(c=>host.writableLayers.includes(c.layer))||
      !equal(activationDependencies(state,plan.request.changes),plan.dependencies)||bytes(state)>host.maxStateBytes||bytes(plan)>host.maxPlanBytes||plan.request.changes.length>host.maxChanges)return refuse();
  if(!equal(plan.requiredDependencies,normalizedDependencies(host.requiredDependencies))||
      !mergeDependencies(host.requiredDependencies,plan.request.dependencies,state.contexts,host.maxDependencies))return refuse();
  if(!equal(keymapDependencies(state,plan.request.keymaps),plan.keymapPart.dependencies)||
      plan.request.keymaps.some(i=>!host.writableKeymaps?.includes(i.map))||
      (plan.request.keymaps.length&&(!host.keymapLimits||plan.request.keymaps.length>host.keymapLimits.maxMaps||
        plan.request.keymaps.some(i=>i.changes.length+i.defaults.length>host.keymapLimits.maxChanges)||bytes(plan.keymapPart)>host.keymapLimits.maxBytes)))return refuse();
  const written=new Set(plan.request.changes.map(c=>c.layer));
  const changed=state.layers.filter(l=>written.has(l.reference)&&!equal(l.values,plan.candidate.layers.find(x=>x.reference===l.reference).values));
  if(!changed.length&&!keymapChanged(state,plan.keymapPart))return host.completeIdentity?{status:'unchanged',state}:refuse();
  if(!host.versionsAvailable)return refuse();
  const requiredOld=host.heldCurrent?1:0;
  if(state.retained.length+requiredOld>host.maxRetainedContexts)return refuse();
  const n=copy(state);n.publication++;
  for(const layer of n.layers)if(changed.some(l=>l.reference===layer.reference)){
    layer.values=copy(plan.candidate.layers.find(l=>l.reference===layer.reference).values);layer.generation++;
  }
  if(host.heldCurrent)n.retained.push(copy(state.current));
  n.current={...copy(state.current),generation:state.current.generation+1,resolved:copy(state.current.resolved),domains:copy(state.current.domains)};
  for(const target of plan.dependencies.targets)put(n.current.resolved,target.reference,copy(plan.resolved[target.reference]));
  for(const target of plan.dependencies.targets)for(const domain of catalog.domains){
    const keys=n.schemas.filter(s=>s.domain===domain).map(s=>s.key);
    if(keys.some(k=>!equal(state.current.resolved[target.reference][k].value,n.current.resolved[target.reference][k].value)))n.current.domains[target.reference][domain]++;
  }
  publishKeymapPart(n,state,plan.keymapPart);
  return {status:'applied',state:n};
}
const projectOrigins=(value,refs)=>Array.isArray(value)?value.flatMap(internal=>{
  const found=refs.find(r=>r.internal===internal);return found?[found.reference]:[];
}):Object.fromEntries(Object.entries(value).map(([k,v])=>[k,projectOrigins(v,refs)]));
export function captureConfiguration(state,binding,visibility,limits){
  if(state.schemas.length>limits.maxKeys||state.layers.length>limits.maxLayers||state.targets.length>limits.maxTargets||bytes(state)>limits.maxInputBytes)return null;
  const target=Object.hasOwn(state.current.resolved,binding.target)?state.current.resolved[binding.target]:null;if(!target)return null;
  const rows=Object.fromEntries(state.schemas.filter(s=>visibility.keys.includes(s.key)).map(s=>[s.key,{
    ...(visibility.values?{value:copy(target[s.key].value)}:{}),...(visibility.origins?{origins:projectOrigins(target[s.key].origins,visibility.originRefs)}:{})}]));
  const capture={binding:copy(binding),rows,capabilities:visibility.capabilities?copy(visibility.capabilityFacts):null};
  return bytes(capture)<=limits.maxCaptureBytes?freeze(capture):null;
}
export function readConfiguration(capture,request,current){
  const unavailable=()=>({status:'unavailable'});
  if(!capture||!request||!Object.hasOwn(catalog.queries,request.operation)||!exact(request,[...catalog.queryCommon,...catalog.queries[request.operation]])||
      request.contract!==catalog.queryContract||!equal(current.binding,capture.binding)||request.context!==capture.binding.context||
      request.target!==capture.binding.target||request.profile!==capture.binding.profile||
      !['granted','retained','live','continuous','transferReady','configurationCurrent'].every(k=>current[k]))return unavailable();
  let out;
  if(request.operation==='capabilities'){
    if(capture.capabilities===null)return unavailable();out={status:'capabilities',context:capture.binding.context,capabilities:copy(capture.capabilities)};
  }else{
    const row=Object.hasOwn(capture.rows,request.key)?capture.rows[request.key]:null,field=request.operation==='effective-configuration'?'value':'origins';
    if(!row||!Object.hasOwn(row,field))return unavailable();out={status:field,context:capture.binding.context,key:request.key,
      ...(field==='origins'?{coverage:'permitted-only'}:{}),[field]:copy(row[field])};
  }
  return bytes(out)<=current.maxReplyBytes?out:unavailable();
}

export function checkConfigurationProducts(){
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  assert.deepEqual(Object.keys(catalog.queries).sort(),families.find(f=>f.id==='F07').variants.toSorted());
  assert(families.find(f=>f.id==='F16').variants.includes('semantic-settings'));
  const bool={kind:'boolean'},integer={kind:'integer',min:0,max:8},list={kind:'list',element:{kind:'text'}},map={kind:'map',fields:{left:bool,right:integer}};
  const schemas=[{key:'scalar',type:integer,default:1,domain:'source',scopes:['host','workspace','document'],composition:'replace'},
    {key:'list',type:list,default:['base'],domain:'analysis',scopes:['host','workspace','document'],composition:'replace'},
    {key:'map',type:map,default:{left:false,right:1},domain:'view',scopes:['host','workspace','document','view'],composition:'merge-map'}];
  const state={schemas,restrictions:{scalar:[0,1,2,3,4,5,6,7,8]},schemaGeneration:1,restrictionGeneration:1,membershipGeneration:1,publication:0,
    layers:[{reference:'h',kind:'host',generation:1,values:{}},{reference:'w1',kind:'workspace',generation:1,values:{}},{reference:'w2',kind:'workspace',generation:1,values:{}},{reference:'d',kind:'document',generation:1,values:{}}],
    targets:[{reference:'document',layers:['h','w1','w2','d']}],retained:[],authored:{source:[97],annotations:['comment'],history:['G'],effects:['unresolved-write']},
    contexts:[{domain:'interpretation',identity:'configuration',generation:1},{domain:'read',identity:'source-context',generation:1}]};
  state.current={generation:1,resolved:resolveConfiguration(state),domains:{document:{source:1,view:1,analysis:1,host:1}}};
  const host={owner:{},recipient:'agent',profile:'p',...Object.fromEntries(gates.map(k=>[k,true])),completeIdentity:true,versionsAvailable:true,
    maxChanges:8,maxPlanBytes:32768,maxStateBytes:32768,maxDependencies:4,requiredDependencies:[state.contexts[0]],writableLayers:['w1','w2','d'],heldCurrent:true,maxRetainedContexts:2};
  const change=(key,value,layer='d',operation='set',expectedGeneration=1)=>({layer,expectedGeneration,key,operation,value});
  const request=changes=>({contract:catalog.activationContract,operation:'settings-keymap',invocation:'activate-1',profile:'p',changes,keymaps:[],dependencies:[]});
  const r=request([change('scalar',2),change('list',['new']),change('map',{left:true})]);
  const controls=[],control=(name,condition)=>{assert(condition,name);controls.push(name);};
  const plan=prepareSettings(state,r,host),applied=activateSettings(state,plan,host);assert.equal(applied.status,'applied');
  control('one-coherent-settings-publication',applied.state.publication===1&&applied.state.current.resolved.document.scalar.value===2&&equal(applied.state.current.resolved.document.list.value,['new'])&&equal(applied.state.current.resolved.document.map.value,{left:true,right:1}));
  control('retained-interpretation-unchanged',equal(applied.state.retained[0],state.current));
  control('configuration-does-not-edit-source-history-or-effects',equal(applied.state.authored,state.authored));
  const sourceEdited=copy(state);sourceEdited.authored.source=[98];
  const afterSource=activateSettings(sourceEdited,plan,host);
  control('unrelated-authored-edit-is-not-configuration-conflict',afterSource.status==='applied'&&equal(afterSource.state.authored,sourceEdited.authored));
  const independent=copy(state);independent.layers.push({reference:'other-layer',kind:'document',generation:1,values:{}});
  independent.targets.push({reference:'other-target',layers:['h','other-layer']});independent.current.resolved=resolveConfiguration(independent);
  independent.current.domains['other-target']={source:1,view:1,analysis:1,host:1};
  const independentHost={...host,writableLayers:[...host.writableLayers,'other-layer']},originalPlan=prepareSettings(independent,r,independentHost);
  const otherActivated=activateSettings(independent,prepareSettings(independent,request([change('scalar',3,'other-layer')]),independentHost),independentHost);
  const bothActivated=activateSettings(otherActivated.state,originalPlan,independentHost);
  control('independent-configuration-scope-does-not-stale-or-roll-back',bothActivated.status==='applied'&&bothActivated.state.current.resolved['other-target'].scalar.value===3&&bothActivated.state.current.resolved.document.scalar.value===2);
  let gateCases=0,resolutionCases=0,queryCases=0,pairedCases=0;
  for(let mask=0;mask<128;mask++){
    const h={...host,...Object.fromEntries(gates.map((k,i)=>[k,!!(mask&(1<<i))]))},before=copy(state),out=activateSettings(state,plan,h);
    assert.equal(out.status,mask===127?'applied':'not-published');if(mask!==127)assert.deepEqual(out.state,state);assert.deepEqual(state,before);gateCases++;
  }
  for(let a=0;a<=2;a++)for(let b=0;b<=2;b++)for(const reverse of [false,true]){
    const rows=[{reference:'a',kind:'workspace',values:{scalar:{kind:'set',value:a}}},{reference:'b',kind:'workspace',values:{scalar:{kind:'set',value:b}}}];
    const out=resolveSetting(schemas[0],reverse?[...rows].reverse():rows,null);assert.equal(out===null,a!==b);if(a===b)assert.equal(out.value,a);resolutionCases++;
  }
  const mapRows=[{reference:'a',kind:'workspace',values:{map:{kind:'set',value:{left:true}}}},{reference:'b',kind:'workspace',values:{map:{kind:'set',value:{right:2}}}}];
  control('disjoint-map-keys-merge-at-equal-rank',equal(resolveSetting(schemas[2],mapRows,null).value,{left:true,right:2}));
  control('same-map-key-conflict-not-open-order',resolveSetting(schemas[2],[{...mapRows[0],values:{map:{kind:'set',value:{right:3}}}},mapRows[1]],null)===null);
  control('higher-override-does-not-hide-equal-rank-conflict',resolveSetting(schemas[0],[{reference:'a',kind:'workspace',values:{scalar:{kind:'set',value:2}}},
    {reference:'b',kind:'workspace',values:{scalar:{kind:'set',value:3}}},{reference:'d',kind:'document',values:{scalar:{kind:'set',value:4}}}],null)===null);
  const nestedSchema={...schemas[1],type:{kind:'list',element:map},default:[]};
  control('nested-map-order-not-list-value-inequality',resolveSetting(nestedSchema,[{reference:'a',kind:'workspace',values:{list:{kind:'set',value:[{left:true,right:1}]}}},
    {reference:'b',kind:'workspace',values:{list:{kind:'set',value:[{right:1,left:true}]}}}],null)!==null);
  const reject=(name,req=r,s=state,h=host)=>control(name,prepareSettings(s,req,h)===null);
  reject('unknown-key',request([change('unknown',1)]));reject('wrong-type',request([change('scalar','2')]));
  reject('integer-domain',request([change('scalar',9)]));reject('unknown-map-key',request([change('map',{unknown:true})]));
  reject('illegal-scope',request([change('scalar',2,'h')]),state,{...host,writableLayers:['d']});
  reject('same-rank-conflict',request([change('scalar',2,'w1'),change('scalar',3,'w2')]));
  reject('host-restriction-not-overridden',r,{...state,restrictions:{scalar:[1]}});
  reject('duplicate-intent',request([change('scalar',2),change('scalar',3)]));
  reject('stale-caller-layer-generation',request([change('scalar',2,'d','set',2)]));
  reject('caller-dependency-conflict',{...r,dependencies:[{...state.contexts[0],generation:2}]});
  reject('caller-read-dependency-missing',{...r,dependencies:[{domain:'read',identity:'unknown',generation:1}]});
  reject('mandatory-dependency-not-omittable',r,{...state,contexts:[state.contexts[1]]});
  reject('unsupported-keymap-not-partial',{...r,keymaps:[{kind:'bind'}]});
  reject('unknown-variant',{...r,operation:'execute'});reject('unknown-field',{...r,callback:'run'});
  reject('request-bound',r,state,{...host,maxPlanBytes:1});
  const refuse=(name,p=plan,s=state,h=host)=>control(name,equal(activateSettings(s,p,h),{status:'not-published',state:s}));
  refuse('copied-plan-not-owned',copy(plan));refuse('different-owner',plan,state,{...host,owner:{}});
  refuse('different-recipient',plan,state,{...host,recipient:'other'});
  refuse('current-layer-authority-revoked',plan,state,{...host,writableLayers:[]});
  refuse('mandatory-dependency-scope-expands',plan,state,{...host,requiredDependencies:state.contexts});
  const readPlan=prepareSettings(state,{...r,dependencies:[state.contexts[1]]},host),changedRead=copy(state);changedRead.contexts[1].generation=2;
  refuse('declared-read-dependency-stale',readPlan,changedRead);
  for(const key of ['schemaGeneration','restrictionGeneration','membershipGeneration'])refuse(`stale-${key}`,plan,{...copy(state),[key]:2});
  refuse('superseded-activation',plan,applied.state);refuse('held-retention-exhaustion',plan,state,{...host,maxRetainedContexts:0});
  refuse('version-exhaustion',plan,state,{...host,versionsAvailable:false});
  const noop=prepareSettings(state,request([]),host);
  control('complete-identity-needs-no-unused-version',equal(activateSettings(state,noop,{...host,versionsAvailable:false,maxRetainedContexts:0}),{status:'unchanged',state}));
  const cosmetic=activateSettings(state,prepareSettings(state,request([change('map',{right:2})]),host),host);
  control('cosmetic-change-does-not-change-source-domain',cosmetic.state.current.domains.document.source===1&&cosmetic.state.current.domains.document.view===2);
  const inherited=copy(applied.state);inherited.layers[0].values.scalar={kind:'set',value:3};inherited.current.resolved=resolveConfiguration(inherited);
  const removed=activateSettings(inherited,prepareSettings(inherited,request([change('scalar',null,'d','remove',2)]),host),host);
  control('removal-reveals-inherited-value',removed.state.current.resolved.document.scalar.value===3);
  const reset=activateSettings(inherited,prepareSettings(inherited,request([change('scalar',null,'d','reset',2)]),host),host);
  control('reset-is-explicit-library-default',reset.state.current.resolved.document.scalar.value===1&&equal(reset.state.current.resolved.document.scalar.origins,['d']));
  const sameValue=activateSettings(state,prepareSettings(state,request([change('scalar',1)]),host),host);
  control('equal-value-explicit-origin-is-not-noop',sameValue.status==='applied'&&sameValue.state.current.domains.document.source===1);
  const protoState=copy(state);protoState.schemas[0].key='__proto__';protoState.restrictions=Object.fromEntries([['__proto__',[1,2]]]);protoState.current.resolved=resolveConfiguration(protoState);
  const protoResult=activateSettings(protoState,prepareSettings(protoState,request([change('__proto__',2)]),host),host);
  control('registered-key-is-not-object-prototype',protoResult.status==='applied'&&Object.hasOwn(protoResult.state.current.resolved.document,'__proto__')&&protoResult.state.current.resolved.document.__proto__.value===2);
  const binding={context:'configuration-context',target:'document',profile:'p',recipient:'reader',purpose:'inspect',destination:'view',policy:'epoch1',continuity:'c1'};
  const visible={keys:['scalar','list','map'],values:true,origins:true,originRefs:[{internal:'library',reference:'default-origin'},{internal:'d',reference:'document-origin'}],capabilities:true,capabilityFacts:['supported-profile']};
  const limits={maxKeys:8,maxLayers:8,maxTargets:4,maxInputBytes:32768,maxCaptureBytes:8192};
  const capture=captureConfiguration(applied.state,binding,visible,limits),current={binding:copy(binding),granted:true,retained:true,live:true,continuous:true,transferReady:true,configurationCurrent:true,maxReplyBytes:4096};
  const q=operation=>({contract:catalog.queryContract,operation,context:binding.context,target:'document',profile:'p',...(operation==='capabilities'?{}:{key:'scalar'})});
  for(const operation of Object.keys(catalog.queries))for(let mask=0;mask<64;mask++){
    const fields=['granted','retained','live','continuous','transferReady','configurationCurrent'],c={...copy(current),...Object.fromEntries(fields.map((k,i)=>[k,!!(mask&(1<<i))]))};
    const before=copy(capture),out=readConfiguration(capture,q(operation),c);assert.equal(out.status,mask===63?({'effective-configuration':'value','permitted-origin':'origins',capabilities:'capabilities'})[operation]:'unavailable');assert.deepEqual(capture,before);queryCases++;
  }
  const unavailable={status:'unavailable'};
  const narrow=captureConfiguration(applied.state,binding,{...visible,values:false,origins:false,capabilities:false},limits);
  control('withheld-values-origins-capabilities-unavailable',Object.keys(catalog.queries).every(op=>equal(readConfiguration(narrow,q(op),current),unavailable)));
  control('unknown-key-not-default',equal(readConfiguration(capture,{...q('effective-configuration'),key:'unknown'},current),unavailable));
  control('stale-inspection-not-current',equal(readConfiguration(capture,q('effective-configuration'),{...current,configurationCurrent:false}),unavailable));
  control('query-transfer-bound',equal(readConfiguration(capture,q('effective-configuration'),{...current,maxReplyBytes:1}),unavailable));
  control('origin-is-scoped-not-raw-layer',equal(readConfiguration(capture,q('permitted-origin'),current).origins,['document-origin']));
  const hiddenOrigins=captureConfiguration(applied.state,binding,{...visible,originRefs:[]},limits);
  control('hidden-origin-not-complete-absence',equal(readConfiguration(hiddenOrigins,q('permitted-origin'),current),{status:'origins',context:binding.context,key:'scalar',coverage:'permitted-only',origins:[]}));
  control('query-capture-bound',captureConfiguration(applied.state,binding,visible,{...limits,maxCaptureBytes:1})===null);
  for(const k of Object.keys(binding))control(`current-query-${k}`,equal(readConfiguration(capture,q('effective-configuration'),{...current,binding:{...binding,[k]:'other'}}),unavailable));
  control('query-unknown-field',equal(readConfiguration(capture,{...q('effective-configuration'),callback:'run'},current),unavailable));
  control('query-unknown-variant',equal(readConfiguration(capture,{...q('effective-configuration'),operation:'execute'},current),unavailable));
  control('unregistered-prototype-key-unavailable',equal(readConfiguration(capture,{...q('effective-configuration'),key:'__proto__'},current),unavailable));
  const hiddenProjection={...visible,originRefs:[]},baselineHidden=captureConfiguration(applied.state,binding,hiddenProjection,limits);
  for(let secret=0;secret<8;secret++){
    const hidden=copy(applied.state);hidden.layers.at(-1).reference=`private-${secret}`;
    hidden.targets[0].layers=hidden.targets[0].layers.map(ref=>ref==='d'?`private-${secret}`:ref);hidden.current.resolved=resolveConfiguration(hidden);
    const other=captureConfiguration(hidden,binding,hiddenProjection,limits);assert.deepEqual(other,baselineHidden);
    for(const operation of Object.keys(catalog.queries)){assert.deepEqual(readConfiguration(other,q(operation),current),readConfiguration(baselineHidden,q(operation),current));pairedCases++;}
  }
  const saved=copy(capture);applied.state.current.resolved.document.scalar.value=8;
  control('capture-owns-value-origin-snapshot',equal(capture,saved));
  const output=readConfiguration(capture,q('permitted-origin'),current);output.origins.push('mutated');
  control('query-result-does-not-alias',equal(capture,saved));
  return {status:'configuration-products-only',gateCases,resolutionCases,queryCases,pairedCases,controls,keymapRegistryProviderRefinementProved:false,physicalAccountingProved:false,implementationRefinement:false};
}
export async function checkConfigurationMutations(){
  const source=fs.readFileSync(new URL('./frontend-configuration-reference.mjs',import.meta.url),'utf8');
  const mutants=[
    ['higher-override-hides-peer-conflict',"if(resolvePreference(rows.filter(r=>r.rank===rank).map(r=>({rank,value:values.indexOf(canonical(r.value))})),values.map((_,i)=>i)).kind==='conflict')return {kind:'conflict'};","if(false)return {kind:'conflict'};"],
    ['ignore-host-restriction',"(restriction&&!restriction.some(v=>equal(v,value)))",'false'],
    ['ignore-held-retention','state.retained.length+requiredOld>host.maxRetainedContexts','false'],
    ['ignore-current-layer-authority','!plan.request.changes.every(c=>host.writableLayers.includes(c.layer))','false'],
    ['cosmetic-invalidates-source',"if(keys.some(k=>!equal(state.current.resolved[target.reference][k].value,n.current.resolved[target.reference][k].value)))",'if(true)'],
    ['copied-activation-plan-is-owned','!plans.has(plan)||plans.get(plan)!==host.owner','false'],
    ['expose-raw-origins','projectOrigins(target[s.key].origins,visibility.originRefs)','copy(target[s.key].origins)'],
    ['ignore-caller-generation','change.expectedGeneration!==layer.generation','false']
  ];
  const killed=[];
  for(const [name,from,to] of mutants){
    assert(source.includes(from),name);let code=source.replace(from,to);
    for(const file of ['baseline-contracts.mjs','annotation-operations.mjs','frontend-keymaps.mjs'])code=code.replace(`'./${file}'`,JSON.stringify(new URL(`./${file}`,import.meta.url).href));
    for(const file of ['frontend-configuration-reference.json','operation-families.json'])code=code.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(`./${file}`,import.meta.url).href)})`);
    const module=await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`);
    assert.throws(()=>module.checkConfigurationProducts(),{code:'ERR_ASSERTION'},name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkConfigurationProducts(),mutations:await checkConfigurationMutations()},null,2));
