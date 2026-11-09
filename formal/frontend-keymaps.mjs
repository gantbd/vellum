// Frontend-owned bounded keymap/chord helper; not an engine input or execution API.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {decodeDynamic,planRoute} from './operation-catalog.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./frontend-keymaps.json',import.meta.url),'utf8'));
const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
const copy=structuredClone;
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const bytes=v=>Buffer.byteLength(JSON.stringify(v),'utf8');
const key=s=>JSON.stringify(s);
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const sorted=rows=>copy(rows).sort((a,b)=>key(a.sequence??a.prefix).localeCompare(key(b.sequence??b.prefix)));
const signature=c=>({reference:c.reference,generation:c.generation,target:copy(c.target),profile:c.profile,maps:copy(c.maps),configurationTarget:c.configurationTarget});
function actionValid(action,commands){
  if(action===null)return true;
  return action&&Object.hasOwn(catalog.actions,action.kind)&&exact(action,catalog.actions[action.kind])&&
    (action.kind!=='bind'||commands.some(c=>c.reference===action.command&&families.find(f=>f.id===c.family)?.kind!=='effectObservation'));
}
const sequenceValid=(sequence,profile,allowEmpty=false)=>Array.isArray(sequence)&&(allowEmpty||sequence.length>0)&&
  sequence.length<=profile.maxChordLength&&sequence.every(k=>typeof k==='string'&&profile.keys.includes(k));
