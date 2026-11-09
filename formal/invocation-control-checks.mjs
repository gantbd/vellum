import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import * as native from './annotation-editing.mjs';
import * as reversal from './history-reversal.mjs';
import * as histories from './history-contexts.mjs';
import * as observer from './publication-observation.mjs';
import * as observation from './invocation-observation.mjs';
import * as control from './invocation-control.mjs';
import {selectionGatewayFixture} from './selection-gateway-checks.mjs';
import {replacementGatewayFixture} from './replacement-gateway-checks.mjs';
import * as selection from './selection-products.mjs';
import * as line from './line-edits.mjs';
import * as boundary from './boundary-commands.mjs';
import * as search from './literal-search.mjs';
import * as clipboard from './clipboard-products.mjs';
import {editingInvocationFixture,editingInvocationContext,editingInvocationPolicy} from './editing-invocation-checks.mjs';
import {clipboardInvocationFixture,clipboardInvocationContext,clipboardInvocationPolicy} from './clipboard-invocation-checks.mjs';
import {lifecycleInvocationFixture} from './lifecycle-invocation-checks.mjs';
import {saveAsInvocationFixture} from './saveas-invocation-checks.mjs';
import {loadInvocationFixture} from './load-invocation-checks.mjs';
import {annotationLoadInvocationFixture} from './annotation-load-invocation-checks.mjs';
import {registrationInvocationFixture} from './registration-invocation-checks.mjs';
import {scopedRegistrationInvocationFixture} from './registration-scoped-checks.mjs';
import {relationInvocationFixture} from './relation-invocation-checks.mjs';
import {engineSettingsFixture} from './engine-settings-checks.mjs';
import {engineSettingsDefinitionFixture} from './engine-settings-definition-checks.mjs';
import {engineSettingsLayerFixture} from './engine-settings-layer-checks.mjs';
import {scopedSettingsFixture} from './scoped-settings-checks.mjs';
import {configuredLineFixture} from './configured-line-checks.mjs';
import {scopedSerializationFixture} from './scoped-serialization-checks.mjs';
import {sourceSerializationFixture} from './source-serialization-checks.mjs';
import {scopedDefinitionFixture} from './scoped-definition-checks.mjs';
import {settingsCreatedResultFixture} from './settings-created-result-checks.mjs';
import {scopedLifecycleFixture} from './scoped-lifecycle-checks.mjs';
import {basisLifecycleFixture} from './basis-lifecycle-checks.mjs';
import {g as clipboardHost,retire as clipboardRetirement} from './clipboard-product-checks.mjs';
import {invocationObservationFixture as fixture,invocationObservationContext as c,invocationObservationPolicy as policy,invocationObservationGates as good,invocationObservationLimits as observerLimits} from './invocation-observation-checks.mjs';
const original={...kernel,...native,...reversal,...histories,...observer,...observation,...control,...selection,...line,...boundary,...search,...clipboard},copy=structuredClone,unavailable={status:'unavailable'};
const catalog=JSON.parse(fs.readFileSync(new URL('./invocation-control.json',import.meta.url))),annotationKinds=['create','update-body','reattach','delete','compound','undo','redo'],editingKinds=['selection','line','boundary','replacement'],kinds=[...annotationKinds,...editingKinds,'paste-request','cut-export-stage','create-view','create-scratch','source-save-as','source-load','annotation-restore','annotation-adopt','resource-registration','scoped-resource-registration','semantic-settings','settings-definition','configured-line','settings-layers','scoped-settings','source-serialization','scoped-definition','scoped-lifecycle','scoped-serialization','settings-created-result'];
const gates={...good,...Object.fromEntries(catalog.gates.map(k=>[k,true]))};
kinds.push('basis-lifecycle');
kinds.push(...['set','logical','boundary','match'].map(k=>'selection-gateway-'+k));
kinds.push(...['replace-current','replace-all'].map(k=>'replacement-gateway-'+k));
function setup(api,kind='update-body'){
 // Composed clipboard/configured control fixtures fund their requested observer
 // owner in addition to the native producer. These are not product defaults.
 let z;
 if(annotationKinds.includes(kind)){z=fixture(api,kind);z.controlContext=c;z.controlPolicy=policy;}
 else if(kind==='basis-lifecycle'||kind.startsWith('selection-gateway-')||kind.startsWith('replacement-gateway-')){
  z=kind==='basis-lifecycle'?basisLifecycleFixture(api):kind.startsWith('selection-gateway-')?selectionGatewayFixture(api,kind.slice('selection-gateway-'.length)):replacementGatewayFixture(api,kind.slice('replacement-gateway-'.length));z.p={ticket:z.ticket,reference:z.reference};z.current=z.root;z.finish=z.run;z.admit=()=>z.submit();
  z.open=()=>api.openInvocationObservation(z.domain,z.ticket,z.controlContext,z.controlPolicy,observerLimits,good);
  z.read=(context,operation='status')=>api.queryInvocationObservation(context,{contract:'execution-observation/1',operation,invocation:z.reference,context,profile:z.controlContext.profile},z.controlContext,z.controlPolicy,good);
 }
 else{
  const relation=kind==='create-view',lifecycle=['create-scratch','source-save-as','source-load','annotation-restore','annotation-adopt','resource-registration','scoped-resource-registration'].includes(kind),editing=editingKinds.includes(kind);z=kind==='settings-created-result'?settingsCreatedResultFixture(api):kind==='scoped-serialization'?scopedSerializationFixture(api):kind==='scoped-lifecycle'?scopedLifecycleFixture(api):kind==='scoped-definition'?scopedDefinitionFixture(api):kind==='source-serialization'?sourceSerializationFixture(api):kind==='scoped-settings'?scopedSettingsFixture(api):kind==='settings-layers'?engineSettingsLayerFixture(api):kind==='configured-line'?configuredLineFixture(api,{limits:{maxHeldBytes:8388608}}):kind==='settings-definition'?engineSettingsDefinitionFixture(api):kind==='semantic-settings'?engineSettingsFixture(api):kind==='scoped-resource-registration'?scopedRegistrationInvocationFixture(api):kind==='resource-registration'?registrationInvocationFixture(api):kind==='annotation-restore'||kind==='annotation-adopt'?annotationLoadInvocationFixture(api,kind==='annotation-restore'?'restore':'adopt-baseline'):kind==='source-load'?loadInvocationFixture(api):kind==='source-save-as'?saveAsInvocationFixture(api):lifecycle?lifecycleInvocationFixture(api):relation?relationInvocationFixture(api):editing?editingInvocationFixture(kind,api):clipboardInvocationFixture(api,kind,{limits:{maxHeldBytes:4194304}});z.controlContext=['semantic-settings','settings-definition','configured-line','settings-layers','scoped-settings','source-serialization','scoped-definition','scoped-lifecycle','scoped-serialization','settings-created-result'].includes(kind)||relation||lifecycle?z.controlContext:editing?editingInvocationContext:clipboardInvocationContext;z.controlPolicy=['semantic-settings','settings-definition','configured-line','settings-layers','scoped-settings','source-serialization','scoped-definition','scoped-lifecycle','scoped-serialization','settings-created-result'].includes(kind)||relation||lifecycle?z.controlPolicy:editing?editingInvocationPolicy:clipboardInvocationPolicy;
  z.p={ticket:z.ticket,reference:z.reference};z.current=z.root;z.finish=z.run;z.admit=()=>z.submit();
  z.open=()=>api.openInvocationObservation(z.domain,z.ticket,z.controlContext,z.controlPolicy,observerLimits,good);
  z.read=(context,operation='status')=>api.queryInvocationObservation(context,{contract:'execution-observation/1',operation,invocation:z.reference,context,profile:z.controlContext.profile},z.controlContext,z.controlPolicy,good);
 }
 assert.equal(z.admit().status,'accepted');return z;
}
const stop=(api,z,g=gates,context=z.controlContext??c,ticket=z.p.ticket,domain=z.domain)=>api.requestInvocationStop(domain,ticket,context,g);
const report=(api,z)=>api.observeInvocation(z.domain,z.p.ticket,z.controlContext??c,z.controlPolicy??policy,good);
function settled(api,z,stopped){const before=z.current();assert.equal(z.finish().status,'settled');const out=report(api,z);assert.equal(out.outcome.publication,stopped?'notApplied':'applied');assert.equal(out.outcome.control,stopped?'stopped':'done');if(stopped)assert.equal(z.current(),before);return out;}
export function checkInvocationControl(api=original,short=false){
 let gateCases=0,contextCases=0,raceCases=0,traces=0,traceEdges=0;const controls=[],check=(name,f)=>{f();controls.push(name);},seenKinds=new Set();
 for(const kind of short?['update-body','compound','undo']:kinds){
  for(let mask=0;mask<16;mask++){const z=setup(api,kind),g={...gates,...Object.fromEntries(catalog.gates.map((k,i)=>[k,!!(mask&(1<<i))]))},root=z.current(),usage=copy(api.invocationUsage(z.domain));seenKinds.add(z.job.kind);assert.equal(stop(api,z,g).status,mask===15?'stop-requested':'unavailable');assert.equal(z.current(),root);assert.deepEqual(api.invocationUsage(z.domain),usage);settled(api,z,mask===15);gateCases++;}
  for(const key of [...Object.keys(c),'extra']){const z=setup(api,kind),context={...z.controlContext,[key]:'other'};assert.deepEqual(stop(api,z,gates,context),unavailable);settled(api,z,false);contextCases++;}
  for(const first of ['stop','run']){const z=setup(api,kind),before=z.current();if(first==='stop'){assert.equal(stop(api,z).status,'stop-requested');assert.equal(stop(api,z).status,'stop-requested');settled(api,z,true);assert.equal(z.current(),before);}else settled(api,z,false);const root=z.current(),usage=copy(api.invocationUsage(z.domain));assert.deepEqual(stop(api,z),unavailable);assert.equal(z.admit().status,'observe-existing');assert.deepEqual(z.finish(),unavailable);assert.equal(z.current(),root);assert.deepEqual(api.invocationUsage(z.domain),usage);raceCases++;}
 }
 check('foreign-cloned-retired-and-unsubmitted-tickets-have-one-safe-refusal',()=>{const a=setup(api),b=setup(api);for(const ticket of [copy(a.p.ticket),b.p.ticket,null])assert.deepEqual(stop(api,a,gates,c,ticket),unavailable);assert.deepEqual(stop(api,a,gates,c,a.p.ticket,b.domain),unavailable);const pending=fixture(api);assert.deepEqual(stop(api,pending),unavailable);settled(api,a,false);api.retireInvocation(a.domain,a.p.ticket,good);assert.deepEqual(stop(api,a),unavailable);});
 check('control-permission-is-exact-true-not-a-content-or-administration-grant',()=>{for(const key of catalog.gates)for(const value of [undefined,null,0,1,'true']){const z=setup(api);assert.deepEqual(stop(api,z,{...gates,[key]:value}),unavailable);settled(api,z,false);}for(const kind of short?['update-body','selection','paste-request']:kinds){const z=setup(api,kind),g={...gates,authorized:false,scope:false,ordinaryReady:false,resourcesReady:false,transferReady:false,retentionReady:false,capacityActivationAuthorized:false};assert.equal(stop(api,z,g).status,'stop-requested');settled(api,z,true);}});
 check('stop-does-not-settle-refund-retire-forget-or-start-the-producer',()=>{const z=setup(api),root=z.current(),usage=copy(api.invocationUsage(z.domain));assert.equal(stop(api,z).status,'stop-requested');assert.equal(z.current(),root);assert.deepEqual(api.invocationUsage(z.domain),usage);assert.equal(api.invocationExecutionState(z.domain,z.p.ticket,c,false).phase,'pending');assert.deepEqual(api.retireInvocation(z.domain,z.p.ticket,good),unavailable);assert.deepEqual(api.forgetInvocationResult(z.domain,z.p.ticket,good),unavailable);assert.equal(z.read(z.open(),'minimal-control-status').phase,'pending');settled(api,z,true);});
 check('observer-close-never-stops-producer-and-producer-stop-never-closes-observer',()=>{const a=setup(api),ca=a.open();assert.equal(api.closeInvocationObservation(ca,c,good).status,'stopped');settled(api,a,false);const b=setup(api),cb=b.open();assert.equal(stop(api,b).status,'stop-requested');assert.equal(b.read(cb,'minimal-control-status').phase,'pending');settled(api,b,true);assert.equal(b.read(cb,'minimal-control-status').phase,'terminal');assert.equal(b.read(cb,'retained-outcome').outcome.control,'stopped');});
 check('unrelated-jobs-remain-eligible-on-the-same-root',()=>{const z=setup(api),other=z.prepare({operation:'update-body'},{body:'other'});assert.equal(api.submitInvocation(z.domain,other.ticket,{kind:'annotation',plan:other.plan},'typed',c,policy,good).status,'accepted');assert.equal(stop(api,z).status,'stop-requested');assert.equal(api.runInvocation(z.domain,other.ticket,c,good,other.checks).status,'settled');const root=z.current();settled(api,z,true);assert.equal(z.current(),root);assert.equal(api.observeInvocation(z.domain,other.ticket,c,policy,good).outcome.publication,'applied');});
 check('fault-knowledge-is-not-relabelled-by-late-cancellation',()=>{const z=setup(api),h={...z.host};Object.defineProperty(h,'annotationLimits',{get(){throw Error('trusted host fault');}});assert.equal(z.finish(h).status,'faulted');assert.deepEqual(stop(api,z),unavailable);assert.equal(api.observeInvocation(z.domain,z.p.ticket,c,policy,good).outcome.publication,'unknown');});
 check('running-turn-is-not-preempted-or-reentered-by-control',()=>{const z=setup(api),h={...z.host};let calls=0;Object.defineProperty(h,'annotationLimits',{get(){calls++;assert.deepEqual(stop(api,z),unavailable);return z.host.annotationLimits;}});settledWithHost();function settledWithHost(){assert.equal(z.finish(h).status,'settled');assert(calls>0);assert.equal(api.observeInvocation(z.domain,z.p.ticket,c,policy,good).outcome.publication,'applied');}});
 check('forgotten-terminal-result-does-not-reopen-control-or-execution',()=>{const z=setup(api);settled(api,z,false);api.forgetInvocationResult(z.domain,z.p.ticket,good);assert.deepEqual(stop(api,z),unavailable);assert.equal(z.admit().status,'observe-existing');assert.deepEqual(z.finish(),unavailable);assert.equal(api.invocationUsage(z.domain).starts,1);});
 check('acknowledged-clipboard-control-preserves-provider-facts-and-held-obligations',()=>{for(const kind of ['paste-request','cut-export-stage']){const z=setup(api,kind),before=copy(z.inspect()),held=copy(api.clipboardUsage(z.z.owner)),root=z.current();assert.equal(stop(api,z).status,'stop-requested');assert.deepEqual(z.inspect(),before);assert.deepEqual(api.clipboardUsage(z.z.owner),held);assert.equal(z.current(),root);assert.deepEqual(api.retireInvocation(z.domain,z.p.ticket,good),unavailable);settled(api,z,true);assert.equal(z.inspect().localDecision,'not-applied');assert.equal(report(api,z).outcome.effect,'confirmed');assert.equal(api.clipboardUsage(z.z.owner).outstanding,1);assert.equal(api.retireClipboard(z.z.owner,z.task,clipboardRetirement).status,'unavailable');assert.equal(api.retireInvocation(z.domain,z.p.ticket,good).status,'retired');assert.equal(api.clipboardUsage(z.z.owner).outstanding,1);assert.equal(api.retireClipboard(z.z.owner,z.task,clipboardRetirement).status,'unavailable');z.z.quiet(z.task);assert.equal(api.retireClipboard(z.z.owner,z.task,clipboardRetirement).status,'retired');}});
 check('clipboard-task-stop-and-invocation-stop-are-distinct-controls',()=>{const z=setup(api,'paste-request');assert.equal(api.controlClipboard(z.z.owner,z.task,clipboardHost).status,'stopped');assert.equal(api.invocationExecutionState(z.domain,z.p.ticket,z.controlContext,false).phase,'pending');assert.equal(z.finish().status,'settled');assert.equal(report(api,z).outcome.publication,'notApplied');assert.equal(report(api,z).outcome.control,'rejected');assert.equal(report(api,z).outcome.effect,'confirmed');});
 const events=['cancel','run','replay','observer-stop','forget','retire'];
 function explore(trace,kind){const z=setup(api,kind),context=z.open(),root=z.current(),initialStarts=api.invocationUsage(z.domain).starts;assert(context,kind+'/prepaid-observation-context');let requested=false,ran=false,retired=false,closed=false,forgotten=false,finalRoot=root;
  for(const event of trace){
   if(event==='cancel'){const out=stop(api,z);assert.equal(out.status,!ran&&!retired?'stop-requested':'unavailable');if(out.status==='stop-requested')requested=true;}
   if(event==='run'){const out=z.finish();assert.equal(out.status,ran||retired?'unavailable':'settled');if(!ran&&!retired){ran=true;finalRoot=z.current();assert.equal(report(api,z).outcome.publication,requested?'notApplied':'applied');if(requested)assert.equal(finalRoot,root);}}
   if(event==='replay')assert.equal(z.admit().status,retired?'unavailable':ran?'observe-existing':'pending');
   if(event==='observer-stop'){assert.equal(api.closeInvocationObservation(context,z.controlContext,good).status,'stopped');closed=true;}
   if(event==='forget'){assert.equal(api.forgetInvocationResult(z.domain,z.p.ticket,good).status,ran&&!retired?'forgotten':'unavailable');if(ran&&!retired)forgotten=true;}
   if(event==='retire'){assert.equal(api.retireInvocation(z.domain,z.p.ticket,good).status,ran&&!retired?'retired':'unavailable');if(ran)retired=true;}
   assert.equal(z.current(),finalRoot);assert.equal(api.invocationUsage(z.domain).starts,initialStarts+(ran?1:0));if(closed||retired)assert.deepEqual(z.read(context),unavailable);if(forgotten&&!retired)assert.deepEqual(z.read(context,'retained-outcome'),unavailable);traceEdges++;
  }
  if(!ran)settled(api,z,requested);traces++;if(trace.length<(short?1:3))for(const event of events)explore([...trace,event],kind);
 }for(const kind of short?['update-body','selection','paste-request']:kinds)explore([],kind);
 if(!short)assert.deepEqual([...seenKinds].sort(),JSON.parse(fs.readFileSync(new URL('./invocation-publication.json',import.meta.url))).jobKinds.toSorted());
 return {scope:catalog.scope,gateCases,contextCases,raceCases,traces,traceEdges,jobForms:short?3:kinds.length,nativeJobKinds:seenKinds.size,controls,...catalog.scopeFlags};
}
function moduleURL(file,m=null,dependencies={}){let raw=fs.readFileSync(new URL(file,import.meta.url),'utf8');if(m){assert(raw.includes(m.from),m.name);raw=raw.replace(m.from,m.to);}raw=raw.replace(/from '(\.\/[^']+)'/g,(_,r)=>`from ${JSON.stringify(dependencies[r]??new URL(r,import.meta.url).href)}`).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,r)=>`new URL(${JSON.stringify(new URL(r,import.meta.url).href)},import.meta.url)`);return `data:text/javascript;base64,${Buffer.from(raw).toString('base64')}#${m?.name??file}`;}
export async function checkInvocationControlMutations(){const mutations=[
 ['owner-control-bypassed',"g?.[k]===true","k==='ownerControlAllowed'||g?.[k]===true"],
 ['current-context-premise-bypassed',"g?.[k]===true","k==='controlContextCurrent'||g?.[k]===true"],
 ['control-scope-premise-bypassed',"g?.[k]===true","k==='controlScopeValid'||g?.[k]===true"],
 ['protected-readiness-bypassed',"g?.[k]===true","k==='protectedControlReady'||g?.[k]===true"],
 ['truthy-control-grant',"g?.[k]===true","g?.[k]"],
 ['complete-context-check-bypassed','!invocationExecutionState(domain,ticket,context,false)','false'],
 ['ordinary-capacity-required','if(!catalog.gates.every','if(g.ordinaryReady!==true||!catalog.gates.every'],
 ['accepted-control-without-native-stop',"return stopInvocation(domain,ticket,g);","return {status:'stop-requested'};"],
 ['late-control-claims-accepted',"return stopInvocation(domain,ticket,g);","stopInvocation(domain,ticket,g);return {status:'stop-requested'};"]
 ].map(([name,from,to])=>({name,file:'invocation-control.mjs',from,to}));
 mutations.push({name:'clipboard-stop-not-passed-to-native-stage',file:'invocation-publication.mjs',from:'stageClipboardInvocation(entry.plan,ticket,before,host,entry.stop)',to:'stageClipboardInvocation(entry.plan,ticket,before,host,false)'},
  {name:'clipboard-stop-fabricates-provider-quiescence',file:'clipboard-products.mjs',from:'p.t.stage=record;',to:'if(stopped)p.t.quiescent=true;p.t.stage=record;'},
  {name:'clipboard-stop-erases-confirmed-provider-fact',file:'invocation-publication.mjs',from:'effect=clipboardInvocationEffect(entry.plan)',to:"effect=entry.stop?'none':clipboardInvocationEffect(entry.plan)"});
 const killed=[],load=async(m=null)=>{
  const cu=moduleURL('clipboard-products.mjs',m?.file==='clipboard-products.mjs'?m:null),ku=moduleURL('invocation-publication.mjs',m?.file==='invocation-publication.mjs'?m:null,{'./clipboard-products.mjs':cu}),dependencies={'./invocation-publication.mjs':ku},hu=moduleURL('history-contexts.mjs',null,dependencies),ou=moduleURL('invocation-observation.mjs',null,dependencies),su=moduleURL('invocation-control.mjs',m?.file==='invocation-control.mjs'?m:null,dependencies);
  return {...original,...await import(cu),...await import(ku),...await import(hu),...await import(ou),...await import(su)};
 };
 checkInvocationControl(await load(),true);for(const m of mutations){let failed=false;const loadedMutationApi=await load(m);try{checkInvocationControl(loadedMutationApi,true);}catch(e){assert.equal(e.code,'ERR_ASSERTION',m.name+': '+e.message);failed=true;}assert(failed,m.name);killed.push(m.name);}return killed;}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkInvocationControl(),mutations:await checkInvocationControlMutations(),mutationBaselineChecked:true},null,2));
