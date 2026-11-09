// Bounded observation reference. Scheduling/clock/registry truth are premises.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {step as replayStep} from './replay-reference.mjs';
import {prepareOutcome,releaseOutcome} from './outcome-projection.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./execution-observation.json',import.meta.url),'utf8'));
const copy=structuredClone;
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const unavailable=()=>({status:'unavailable'});
const flags=['publication','control','effect','reasons','details','recovery'];
const ownedWaits=new WeakMap();
const ticket=r=>r.epoch*2+r.generation; // Private two-by-two replay fixture, not a public identity.
export function captureExecutionContext(binding,ceiling){return freeze({binding:copy(binding),ceiling:copy(ceiling)});}
function allowed(world,request,context,current){
  return context&&request&&Object.hasOwn(catalog.variants,request.operation)&&
    exact(request,[...catalog.common,...catalog.variants[request.operation]])&&request.contract===catalog.contract&&
    request.invocation===world.reference&&request.invocation===context.binding.invocation&&world.ticket===context.binding.ticket&&
    world.ticket===ticket(world.replay)&&['pending','done'].includes(world.replay.phase)&&
    request.context===context.binding.context&&request.profile===context.binding.profile&&
    equal(current.binding,context.binding)&&current.granted&&current.retained&&current.live&&current.continuous&&
    current.policy.grant&&context.ceiling.grant&&current.policy.epoch===context.ceiling.epoch&&
    current.policy.recipient===context.ceiling.recipient&&
    (request.operation==='minimal-control-status'?current.controlReady:current.ordinaryReady);
}
function projectedFacts(world,context,current){
  if(!world.replay.retained||!world.facts)return null;
  const expected={applied:'applied',unchanged:'unchanged',rejected:'notApplied'}[world.replay.result];
  if(expected&&world.facts.publication!==expected)return null;
  return releaseOutcome(prepareOutcome(world.facts,context.ceiling),current.policy);
}
export function readExecution(world,request,context,current){
  if(!allowed(world,request,context,current)||request.operation==='bounded-observation-wait')return unavailable();
  let out;
  if(request.operation==='retained-outcome'){
    const outcome=world.replay.phase==='done'?projectedFacts(world,context,current):null;
    if(!outcome)return unavailable();out={status:'outcome',observation:world.observation,outcome};
  }else{
    if(!current.policy.control||!context.ceiling.control)return unavailable();
    out={status:'execution',observation:world.observation,phase:world.replay.phase==='pending'?'pending':'terminal'};
    if(request.operation==='status'&&world.replay.phase==='done'){
      const outcome=projectedFacts(world,context,current);if(outcome)out.outcome=outcome;
    }
  }
  const bound=request.operation==='minimal-control-status'?current.maxControlReplyBytes:current.maxReplyBytes;
  return Buffer.byteLength(JSON.stringify(out),'utf8')<=bound?out:unavailable();
}
export function startExecutionWait(world,request,context,current,owner){
  if(!allowed(world,request,context,current)||request.operation!=='bounded-observation-wait'||
      !Number.isSafeInteger(request.budget)||request.budget<1||request.budget>current.maxWaitSteps||
      !current.waitCapacity||!current.waitClockValid||!current.policy.control||!context.ceiling.control||
      typeof request.after!=='string'||!current.validObservationRefs.includes(request.after))return null;
  const entry=freeze({request:copy(request),context,remaining:request.budget,phase:'waiting',ticket:world.ticket});
  ownedWaits.set(entry,owner);return entry;
}
export function advanceExecutionWait(world,entry,event,current,owner){
  const close=result=>{ownedWaits.delete(entry);return result;};
  const ended=()=>close({status:'observation-ended',entry:null});
  if(!entry||ownedWaits.get(entry)!==owner||!ownedWaits.has(entry))return {status:'unavailable',entry:null};
  if(!['poll','tick','expire','stop'].includes(event))return {status:'unavailable',entry};
  // Protected owner stop/expiry closes observation, never the producer or its obligations.
  if(event==='stop'||event==='expire')return ended();
  if(world.ticket!==entry.ticket)return close({status:'unavailable',entry:null});
  if(event==='tick'){
    if(entry.remaining<=1)return ended();
    const next=freeze({...entry,remaining:entry.remaining-1});ownedWaits.delete(entry);ownedWaits.set(next,owner);
    return {status:'waiting',entry:next};
  }
  const request={...entry.request,operation:'status'};delete request.after;delete request.budget;
  const result=readExecution(world,request,entry.context,current);
  if(result.status==='unavailable')return close({status:'unavailable',entry:null});
  if(world.observation!==entry.request.after||result.phase==='terminal'){
    const reply={status:'observed',result,entry:null};
    return close(Buffer.byteLength(JSON.stringify(reply),'utf8')<=current.maxReplyBytes?reply:{status:'unavailable',entry:null});
  }
  return {status:'waiting',entry};
}

