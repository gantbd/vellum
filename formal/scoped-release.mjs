// Private mediated-release fixture; trusted host contexts, not a security sandbox.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
const clone=structuredClone;
const fields=['source','existence','body','target','author','events'];
const scopeFields=['authority','recipient','document','incarnation','snapshot','purpose','destination','projectionContext','stream'];
const freeze=x=>{if(x&&typeof x==='object'){for(const v of Object.values(x))freeze(v);Object.freeze(x);}return x;};
const unavailable=()=>freeze({kind:'unavailable',source:null,revision:null,records:[],count:null,cursor:null,events:[]});
const profiles=Array.from({length:64},(_,mask)=>Object.fromEntries(fields.map((k,i)=>[k,Boolean(mask&(1<<i))])))
  .filter(p=>p.existence||!(p.body||p.target||p.author||p.events));
const scope=()=>({authority:'local',recipient:'reader',document:'doc',incarnation:1,snapshot:'snapshot-0',
  purpose:'edit',destination:'view',projectionContext:'restricted',stream:'stream-0'});
const context=()=>({...scope(),epoch:0,grant:true,available:true,valid:true,continuous:true});
const world=(hiddenPresent=true,hiddenValue='secret-A')=>({source:'source',revision:'source-v0',
  records:[{id:'visible',body:'comment',target:'point-0',author:'allowed-author',visible:true},
    ...(hiddenPresent?[{id:hiddenValue,body:hiddenValue,target:hiddenValue,author:hiddenValue,visible:false}]:[])],
  events:[{id:'visible',kind:'updated',visible:true},
    ...(hiddenPresent?[{id:hiddenValue,kind:hiddenValue,visible:false}]:[])]});

export function projectRelease(data,p) {
  const visible=p.existence?data.records.filter(r=>r.visible):[];
  return freeze({kind:'data',source:p.source?data.source:null,revision:p.source?data.revision:null,
    records:visible.map(r=>({id:r.id,...(p.body?{body:r.body}:{}),...(p.target?{target:r.target}:{}),
      ...(p.author?{author:r.author}:{})})),count:p.existence?visible.length:null,
    cursor:p.existence?'visible-scope-cursor':null,
    events:p.existence&&p.events?data.events.filter(e=>e.visible).map(e=>({id:e.id,kind:e.kind})):[]});
}
export function prepareRelease(data,ceiling,captured) {
  // Internal engine-owned candidate. Returning this structure to a recipient would disclose it.
  return freeze({data:clone(data),ceiling:clone(ceiling),captured:clone(captured)});
}
export function handoffRelease(prepared,current,approvedProjection) {
  // approvedProjection is supplied by current trusted policy, never raw caller flags.
  if(!prepared||!prepared.captured.grant||!current.grant||!current.available||!current.valid||!current.continuous||
    current.epoch!==prepared.captured.epoch||scopeFields.some(k=>current[k]!==prepared.captured[k]))return unavailable();
  const mask=Object.fromEntries(fields.map(k=>[k,Boolean(prepared.ceiling[k]&&approvedProjection[k])]));
  return projectRelease(prepared.data,mask);
}

const actions=['prepare','revoke','regrant','gap','resync','release'];
const initial=()=>({epoch:0,grant:true,baseline:0,queued:null,delivered:false,lastHandoff:'none'});
export function streamStep(s,action) {
  const n=clone(s);
  n.lastHandoff='none';
  switch(action) {
    case 'prepare':if(s.grant&&s.baseline===s.epoch&&s.queued===null)n.queued=s.epoch;break;
    case 'revoke':if(s.grant&&s.epoch===0){n.grant=false;n.epoch=1;n.baseline=-1;}break;
    case 'regrant':if(!s.grant&&s.epoch===1){n.grant=true;n.epoch=2;n.baseline=-1;}break;
    case 'gap':n.baseline=-1;n.queued=null;break;
    case 'resync':if(s.grant)n.baseline=s.epoch;break;
    case 'release':if(s.queued!==null){
      const allowed=s.grant&&s.baseline===s.epoch&&s.queued===s.epoch;
      n.delivered=s.delivered||allowed;n.queued=null;
      n.lastHandoff=allowed?'data':'refused';
    }break;
    default:throw Error('Unsupported private action');
  }
  return n;
}

function checkStreamObservation(s,action,n) {
  // Inspect this handoff even after any earlier successful delivery.
  const canHandOff=action==='release'&&s.queued!==null&&s.grant&&
    s.baseline===s.epoch&&s.queued===s.epoch;
  const expected=action==='release'&&s.queued!==null?(canHandOff?'data':'refused'):'none';
  assert.equal(n.lastHandoff,expected);
  assert.equal(n.delivered,s.delivered||canHandOff);
}