function node(){return {edges:new Map(),action:null,fallback:null};}
function at(root,prefix){let n=root;for(const k of prefix){n=n.edges.get(k);if(!n)return null;}return n;}
function trie(map,profile,commands){
  if(map.entries.length>profile.maxEntries||map.defaults.length>profile.maxDefaults||!Number.isSafeInteger(map.rank))return null;
  const root=node(),seen=new Set(),defaultSeen=new Set();
  const descend=sequence=>{let n=root;for(const k of sequence){if(!n.edges.has(k))n.edges.set(k,node());n=n.edges.get(k);}return n;};
  for(const entry of map.entries){
    if(!exact(entry,catalog.bindingChangeFields)||!sequenceValid(entry.sequence,profile)||!entry.action||!actionValid(entry.action,commands)||seen.has(key(entry.sequence)))return null;
    seen.add(key(entry.sequence));descend(entry.sequence).action=copy(entry.action);
  }
  for(const entry of map.defaults){
    if(!exact(entry,catalog.defaultChangeFields)||!sequenceValid(entry.prefix,profile,true)||entry.prefix.length>=profile.maxChordLength||
        !entry.action||!actionValid(entry.action,commands)||defaultSeen.has(key(entry.prefix)))return null;
    defaultSeen.add(key(entry.prefix));descend(entry.prefix).fallback=copy(entry.action);
  }
  const queue=[root];for(let i=0;i<queue.length;i++){
    const n=queue[i];if(n.action&&(n.edges.size||n.fallback))return null;queue.push(...n.edges.values());
  }
  return root;
}
function localDecision(root,prefix,input){
  const n=at(root,prefix);if(!n||n.action)return null;
  const child=n.edges.get(input),out=child?(child.action??{kind:'prefix'}):n.fallback;
  return out?.kind==='fallthrough'?null:out;
}
export function compileKeymap(maps,profile,commands){
  if(!profile||!Array.isArray(profile.keys)||new Set(profile.keys).size!==profile.keys.length||profile.keys.some(k=>typeof k!=='string'||!k)||
      !['maxChordLength','maxEntries','maxDefaults','maxMaps','maxCells'].every(k=>Number.isSafeInteger(profile[k])&&profile[k]>=0)||
      profile.maxChordLength<1||maps.length>profile.maxMaps||new Set(maps.map(m=>m.reference)).size!==maps.length)return null;
  if(commands.some(c=>{
    const family=families.find(f=>f.id===c.family);
    return !family||!decodeDynamic(families,{name:family.name,variant:c.variant,version:c.version});
  })||new Set(commands.map(c=>c.reference)).size!==commands.length)return null;
  const roots=maps.map(m=>trie(m,profile,commands));if(roots.some(r=>r===null))return null;
  const prefixes=new Map([[key([]),[]]]);
  for(const map of maps)for(const sequence of [...map.entries.map(e=>e.sequence),...map.defaults.map(e=>e.prefix)])
    for(let n=1;n<=sequence.length;n++)prefixes.set(key(sequence.slice(0,n)),sequence.slice(0,n));
  if(prefixes.size*profile.keys.length>profile.maxCells)return null;
  const rows=[];
  for(const prefix of [...prefixes.values()].sort((a,b)=>key(a).localeCompare(key(b))))for(const input of [...profile.keys].sort()){
    const choices=maps.map((m,i)=>({rank:m.rank,result:localDecision(roots[i],prefix,input)})).filter(c=>c.result);
    for(const rank of new Set(choices.map(c=>c.rank))){
      const peers=choices.filter(c=>c.rank===rank);if(peers.some(p=>!equal(p.result,peers[0].result)))return null;
    }
    const result=choices.length?choices.find(c=>c.rank===Math.max(...choices.map(c=>c.rank))).result:{kind:'unbound'};
    if(result.kind==='prefix'&&prefix.length+1>=profile.maxChordLength)return null;
    rows.push({prefix:copy(prefix),key:input,result:copy(result)});
  }
  return freeze({profile:profile.reference,maxChordLength:profile.maxChordLength,rows});
}
export function stepChord(compiled,context,pending,event){
  const no=(kind,currentUnhandled=false)=>({kind,pending:null,proposal:null,prefixConsumed:!!pending,currentUnhandled});
  const stale=()=>({kind:'stale',pending,proposal:null,prefixConsumed:false,currentUnhandled:false});
  if(!event||!['key','cancel','cutoff'].includes(event.kind))return no('unavailable');
  if(event.kind==='cancel'||event.kind==='cutoff')return pending&&
    (event.chord!==pending.token||!equal(event.context,pending.context))?stale():no('cancelled');
  if(!compiled||compiled.profile!==context.profile||!equal(event.context,signature(context)))return stale();
  if(pending&&!equal(pending.context,signature(context)))return no('cancelled',true);
  if(typeof event.chord!=='string'||!event.chord||(pending&&event.chord!==pending.token))return stale();
  const prefix=pending?.keys??[],row=compiled.rows.find(r=>equal(r.prefix,prefix)&&r.key===event.key);
  if(!row)return no('unbound',true);
  const result=row.result;
  if(result.kind==='prefix')return {kind:'pending',pending:freeze({keys:[...prefix,event.key],token:pending?.token??event.chord,context:signature(context)}),proposal:null,prefixConsumed:true,currentUnhandled:false};
  if(result.kind==='bind')return {kind:'command',pending:null,proposal:{command:result.command,target:copy(context.target),inputContext:signature(context)},prefixConsumed:!!pending,currentUnhandled:false};
  return no(result.kind==='block'?'blocked':'unbound',result.kind!=='block');
}
export function keymapDependencies(state,intents){
  if(!intents.length)return null;
  if(!state.keymapLayers||!state.inputContexts||!state.keyProfiles||!state.registeredCommands)return null;
  const written=new Set(intents.map(i=>i.map)),contexts=state.inputContexts.filter(c=>c.maps.some(m=>written.has(m))),needed=new Set([...written,...contexts.flatMap(c=>c.maps)]);
  return {commandGeneration:state.commandGeneration,registeredCommands:copy(state.registeredCommands),
    maps:copy(state.keymapLayers.filter(m=>needed.has(m.reference))).sort((a,b)=>a.reference.localeCompare(b.reference)),
    contexts:contexts.map(signature).sort((a,b)=>a.reference.localeCompare(b.reference)),
    profiles:copy(state.keyProfiles.filter(p=>contexts.some(c=>c.profile===p.reference))).sort((a,b)=>a.reference.localeCompare(b.reference))};
}
export function prepareKeymapPart(state,intents,host){
  if(!intents.length)return {dependencies:null,maps:[],compiled:[]};
  const dependencies=keymapDependencies(state,intents),limits=host.keymapLimits;
  if(!dependencies||!limits||!['maxMaps','maxChanges','maxBytes'].every(k=>Number.isSafeInteger(limits[k])&&limits[k]>=0)||intents.length>limits.maxMaps||bytes(intents)>limits.maxBytes)return null;
  const maps=copy(state.keymapLayers),seenMaps=new Set();
  for(const intent of intents){
    if(!exact(intent,catalog.mapIntentFields)||seenMaps.has(intent.map)||!host.writableKeymaps?.includes(intent.map)||
        !Array.isArray(intent.changes)||!Array.isArray(intent.defaults)||intent.changes.length+intent.defaults.length>limits.maxChanges)return null;
    seenMaps.add(intent.map);const map=maps.find(m=>m.reference===intent.map);
    if(!map||map.generation!==intent.expectedGeneration)return null;
    const ownProfile=state.keyProfiles.find(p=>p.reference===map.profile);if(!ownProfile)return null;
    for(const [field,changes,path,fields] of [['entries',intent.changes,'sequence',catalog.bindingChangeFields],['defaults',intent.defaults,'prefix',catalog.defaultChangeFields]]){
      const seen=new Set();for(const change of changes){
        if(!exact(change,fields)||!sequenceValid(change[path],ownProfile,path==='prefix')||
            (path==='prefix'&&change[path].length>=ownProfile.maxChordLength)||seen.has(key(change[path])))return null;seen.add(key(change[path]));
        map[field]=map[field].filter(e=>!equal(e[path],change[path]));if(change.action!==null)map[field].push(copy(change));
      }
      map[field]=sorted(map[field]);
    }
  }
  const compiled=[];
  for(const context of dependencies.contexts){
    const profile=state.keyProfiles.find(p=>p.reference===context.profile),applicable=maps.filter(m=>context.maps.includes(m.reference));
    if(applicable.length!==context.maps.length)return null;
    const result=compileKeymap(applicable,profile,state.registeredCommands);if(!result)return null;
    compiled.push({context:context.reference,result});
  }
  // A changed unused map still requires a named admitted profile, not deferred validation.
  for(const ref of seenMaps){
    const map=maps.find(m=>m.reference===ref),profile=state.keyProfiles.find(p=>p.reference===map.profile);
    if(!compileKeymap([map],profile,state.registeredCommands))return null;
  }
  const part={dependencies,maps:maps.filter(m=>seenMaps.has(m.reference)),compiled};
  return bytes(part)<=limits.maxBytes?part:null;
}
export function keymapChanged(state,part){
  return part.maps.some(m=>{const old=state.keymapLayers.find(x=>x.reference===m.reference);return !equal(sorted(old.entries),m.entries)||!equal(sorted(old.defaults),m.defaults);});
}
export function publishKeymapPart(next,before,part){
  const changed=new Set();
  for(const map of part.maps){
    const old=before.keymapLayers.find(m=>m.reference===map.reference);
    if(equal(sorted(old.entries),map.entries)&&equal(sorted(old.defaults),map.defaults))continue;
    const index=next.keymapLayers.findIndex(m=>m.reference===map.reference);next.keymapLayers[index]={...copy(map),generation:old.generation+1};changed.add(map.reference);
  }
  for(const context of next.inputContexts??[]){
    if(!context.maps.some(ref=>changed.has(ref)))continue;
    context.generation++;context.pending=null;
    next.current.keymaps??={};
    Object.defineProperty(next.current.keymaps,context.reference,{value:copy(part.compiled.find(c=>c.context===context.reference).result),enumerable:true,writable:true,configurable:true});
    const target=context.configurationTarget;
    if(target&&next.current.domains[target].view===before.current.domains[target].view)next.current.domains[target].view++;
  }
}