export function checkExecutionObservation(){
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  assert.deepEqual(Object.keys(catalog.variants).sort(),families.find(f=>f.id==='F06').variants.toSorted());
  const initial={epoch:0,generation:0,phase:'free',intent:-1,result:'none',retained:false,grant:true,dispatched:false,
    starts:[0,0,0,0],pubs:[0,0,0,0],dispatches:[0,0,0,0],outstanding:[false,false,false,false]};
  const pending=replayStep(initial,{kind:'submit',ticket:0,intent:0}).state;
  const active=replayStep(pending,{kind:'dispatch'}).state;
  const done=replayStep(active,{kind:'finish',ticket:0,result:'applied'}).state;
  const facts={publication:'applied',control:'done',effect:'unknown',basis:'none',causes:[],participants:[{id:'visible',visible:true},{id:'private',visible:false}],publicCorrelation:'allowed-correlation'};
  const world={reference:'invocation-ref',ticket:0,observation:'observation-1',replay:done,facts:copy(facts)};
  const binding={context:'execution-context',profile:'p',invocation:'invocation-ref',ticket:0,recipient:'reader',purpose:'observe',destination:'view',policy:'policy1',continuity:'continuity1'};
  const policy={...Object.fromEntries(flags.map(k=>[k,true])),grant:true,epoch:1,recipient:'reader'};
  const context=captureExecutionContext(binding,policy);
  const current={binding:copy(binding),granted:true,retained:true,live:true,continuous:true,ordinaryReady:true,controlReady:true,policy:copy(policy),
    maxReplyBytes:4096,maxControlReplyBytes:4096,maxWaitSteps:3,waitCapacity:true,waitClockValid:true,validObservationRefs:['observation-0','observation-1']};
  const request=operation=>({contract:catalog.contract,operation,invocation:'invocation-ref',context:binding.context,profile:'p',
    ...(operation==='bounded-observation-wait'?{after:'observation-0',budget:2}:{})});
  const controls=[],control=(name,condition)=>{assert(condition,name);controls.push(name);};
  const deny=(name,r=request('status'),c=current,w=world,ctx=context)=>control(name,equal(readExecution(w,r,ctx,c),unavailable()));
  let releaseCases=0,projectionCases=0,waitCases=0;
  const gateNames=['granted','retained','live','continuous','ordinaryReady','controlReady'];
  for(const operation of ['status','retained-outcome','minimal-control-status'])for(let mask=0;mask<64;mask++){
    const c={...copy(current),...Object.fromEntries(gateNames.map((k,i)=>[k,!!(mask&(1<<i))]))},r=request(operation),before=copy({world,r,c,context});
    const allowed=!!(mask&1)&&!!(mask&2)&&!!(mask&4)&&!!(mask&8)&&!!(mask&(operation==='minimal-control-status'?32:16));
    const out=readExecution(world,r,context,c);assert.equal(out.status,allowed?(operation==='retained-outcome'?'outcome':'execution'):'unavailable');
    assert.deepEqual({world,r,c,context},before);releaseCases++;
  }
  for(let mask=0;mask<64;mask++){
    const ceiling={...policy,...Object.fromEntries(flags.map((k,i)=>[k,!!(mask&(1<<i))]))},ctx=captureExecutionContext(binding,ceiling);
    const result=readExecution(world,request('retained-outcome'),ctx,current);
    assert.equal(result.outcome.publication,mask&1?'applied':'withheld');
    assert.deepEqual(result.outcome.participants,mask&16?['visible']:[]);
    const other={...copy(world),facts:{...copy(facts),participants:[{id:'visible',visible:true},{id:`private-${mask}`,visible:false}]}};
    assert.deepEqual(readExecution(other,request('retained-outcome'),ctx,current),result);projectionCases++;
  }
  control('minimal-status-uses-reserved-capacity',readExecution(world,request('minimal-control-status'),context,{...current,ordinaryReady:false}).status==='execution');
  control('minimal-status-has-no-rich-outcome',!Object.hasOwn(readExecution(world,request('minimal-control-status'),context,current),'outcome'));
  control('known-publication-survives-write-revocation',readExecution({...world,replay:replayStep(done,{kind:'grant',value:false}).state},request('retained-outcome'),context,current).outcome.publication==='applied');
  const forgotten={...world,replay:replayStep(done,{kind:'forget'}).state};
  deny('forgotten-outcome-not-nonapplication',request('retained-outcome'),current,forgotten);
  control('terminal-bookkeeping-not-full-outcome',readExecution(forgotten,request('status'),context,current).phase==='terminal'&&!Object.hasOwn(readExecution(forgotten,request('status'),context,current),'outcome'));
  control('forgetting-does-not-restart',replayStep(forgotten.replay,{kind:'submit',ticket:0,intent:0}).observation==='unavailable');
  control('forgetting-retains-effect-obligation',forgotten.replay.outstanding[0]&&forgotten.replay.pubs[0]===1);
  const retired={...world,replay:replayStep(done,{kind:'retire'}).state};
  deny('retired-not-new-execution',undefined,current,retired);
  deny('unissued-is-not-rejected',undefined,current,{...world,replay:initial});
  deny('new-generation-cannot-alias',undefined,current,{...world,replay:replayStep(retired.replay,{kind:'next'}).state});
  deny('new-incarnation-cannot-alias',undefined,current,{...world,replay:replayStep(done,{kind:'epoch'}).state});
  const nextGeneration=replayStep(retired.replay,{kind:'next'}).state,nextIncarnation=replayStep(done,{kind:'epoch'}).state;
  deny('pending-reused-slot-cannot-alias',undefined,current,{...world,replay:replayStep(nextGeneration,{kind:'submit',ticket:1,intent:1}).state});
  deny('pending-new-incarnation-cannot-alias',undefined,current,{...world,replay:replayStep(nextIncarnation,{kind:'submit',ticket:2,intent:1}).state});
  deny('unknown-contract',{...request('status'),contract:'execution-observation/2'});
  deny('unknown-operation',{...request('status'),operation:'restart'});
  deny('unknown-field',{...request('status'),callback:'run'});
  deny('wrong-invocation',{...request('status'),invocation:'other'});
  deny('missing-context',Object.fromEntries(Object.entries(request('status')).filter(([k])=>k!=='context')));
  for(const k of Object.keys(binding))deny(`current-${k}`,undefined,{...current,binding:{...binding,[k]:'other'}});
  deny('status-grant-revoked',undefined,{...current,policy:{...policy,grant:false}});
  deny('captured-grant-not-expanded',undefined,current,world,captureExecutionContext(binding,{...policy,grant:false}));
  deny('minimal-needs-control-projection',request('minimal-control-status'),{...current,policy:{...policy,control:false}});
  deny('ordinary-transfer-bound',undefined,{...current,maxReplyBytes:1});
  deny('protected-transfer-bound',request('minimal-control-status'),{...current,maxControlReplyBytes:1});
  const mismatch={...copy(world),facts:{...facts,publication:'notApplied'}};
  deny('contradictory-retained-facts-not-rewritten',request('retained-outcome'),current,mismatch);
  const waitingWorld={...copy(world),observation:'observation-0',replay:active,facts:null},owner={};
  const newEntry=()=>startExecutionWait(waitingWorld,request('bounded-observation-wait'),context,current,owner);
  control('bounded-wait-admitted',newEntry()!==null);
  control('wait-does-not-start-execution',equal(waitingWorld.replay,active));
  for(const event of ['poll','tick','expire','stop']){
    const before=copy(waitingWorld),out=advanceExecutionWait(waitingWorld,newEntry(),event,current,owner);
    assert.deepEqual(waitingWorld,before);assert(waitingWorld.replay.outstanding[0]);
    assert.equal(out.status,event==='poll'||event==='tick'?'waiting':'observation-ended');waitCases++;
  }
  const delivered=newEntry(),observed=advanceExecutionWait(world,delivered,'poll',current,owner);
  control('completion-observed-without-reexecution',observed.status==='observed'&&observed.result.outcome.publication==='applied'&&world.replay.starts[0]===1);
  const bodyBytes=Buffer.byteLength(JSON.stringify(readExecution(world,request('status'),context,current)),'utf8');
  control('complete-wait-reply-bound',advanceExecutionWait(world,newEntry(),'poll',{...current,maxReplyBytes:bodyBytes},owner).status==='unavailable');
  control('delivered-wait-cannot-deliver-twice',advanceExecutionWait(world,delivered,'poll',current,owner).status==='unavailable');
  const oldTick=newEntry(),cutoff=advanceExecutionWait(waitingWorld,oldTick,'tick',current,owner);
  control('finite-wait-rank-decreases',cutoff.entry.remaining===1&&advanceExecutionWait(waitingWorld,cutoff.entry,'tick',current,owner).status==='observation-ended');
  control('old-tick-handle-retired',advanceExecutionWait(world,oldTick,'poll',current,owner).status==='unavailable');
  const stopped=newEntry();
  control('stop-survives-content-revocation',advanceExecutionWait(waitingWorld,stopped,'stop',{...current,granted:false,ordinaryReady:false},owner).status==='observation-ended');
  control('stopped-wait-not-revived',advanceExecutionWait(world,stopped,'poll',current,owner).status==='unavailable');
  const revoked=newEntry();
  control('revoked-wait-cannot-release',advanceExecutionWait(world,revoked,'poll',{...current,granted:false},owner).status==='unavailable');
  control('restored-grant-cannot-revive-closed-wait',advanceExecutionWait(world,revoked,'poll',current,owner).status==='unavailable');
  const expired=newEntry();advanceExecutionWait(waitingWorld,expired,'expire',current,owner);
  control('expired-wait-cannot-release-late-result',advanceExecutionWait(world,expired,'poll',current,owner).status==='unavailable');
  control('copied-wait-not-owned',advanceExecutionWait(world,copy(newEntry()),'poll',current,owner).status==='unavailable');
  control('different-wait-owner',advanceExecutionWait(world,newEntry(),'stop',current,{}).status==='unavailable');
  control('wait-cannot-retarget-reused-slot',advanceExecutionWait({...world,ticket:1},newEntry(),'poll',current,owner).status==='unavailable');
  const unchanged=newEntry();
  control('polling-alone-does-not-prove-time-progress',Array.from({length:4},()=>advanceExecutionWait(waitingWorld,unchanged,'poll',current,owner)).every(r=>r.status==='waiting'&&r.entry.remaining===2));
  for(const [name,r,c] of [['wait-capacity',request('bounded-observation-wait'),{...current,waitCapacity:false}],
    ['wait-zero-budget',{...request('bounded-observation-wait'),budget:0},current],['wait-over-budget',{...request('bounded-observation-wait'),budget:4},current],
    ['unknown-observation',{...request('bounded-observation-wait'),after:'forged'},current],
    ['invalid-wait-clock',request('bounded-observation-wait'),{...current,waitClockValid:false}]])
    control(name,startExecutionWait(waitingWorld,r,context,c,owner)===null);
  const before=copy(world);readExecution(world,request('retained-outcome'),context,current);
  control('read-does-not-discharge-or-mutate',equal(world,before));
  control('diagnostics-not-authority',equal(readExecution(world,request('status'),context,current),readExecution(world,request('status'),context,{...current,diagnosticFailed:true})));
  let traces=0,waitEdges=0;
  const actions=['poll','tick','expire','stop','complete','revoke','restore'];
  const explore=trace=>{
    let w=copy(waitingWorld),c=copy(current),e=newEntry(),closed=false;
    for(const event of trace){
      if(event==='complete'){w.replay=replayStep(w.replay,{kind:'finish',ticket:0,result:'applied'}).state;w.observation='observation-1';w.facts=copy(facts);}
      else if(event==='revoke'||event==='restore')c.granted=event==='restore';
      else{
        const before=copy(w),out=advanceExecutionWait(w,e,event,c,owner);assert.deepEqual(w,before);
        if(closed)assert.equal(out.status,'unavailable');
        if(out.entry)e=out.entry;else closed=true;
        assert(w.replay.starts[0]===1&&w.replay.pubs[0]<=1&&w.replay.outstanding[0]);waitEdges++;
      }
    }
    traces++;
    if(trace.length<4)for(const event of actions)explore([...trace,event]);
  };
  explore([]);
  return {status:'execution-observation-products-only',releaseCases,projectionCases,waitCases,traces,waitEdges,controls,
    actualClockProved:false,physicalAccountingProved:false,fullInvocationRefinementProved:false,implementationRefinement:false};
}
export async function checkObservationMutations(){
  const source=fs.readFileSync(new URL('./execution-observation.mjs',import.meta.url),'utf8');
  const mutants=[
    ['expand-captured-outcome','prepareOutcome(world.facts,context.ceiling)','prepareOutcome(world.facts,current.policy)'],
    ['rich-protected-status',"if(request.operation==='status'&&world.replay.phase==='done')","if(['status','minimal-control-status'].includes(request.operation)&&world.replay.phase==='done')"],
    ['protected-status-needs-ordinary-capacity',"(request.operation==='minimal-control-status'?current.controlReady:current.ordinaryReady)",'current.ordinaryReady'],
    ['forgotten-outcome-still-released','if(!world.replay.retained||!world.facts)return null;','if(!world.facts)return null;'],
    ['closed-wait-still-live','ownedWaits.delete(entry);return result;','return result;'],
    ['ignore-invocation-generation','world.ticket===ticket(world.replay)','true']
  ];
  const killed=[];
  for(const [name,from,to] of mutants){
    assert(source.includes(from),name);let code=source.replace(from,to);
    for(const file of ['replay-reference.mjs','outcome-projection.mjs'])code=code.replace(`'./${file}'`,JSON.stringify(new URL(`./${file}`,import.meta.url).href));
    for(const file of ['execution-observation.json','operation-families.json'])code=code.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(`./${file}`,import.meta.url).href)})`);
    const module=await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`);
    assert.throws(()=>module.checkExecutionObservation(),{code:'ERR_ASSERTION'},name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkExecutionObservation(),mutations:await checkObservationMutations()},null,2));
