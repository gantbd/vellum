// Engine-only typed configuration participants in the shared native coordinator.
// Normalized bounded data, not a hostile decoder or a general JSON Schema interpreter.
import {copySettingsData,normalizeSemanticSettings,semanticSettingsInputForm} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {mergeDependencies} from './annotation-operations.mjs';
const pools=new WeakMap(),plans=new WeakMap(),writeBases=new WeakMap(),definitionBases=new WeakMap(),lifecycleBases=new WeakMap();
const layerInput=Object.freeze({family:'F16',contract:'engine-settings-layers/1',operation:'semantic-settings',variant:'semantic-settings',fields:Object.freeze(['contract','operation','invocation','profile','expected','create','remove','targets','dependencies'])});
const definitionInput=Object.freeze({family:'F16',contract:'engine-settings-definition/1',operation:'supported-registry',variant:'supported-registry',fields:Object.freeze(['contract','operation','invocation','profile','expected','definition','dependencies'])});
const scopedInput=Object.freeze({family:'F16',contract:'scoped-semantic-settings/1',operation:'semantic-settings',variant:'semantic-settings',fields:Object.freeze(['contract','operation','invocation','profile','changes','dependencies'])});
const scopedDefinitionInput=Object.freeze({family:'F16',contract:'scoped-engine-settings-definition/1',operation:'supported-registry',variant:'supported-registry',fields:Object.freeze(['contract','operation','invocation','profile','definition','dependencies'])});
const scopedLifecycleInput=Object.freeze({family:'F16',contract:'scoped-engine-settings-layers/1',operation:'semantic-settings',variant:'semantic-settings',fields:Object.freeze(['contract','operation','invocation','profile','create','remove','targets','dependencies'])});
const basisLifecycleInput=Object.freeze({...scopedLifecycleInput,contract:'basis-engine-settings-layers/1'});
export function engineSettingsInputForms(){return Object.freeze([semanticSettingsInputForm(),layerInput,definitionInput,scopedInput,scopedDefinitionInput,scopedLifecycleInput,basisLifecycleInput]);}
const domains=Object.freeze(['source','serialization','analysis']),scopes=Object.freeze(['host','language','workspace','document','view']),compositions=Object.freeze(['replace','merge-map']);
const typeForms=Object.freeze(Object.fromEntries(Object.entries({boolean:['kind'],text:['kind'],integer:['kind','min','max'],enum:['kind','values'],list:['kind','element'],map:['kind','fields']}).map(([kind,fields])=>[kind,Object.freeze(fields)])));
const typeDeclarations=Object.freeze({forms:typeForms,domains,scopes,compositions});
export function engineSettingsTypeDeclarations(){return typeDeclarations;}
const profileFields=['argumentLimits','maxStateBytes','maxPlanBytes','maxKeys','maxLayers','maxTargets','maxSchemaNodes','maxSchemaDepth','maxContexts','maxIssuedContexts','maxContextBytes','maxHeldBytes'];
const argumentFields=['maxChanges','maxDependencies','maxRequestBytes','maxValueBytes','maxIdentifierBytes','maxInputNodes','maxInputDepth'];
const cfgFields=['schemaGeneration','restrictionGeneration','associationGeneration','generation','layerIncarnation','schemas','restrictions','layers','targets','resolved','domains'];
const natural=n=>Number.isSafeInteger(n)&&n>=0,version=n=>natural(n)&&n>0;
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&Object.keys(v).length===keys.length&&keys.every(k=>Object.hasOwn(v,k));
const object=v=>v&&typeof v==='object'&&!Array.isArray(v);
const put=(o,k,v)=>Object.defineProperty(o,k,{value:v,enumerable:true,writable:true,configurable:true});
const key=r=>JSON.stringify([r.authority,r.id,r.incarnation]);
const label=(s,p)=>typeof s==='string'&&s.length>0&&Buffer.byteLength(s)<=p.argumentLimits.maxIdentifierBytes;
const identity=(r,a,p)=>exact(r,['authority','id','incarnation'])&&r.authority===a&&label(r.authority,p)&&label(r.id,p)&&version(r.incarnation);
const allowed=(g,ks)=>ks.every(k=>g?.[k]===true);
const gates=['authorized','scope','continuous','live','notStopped','interpretationCurrent','companionsReady','settingsWritable','resourcesReady'];
const limits=p=>({...p.argumentLimits,maxRequestBytes:p.maxStateBytes});
function profileValid(p){return exact(p,profileFields)&&exact(p.argumentLimits,argumentFields)&&Object.values(p.argumentLimits).every(natural)&&profileFields.filter(k=>k!=='argumentLimits').every(k=>natural(p[k]));}
function owned(value,p,maximum=p.maxStateBytes){return copySettingsData(value,{...limits(p),maxRequestBytes:maximum})?.value??null;}
function bytes(value,p,maximum){return copySettingsData(value,{...limits(p),maxRequestBytes:maximum})?.bytes??null;}
// JSON instance equality: mathematical numbers, ordered lists, unordered properties.
// Inputs have already passed the bounded, cycle-free normalized-data copier.
function same(a,b,exactNumbers=false){
 const work=[[a,b]];
 while(work.length){const [x,y]=work.pop();if(exactNumbers?Object.is(x,y):x===y)continue;if(x===null||y===null||typeof x!==typeof y||typeof x!=='object')return false;
  if(Array.isArray(x)!==Array.isArray(y))return false;const ks=Object.keys(x);if(ks.length!==Object.keys(y).length)return false;
  for(const k of ks){if(!Object.hasOwn(y,k))return false;work.push([x[k],y[k]]);}
 }
 return true;
}
export function equalEngineSettingValues(a,b,p){if(!profileValid(p))return null;const pair=owned([a,b],p);return pair?same(...pair):null;}
function typeValid(type,p){
 const todo=[{type,depth:0}];let n=0;
 while(todo.length){const {type:t,depth}=todo.pop();if(++n>p.maxSchemaNodes||depth>p.maxSchemaDepth||!object(t))return false;
  if(['boolean','text'].includes(t.kind)){if(!exact(t,typeForms[t.kind]))return false;}
  else if(t.kind==='integer'){if(!exact(t,typeForms.integer)||!Number.isSafeInteger(t.min)||!Number.isSafeInteger(t.max)||t.min>t.max)return false;}
  else if(t.kind==='enum'){if(!exact(t,typeForms.enum)||!Array.isArray(t.values)||!t.values.length||t.values.some((v,i)=>t.values.slice(0,i).some(w=>same(v,w))))return false;}
  else if(t.kind==='list'){if(!exact(t,typeForms.list))return false;todo.push({type:t.element,depth:depth+1});}
  else if(t.kind==='map'){if(!exact(t,typeForms.map)||!object(t.fields)||Object.keys(t.fields).some(k=>!label(k,p)))return false;for(const t2 of Object.values(t.fields))todo.push({type:t2,depth:depth+1});}
  else return false;
 }
 return true;
}
function typed(type,value,p,partial=false){
 const todo=[{type,value,partial}];let n=0;
 while(todo.length){const {type:t,value:v,partial:part}=todo.pop();if(++n>p.argumentLimits.maxInputNodes)return false;
  if(t.kind==='boolean'){if(typeof v!=='boolean')return false;}
  else if(t.kind==='integer'){if(!Number.isSafeInteger(v)||v<t.min||v>t.max)return false;}
  else if(t.kind==='text'){if(typeof v!=='string')return false;}
  else if(t.kind==='enum'){if(!t.values.some(x=>same(x,v)))return false;}
  else if(t.kind==='list'){if(!Array.isArray(v))return false;for(const x of v)todo.push({type:t.element,value:x,partial:false});}
  else if(t.kind==='map'){
   if(!object(v)||Object.keys(v).some(k=>!Object.hasOwn(t.fields,k))||!part&&Object.keys(t.fields).some(k=>!Object.hasOwn(v,k)))return false;
   for(const k of Object.keys(v))todo.push({type:t.fields[k],value:v[k],partial:false});
  }else return false;
 }
 return true;
}
function resolveRows(rows){
 for(const rank of new Set(rows.map(r=>r.rank))){const peers=rows.filter(r=>r.rank===rank);if(peers.some(r=>!same(r.value,peers[0].value)))return null;}
 const rank=rows.reduce((n,r)=>Math.max(n,r.rank),-1),top=rows.filter(r=>r.rank===rank);
 return {value:top[0].value,origins:top.map(r=>r.origin).sort()};
}
function resolveSetting(schema,layers,restriction,p){
 const rows=[{rank:-1,value:schema.default,origin:'library'}];
 for(const layer of layers){if(!Object.hasOwn(layer.values,schema.key))continue;if(!schema.scopes.includes(layer.kind))return null;const e=layer.values[schema.key];
  if(!exact(e,e?.kind==='reset'?['kind']:['kind','value'])||!['set','reset'].includes(e.kind))return null;
  const value=e.kind==='reset'?schema.default:e.value;if(!typed(schema.type,value,p,schema.composition==='merge-map'))return null;
  rows.push({rank:scopes.indexOf(layer.kind),value,origin:key(layer.layer)});
 }
 let result;
 if(schema.composition==='replace')result=resolveRows(rows);
 else{const value={},origins={};for(const field of Object.keys(schema.type.fields)){const r=resolveRows(rows.filter(r=>Object.hasOwn(r.value,field)).map(r=>({...r,value:r.value[field]})));if(!r)return null;put(value,field,r.value);put(origins,field,r.origins);}result={value,origins};}
 return result&&typed(schema.type,result.value,p)&&(!restriction||restriction.some(v=>same(v,result.value)))?result:null;
}
function resolve(cfg,p){
 const out={};for(const target of cfg.targets){const values={},layers=target.layers.map(ref=>cfg.layers.find(l=>same(l.layer,ref)));if(layers.some(l=>!l))return null;
  for(const schema of cfg.schemas){const r=resolveSetting(schema,layers,Object.hasOwn(cfg.restrictions,schema.key)?cfg.restrictions[schema.key]:null,p);if(!r)return null;put(values,schema.key,r);}
  put(out,key(target.target),values);
 }
 return out;
}
function definitionValid(cfg,authority,p){
 if(!profileValid(p)||!exact(cfg,cfgFields)||!['schemaGeneration','restrictionGeneration','associationGeneration','generation'].every(k=>version(cfg[k]))||!Array.isArray(cfg.schemas)||cfg.schemas.length>p.maxKeys||!object(cfg.restrictions)||!Array.isArray(cfg.layers)||cfg.layers.length>p.maxLayers||!Array.isArray(cfg.targets)||cfg.targets.length>p.maxTargets)return false;
 const names=new Set();for(const s of cfg.schemas){
  if(!exact(s,['key','type','default','domain','scopes','composition','sensitivity'])||typeof s.sensitivity!=='boolean'||!label(s.key,p)||names.has(s.key)||!domains.includes(s.domain)||!Array.isArray(s.scopes)||!s.scopes.length||new Set(s.scopes).size!==s.scopes.length||s.scopes.some(k=>!scopes.includes(k))||!typeValid(s.type,p)||!typed(s.type,s.default,p)||!compositions.includes(s.composition)||s.composition==='merge-map'&&s.type.kind!=='map')return false;names.add(s.key);
 }
 for(const [k,vs]of Object.entries(cfg.restrictions)){const s=cfg.schemas.find(s=>s.key===k);if(!s||!Array.isArray(vs)||!vs.length||vs.some(v=>!typed(s.type,v,p)))return false;}
 if(!natural(cfg.layerIncarnation))return false;
 const ids=new Set();for(const l of cfg.layers){if(!exact(l,['layer','kind','generation','values'])||!identity(l.layer,authority,p)||l.layer.incarnation>cfg.layerIncarnation||ids.has(key(l.layer))||!scopes.includes(l.kind)||!version(l.generation)||!object(l.values)||Object.keys(l.values).some(k=>!names.has(k)))return false;ids.add(key(l.layer));
  // Check contributions even when no current target uses this layer.
  for(const s of cfg.schemas)if(Object.hasOwn(l.values,s.key)&&!resolveSetting(s,[l],null,p))return false;
 }
 const targets=new Set();for(const t of cfg.targets){if(!exact(t,['target','layers'])||!identity(t.target,authority,p)||targets.has(key(t.target))||!Array.isArray(t.layers)||t.layers.some(r=>!identity(r,authority,p)||!ids.has(key(r)))||new Set(t.layers.map(key)).size!==t.layers.length)return false;targets.add(key(t.target));}
 if(!object(cfg.domains)||!same(Object.keys(cfg.domains).sort(),[...targets].sort()))return false;
 return Object.values(cfg.domains).every(v=>exact(v,domains)&&Object.values(v).every(version));
}
function checked(cfg,authority,p){
 if(!profileValid(p))return null;const c=owned(cfg,p);if(!c||!definitionValid(c,authority,p))return null;const r=resolve(c,p);return r&&same(r,c.resolved)?c:null;
}
// Pure bounded coherence validator. It neither establishes ownership nor grants access.
export function validatedEngineSettingsState(root,p){return root&&typeof root.authority==='string'?checked(root.engineSettings,root.authority,p):null;}
// Trusted composition only: validates actual owner state, not recipient disclosure.
export function nativeEngineSettingsBasis(owner,p){return owner&&!owner.faulted&&owner.root?.authority===owner.context?.authority?validatedEngineSettingsState(owner.root,p):null;}
// Initial accepted declaration data; this constructor neither installs a native
// root nor grants real workspace association, consumer behavior or permission.
export function compileEngineSettings(definition,authority,p){
 if(!profileValid(p)||!exact(definition,['schemas','restrictions','layers','targets']))return null;const d=owned(definition,p);if(!d||!Array.isArray(d.targets)||d.targets.some(t=>!exact(t,['target','layers'])||!identity(t.target,authority,p)))return null;
 if(!Array.isArray(d.layers)||d.layers.some(l=>!version(l?.layer?.incarnation)))return null;
 const cfg={...d,schemaGeneration:1,restrictionGeneration:1,associationGeneration:1,generation:1,layerIncarnation:d.layers.reduce((n,l)=>Math.max(n,l.layer.incarnation),0),resolved:{},domains:{}};
 for(const t of d.targets)put(cfg.domains,key(t.target),Object.fromEntries(domains.map(k=>[k,1])));
 if(!definitionValid(cfg,authority,p))return null;const r=resolve(cfg,p);return r?owned({...cfg,resolved:r},p):null;
}
function dependencies(cfg,changes){
 const written=new Set(changes.map(c=>key(c.layer))),targets=cfg.targets.filter(t=>t.layers.some(l=>written.has(key(l)))),needed=new Set([...written,...targets.flatMap(t=>t.layers.map(key))]);
 return {associationGeneration:cfg.associationGeneration,schemaGeneration:cfg.schemaGeneration,restrictionGeneration:cfg.restrictionGeneration,schemas:cfg.schemas,restrictions:cfg.restrictions,
  targets:[...targets].sort((a,b)=>key(a.target).localeCompare(key(b.target))),layers:cfg.layers.filter(l=>needed.has(key(l.layer))).sort((a,b)=>key(a.layer).localeCompare(key(b.layer)))};
}
function extraDependencies(request,h,p){
 if(!Array.isArray(h?.requiredDependencies)||!Array.isArray(h.contexts)||h.requiredDependencies.length>p.argumentLimits.maxDependencies||h.contexts.length>p.argumentLimits.maxInputNodes)return null;
 const data=owned({required:h.requiredDependencies,current:h.contexts},p,p.maxPlanBytes);
 if(!data||[...data.required,...data.current].some(d=>!exact(d,['domain','identity','generation'])||!label(d.domain,p)||!label(d.identity,p)||!version(d.generation)))return null;
 const ds=[...data.required,...request.dependencies],ks=ds.map(d=>JSON.stringify([d.domain,d.identity]));
 if(!mergeDependencies(data.required,request.dependencies,data.current,p.argumentLimits.maxDependencies))return null;
 return [...new Map(ds.map((d,i)=>[ks[i],d])).values()].sort((a,b)=>JSON.stringify(a).localeCompare(JSON.stringify(b)));
}
function pool(owner,domain,p){
 const old=pools.get(owner);if(old)return old.domain===domain&&!old.closed&&same(old.profile,p)?old:null;
 const profileBytes=bytes(p,p,p.maxContextBytes),charge=profileBytes===null?null:profileBytes+p.maxContexts*p.maxContextBytes;
 if(charge===null||!natural(charge)||charge>p.maxHeldBytes||!natural(owner.charge)||charge>owner.limits.maxHeldBytes-owner.charge)return null;
 const handle=Object.freeze({kind:'native-engine-settings-pool/1'}),m={domain,handle,profile:owned(p,p,p.maxContextBytes),charge,leases:new Map(),plans:new Set(),issued:0,closed:false};owner.charge+=charge;pools.set(owner,m);return m;
}
// First-party witness; the facade still returns contexts/plans, never this pool.
export function nativeEngineSettingsPoolHandle(owner,domain){const m=pools.get(owner);return m?.domain===domain?m.handle:null;}
function writable(request,h,p){return allowed(h,gates)&&Array.isArray(h.settingsWritableLayers)&&h.settingsWritableLayers.length<=p.maxLayers&&request.changes.every(c=>h.settingsWritableLayers.some(l=>same(l,c.layer)));}
function decide(before,request,authority,p,h){
 const cfg=checked(before.engineSettings,authority,p);if(!cfg||!writable(request,h,p))return null;
 const layers=cfg.layers.map(l=>({...l,values:{...l.values}})),changed=[];
 for(const c of request.changes){const layer=layers.find(l=>same(l.layer,c.layer)),schema=cfg.schemas.find(s=>s.key===c.key);
  if(!layer||!schema||layer.generation!==c.expectedGeneration||!schema.scopes.includes(layer.kind))return null;
  if(c.operation==='remove')delete layer.values[c.key];else put(layer.values,c.key,c.operation==='reset'?{kind:'reset'}:{kind:'set',value:c.value});
 }
 for(let i=0;i<layers.length;i++)if(!same(layers[i].values,cfg.layers[i].values))changed.push(layers[i]);
 const r=resolve({...cfg,layers},p);if(!r)return null;
 if(!changed.length)return {state:before,cfg,changed:false};
 if(h.versionsAvailable!==true||!natural(before.publication)||before.publication===Number.MAX_SAFE_INTEGER||cfg.generation===Number.MAX_SAFE_INTEGER||changed.some(l=>l.generation===Number.MAX_SAFE_INTEGER))return null;
 for(const l of changed)l.generation++;
 const generations={};for(const target of cfg.targets){const k=key(target.target),d={...cfg.domains[k]};for(const domain of domains)if(cfg.schemas.some(s=>s.domain===domain&&!same(cfg.resolved[k][s.key].value,r[k][s.key].value))){if(d[domain]===Number.MAX_SAFE_INTEGER)return null;d[domain]++;}put(generations,k,d);}
 const next=owned({...cfg,generation:cfg.generation+1,layers,resolved:r,domains:generations},p);return next?{state:{...before,publication:before.publication+1,engineSettings:next},cfg,changed:true}:null;
}
export function prepareNativeEngineSettings(owner,domain,ticket,reference,before,envelope,c,p,h,existing=null){
 return prepareSettings(owner,domain,ticket,reference,before,envelope,c,p,h,existing,null);
}
function prepareSettings(owner,domain,ticket,reference,before,envelope,c,p,h,existing,scoped){
 if(!profileValid(p)||c.authority!==before.authority||owner.context.authority!==c.authority||owner.context.engine!==c.engine||!same(h.semanticSettingsContext,c)||h.semanticSettingsBasis!==before||!same(h.semanticSettingsProfile,p)||!allowed(h,['outsidePublicationGuard','preparationReserved']))return null;
 const n=normalizeLayerLifecycle(envelope,c,p)??normalizeDefinition(envelope,c,p)??normalizeSemanticSettings(envelope,c,p.argumentLimits);if(!n||n.arguments.invocation!==reference)return null;
 if(existing){const prior=plans.get(existing);return prior&&prior.owner===owner&&prior.domain===domain&&prior.ticket===ticket&&Boolean(prior.scoped)===Boolean(scoped)&&(!scoped||same(prior.value.scoped.request,scoped.request,true)&&prior.scoped.bases.length===scoped.bases.length&&prior.scoped.bases.every((b,i)=>b===scoped.bases[i]))&&same(prior.value.request,n.arguments,true)&&same(prior.value.context,n.context,true)&&same(prior.value.profile,p,true)?existing:null;}
 const d=chooseDecision(before,n.arguments,c.authority,p,h),extra=extraDependencies(n.arguments,h,p);if(!d||!extra||Buffer.byteLength(JSON.stringify(d.state))>owner.limits.maxRootBytes)return null;
 const value=owned({request:n.arguments,context:n.context,dependencies:preparationDependencies(d.cfg,n.arguments),extra,required:h.requiredDependencies,profile:p,...(scoped?{scoped:{request:scoped.request,basisCount:scoped.bases.length}}:{})},p,Math.min(p.maxPlanBytes,owner.limits.maxIntentBytes));if(!value)return null;
 if(bytes({kind:'settings',plan:value},p,owner.limits.maxIntentBytes)===null)return null;
 const m=pool(owner,domain,p);if(!m)return null;const token=Object.freeze({kind:'native-engine-settings-plan/1'});plans.set(token,{owner,domain,ticket,value,...(scoped?{scoped:{bases:[...scoped.bases]}}:{})});m.plans.add(token);return token;
}
export function nativeEngineSettingsIntent(plan,maximum,owner,domain,ticket){const p=plans.get(plan);return p&&p.owner===owner&&p.domain===domain&&p.ticket===ticket&&natural(maximum)&&bytes(p.value,p.value.profile,maximum)!==null?{invocation:p.value.request.invocation,value:p.value}:null;}
export function publishNativeEngineSettings(before,plan,h,owner,domain,ticket){
 const no=()=>({status:'not-published',state:before}),p=plans.get(plan),m=pools.get(owner);if(!p||p.owner!==owner||p.domain!==domain||p.ticket!==ticket||!m||m.closed)return no();
 const v=p.value,profile=v.profile;if(!same(m.profile,profile)||!same(h.semanticSettingsProfile,profile)||!same(h.semanticSettingsContext,v.context)||h.semanticSettingsBasis!==before||!same(v.required,h.requiredDependencies))return no();
 if(p.scoped){const current=lowerScopedSettings(owner,domain,p.scoped.bases,v.scoped.request,v.context,profile,h);if(!current||!same(current.arguments,v.request,true))return no();}
 const cfg=checked(before.engineSettings,owner.context.authority,profile);if(!cfg||!same(preparationDependencies(cfg,v.request),v.dependencies)||!same(extraDependencies(v.request,h,profile),v.extra))return no();
 const d=chooseDecision(before,v.request,owner.context.authority,profile,h);return d?{status:d.changed?'applied':'unchanged',state:d.state}:no();
}
export function forgetNativeEngineSettings(plan,owner,domain,ticket){const p=plans.get(plan);if(!p||p.owner!==owner||p.domain!==domain||p.ticket!==ticket)return false;pools.get(owner)?.plans.delete(plan);return plans.delete(plan);}
export function acquireNativeEngineSettingsContext(owner,domain,c,p,g){
 if(owner.faulted||!profileValid(p)||!same(owner.context,c)||!allowed(g,['authorized','scope','continuous','live','notStopped','ownerControlAllowed','settingsReadable','outsidePublicationGuard','resourcesReady']))return null;
 const cfg=checked(owner.root.engineSettings,c.authority,p);if(!cfg||p.maxContexts===0||p.maxIssuedContexts===0)return null;const capture=owned({context:c,configuration:cfg},p,p.maxContextBytes);if(!capture)return null;const m=pool(owner,domain,p);if(!m||m.leases.size>=p.maxContexts||m.issued>=p.maxIssuedContexts)return null;
 const token=Object.freeze({kind:'native-engine-settings-context/1'});m.leases.set(token,capture);m.issued++;return token;
}
export function readNativeEngineSettingsContext(owner,domain,token,c,g){const m=pools.get(owner),v=m?.leases.get(token);return m&&!m.closed&&m.domain===domain&&!owner.faulted&&v&&!v.writeBasis&&same(v.context,c)&&allowed(g,['authorized','scope','continuous','live','notStopped','ownerControlAllowed','settingsReadable'])?v.configuration:null;}
export function releaseNativeEngineSettingsContext(owner,domain,token,c,g){const m=pools.get(owner),v=m?.leases.get(token);return !!(m&&!m.closed&&m.domain===domain&&v&&!v.writeBasis&&same(v.context,c)&&allowed(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])&&m.leases.delete(token));}
export function retireNativeEngineSettingsPool(owner,domain,g){const m=pools.get(owner);if(!m||m.closed||m.domain!==domain||m.leases.size||m.plans.size||!allowed(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;m.closed=true;m.profile=null;owner.charge-=m.charge;return true;}
export function nativeEngineSettingsUsage(owner){const m=pools.get(owner);return m?{contexts:m.leases.size,issued:m.issued,plans:m.plans.size,closed:m.closed,declaredHeldBytes:m.closed?0:m.charge}:null;}

function normalizeLayerLifecycle(envelope,context,p){
 const captured=copySettingsData(envelope,p.argumentLimits),bound=copySettingsData(context,p.argumentLimits);if(!captured||!bound)return null;
 const e=captured.value,c=bound.value;
 if(!exact(c,['authority','engine','recipient','purpose','destination','profile','continuity'])||!Object.values(c).every(v=>label(v,p))||!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route)||!exact(e.request,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:r,...tuple}=e.request,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);
 if(decoded?.descriptor.id!==layerInput.family||decoded.variant!==layerInput.variant||!exact(r,layerInput.fields)||r.contract!==layerInput.contract||r.operation!==layerInput.operation||!label(r.invocation,p)||r.profile!==c.profile||!exact(r.expected,['generation','layerIncarnation'])||!version(r.expected.generation)||!natural(r.expected.layerIncarnation))return null;
 if(!Array.isArray(r.create)||!Array.isArray(r.remove)||r.create.length+r.remove.length>p.argumentLimits.maxChanges||!Array.isArray(r.targets)||r.targets.length>p.maxTargets||!Array.isArray(r.dependencies)||r.dependencies.length>p.argumentLimits.maxDependencies)return null;
 const ids=new Set();let valueBytes=0;for(const l of r.create){if(!exact(l,['layer','kind','values'])||!identity(l.layer,c.authority,p)||!scopes.includes(l.kind)||!object(l.values)||ids.has(key(l.layer)))return null;ids.add(key(l.layer));const size=bytes(l.values,p,p.argumentLimits.maxValueBytes-valueBytes);if(size===null)return null;valueBytes+=size;}
 for(const l of r.remove){if(!exact(l,['layer','expectedGeneration'])||!identity(l.layer,c.authority,p)||!version(l.expectedGeneration)||ids.has(key(l.layer)))return null;ids.add(key(l.layer));}
 const seen=new Set();for(const d of r.dependencies){if(!exact(d,['domain','identity','generation'])||!label(d.domain,p)||!label(d.identity,p)||!version(d.generation))return null;const k=JSON.stringify([d.domain,d.identity]);if(seen.has(k))return null;seen.add(k);}
 return {arguments:r,context:c};
}
function layerLifecycleDecision(before,r,authority,p,h){
 if(!allowed(h,gates))return null;const cfg=checked(before.engineSettings,authority,p);if(!cfg||r.expected.generation!==cfg.generation||r.expected.layerIncarnation!==cfg.layerIncarnation)return null;
 const changed=r.create.length>0||r.remove.length>0||!same(r.targets,cfg.targets);
 if(!changed)return {state:before,cfg,changed:false};
 const layerChange=r.create.length>0||r.remove.length>0,associationChange=!same(r.targets,cfg.targets);
 if(layerChange&&(h.layerRegistryWritable!==true||!Array.isArray(h.settingsWritableLayers)||h.settingsWritableLayers.length>p.argumentLimits.maxInputNodes||[...r.create,...r.remove].some(l=>!h.settingsWritableLayers.some(ref=>same(ref,l.layer))))||associationChange&&h.associationRegistryWritable!==true)return null;
 if(h.versionsAvailable!==true||!natural(before.publication)||before.publication===Number.MAX_SAFE_INTEGER||cfg.generation===Number.MAX_SAFE_INTEGER||associationChange&&cfg.associationGeneration===Number.MAX_SAFE_INTEGER||r.create.length>Number.MAX_SAFE_INTEGER-cfg.layerIncarnation)return null;
 for(let i=0;i<r.create.length;i++)if(r.create[i].layer.incarnation!==cfg.layerIncarnation+i+1||cfg.layers.some(l=>same(l.layer,r.create[i].layer)))return null;
 for(const old of r.remove)if(!cfg.layers.some(l=>same(l.layer,old.layer)&&l.generation===old.expectedGeneration))return null;
 const layers=[...cfg.layers.filter(l=>!r.remove.some(x=>same(x.layer,l.layer))),...r.create.map(l=>({...l,generation:1}))];
 const next={...cfg,layers,targets:r.targets,generation:cfg.generation+1,associationGeneration:cfg.associationGeneration+Number(associationChange),layerIncarnation:cfg.layerIncarnation+r.create.length,domains:{}};
 for(const t of r.targets){if(!identity(t?.target,authority,p))return null;put(next.domains,key(t.target),Object.fromEntries(domains.map(d=>[d,1])));}
 if(!definitionValid(next,authority,p))return null;const resolved=resolve(next,p);if(!resolved)return null;
 for(const t of r.targets){const k=key(t.target);if(!Object.hasOwn(cfg.domains,k))continue;const counters={...cfg.domains[k]};
  for(const d of domains)if(cfg.schemas.some(s=>s.domain===d&&!same(cfg.resolved[k][s.key].value,resolved[k][s.key].value))){if(counters[d]===Number.MAX_SAFE_INTEGER)return null;counters[d]++;}
  put(next.domains,k,counters);
 }
 next.resolved=resolved;const value=owned(next,p);if(!value||!checked(value,authority,p))return null;
 return {state:{...before,publication:before.publication+1,engineSettings:value},cfg,changed:true};
}
function definitionCandidate(before, proposal, expected, p, versionsAvailable) {
 if(!profileValid(p)||!exact(proposal,['schemas','restrictions','targets'])||!exact(expected,['schemaGeneration','restrictionGeneration','associationGeneration']))return null;
 const cfg=checked(before?.engineSettings,before?.authority,p),input=owned(proposal,p);
 if(!cfg||!input||Object.keys(expected).some(k=>expected[k]!==cfg[k]))return null;
 const changes={schemaGeneration:!same(input.schemas,cfg.schemas),restrictionGeneration:!same(input.restrictions,cfg.restrictions),associationGeneration:!same(input.targets,cfg.targets)};
 if(!Object.values(changes).some(Boolean))return {status:'unchanged',state:before};
 if(versionsAvailable!==true||!natural(before.publication)||before.publication===Number.MAX_SAFE_INTEGER||cfg.generation===Number.MAX_SAFE_INTEGER||Object.entries(changes).some(([k,changed])=>changed&&cfg[k]===Number.MAX_SAFE_INTEGER))return null;
 const next={...cfg,...input,generation:cfg.generation+1,domains:{}};
 for(const [k,changed]of Object.entries(changes))if(changed)next[k]++;
 if(!Array.isArray(next.targets))return null;
 for(const t of next.targets){if(!identity(t?.target,before.authority,p))return null;put(next.domains,key(t.target),Object.fromEntries(domains.map(d=>[d,1])));}
 if(!definitionValid(next,before.authority,p))return null;
 const r=resolve(next,p);if(!r)return null;
 const values=(schemas,resolved,domain)=>Object.fromEntries(schemas.filter(s=>s.domain===domain).map(s=>[s.key,resolved[s.key].value]));
 for(const t of next.targets){const k=key(t.target);if(!Object.hasOwn(cfg.domains,k))continue;
  const versions={...cfg.domains[k]};
  for(const d of domains)if(!same(values(cfg.schemas,cfg.resolved[k],d),values(next.schemas,r[k],d))){if(versions[d]===Number.MAX_SAFE_INTEGER)return null;versions[d]++;}
  put(next.domains,k,versions);
 }
 next.resolved=r;const value=owned(next,p);if(!value||!checked(value,before.authority,p))return null;
 return {status:'candidate',state:{...before,publication:before.publication+1,engineSettings:value}};
}
function normalizeDefinition(envelope,context,p){
 const captured=copySettingsData(envelope,p.argumentLimits),bound=copySettingsData(context,p.argumentLimits);
 if(!captured||!bound)return null;const e=captured.value,c=bound.value;
 const fields=['authority','engine','recipient','purpose','destination','profile','continuity'];
 if(!exact(c,fields)||!Object.values(c).every(v=>label(v,p))||!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const idKeys=e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments'];
 if(!exact(e.request,idKeys))return null;const {arguments:a,...tuple}=e.request;
 const decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);
 if(decoded?.descriptor.id!==definitionInput.family||decoded.variant!==definitionInput.variant||!exact(a,definitionInput.fields)||a.contract!==definitionInput.contract||a.operation!==definitionInput.operation||!label(a.invocation,p)||a.profile!==c.profile||!exact(a.expected,['schemaGeneration','restrictionGeneration','associationGeneration'])||!Object.values(a.expected).every(version)||!exact(a.definition,['schemas','restrictions','targets'])||!Array.isArray(a.dependencies)||a.dependencies.length>p.argumentLimits.maxDependencies)return null;
 const seen=new Set();for(const d of a.dependencies){if(!exact(d,['domain','identity','generation'])||!label(d.domain,p)||!label(d.identity,p)||!version(d.generation))return null;const k=JSON.stringify([d.domain,d.identity]);if(seen.has(k))return null;seen.add(k);}
 return {arguments:a,context:c};
}
function definitionDependencies(cfg){return {schemaGeneration:cfg.schemaGeneration,restrictionGeneration:cfg.restrictionGeneration,associationGeneration:cfg.associationGeneration,schemas:cfg.schemas,restrictions:cfg.restrictions,layers:cfg.layers,targets:cfg.targets,layerIncarnation:cfg.layerIncarnation};}
function preparationDependencies(cfg,r){return [definitionInput.contract,layerInput.contract].includes(r.contract)?definitionDependencies(cfg):dependencies(cfg,r.changes);}
function chooseDecision(before,request,authority,p,h){
 if(request.contract===layerInput.contract)return layerLifecycleDecision(before,request,authority,p,h);
 if(request.contract!==definitionInput.contract)return decide(before,request,authority,p,h);
 if(!allowed(h,gates))return null;
 const cfg=checked(before.engineSettings,authority,p);if(!cfg)return null;
 for(const [field,role]of [['schemas','definitionRegistryWritable'],['restrictions','restrictionRegistryWritable'],['targets','associationRegistryWritable']])if(!same(cfg[field],request.definition[field])&&h[role]!==true)return null;
 const d=definitionCandidate(before,request.definition,request.expected,p,h.versionsAvailable);
 return d?{state:d.state,cfg:before.engineSettings,changed:d.status==='candidate'}:null;
}

// Conditional ordinary-layer mutation bases. No recipient receives native epochs,
// whole configuration or an interpretation lease through these functions.
const basisGates=['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady','ownerControlAllowed','settingsWritable','settingsBasisAllowed'];
function basisLayerFacts(p,authority,g){const facts=owned(g?.settingsWritableLayers,p,p.argumentLimits.maxRequestBytes);return Array.isArray(facts)&&facts.length<=p.maxLayers&&facts.every(l=>identity(l,authority,p))?facts:null;}
function basisLayerAllowed(layer,p,g){const facts=basisLayerFacts(p,layer.authority,g);return facts!==null&&facts.some(l=>same(l,layer));}
function retainedWriteBasis(owner,domain,token,c){const m=pools.get(owner),v=m?.leases.get(token),s=writeBases.get(token);return m&&!m.closed&&m.domain===domain&&s?.owner===owner&&s.domain===domain&&v?.writeBasis&&same(v.context,c)&&owner.context.authority===c.authority&&owner.context.engine===c.engine?{m,v,s}:null;}
export function captureNativeEngineSettingsWriteBasis(owner,domain,layer,c,p,g){
 if(!owner||owner.faulted||!profileValid(p)||!same(g?.semanticSettingsContext,c)||!same(g?.semanticSettingsProfile,p)||!allowed(g,[...basisGates,'outsidePublicationGuard','preparationReserved']))return null;
 const context=owned(c,p,p.argumentLimits.maxRequestBytes);if(!context||!exact(context,['authority','engine','recipient','purpose','destination','profile','continuity'])||!Object.values(context).every(v=>label(v,p))||context.authority!==owner.context.authority||context.engine!==owner.context.engine)return null;
 const ref=owned(layer,p,p.argumentLimits.maxRequestBytes),cfg=nativeEngineSettingsBasis(owner,p);if(!ref||!identity(ref,context.authority,p)||!cfg||!basisLayerAllowed(ref,p,g)||!cfg.layers.some(l=>same(l.layer,ref)))return null;
 const capture=owned({context,configuration:cfg,writeBasis:{layer:ref,dependencies:dependencies(cfg,[{layer:ref}])}},p,p.maxContextBytes);if(!capture||p.maxContexts===0||p.maxIssuedContexts===0)return null;
 const m=pool(owner,domain,p);if(!m||m.leases.size>=p.maxContexts||m.issued>=p.maxIssuedContexts)return null;
 const token=Object.freeze({kind:'native-settings-write-basis/1'});m.leases.set(token,capture);m.issued++;writeBases.set(token,{owner,domain,stopped:false});return token;
}
function currentWriteBasis(owner,domain,token,c,g){
 const held=retainedWriteBasis(owner,domain,token,c);if(!held||held.s.stopped||owner.faulted)return null;
 const {m,v,s}=held;
 if(!same(g?.semanticSettingsProfile,m.profile)||!same(g?.semanticSettingsContext,c))return null;
 if(['authorized','scope','continuous','live','interpretationCurrent','settingsWritable','settingsBasisAllowed'].some(k=>g?.[k]===false)){s.stopped=true;return null;}
 if(!allowed(g,basisGates))return null;
 const facts=basisLayerFacts(m.profile,c.authority,g);if(facts===null)return null;
 if(!facts.some(l=>same(l,v.writeBasis.layer))){s.stopped=true;return null;}
 const cfg=nativeEngineSettingsBasis(owner,m.profile);if(!cfg)return null;
 if(!same(dependencies(cfg,[{layer:v.writeBasis.layer}]),v.writeBasis.dependencies)){s.stopped=true;return null;}
 return held;
}
export function inspectNativeEngineSettingsWriteBasis(owner,domain,token,c,g){if(g?.outsidePublicationGuard!==true)return {status:'unavailable'};return currentWriteBasis(owner,domain,token,c,g)?{status:'settings-write-basis',availability:'current'}:{status:'unavailable'};}
// Only a fixed first-party mutation adapter may receive the private condition.
export function resolveNativeEngineSettingsWriteBasis(owner,domain,token,c,g){if(g?.settingsHostBasisAllowed!==true)return null;const held=currentWriteBasis(owner,domain,token,c,g);if(!held)return null;const layer=held.v.configuration.layers.find(l=>same(l.layer,held.v.writeBasis.layer));return layer?owned({layer:layer.layer,expectedGeneration:layer.generation},held.m.profile,held.m.profile.maxContextBytes):null;}
export function stopNativeEngineSettingsWriteBasis(owner,domain,token,c,g){const held=retainedWriteBasis(owner,domain,token,c);if(!held||g?.ownerControlAllowed!==true)return false;held.s.stopped=true;return true;}
export function releaseNativeEngineSettingsWriteBasis(owner,domain,token,c,g){const held=retainedWriteBasis(owner,domain,token,c);if(!held||!allowed(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;held.m.leases.delete(token);writeBases.delete(token);return true;}

// Conditional declaration state is not a write grant.
const definitionBasisGates=['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady','ownerControlAllowed','settingsWritable','settingsDefinitionBasisAllowed'];
const definitionCondition=cfg=>Object.fromEntries(['schemaGeneration','restrictionGeneration','associationGeneration'].map(k=>[k,cfg[k]]));
function retainedDefinitionBasis(owner,domain,token,c){
 const m=pools.get(owner),v=m?.leases.get(token),s=definitionBases.get(token);
 return m&&!m.closed&&m.domain===domain&&s?.owner===owner&&s.domain===domain&&v?.writeBasis?.kind==='definition'&&same(v.context,c)&&owner.context.authority===c.authority&&owner.context.engine===c.engine?{m,v,s}:null;
}
export function captureNativeEngineSettingsDefinitionBasis(owner,domain,c,p,g){
 if(!owner||owner.faulted||!profileValid(p)||!same(g?.semanticSettingsContext,c)||!same(g?.semanticSettingsProfile,p)||!allowed(g,[...definitionBasisGates,'outsidePublicationGuard','preparationReserved']))return null;
 const context=owned(c,p,p.argumentLimits.maxRequestBytes);if(!context||!exact(context,['authority','engine','recipient','purpose','destination','profile','continuity'])||!Object.values(context).every(v=>label(v,p))||context.authority!==owner.context.authority||context.engine!==owner.context.engine)return null;
 const cfg=nativeEngineSettingsBasis(owner,p);if(!cfg||p.maxContexts===0||p.maxIssuedContexts===0)return null;
 const capture=owned({context,writeBasis:{kind:'definition',expected:definitionCondition(cfg)}},p,p.maxContextBytes);if(!capture)return null;
 const m=pool(owner,domain,p);if(!m||m.leases.size>=p.maxContexts||m.issued>=p.maxIssuedContexts)return null;
 const token=Object.freeze({kind:'native-settings-definition-basis/1'});m.leases.set(token,capture);m.issued++;definitionBases.set(token,{owner,domain,stopped:false});return token;
}
function currentDefinitionBasis(owner,domain,token,c,g){
 const held=retainedDefinitionBasis(owner,domain,token,c);if(!held||held.s.stopped||owner.faulted)return null;
 if(!same(g?.semanticSettingsProfile,held.m.profile)||!same(g?.semanticSettingsContext,c))return null;
 if(['authorized','scope','continuous','live','interpretationCurrent','settingsWritable','settingsDefinitionBasisAllowed'].some(k=>g?.[k]===false)){held.s.stopped=true;return null;}
 if(!allowed(g,definitionBasisGates))return null;
 const cfg=nativeEngineSettingsBasis(owner,held.m.profile);if(!cfg||!same(definitionCondition(cfg),held.v.writeBasis.expected)){held.s.stopped=true;return null;}
 return held;
}
export function inspectNativeEngineSettingsDefinitionBasis(owner,domain,token,c,g){return g?.outsidePublicationGuard===true&&currentDefinitionBasis(owner,domain,token,c,g)?{status:'settings-definition-basis',availability:'current'}:{status:'unavailable'};}
export function resolveNativeEngineSettingsDefinitionBasis(owner,domain,token,c,g){if(g?.settingsHostBasisAllowed!==true)return null;const held=currentDefinitionBasis(owner,domain,token,c,g);return held?owned(held.v.writeBasis.expected,held.m.profile,held.m.profile.maxContextBytes):null;}
export function stopNativeEngineSettingsDefinitionBasis(owner,domain,token,c,g){const held=retainedDefinitionBasis(owner,domain,token,c);if(!held||g?.ownerControlAllowed!==true)return false;held.s.stopped=true;return true;}
export function releaseNativeEngineSettingsDefinitionBasis(owner,domain,token,c,g){const held=retainedDefinitionBasis(owner,domain,token,c);if(!held||!allowed(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;held.m.leases.delete(token);definitionBases.delete(token);return true;}

const lifecycleBasisGates=['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady','ownerControlAllowed','settingsWritable','settingsLifecycleBasisAllowed'];
const lifecycleCondition=cfg=>Object.fromEntries(['generation','layerIncarnation'].map(k=>[k,cfg[k]]));
function retainedLifecycleBasis(owner,domain,token,c){
 const m=pools.get(owner),v=m?.leases.get(token),s=lifecycleBases.get(token);
 return m&&!m.closed&&m.domain===domain&&s?.owner===owner&&s.domain===domain&&v?.writeBasis?.kind==='lifecycle'&&same(v.context,c)&&owner.context.authority===c.authority&&owner.context.engine===c.engine?{m,v,s}:null;
}
export function captureNativeEngineSettingsLifecycleBasis(owner,domain,c,p,g){
 if(!owner||owner.faulted||!profileValid(p)||!same(g?.semanticSettingsContext,c)||!same(g?.semanticSettingsProfile,p)||!allowed(g,[...lifecycleBasisGates,'outsidePublicationGuard','preparationReserved']))return null;
 const context=owned(c,p,p.argumentLimits.maxRequestBytes);if(!context||!exact(context,['authority','engine','recipient','purpose','destination','profile','continuity'])||!Object.values(context).every(v=>label(v,p))||context.authority!==owner.context.authority||context.engine!==owner.context.engine)return null;
 const cfg=nativeEngineSettingsBasis(owner,p);if(!cfg||p.maxContexts===0||p.maxIssuedContexts===0)return null;
 const capture=owned({context,writeBasis:{kind:'lifecycle',expected:lifecycleCondition(cfg)}},p,p.maxContextBytes);if(!capture)return null;
 const m=pool(owner,domain,p);if(!m||m.leases.size>=p.maxContexts||m.issued>=p.maxIssuedContexts)return null;
 const token=Object.freeze({kind:'native-settings-lifecycle-basis/1'});m.leases.set(token,capture);m.issued++;lifecycleBases.set(token,{owner,domain,stopped:false});return token;
}
function currentLifecycleBasis(owner,domain,token,c,g){
 const held=retainedLifecycleBasis(owner,domain,token,c);if(!held||held.s.stopped||owner.faulted)return null;
 if(!same(g?.semanticSettingsProfile,held.m.profile)||!same(g?.semanticSettingsContext,c))return null;
 if(['authorized','scope','continuous','live','interpretationCurrent','settingsWritable','settingsLifecycleBasisAllowed'].some(k=>g?.[k]===false)){held.s.stopped=true;return null;}
 if(!allowed(g,lifecycleBasisGates))return null;
 const cfg=nativeEngineSettingsBasis(owner,held.m.profile);if(!cfg||!same(lifecycleCondition(cfg),held.v.writeBasis.expected)){held.s.stopped=true;return null;}
 return held;
}
export function inspectNativeEngineSettingsLifecycleBasis(owner,domain,token,c,g){return g?.outsidePublicationGuard===true&&currentLifecycleBasis(owner,domain,token,c,g)?{status:'settings-lifecycle-basis',availability:'current'}:{status:'unavailable'};}
export function resolveNativeEngineSettingsLifecycleBasis(owner,domain,token,c,g){if(g?.settingsHostBasisAllowed!==true)return null;const held=currentLifecycleBasis(owner,domain,token,c,g);return held?owned(held.v.writeBasis.expected,held.m.profile,held.m.profile.maxContextBytes):null;}
export function stopNativeEngineSettingsLifecycleBasis(owner,domain,token,c,g){const held=retainedLifecycleBasis(owner,domain,token,c);if(!held||g?.ownerControlAllowed!==true)return false;held.s.stopped=true;return true;}
export function releaseNativeEngineSettingsLifecycleBasis(owner,domain,token,c,g){const held=retainedLifecycleBasis(owner,domain,token,c);if(!held||!allowed(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;held.m.leases.delete(token);lifecycleBases.delete(token);return true;}

function normalizeScopedSettings(envelope,c,p){
 if(!profileValid(p))return null;const data=copySettingsData(envelope,p.argumentLimits),context=copySettingsData(c,p.argumentLimits);if(!data||!context)return null;
 const e=data.value,scope=context.value;if(!exact(scope,['authority','engine','recipient','purpose','destination','profile','continuity'])||!Object.values(scope).every(v=>label(v,p))||!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route)||!exact(e.request,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:r,...tuple}=e.request,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);
 if(decoded?.descriptor.id!==scopedInput.family||decoded.variant!==scopedInput.variant||!exact(r,scopedInput.fields)||r.contract!==scopedInput.contract||r.operation!==scopedInput.operation||!label(r.invocation,p)||r.profile!==scope.profile||!Array.isArray(r.changes)||r.changes.length>p.argumentLimits.maxChanges||!Array.isArray(r.dependencies)||r.dependencies.length>p.argumentLimits.maxDependencies)return null;
 if(r.changes.some(change=>!exact(change,['basis','key','operation','value'])||!natural(change.basis)))return null;
 return {request:r,context:scope};
}
function lowerScopedSettings(owner,domain,bases,r,c,p,h){
 if(r?.contract===basisLifecycleInput.contract)return lowerBasisLifecycle(owner,domain,bases,r,c,p,h);
 if(r?.contract===scopedDefinitionInput.contract)return lowerScopedDefinition(owner,domain,bases,r,c,p,h);
 if(r?.contract===scopedLifecycleInput.contract)return lowerScopedLifecycle(owner,domain,bases,r,c,p,h);
 if(h?.settingsHostBasisAllowed!==true||!same(h.semanticSettingsContext,c)||!same(h.semanticSettingsProfile,p)||!Array.isArray(bases)||bases.length>p.argumentLimits.maxChanges||new Set(bases).size!==bases.length)return null;
 const used=new Set(r.changes.map(change=>change.basis));if(used.size!==bases.length||[...used].some(i=>!natural(i)||i>=bases.length))return null;
 // Bind every reference and validate the complete proposal before consulting any
 // stateful loss observer. A malformed later reference cannot poison earlier ones.
 const conditions=[];for(const basis of bases){const held=retainedWriteBasis(owner,domain,basis,c);if(!held||held.s.stopped||!same(held.m.profile,p))return null;const layer=held.v.configuration.layers.find(l=>same(l.layer,held.v.writeBasis.layer));if(!layer)return null;conditions.push({layer:layer.layer,expectedGeneration:layer.generation});}
 const request={...r,contract:'semantic-settings-activation/1',changes:r.changes.map(({basis,...change})=>({...conditions[basis],...change}))};
 const normalized=normalizeSemanticSettings({route:'typed',request:{family:15,variant:'semantic-settings',version:1,arguments:request}},c,p.argumentLimits);
 if(!normalized||!decide(owner.root,normalized.arguments,c.authority,p,h))return null;
 for(let i=0;i<bases.length;i++){const current=resolveNativeEngineSettingsWriteBasis(owner,domain,bases[i],c,h);if(!current||!same(current,conditions[i]))return null;}
 return allowed(h,basisGates)?normalized:null;
}
export function prepareScopedNativeEngineSettings(owner,domain,ticket,reference,before,envelope,bases,c,p,h,existing=null){
 if(!allowed(h,['outsidePublicationGuard','preparationReserved']))return null;
 const input=normalizeScopedSettings(envelope,c,p);if(!input||input.request.invocation!==reference)return null;
 const lowered=lowerScopedSettings(owner,domain,bases,input.request,input.context,p,h);if(!lowered)return null;
 return prepareSettings(owner,domain,ticket,reference,before,{route:'typed',request:{family:15,variant:'semantic-settings',version:1,arguments:lowered.arguments}},input.context,p,h,existing,{request:input.request,bases});
}

function normalizeScopedDefinition(envelope,c,p){
 if(!profileValid(p))return null;const data=copySettingsData(envelope,p.argumentLimits),context=copySettingsData(c,p.argumentLimits);if(!data||!context)return null;
 const e=data.value,scope=context.value;
 if(!exact(scope,['authority','engine','recipient','purpose','destination','profile','continuity'])||!Object.values(scope).every(v=>label(v,p))||!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route)||!exact(e.request,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:r,...tuple}=e.request,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);
 if(decoded?.descriptor.id!==scopedDefinitionInput.family||decoded.variant!==scopedDefinitionInput.variant||!exact(r,scopedDefinitionInput.fields)||r.contract!==scopedDefinitionInput.contract||r.operation!==scopedDefinitionInput.operation||!label(r.invocation,p)||r.profile!==scope.profile)return null;
 return {request:r,context:scope};
}
function lowerScopedDefinition(owner,domain,bases,r,c,p,h){
 if(h?.settingsHostBasisAllowed!==true||!same(h.semanticSettingsContext,c)||!same(h.semanticSettingsProfile,p)||!Array.isArray(bases)||bases.length!==1)return null;
 const basis=bases[0],held=retainedDefinitionBasis(owner,domain,basis,c);if(!held||held.s.stopped||!same(held.m.profile,p))return null;
 const request={...r,contract:definitionInput.contract,expected:held.v.writeBasis.expected};
 const normalized=normalizeDefinition({route:'typed',request:{family:15,variant:'supported-registry',version:1,arguments:request}},c,p);
 // Full normalized semantic/authority preflight precedes any stateful loss observer.
 if(!normalized||!chooseDecision(owner.root,normalized.arguments,c.authority,p,h))return null;
 const current=resolveNativeEngineSettingsDefinitionBasis(owner,domain,basis,c,h);
 return current&&same(current,request.expected)?normalized:null;
}
export function prepareScopedNativeEngineSettingsDefinition(owner,domain,ticket,reference,before,envelope,basis,c,p,h,existing=null){
 if(!allowed(h,['outsidePublicationGuard','preparationReserved']))return null;
 const input=normalizeScopedDefinition(envelope,c,p);if(!input||input.request.invocation!==reference)return null;
 const bases=[basis],lowered=lowerScopedDefinition(owner,domain,bases,input.request,input.context,p,h);if(!lowered)return null;
 return prepareSettings(owner,domain,ticket,reference,before,{route:'typed',request:{family:15,variant:'supported-registry',version:1,arguments:lowered.arguments}},input.context,p,h,existing,{request:input.request,bases});
}

function normalizeScopedLifecycle(envelope,c,p){
 if(!profileValid(p))return null;const data=copySettingsData(envelope,p.argumentLimits),context=copySettingsData(c,p.argumentLimits);if(!data||!context)return null;
 const e=data.value,scope=context.value;
 if(!exact(scope,['authority','engine','recipient','purpose','destination','profile','continuity'])||!Object.values(scope).every(v=>label(v,p))||!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route)||!exact(e.request,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:r,...tuple}=e.request,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);
 if(decoded?.descriptor.id!==scopedLifecycleInput.family||decoded.variant!==scopedLifecycleInput.variant||!exact(r,scopedLifecycleInput.fields)||r.contract!==scopedLifecycleInput.contract||r.operation!==scopedLifecycleInput.operation||!label(r.invocation,p)||r.profile!==scope.profile||!Array.isArray(r.create)||!Array.isArray(r.remove)||r.create.length+r.remove.length>p.argumentLimits.maxChanges||!Array.isArray(r.targets)||r.targets.length>p.maxTargets||!Array.isArray(r.dependencies)||r.dependencies.length>p.argumentLimits.maxDependencies)return null;
 let used=0;for(const row of r.create){if(!exact(row,['id','kind','values'])||!label(row.id,p)||!scopes.includes(row.kind)||!object(row.values))return null;const n=bytes(row.values,p,p.argumentLimits.maxValueBytes-used);if(n===null)return null;used+=n;}
 if(r.remove.some(ref=>!identity(ref,scope.authority,p)))return null;
 for(const target of r.targets)if(!exact(target,['target','layers'])||!identity(target.target,scope.authority,p)||!Array.isArray(target.layers)||target.layers.some(ref=>!identity(ref,scope.authority,p)&&!(exact(ref,['create'])&&natural(ref.create)&&ref.create<r.create.length)))return null;
 return {request:r,context:scope};
}
function lowerScopedLifecycle(owner,domain,bases,r,c,p,h){
 if(h?.settingsHostBasisAllowed!==true||!same(h.semanticSettingsContext,c)||!same(h.semanticSettingsProfile,p)||!Array.isArray(bases)||bases.length!==1)return null;
 const basis=bases[0],held=retainedLifecycleBasis(owner,domain,basis,c);if(!held||held.s.stopped||!same(held.m.profile,p))return null;
 const expected=held.v.writeBasis.expected,cfg=nativeEngineSettingsBasis(owner,p);if(!cfg||!same(lifecycleCondition(cfg),expected)||r.create.length>Number.MAX_SAFE_INTEGER-expected.layerIncarnation)return null;
 const create=r.create.map((row,i)=>({layer:{authority:c.authority,id:row.id,incarnation:expected.layerIncarnation+i+1},kind:row.kind,values:row.values})),remove=[];
 for(const ref of r.remove){const old=cfg.layers.find(l=>same(l.layer,ref));if(!old)return null;remove.push({layer:ref,expectedGeneration:old.generation});}
 const targets=r.targets.map(t=>({target:t.target,layers:t.layers.map(ref=>exact(ref,['create'])?create[ref.create].layer:ref)}));
 const request={...r,contract:layerInput.contract,expected,create,remove,targets};
 const normalized=normalizeLayerLifecycle({route:'typed',request:{family:15,variant:'semantic-settings',version:1,arguments:request}},c,p);
 if(!normalized||!layerLifecycleDecision(owner.root,normalized.arguments,c.authority,p,h))return null;
 const current=resolveNativeEngineSettingsLifecycleBasis(owner,domain,basis,c,h);return current&&same(current,expected)?normalized:null;
}
export function prepareScopedNativeEngineSettingsLifecycle(owner,domain,ticket,reference,before,envelope,basis,c,p,h,existing=null){
 if(!allowed(h,['outsidePublicationGuard','preparationReserved']))return null;
 const input=normalizeScopedLifecycle(envelope,c,p);if(!input||input.request.invocation!==reference)return null;
 const bases=[basis],lowered=lowerScopedLifecycle(owner,domain,bases,input.request,input.context,p,h);if(!lowered)return null;
 return prepareSettings(owner,domain,ticket,reference,before,{route:'typed',request:{family:15,variant:'semantic-settings',version:1,arguments:lowered.arguments}},input.context,p,h,existing,{request:input.request,bases});
}

// Native first-party fresh-basis selectors; not a public wire/Rust contract.
function normalizeBasisLifecycle(envelope,c,p){
 if(!profileValid(p))return null;const data=copySettingsData(envelope,p.argumentLimits),context=copySettingsData(c,p.argumentLimits);if(!data||!context)return null;
 const e=data.value,scope=context.value;if(!exact(scope,['authority','engine','recipient','purpose','destination','profile','continuity'])||!Object.values(scope).every(v=>label(v,p))||!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route)||!exact(e.request,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:r,...tuple}=e.request,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);
 if(decoded?.descriptor.id!==basisLifecycleInput.family||decoded.variant!==basisLifecycleInput.variant||!exact(r,basisLifecycleInput.fields)||r.contract!==basisLifecycleInput.contract||r.operation!==basisLifecycleInput.operation||!label(r.invocation,p)||r.profile!==scope.profile||!Array.isArray(r.create)||!Array.isArray(r.remove)||r.create.length+r.remove.length>p.argumentLimits.maxChanges||!Array.isArray(r.targets)||r.targets.length>p.maxTargets||!Array.isArray(r.dependencies)||r.dependencies.length>p.argumentLimits.maxDependencies)return null;
 let used=0;for(const row of r.create){if(!exact(row,['id','kind','values'])||!label(row.id,p)||!scopes.includes(row.kind)||!object(row.values))return null;const n=bytes(row.values,p,p.argumentLimits.maxValueBytes-used);if(n===null)return null;used+=n;}
 const selector=ref=>exact(ref,['basis'])&&natural(ref.basis);
 if(r.remove.some(ref=>!selector(ref)))return null;
 for(const target of r.targets)if(!exact(target,['target','layers'])||!identity(target.target,scope.authority,p)||!Array.isArray(target.layers)||target.layers.some(ref=>!identity(ref,scope.authority,p)&&!selector(ref)&&!(exact(ref,['create'])&&natural(ref.create)&&ref.create<r.create.length)))return null;
 return {request:r,context:scope};
}
function lowerBasisLifecycle(owner,domain,bases,r,c,p,h){
 if(h?.settingsHostBasisAllowed!==true||!same(h.semanticSettingsContext,c)||!same(h.semanticSettingsProfile,p)||h.semanticSettingsBasis!==owner.root||!Array.isArray(bases)||bases.length<1||bases.length>p.maxLayers+1||new Set(bases).size!==bases.length)return null;
 const life=retainedLifecycleBasis(owner,domain,bases[0],c);if(!life||life.s.stopped||!same(life.m.profile,p))return null;
 const cfg=nativeEngineSettingsBasis(owner,p);if(!cfg||!same(lifecycleCondition(cfg),life.v.writeBasis.expected)||bases.length>1&&basisLayerFacts(p,c.authority,h)===null)return null;
 const used=new Set([...r.remove,...r.targets.flatMap(target=>target.layers)].filter(ref=>exact(ref,['basis'])).map(ref=>ref.basis)),writeBases=bases.slice(1);
 if(used.size!==writeBases.length||[...used].some(index=>!natural(index)||index>=writeBases.length))return null;
 const conditions=[];for(const basis of writeBases){const held=retainedWriteBasis(owner,domain,basis,c);if(!held||held.s.stopped||!same(held.m.profile,p)||!same(dependencies(cfg,[{layer:held.v.writeBasis.layer}]),held.v.writeBasis.dependencies))return null;const layer=cfg.layers.find(row=>same(row.layer,held.v.writeBasis.layer));if(!layer)return null;conditions.push({layer:layer.layer,expectedGeneration:layer.generation});}
 if(new Set(conditions.map(condition=>key(condition.layer))).size!==conditions.length)return null;
 const resolve=ref=>exact(ref,['basis'])?conditions[ref.basis].layer:ref;
 const converted={...r,contract:scopedLifecycleInput.contract,remove:r.remove.map(resolve),targets:r.targets.map(target=>({...target,layers:target.layers.map(resolve)}))};
 const lowered=lowerScopedLifecycle(owner,domain,[bases[0]],converted,c,p,h);if(!lowered)return null;
 for(let index=0;index<writeBases.length;index++){const current=resolveNativeEngineSettingsWriteBasis(owner,domain,writeBases[index],c,h);if(!current||!same(current,conditions[index]))return null;}
 return lowered;
}
export function prepareBasisNativeEngineSettingsLifecycle(owner,domain,ticket,reference,before,envelope,lifecycleBasis,writeBases,c,p,h,existing=null){
 if(!allowed(h,['outsidePublicationGuard','preparationReserved'])||!Array.isArray(writeBases))return null;
 const input=normalizeBasisLifecycle(envelope,c,p);if(!input||input.request.invocation!==reference||writeBases.length>p.maxLayers)return null;
 const bases=[lifecycleBasis,...writeBases],lowered=lowerBasisLifecycle(owner,domain,bases,input.request,input.context,p,h);if(!lowered)return null;
 return prepareSettings(owner,domain,ticket,reference,before,{route:'typed',request:{family:15,variant:'semantic-settings',version:1,arguments:lowered.arguments}},input.context,p,h,existing,{request:input.request,bases});
}