export function checkScopedRelease() {
  let projectionCases=0,handoffCases=0,frameCases=0;
  for(const ceiling of profiles)for(const requested of profiles) {
    const data=world(),c=context(),before=clone(data),prepared=prepareRelease(data,ceiling,c),saved=clone(prepared);
    const mask=Object.fromEntries(fields.map(k=>[k,ceiling[k]&&requested[k]]));
    // Independent field oracle reads a single public record, not the filtering helper.
    const expected={kind:'data',source:mask.source?'source':null,revision:mask.source?'source-v0':null,
      records:mask.existence?[{id:'visible',...(mask.body?{body:'comment'}:{}),...(mask.target?{target:'point-0'}:{}),
        ...(mask.author?{author:'allowed-author'}:{})}]:[],count:mask.existence?1:null,
      cursor:mask.existence?'visible-scope-cursor':null,
      events:mask.existence&&mask.events?[{id:'visible',kind:'updated'}]:[]};
    const delivered=handoffRelease(prepared,c,requested);assert.deepEqual(delivered,expected);handoffCases++;
    for(const other of [world(false),world(true,'secret-B')]) {
      assert.deepEqual(handoffRelease(prepareRelease(other,ceiling,c),c,requested),expected);projectionCases++;
    }
    const badContexts=[...scopeFields.map(k=>({...c,[k]:`other-${k}`})),{...c,grant:false},{...c,epoch:1},
      {...c,available:false},{...c,valid:false},{...c,continuous:false}];
    for(const current of badContexts){assert.deepEqual(handoffRelease(prepared,current,requested),unavailable());handoffCases++;}
    assert.deepEqual(handoffRelease(null,c,requested),unavailable());handoffCases++;
    assert.deepEqual(handoffRelease(prepareRelease(data,ceiling,{...c,grant:false}),c,requested),unavailable());handoffCases++;
    assert.deepEqual(prepared,saved);assert.deepEqual(data,before);
    // Recipient-owned delivered values stay frozen/as-observed after local revocation.
    const prior=clone(delivered);c.grant=false;c.epoch=1;
    assert.deepEqual(delivered,prior);assert(Object.isFrozen(delivered)&&Object.isFrozen(delivered.records));frameCases++;
  }
  const seen=new Map(),queue=[initial()];let transitions=0;
  seen.set(JSON.stringify(queue[0]),queue[0]);
  for(let i=0;i<queue.length;i++)for(const action of actions) {
    const s=queue[i],before=clone(s),n=streamStep(s,action);transitions++;
    assert.deepEqual(s,before);checkStreamObservation(s,action,n);
    if(action==='revoke'||action==='regrant'||action==='gap')assert.equal(n.delivered,s.delivered);
    const key=JSON.stringify(n);if(!seen.has(key)){seen.set(key,n);queue.push(n);}
  }
  const controls=[];const control=(name,predicate)=>{assert(predicate());controls.push(name);};
  const p=profiles.find(p=>fields.every(k=>p[k])),c=context(),prepared=prepareRelease(world(),p,c);
  control('possession-not-grant',()=>handoffRelease(prepared,{...c,grant:false},p).kind==='unavailable');
  control('recipient-substitution',()=>handoffRelease(prepared,{...c,recipient:'other'},p).kind==='unavailable');
  control('purpose-not-export',()=>handoffRelease(prepared,{...c,purpose:'export'},p).kind==='unavailable');
  control('snapshot-not-latest',()=>handoffRelease(prepared,{...c,snapshot:'snapshot-1'},p).kind==='unavailable');
  control('retention-not-authority',()=>handoffRelease(prepared,{...c,grant:false,available:true},p).count===null);
  control('scope-not-source-revision-only',()=>handoffRelease(prepared,{...c,projectionContext:'rich'},p).kind==='unavailable');
  control('capture-ceiling-not-expanded',()=>handoffRelease(prepareRelease(world(),profiles[0],c),c,p).records.length===0);
  control('hidden-count-not-global',()=>projectRelease(world(),p).count!==world().records.length);
  control('hidden-events-not-global',()=>projectRelease(world(),p).events.length!==world().events.length);
  control('unknown-retention-not-empty',()=>handoffRelease(prepared,{...c,available:false},p).count===null);
  let s=streamStep(initial(),'prepare');s=streamStep(s,'revoke');s=streamStep(s,'regrant');
  control('regrant-does-not-resume',()=>!streamStep(s,'release').delivered);
  control('resync-does-not-refresh-old-queue',()=>!streamStep(streamStep(s,'resync'),'release').delivered);
  s=streamStep(streamStep(initial(),'prepare'),'gap');
  control('gap-blocks-incremental',()=>!streamStep(s,'release').delivered);
  control('gap-resync-does-not-revive-old-queue',()=>!streamStep(streamStep(s,'resync'),'release').delivered);
  s=streamStep(streamStep(streamStep(s,'release'),'resync'),'prepare');
  control('fresh-resync-can-deliver',()=>streamStep(s,'release').delivered);
  let prior=initial();
  for(const action of ['prepare','release','prepare','revoke','regrant','resync'])prior=streamStep(prior,action);
  const refused=streamStep(prior,'release');
  control('prior-success-does-not-mask-stale-handoff',()=>prior.delivered&&refused.lastHandoff==='refused');
  let detected=false;
  try{checkStreamObservation(prior,'release',{...refused,lastHandoff:'data'});}catch{detected=true;}
  control('monitor-detects-faulty-second-emission',()=>detected);
  let rejected=false;try{streamStep(initial(),'surprise');}catch{rejected=true;}
  control('closed-private-vocabulary',()=>rejected);
  return {status:'finite-scoped-release-only',profiles:profiles.length,projectionCases,handoffCases,frameCases,
    streamStates:seen.size,streamTransitions:transitions,controls};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(checkScopedRelease(),null,2));