function fixture(){
  const commands=[{reference:'cmd-a',family:'F12',variant:'logical-movement',version:1},{reference:'cmd-b',family:'F12',variant:'set-selection',version:1}];
  const bind=command=>({kind:'bind',command});
  const profile={reference:'keys',keys:['g','x','y','z'],maxChordLength:3,maxEntries:16,maxDefaults:8,maxMaps:4,maxCells:128};
  const maps=[{reference:'low',profile:'keys',rank:0,generation:1,entries:[{sequence:['x'],action:bind('cmd-a')},{sequence:['g','y'],action:bind('cmd-a')}],defaults:[]},
    {reference:'high',profile:'keys',rank:1,generation:1,entries:[],defaults:[]}];
  const context={reference:'input',generation:1,maps:['low','high'],profile:'keys',target:{view:'v',document:'d',revision:1},configurationTarget:'document',pending:null};
  return {commands,bind,profile,maps,context};
}
export function checkKeymapProducts(){
  assert.equal(catalog.layer,'frontend');assert.equal(catalog.example,'ratatui');
  const {commands,bind,profile,maps,context}=fixture();const controls=[],control=(name,condition)=>{assert(condition,name);controls.push(name);};
  const table=compileKeymap(maps,profile,commands);assert(table);
  const event=key=>({kind:'key',key,chord:'chord-1',context:signature(context)});
  const start=stepChord(table,context,null,event('g')),end=stepChord(table,context,start.pending,event('y'));
  control('prefix-then-registered-intent',start.kind==='pending'&&end.kind==='command'&&end.proposal.command==='cmd-a'&&equal(end.proposal.target,context.target));
  const candidate=copy(maps);candidate[1].defaults=[{prefix:[],action:bind('cmd-b')}];
  const fallback=compileKeymap(candidate,profile,commands);
  control('removal-does-not-block-applicable-default',stepChord(fallback,context,null,event('x')).proposal.command==='cmd-b');
  candidate[1].entries=[{sequence:['x'],action:{kind:'fallthrough'}}];
  control('fallthrough-bypasses-own-default',stepChord(compileKeymap(candidate,profile,commands),context,null,event('x')).proposal.command==='cmd-a');
  candidate[1].entries=[{sequence:['x'],action:{kind:'block'}}];
  control('block-stops-lower-lookup',stepChord(compileKeymap(candidate,profile,commands),context,null,event('x')).kind==='blocked');
  control('absence-without-default-permits-lower-map',stepChord(table,context,null,event('x')).proposal.command==='cmd-a');
  const conflict=copy(maps);conflict[1].rank=0;conflict[1].entries=[{sequence:['x'],action:bind('cmd-b')}];
  control('equal-rank-conflict-refused',compileKeymap(conflict,profile,commands)===null);
  conflict[1].entries=[{sequence:['g'],action:bind('cmd-b')}];
  control('equal-rank-command-prefix-refused',compileKeymap(conflict,profile,commands)===null);
  const ambiguous=copy(maps);ambiguous[0].entries.push({sequence:['g'],action:bind('cmd-b')});
  control('same-map-command-prefix-refused',compileKeymap(ambiguous,profile,commands)===null);
  const backtrack=copy(maps);backtrack[0].entries=[{sequence:['g'],action:bind('cmd-a')}];backtrack[1].entries=[{sequence:['g','y'],action:bind('cmd-b')}];
  const locked=compileKeymap(backtrack,profile,commands),prefix=stepChord(locked,context,null,event('g'));
  control('unmatched-continuation-does-not-run-consumed-prefix',prefix.kind==='pending'&&stepChord(locked,context,prefix.pending,event('z')).kind==='unbound');
  const stale=stepChord(table,{...context,target:{...context.target,view:'other'}},start.pending,event('y'));
  control('stale-focus-does-not-retarget',stale.kind==='stale'&&stale.proposal===null);
  control('stale-root-event-not-retargeted',stepChord(table,{...context,target:{...context.target,view:'other'}},null,event('x')).kind==='stale');
  const cutoff=stepChord(table,context,start.pending,{kind:'cutoff',chord:'chord-1',context:signature(context)});
  control('cutoff-does-not-replay-prefix',cutoff.kind==='cancelled'&&cutoff.pending===null&&cutoff.proposal===null);
  control('cancel-does-not-replay-prefix',stepChord(table,context,start.pending,{kind:'cancel',chord:'chord-1',context:signature(context)}).pending===null);
  const newer=stepChord(table,context,null,{...event('g'),chord:'chord-2'}).pending;
  control('late-cutoff-cannot-cancel-new-chord',equal(stepChord(table,context,newer,{kind:'cutoff',chord:'chord-1',context:signature(context)}).pending,newer));
  control('old-key-event-cannot-complete-new-chord',stepChord(table,context,newer,event('y')).kind==='stale');
  const family=families.find(f=>f.id==='F12'),decoded=decodeDynamic(families,{name:family.name,variant:'logical-movement',version:1});
  control('bound-intent-is-not-execution-authority',planRoute(decoded,{schemaValid:true,enabled:true,grant:false,current:true,ordinaryCapacity:true}).status==='denied');
  const bad=copy(maps);bad[0].entries[0].action={kind:'bind',command:'shell-string'};
  control('unknown-command-refused',compileKeymap(bad,profile,commands)===null);
  bad[0].entries[0].action={kind:'macro',keys:['x']};control('macro-not-executable-binding',compileKeymap(bad,profile,commands)===null);
  const observations=[...commands,{reference:'evidence',family:'F27',variant:'progress',version:1}],wrongLane=copy(maps);
  wrongLane[0].entries[0].action=bind('evidence');
  control('provider-evidence-not-input-command',compileKeymap(wrongLane,profile,observations)===null);
  control('table-bound',compileKeymap(maps,{...profile,maxCells:1},commands)===null);
  control('entry-bound',compileKeymap(maps,{...profile,maxEntries:1},commands)===null);
  control('chord-bound',compileKeymap(maps,{...profile,maxChordLength:1},commands)===null);
  let traces=0,steps=0;const visit=trace=>{
    let pending=null;for(const key of trace){const out=stepChord(table,context,pending,event(key));if(out.pending)assert(out.pending.keys.length<profile.maxChordLength);pending=out.pending;steps++;}
    traces++;if(trace.length<4)for(const k of profile.keys)visit([...trace,k]);
  };visit([]);
  control('input-owner-not-mutated',context.pending===null&&context.generation===1&&maps[0].generation===1);
  return {status:'keymap-products-only',traces,steps,controls,nativeInputImeRefinementProved:false,registryProviderActivationProved:false,physicalAccountingProved:false,implementationRefinement:false};
}
export function checkKeymapActivation({prepareSettings,activateSettings,resolveConfiguration}){
  const {commands,bind,profile,maps,context}=fixture();
  const state={schemas:[{key:'setting',type:{kind:'integer',min:0,max:3},default:1,domain:'source',scopes:['document'],composition:'replace'}],restrictions:{},schemaGeneration:1,restrictionGeneration:1,membershipGeneration:1,publication:0,
    layers:[{reference:'d',kind:'document',generation:1,values:{}}],targets:[{reference:'document',layers:['d']}],retained:[],contexts:[],
    authored:{source:[97],annotations:['comment'],history:['G'],effects:['unresolved']},keymapLayers:maps,inputContexts:[context],keyProfiles:[profile],registeredCommands:commands,commandGeneration:1};
  const table=compileKeymap(maps,profile,commands);state.inputContexts[0].pending=stepChord(table,context,null,{kind:'key',key:'g',chord:'chord-1',context:signature(context)}).pending;
  state.current={generation:1,resolved:resolveConfiguration(state),domains:{document:{source:1,view:1,analysis:1,host:1}},keymaps:{input:table}};
  const host={owner:{},recipient:'agent',profile:'p',authorized:true,bounded:true,notStopped:true,interpretationCurrent:true,companionsReady:true,retentionReady:true,invocationFresh:true,completeIdentity:true,versionsAvailable:true,
    maxChanges:8,maxPlanBytes:65536,maxStateBytes:65536,maxDependencies:4,requiredDependencies:[],writableLayers:['d'],heldCurrent:true,maxRetainedContexts:2,
    writableKeymaps:['high'],keymapLimits:{maxMaps:4,maxChanges:16,maxBytes:65536}};
  const intent={map:'high',expectedGeneration:1,changes:[{sequence:['g','y'],action:bind('cmd-b')}],defaults:[]};
  const request={contract:'settings-activation/1',operation:'settings-keymap',invocation:'activate',profile:'p',changes:[{layer:'d',expectedGeneration:1,key:'setting',operation:'set',value:2}],keymaps:[intent],dependencies:[]};
  const controls=[],control=(name,condition)=>{assert(condition,name);controls.push(name);};
  const plan=prepareSettings(state,request,host),out=activateSettings(state,plan,host);assert.equal(out.status,'applied');
  control('settings-keymap-one-publication',out.state.publication===1&&out.state.current.resolved.document.setting.value===2&&out.state.keymapLayers[1].generation===2);
  control('current-pending-chord-cancelled',out.state.inputContexts[0].pending===null&&out.state.inputContexts[0].generation===2);
  const nextContext=out.state.inputContexts[0],nextEvent=key=>({kind:'key',key,chord:'new-chord',context:signature(nextContext)});
  const nextPrefix=stepChord(out.state.current.keymaps.input,nextContext,null,nextEvent('g'));
  control('new-compiled-map-installed',stepChord(out.state.current.keymaps.input,nextContext,nextPrefix.pending,nextEvent('y')).proposal.command==='cmd-b');
  control('old-event-not-replayed-under-new-map',stepChord(out.state.current.keymaps.input,nextContext,null,{kind:'key',key:'x',chord:'old-chord',context:signature(context)}).kind==='stale');
  control('retained-old-map-and-authored-effects-preserved',equal(out.state.retained[0],state.current)&&equal(out.state.authored,state.authored));
  const stale=copy(state);stale.keymapLayers[1].generation++;
  control('superseded-map-refuses-whole-settings',activateSettings(stale,plan,host).status==='not-published');
  control('caller-map-generation-required',prepareSettings(state,{...copy(request),keymaps:[{...intent,expectedGeneration:2}]},host)===null);
  control('unknown-key-removal-not-a-noop',prepareSettings(state,{...copy(request),keymaps:[{...intent,changes:[{sequence:['unknown'],action:null}]}]},host)===null);
  control('current-map-authority-refuses-whole-settings',activateSettings(state,plan,{...host,writableKeymaps:[]}).status==='not-published');
  control('retention-exhaustion-refuses-whole-settings',activateSettings(state,plan,{...host,maxRetainedContexts:0}).status==='not-published');
  const moved=copy(state);moved.inputContexts[0].target.view='other';
  control('changed-input-context-stales-preparation',activateSettings(moved,plan,host).status==='not-published');
  const progressed=copy(state);progressed.inputContexts[0].pending=stepChord(table,context,null,{kind:'key',key:'g',chord:'new-current-chord',context:signature(context)}).pending;
  const afterProgress=activateSettings(progressed,plan,host);
  control('activation-cancels-current-not-old-prefix',afterProgress.status==='applied'&&afterProgress.state.inputContexts[0].pending===null);
  const removed={...copy(request),changes:[],keymaps:[{...intent,changes:[{sequence:['x'],action:null}]}]};
  const noop=activateSettings(state,prepareSettings(state,removed,host),{...host,versionsAvailable:false,maxRetainedContexts:0});
  control('missing-binding-removal-is-complete-noop',noop.status==='unchanged'&&noop.state.inputContexts[0].pending!==null);
  const keymapOnly=activateSettings(state,prepareSettings(state,{...copy(request),changes:[]},host),host);
  control('keymap-only-change-is-not-source-interpretation',keymapOnly.state.current.domains.document.source===1&&keymapOnly.state.current.domains.document.view===2);
  const independent=copy(state),otherMap={...copy(maps[0]),reference:'other-map'};
  const otherContext={...copy(context),reference:'other-input',maps:['other-map'],target:{...context.target,view:'other-view'},configurationTarget:null};
  const otherTable=compileKeymap([otherMap],profile,commands);
  otherContext.pending=stepChord(otherTable,otherContext,null,{kind:'key',key:'g',chord:'other-chord',context:signature(otherContext)}).pending;
  independent.keymapLayers.push(otherMap);independent.inputContexts.push(otherContext);independent.current.keymaps['other-input']=otherTable;
  const unrelated=activateSettings(independent,prepareSettings(independent,{...copy(request),changes:[]},host),host);
  control('unrelated-input-chord-and-map-preserved',unrelated.status==='applied'&&equal(unrelated.state.inputContexts[1],otherContext)&&equal(unrelated.state.current.keymaps['other-input'],otherTable));
  return controls;
}
export async function checkKeymapMutations(){
  const source=fs.readFileSync(new URL('./frontend-keymaps.mjs',import.meta.url),'utf8');
  const configuration=fs.readFileSync(new URL('./frontend-configuration-reference.mjs',import.meta.url),'utf8');
  const mutants=[
    ['fallthrough-runs-own-default',"return out?.kind==='fallthrough'?null:out;","return out?.kind==='fallthrough'?n.fallback:out;"],
    ['first-peer-wins',"if(peers.some(p=>!equal(p.result,peers[0].result)))return null;",'if(false)return null;'],
    ['stale-event-retargeted','!equal(event.context,signature(context))','false'],
    ['late-cutoff-cancels-new-chord','event.chord!==pending.token||!equal(event.context,pending.context)','!equal(event.context,pending.context)'],
    ['activation-keeps-old-prefix','context.generation++;context.pending=null;','context.generation++;'],
    ['ignore-caller-map-generation','map.generation!==intent.expectedGeneration','false'],
    ['activation-reuses-input-generation','context.generation++;context.pending=null;','context.pending=null;'],
    ['bind-provider-evidence-as-command',"c.reference===action.command&&families.find(f=>f.id===c.family)?.kind!=='effectObservation'",'c.reference===action.command']
  ];
  const killed=[];
  for(const [name,from,to] of mutants){
    assert(source.includes(from),name);
    let code=source.replace(from,to).replace("'./operation-catalog.mjs'",JSON.stringify(new URL('./operation-catalog.mjs',import.meta.url).href));
    for(const file of ['frontend-keymaps.json','operation-families.json'])code=code.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(`./${file}`,import.meta.url).href)})`);
    const url=`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`,module=await import(url);
    let composed=configuration.replace("'./frontend-keymaps.mjs'",JSON.stringify(url));
    for(const file of ['baseline-contracts.mjs','annotation-operations.mjs'])composed=composed.replace(`'./${file}'`,JSON.stringify(new URL(`./${file}`,import.meta.url).href));
    composed=composed.replace("new URL('./frontend-configuration-reference.json',import.meta.url)",`new URL(${JSON.stringify(new URL('./frontend-configuration-reference.json',import.meta.url).href)})`);
    const actual=await import(`data:text/javascript;base64,${Buffer.from(composed).toString('base64')}#${name}`);
    let failed=false;
    for(const check of [()=>module.checkKeymapProducts(),()=>module.checkKeymapActivation(actual)]){
      try{check();}catch(error){assert.equal(error.code,'ERR_ASSERTION',name);failed=true;}
    }
    assert(failed,name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkKeymapProducts(),null,2));
