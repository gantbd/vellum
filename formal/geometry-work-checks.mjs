import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import * as original from './geometry-work.mjs';
import * as geometry from './geometry-products.mjs';
import * as viewport from './viewport-products.mjs';
import {geometryFixtureForChecks,geometryLimitsForChecks as gl} from './geometry-product-checks.mjs';
const copy=structuredClone;
const limits={maxTasks:4,maxOutstanding:2,maxIssued:8,maxRequestBytes:16384,maxEnvelopeBytes:32768,maxHeldBytes:524288};
const launchGates=['authorized','disclosureAllowed','continuous','live','notStopped','providerAvailable','capacity','reservationReady'];
const geometryGates=['authorized','scope','continuous','capacity','live','retained','notStopped'];
function fixture(api=original,options={}){
  const f=geometryFixtureForChecks(),c=f.packet.context,v=f.state.views[0];
  v.viewportGeneration=c.viewportGeneration;v.viewportContext=Object.fromEntries(['unit','configurationGeneration','metricsGeneration','projectionGeneration'].map(k=>[k,c[k]]));
  f.good.viewportContext=copy(v.viewportContext);
  f.provider={authority:'host',id:'measure',incarnation:1};
  f.request={contract:'geometry-work/1',operation:'enabled-provider-measurement',profile:'captured-geometry-work/1',
    origin:{operation:'input-1',effect:'measure-1',attempt:'attempt-1'},provider:copy(f.provider),context:copy(c)};
  f.g={...Object.fromEntries(launchGates.map(k=>[k,true])),provider:copy(f.provider),geometry:f.current,acceptanceAllowed:true,controlAllowed:true};
  f.ingress={authentic:true,evidenceAllowed:true,ingressReady:true,quiescenceAttested:true};
  f.retirement={retirementAllowed:true,retentionReleased:true,deliveryDrained:true};
  f.owner=api.newGeometryWorkRegistry({...limits,...options},gl);assert(f.owner);return f;
}
function admitted(api,f){const a=api.admitGeometryWork(f.owner,f.state,f.request,f.g);assert.equal(a.status,'admitted');return a.task;}
function submitted(api,f){const t=admitted(api,f);assert.equal(api.submitGeometryWork(f.owner,t,f.state,f.g).status,'submitted');return t;}
const event=(f,t,kind='result',fields=kind==='result'?{packet:copy(f.packet)}:{})=>({contract:'geometry-work-evidence/1',task:t,origin:copy(f.request.origin),provider:copy(f.provider),kind,...fields});
const caret={contract:'geometry-query/1',operation:'caret-hit-conversion',request:{kind:'caret',point:{position:0,affinity:'after'}}};
export function checkGeometryWork(api=original,geometryApi=geometry,viewportApi=viewport,mutant=false){
  const catalog=JSON.parse(fs.readFileSync(new URL('./geometry-work.json',import.meta.url),'utf8'));
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  assert(families.find(f=>f.id==='F25').variants.includes('enabled-provider-measurement'));
  assert(families.find(f=>f.id==='F28').variants.includes('enabled-geometry-result'));
  let admissionGateCases=0,dispatchGateCases=0,acceptanceGateCases=0,intakeGateCases=0;
  for(const mask of mutant?[0,255,32512,32767]:Array.from({length:32768},(_,i)=>i)){
    const f=fixture(api),g={...f.g,...Object.fromEntries(launchGates.map((k,i)=>[k,!!(mask&(1<<i))])),geometry:{...f.current,...Object.fromEntries(geometryGates.map((k,i)=>[k,!!(mask&(1<<(8+i)))]))}};
    assert.equal(api.admitGeometryWork(f.owner,f.state,f.request,g).status,mask===32767?'admitted':'unavailable');admissionGateCases++;
    f.owner=api.newGeometryWorkRegistry(limits,gl);const t=admitted(api,f);assert.equal(api.submitGeometryWork(f.owner,t,f.state,g).status,mask===32767?'submitted':'not-submitted');dispatchGateCases++;
  }
  for(const mask of mutant?[0,254,255]:Array.from({length:256},(_,i)=>i)){
    const f=fixture(api),t=submitted(api,f);assert.equal(api.observeGeometryWork(f.owner,event(f,t),f.ingress).status,'recorded');
    const g={...f.g,acceptanceAllowed:!!(mask&1),geometry:{...f.current,...Object.fromEntries(geometryGates.map((k,i)=>[k,!!(mask&(1<<(1+i)))]))}};
    assert.equal(api.acceptGeometryWork(f.owner,t,f.state,g).status,mask===255?'geometry':'unavailable');acceptanceGateCases++;
  }
  for(let mask=0;mask<16;mask++){
    const f=fixture(api),t=submitted(api,f),g={...Object.fromEntries(['authentic','evidenceAllowed','ingressReady','quiescenceAttested'].map((k,i)=>[k,!!(mask&(1<<i))]))};
    assert.equal(api.observeGeometryWork(f.owner,event(f,t,'quiescence'),g).status,mask===15?'quiescent':'unavailable');intakeGateCases++;
  }
  const controls=[],control=(name,ok)=>{assert(ok,name);controls.push(name);};
  const f=fixture(api),initial=copy(f.state),t=submitted(api,f),before=api.geometryWorkUsage(f.owner),receipt=event(f,t);
  control('capture-is-immutable-at-handoff',(()=>{const s=fixture(api),p=admitted(api,s);s.request.origin.effect='changed';const out=api.submitGeometryWork(s.owner,p,s.state,s.g);return out.status==='submitted'&&out.capture.request.origin.effect==='measure-1';})());
  control('source-is-not-published-by-worker',equal(f.state,initial));
  control('duplicate-origin-not-new-task',api.admitGeometryWork(f.owner,f.state,f.request,f.g).status==='unavailable');
  control('submitted-only-once',api.submitGeometryWork(f.owner,t,f.state,f.g).status==='not-submitted');
  control('copied-task-not-owned',api.observeGeometryWork(f.owner,{...receipt,task:copy(t)},f.ingress).status==='unavailable');
  control('copied-registry-not-owned',api.admitGeometryWork(copy(f.owner),f.state,f.request,f.g).status==='unavailable');
  let reads=0;const traced={...receipt};Object.defineProperty(traced,'packet',{enumerable:true,get(){reads++;return f.packet;}});
  control('unauthenticated-packet-not-traversed',api.observeGeometryWork(f.owner,traced,{...f.ingress,authentic:false}).status==='unavailable'&&reads===0);
  const wrongProvider={...receipt,provider:{...f.provider,incarnation:2}};Object.defineProperty(wrongProvider,'packet',Object.getOwnPropertyDescriptor(traced,'packet'));
  control('wrong-provider-before-packet-traversal',api.observeGeometryWork(f.owner,wrongProvider,f.ingress).status==='unavailable'&&reads===0);
  control('wrong-attempt-refuses',api.observeGeometryWork(f.owner,{...receipt,origin:{...receipt.origin,attempt:'other'}},f.ingress).status==='unavailable');
  control('unknown-evidence-kind-refuses',api.observeGeometryWork(f.owner,{...receipt,kind:'guess'},f.ingress).status==='unavailable');
  control('extra-evidence-field-refuses',api.observeGeometryWork(f.owner,{...receipt,extra:1},f.ingress).status==='unavailable');
  control('first-result-is-evidence-not-quiescence',api.observeGeometryWork(f.owner,receipt,f.ingress).status==='recorded'&&equal(api.geometryWorkUsage(f.owner),before));
  const accepted=api.acceptGeometryWork(f.owner,t,f.state,f.g);assert.equal(accepted.status,'geometry');
  control('actual-geometry-query-consumes-result',geometryApi.queryGeometry(f.state,accepted.layout,caret,f.current,gl).status==='carets');
  control('forged-conflict-cannot-quarantine-issued-layout',api.observeGeometryWork(f.owner,{...receipt,provider:{...f.provider,incarnation:2}},f.ingress).status==='unavailable'&&geometryApi.queryGeometry(f.state,accepted.layout,caret,f.current,gl).status==='carets');
  control('acceptance-reuses-owned-layout',api.acceptGeometryWork(f.owner,t,f.state,f.g).layout===accepted.layout);
  control('duplicate-result-does-not-allocate-witness',api.observeGeometryWork(f.owner,receipt,f.ingress).status==='duplicate'&&api.geometryWorkStatus(f.owner,t).witnesses===1);
  const reordered=copy(receipt.packet);reordered.context=Object.fromEntries(Object.entries(reordered.context).reverse());
  control('record-property-order-not-contradiction',api.observeGeometryWork(f.owner,event(f,t,'result',{packet:reordered}),f.ingress).status==='duplicate');
  const target=geometryApi.queryGeometry(f.state,accepted.layout,{contract:'geometry-query/1',operation:'visual-navigation-target',origin:0,unit:'page-down',count:1,goal:null,choices:[null]},f.current,gl);
  const vr={contract:'viewport-edit/1',operation:'viewport-intent',invocation:'reveal',view:copy(f.packet.context.view),document:copy(f.packet.context.document),sourceRevision:1,expectedViewport:1,
    profile:'integer-viewport/1',context:copy(f.state.views[0].viewportContext),intent:{kind:'reveal',layout:accepted.layout,target:target.reference,xAlign:'nearest',yAlign:'nearest',margin:{x:0,y:0}}};
  const vl={maxCoordinate:10000,maxRequestBytes:16384,maxPlanBytes:32768},vp=viewportApi.prepareViewportEdit(f.state,vr,f.good,vl,gl);assert(vp);
  const changed=copy(f.packet);changed.atEnd=false;
  control('authenticated-conflict-retains-two-witnesses',api.observeGeometryWork(f.owner,event(f,t,'result',{packet:changed}),f.ingress).status==='conflict'&&api.geometryWorkStatus(f.owner,t).witnesses===2);
  control('conflict-not-current-geometry',api.acceptGeometryWork(f.owner,t,f.state,f.g).status==='unavailable');
  control('previously-issued-layout-is-revoked',geometryApi.queryGeometry(f.state,accepted.layout,caret,f.current,gl).status==='unavailable');
  control('previously-prepared-reveal-is-revoked',viewportApi.publishViewportEdit(f.state,vp,f.good,vl,gl).state===f.state);
  control('third-distinct-witness-refuses-without-growth',api.observeGeometryWork(f.owner,event(f,t,'failure',{reason:'fault'}),f.ingress).status==='evidence-full'&&equal(api.geometryWorkUsage(f.owner),before));
  control('evidence-duplicate-remains-idempotent-after-conflict',api.observeGeometryWork(f.owner,receipt,f.ingress).status==='duplicate');
  control('conflict-does-not-prove-quiescence',api.geometryWorkStatus(f.owner,t).workHeld);
  control('cannot-retire-outstanding-task',api.retireGeometryWork(f.owner,t,f.retirement).status==='unavailable');
  control('attested-quiescence-releases-only-work-slot',api.observeGeometryWork(f.owner,event(f,t,'quiescence'),f.ingress).status==='quiescent'&&api.geometryWorkUsage(f.owner).outstanding===0&&api.geometryWorkUsage(f.owner).heldBytes===before.heldBytes);
  control('duplicate-quiescence-idempotent',api.observeGeometryWork(f.owner,event(f,t,'quiescence'),f.ingress).status==='duplicate');
  for(const key of Object.keys(f.retirement))control(`retirement-${key}`,api.retireGeometryWork(f.owner,t,{...f.retirement,[key]:false}).status==='unavailable');
  control('retirement-discharges-capture-and-evidence',api.retireGeometryWork(f.owner,t,f.retirement).status==='retired'&&equal(api.geometryWorkUsage(f.owner),{tasks:0,outstanding:0,issued:1,heldBytes:0}));
  control('retired-task-does-not-accept-late-event',api.observeGeometryWork(f.owner,receipt,f.ingress).status==='unavailable');
  control('retirement-does-not-recycle-origin',api.admitGeometryWork(f.owner,f.state,f.request,f.g).status==='unavailable');
  for(const operation of ['cancel','supersede'])for(const phase of ['ready','submitted','accepted']){
    const s=fixture(api),p=phase==='ready'?admitted(api,s):submitted(api,s);let layout=null;
    if(phase==='accepted'){api.observeGeometryWork(s.owner,event(s,p),s.ingress);layout=api.acceptGeometryWork(s.owner,p,s.state,s.g).layout;assert(layout);}
    const u=api.geometryWorkUsage(s.owner);
    control(`${operation}-${phase}`,api.controlGeometryWork(s.owner,p,operation,s.g).status==='stopped'&&api.geometryWorkStatus(s.owner,p)[operation==='cancel'?'cancelled':'superseded']);
    control(`${operation}-${phase}-does-not-forget-capture`,api.geometryWorkUsage(s.owner).heldBytes===u.heldBytes);
    control(`${operation}-${phase}-no-current-result`,api.acceptGeometryWork(s.owner,p,s.state,s.g).status==='unavailable');
    if(phase==='ready')control(`${operation}-unsent-cannot-launch`,api.submitGeometryWork(s.owner,p,s.state,s.g).status==='not-submitted'&&api.geometryWorkUsage(s.owner).outstanding===0);
    else{
      control(`${operation}-${phase}-still-outstanding`,api.geometryWorkUsage(s.owner).outstanding===1);
      control(`${operation}-${phase}-late-evidence-retained`,['recorded','duplicate'].includes(api.observeGeometryWork(s.owner,event(s,p),s.ingress).status));
      if(layout)control(`${operation}-revokes-issued-geometry`,geometryApi.queryGeometry(s.state,layout,caret,s.current,gl).status==='unavailable');
    }
  }
  for(const mutate of [s=>s.state.documents[0].revision++,s=>s.state.documents[0].annotationSet++,s=>s.state.views[0].viewportGeneration++,s=>s.state.views[0].viewport.y++,s=>s.g.provider.incarnation++,s=>s.current.context.metricsGeneration++]){
    const s=fixture(api),p=submitted(api,s);mutate(s);api.observeGeometryWork(s.owner,event(s,p),s.ingress);
    control(`stale-result-${controls.length}`,api.acceptGeometryWork(s.owner,p,s.state,s.g).status==='unavailable'&&api.geometryWorkStatus(s.owner,p).witnesses===1&&api.geometryWorkStatus(s.owner,p).workHeld);
  }
  const late=fixture(api),lt=submitted(api,late);api.observeGeometryWork(late.owner,event(late,lt,'quiescence'),late.ingress);api.observeGeometryWork(late.owner,event(late,lt),late.ingress);
  control('quiescence-delivery-before-result-is-not-a-lost-result',api.acceptGeometryWork(late.owner,lt,late.state,late.g).status==='geometry');
  const bad=fixture(api),bt=submitted(api,bad),bp=copy(bad.packet);bp.rows[0].stops[0].position=999;
  api.observeGeometryWork(bad.owner,event(bad,bt,'result',{packet:bp}),bad.ingress);
  control('bounded-receipt-not-accepted-geometry',api.acceptGeometryWork(bad.owner,bt,bad.state,bad.g).status==='unavailable');
  for(const reason of catalog.failures){const s=fixture(api),p=submitted(api,s);control(`failure-${reason}-not-quiescence`,api.observeGeometryWork(s.owner,event(s,p,'failure',{reason}),s.ingress).status==='recorded'&&api.geometryWorkStatus(s.owner,p).workHeld&&api.acceptGeometryWork(s.owner,p,s.state,s.g).status==='unavailable');}
  const tiny=fixture(api,{maxEnvelopeBytes:512}),tt=submitted(api,tiny);
  control('oversized-envelope-does-not-alter-task',api.observeGeometryWork(tiny.owner,event(tiny,tt),tiny.ingress).status==='unavailable'&&api.geometryWorkStatus(tiny.owner,tt).witnesses===0);
  for(const key of ['maxTasks','maxOutstanding','maxIssued','maxRequestBytes','maxHeldBytes']){
    const s=fixture(api,{[key]:0});control(`admission-${key}`,api.admitGeometryWork(s.owner,s.state,s.request,s.g).status==='unavailable');
  }
  const slots=fixture(api,{maxTasks:3,maxOutstanding:1}),st=submitted(api,slots);slots.request.origin.attempt='attempt-2';
  control('cancellation-does-not-manufacture-slot',api.controlGeometryWork(slots.owner,st,'cancel',slots.g).status==='stopped'&&api.admitGeometryWork(slots.owner,slots.state,slots.request,slots.g).status==='unavailable');
  api.observeGeometryWork(slots.owner,event({...slots,request:{...slots.request,origin:{...slots.request.origin,attempt:'attempt-1'}}},st,'quiescence'),slots.ingress);
  control('quiescence-permits-new-attempt-not-old-replay',api.admitGeometryWork(slots.owner,slots.state,slots.request,slots.g).status==='admitted');
  control('negative-profile-refuses',api.newGeometryWorkRegistry({...limits,maxTasks:-1},gl)===null);
  control('overflowed-evidence-reservation-refuses',api.newGeometryWorkRegistry({...limits,maxEnvelopeBytes:Number.MAX_SAFE_INTEGER},gl)===null);
  const missing=copy(limits);delete missing.maxIssued;
  control('missing-profile-coordinate-refuses',api.newGeometryWorkRegistry(missing,gl)===null);
  const unsent=fixture(api),ut=admitted(api,unsent);
  control('unsent-cannot-produce-provider-evidence',api.observeGeometryWork(unsent.owner,event(unsent,ut),unsent.ingress).status==='unavailable');
  control('control-denial-does-not-cancel',api.controlGeometryWork(unsent.owner,ut,'cancel',{controlAllowed:false}).status==='unavailable'&&!api.geometryWorkStatus(unsent.owner,ut).cancelled);
  control('unknown-control-refuses',api.controlGeometryWork(unsent.owner,ut,'retry',unsent.g).status==='unavailable');
  control('whole-source-frame-survives-provider-lifecycle',equal(f.state,initial));
  const active=fixture(api),at=submitted(api,active),ar=event(active,at);api.observeGeometryWork(active.owner,ar,active.ingress);ar.packet.rows[0].stops[0].position=999;
  const al=api.acceptGeometryWork(active.owner,at,active.state,active.g);assert.equal(al.status,'geometry');
  control('intake-copies-bounded-evidence',geometryApi.queryGeometry(active.state,al.layout,caret,active.current,gl).status==='carets');
  api.observeGeometryWork(active.owner,event(active,at,'quiescence'),active.ingress);api.retireGeometryWork(active.owner,at,active.retirement);
  control('retirement-revokes-active-issued-layout',geometryApi.queryGeometry(active.state,al.layout,caret,active.current,gl).status==='unavailable');
  const capture=fixture(api),ct=admitted(api,capture);capture.state.documents[0].source[0]=98;
  control('changed-capture-bytes-refuse-dispatch',api.submitGeometryWork(capture.owner,ct,capture.state,capture.g).status==='not-submitted');
  const once=fixture(api,{maxIssued:1}),ot=admitted(api,once);api.controlGeometryWork(once.owner,ot,'cancel',once.g);api.retireGeometryWork(once.owner,ot,once.retirement);once.request.origin.attempt='new';
  control('retirement-does-not-reset-issuance-ceiling',api.admitGeometryWork(once.owner,once.state,once.request,once.g).status==='unavailable');
  const aggregate=fixture(api,{maxHeldBytes:before.heldBytes}),ag=admitted(api,aggregate);aggregate.request.origin.attempt='new';
  control('aggregate-capture-evidence-charge-is-not-per-task-headroom',!!ag&&api.admitGeometryWork(aggregate.owner,aggregate.state,aggregate.request,aggregate.g).status==='unavailable');
  const noReceipt=fixture(api,{maxEnvelopeBytes:10});
  control('admission-reserves-usable-minimal-completion',api.admitGeometryWork(noReceipt.owner,noReceipt.state,noReceipt.request,noReceipt.g).status==='unavailable');
  const competing=fixture(api),cp=submitted(api,competing);api.observeGeometryWork(competing.owner,event(competing,cp),competing.ingress);
  const cl=api.acceptGeometryWork(competing.owner,cp,competing.state,competing.g);assert.equal(cl.status,'geometry');
  competing.request.origin.attempt='competing';const cq=submitted(api,competing);api.observeGeometryWork(competing.owner,event(competing,cq),competing.ingress);
  control('one-current-accepted-layout-per-view',api.acceptGeometryWork(competing.owner,cq,competing.state,competing.g).status==='unavailable'&&geometryApi.queryGeometry(competing.state,cl.layout,caret,competing.current,gl).status==='carets');
  api.controlGeometryWork(competing.owner,cp,'supersede',competing.g);
  control('explicit-supersession-permits-competing-result',api.acceptGeometryWork(competing.owner,cq,competing.state,competing.g).status==='geometry');
  const replacement=fixture(api),rp=submitted(api,replacement);api.observeGeometryWork(replacement.owner,event(replacement,rp),replacement.ingress);
  const rl=api.acceptGeometryWork(replacement.owner,rp,replacement.state,replacement.g);assert.equal(rl.status,'geometry');
  const oldState=copy(replacement.state),oldCurrent=copy(replacement.current);
  replacement.packet.context.metricsGeneration++;replacement.current.context=copy(replacement.packet.context);replacement.state.views[0].viewportContext.metricsGeneration++;
  replacement.request.context=copy(replacement.packet.context);replacement.request.origin.attempt='next-context';const rq=submitted(api,replacement);api.observeGeometryWork(replacement.owner,event(replacement,rq),replacement.ingress);
  control('new-current-context-can-replace-stale-layout',api.acceptGeometryWork(replacement.owner,rq,replacement.state,replacement.g).status==='geometry');
  control('context-replacement-permanently-retires-old-reference',geometryApi.queryGeometry(oldState,rl.layout,caret,oldCurrent,gl).status==='unavailable');
  // Reachability closure, not a fixed trace horizon: independent monotone oracle.
  const actions=['submit','result','other-result','failure','quiescence','cancel','supersede','accept','retire'];
  const start={phase:'ready',cancelled:false,superseded:false,quiescent:false,witnesses:[],accepted:false,retired:false};
  const key=m=>m.retired?'retired':JSON.stringify({...m,witnesses:[...m.witnesses].sort()});
  const oracle=(model,action)=>{
    const m=copy(model);if(m.retired)return {m,status:action==='submit'?'not-submitted':'unavailable'};
    let status='unavailable';
    if(action==='submit'){status=m.phase==='ready'?'submitted':'not-submitted';if(status==='submitted')m.phase='submitted';}
    else if(['cancel','supersede'].includes(action)){
      const field=action==='cancel'?'cancelled':'superseded';status=m[field]?'unchanged':'stopped';m[field]=true;if(m.phase==='ready')m.phase='closed';
    }else if(action==='accept'){
      if(!m.cancelled&&!m.superseded&&m.witnesses.length===1&&m.witnesses[0]!=='failure'){status='geometry';m.accepted=true;}
    }else if(action==='retire'){
      if(m.phase==='closed'||m.phase==='submitted'&&m.quiescent){status='retired';m.retired=true;}
    }else if(m.phase==='submitted'){
      if(action==='quiescence'){status=m.quiescent?'duplicate':'quiescent';m.quiescent=true;}
      else if(m.witnesses.includes(action))status='duplicate';
      else if(m.witnesses.length===2)status='evidence-full';
      else{m.witnesses.push(action);status=m.witnesses.length===2?'conflict':'recorded';}
    }
    return {m,status};
  };
  const replay=path=>{
    const s=fixture(api),p=admitted(api,s);let m=copy(start),layout=null;
    const charge=api.geometryWorkUsage(s.owner).heldBytes;
    for(const action of path){
      const next=oracle(m,action);let out;
      if(action==='submit')out=api.submitGeometryWork(s.owner,p,s.state,s.g);
      else if(['cancel','supersede'].includes(action))out=api.controlGeometryWork(s.owner,p,action,s.g);
      else if(action==='accept'){out=api.acceptGeometryWork(s.owner,p,s.state,s.g);if(out.status==='geometry')layout=out.layout;}
      else if(action==='retire')out=api.retireGeometryWork(s.owner,p,s.retirement);
      else{const packet=copy(s.packet);if(action==='other-result')packet.atEnd=false;
        out=api.observeGeometryWork(s.owner,event(s,p,action==='failure'?'failure':action==='quiescence'?'quiescence':'result',action==='failure'?{reason:'fault'}:action==='quiescence'?{}:{packet}),s.ingress);}
      assert.equal(out.status,next.status,JSON.stringify(path));m=next.m;
      const u=api.geometryWorkUsage(s.owner);assert.deepEqual(u,{tasks:m.retired?0:1,outstanding:!m.retired&&(m.phase==='ready'||m.phase==='submitted'&&!m.quiescent)?1:0,issued:1,heldBytes:m.retired?0:charge});
      if(m.retired)assert.equal(api.geometryWorkStatus(s.owner,p),null);
      else assert.deepEqual(api.geometryWorkStatus(s.owner,p),{phase:m.phase,cancelled:m.cancelled,superseded:m.superseded,quiescent:m.quiescent,contested:m.witnesses.length>1,witnesses:m.witnesses.length,workHeld:u.outstanding===1,heldBytes:charge});
      if(layout)assert.equal(geometryApi.queryGeometry(s.state,layout,caret,s.current,gl).status,!m.retired&&!m.cancelled&&!m.superseded&&m.witnesses.length===1?'carets':'unavailable');
      assert.deepEqual(s.state,fixture(api).state);
    }
    return m;
  };
  const seen=new Set([key(start)]),queue=[[]];let lifecycleTransitions=0;
  if(!mutant)for(let index=0;index<queue.length;index++)for(const action of actions){
    const path=[...queue[index],action],m=replay(path),k=key(m);lifecycleTransitions++;
    if(!seen.has(k)){seen.add(k);queue.push(path);}
  }
  return {status:'owned-geometry-work-reference-only',admissionGateCases,dispatchGateCases,acceptanceGateCases,intakeGateCases,lifecycleStates:seen.size,lifecycleTransitions,controls,...catalog.scope};
}
export async function checkGeometryWorkMutations(){
  const source=fs.readFileSync(new URL('./geometry-work.mjs',import.meta.url),'utf8'),killed=[];
  const mutations=[
    ['admission-gates-ignored','!launchGates.every(k=>g?.[k])','false'],
    ['dispatch-gates-ignored',"t.superseded||!launchGates.every(k=>g?.[k])",'t.superseded||false'],
    ['origin-replayed','o.used.has(key)','false'],
    ['intake-authentication-ignored','!evidenceGates.every(k=>g?.[k])','false'],
    ['provider-correlation-ignored','!equal(event.provider,t.capture.request.provider)','false'],
    ['attempt-correlation-ignored','!equal(event.origin,t.capture.request.origin)','false'],
    ['quiescence-unattested','if(!g.quiescenceAttested)','if(false)'],
    ['result-implies-quiescence','t.witnesses.push(witness);','t.quiescent=true;t.witnesses.push(witness);'],
    ['cancellation-refunds-slot',"t.phase==='submitted'&&!t.quiescent","t.phase==='submitted'&&!t.quiescent&&!t.cancelled"],
    ['conflict-keeps-issued-layout','t.contested=true;invalidate(t);','t.contested=true;'],
    ['new-layout-on-each-accept','if(t.layout)return','if(false)return'],
    ['unbounded-witnesses','t.witnesses.length>=2','false'],
    ['retirement-before-quiescence','||inWork(t))return','||false)return'],
    ['retirement-keeps-issued-layout','invalidate(t);o.tasks.delete(task);','o.tasks.delete(task);'],
    ['retirement-recycles-attempt','o.tasks.delete(task);','o.used.clear();o.tasks.delete(task);'],
    ['intake-evidence-aliased',"const witness=copy(event.kind==='result'?{kind:'result',packet:event.packet}:{kind:'failure',reason:event.reason});","const witness=event.kind==='result'?{kind:'result',packet:event.packet}:{kind:'failure',reason:event.reason};"],
    ['accept-current-provider-ignored','!g?.acceptanceAllowed||!equal(t.capture.request.provider,g.provider)','!g?.acceptanceAllowed||false'],
    ['aggregate-held-capacity-ignored','charge>o.limits.maxHeldBytes-u.heldBytes','charge>o.limits.maxHeldBytes'],
    ['unusable-minimal-receipt-reservation','size(minimal)>o.limits.maxEnvelopeBytes||catalog.failures.some(reason=>size({...minimal,kind:\'failure\',reason})>o.limits.maxEnvelopeBytes)','false'],
    ['competing-current-layouts','peers.some(other=>geometryLayoutCurrent(state,other.layout,g.geometry,o.geometryLimits))','false'],
    ['context-replacement-keeps-old-reference','for(const peer of peers)invalidate(peer);','/* stale references retained */']
  ];
  for(const [name,from,to] of mutations){
    assert(source.includes(from),name);let code=source.replace(from,to);
    code=code.replace("'./geometry-products.mjs'",JSON.stringify(new URL('./geometry-products.mjs',import.meta.url).href));
    for(const file of ['geometry-work.json','geometry-products.json'])code=code.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(file,import.meta.url).href)})`);
    const api=await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`);
    let failed=false;try{checkGeometryWork(api,geometry,viewport,true);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name);failed=true;}assert(failed,name);killed.push(name);
  }
  let gc=fs.readFileSync(new URL('./geometry-products.mjs',import.meta.url),'utf8');assert(gc.includes('&&!p.retired&&'));
  gc=gc.replace('&&!p.retired&&','&&');
  for(const file of ['text-oracles.mjs','selection-products.mjs'])gc=gc.replace(`'./${file}'`,JSON.stringify(new URL(file,import.meta.url).href));
  gc=gc.replace("new URL('./geometry-products.json',import.meta.url)",`new URL(${JSON.stringify(new URL('./geometry-products.json',import.meta.url).href)})`);
  const gu=`data:text/javascript;base64,${Buffer.from(gc).toString('base64')}#retired-consumer`;
  let vc=fs.readFileSync(new URL('./viewport-products.mjs',import.meta.url),'utf8').replace("'./geometry-products.mjs'",JSON.stringify(gu));
  vc=vc.replace("new URL('./viewport-products.json',import.meta.url)",`new URL(${JSON.stringify(new URL('./viewport-products.json',import.meta.url).href)})`);
  let wc=source.replace("'./geometry-products.mjs'",JSON.stringify(gu));
  for(const file of ['geometry-work.json','geometry-products.json'])wc=wc.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(file,import.meta.url).href)})`);
  const name='retired-layout-consumers-ignore-revocation',ga=await import(gu),va=await import(`data:text/javascript;base64,${Buffer.from(vc).toString('base64')}`),wa=await import(`data:text/javascript;base64,${Buffer.from(wc).toString('base64')}`);
  let failed=false;try{checkGeometryWork(wa,ga,va,true);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name);failed=true;}assert(failed,name);killed.push(name);
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkGeometryWork(),mutations:await checkGeometryWorkMutations()},null,2));
