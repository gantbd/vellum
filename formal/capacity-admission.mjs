// D06 finite accounting reference. Shapes/units/limits are private fixtures.
import assert from 'node:assert/strict';
const scopes=['host','origin'];
const dimensions=['bytes','slots','work','fanout'];
const units={bytes:'accounted-byte',slots:'slot',work:'step',fanout:'child'};
const kinds={bytes:'occupancy',slots:'occupancy',work:'consumed',fanout:'consumed'};
const natural=n=>Number.isSafeInteger(n)&&n>=0;
const exact=(value,keys)=>value&&typeof value==='object'&&!Array.isArray(value)&&
  Object.keys(value).sort().join(',')===[...keys].sort().join(',');
const copy=structuredClone;
export function validateCapacityProfile(profile) {
  assert(exact(profile,['scope','deadlineTicks']),'incomplete private profile');
  assert(natural(profile.deadlineTicks)&&profile.deadlineTicks>0,'invalid private time bound');
  assert(exact(profile.scope,scopes),'incomplete private scope set');
  for(const scope of scopes) {
    const p=profile.scope[scope];
    assert(exact(p,['hard','protectedSlots','softBytes']),'incomplete private scope');
    assert(exact(p.hard,dimensions),'incomplete private coordinates');
    for(const d of dimensions) {
      const c=p.hard[d];
      assert(exact(c,['unit','kind','ceiling'])&&c.unit===units[d]&&c.kind===kinds[d]&&natural(c.ceiling),'invalid private coordinate');
    }
    assert(natural(p.protectedSlots)&&p.protectedSlots>=1&&p.protectedSlots<=p.hard.slots.ceiling,'invalid private protected floor');
    assert(natural(p.softBytes)&&p.softBytes<=p.hard.bytes.ceiling,'invalid private soft target');
  }
}
const fits=(p,u)=>exact(u,[...dimensions,'completions'])&&dimensions.every(d=>natural(u[d])&&u[d]<=p.hard[d].ceiling)&&
  natural(u.completions)&&u.completions<=p.protectedSlots&&u.slots<=p.hard.slots.ceiling-p.protectedSlots;
function validateState(profile,state) {
  validateCapacityProfile(profile);
  assert(exact(state.used,scopes)&&scopes.every(s=>fits(profile.scope[s],state.used[s])),'invalid private ledger');
}
export function admitCapacity(profile,state,demand,authorized=true) {
  validateState(profile,state);
  assert(exact(demand,[...dimensions,'completions']),'incomplete private demand');
  if(!authorized)return {status:'denied',state:copy(state)};
  if(Object.values(demand).some(v=>v===null))return {status:'unknownDemand',state:copy(state)};
  assert(Object.values(demand).every(natural),'invalid private demand');
  const accepts=scopes.every(s=>{
    const p=profile.scope[s],u=state.used[s];
    return dimensions.every(d=>demand[d]<=p.hard[d].ceiling-u[d])&&
      demand.slots<=p.hard.slots.ceiling-p.protectedSlots-u.slots&&
      demand.completions<=p.protectedSlots-u.completions;
  });
  if(!accepts)return {status:'capacity',state:copy(state)};
  const next=copy(state);
  for(const s of scopes)for(const d of [...dimensions,'completions'])next.used[s][d]+=demand[d];
  validateState(profile,next);
  return {status:'admitted',state:next};
}
export function activateCapacity(active,state,candidate,authorized=true) {
  validateState(active,state);
  const unchanged=status=>({status,profile:copy(active),state:copy(state)});
  if(!authorized)return unchanged('denied');
  try {validateCapacityProfile(candidate);}catch{return unchanged('invalidProfile');}
  if(!scopes.every(s=>fits(candidate.scope[s],state.used[s])))return unchanged('incompatible');
  return {status:'activated',profile:copy(candidate),state:copy(state)};
}
export function planOptionalBytes(profile,state,requested) {
  validateState(profile,state);assert(natural(requested),'invalid private optional request');
  const bound=Math.min(requested,...scopes.map(s=>Math.max(0,
    Math.min(profile.scope[s].softBytes,profile.scope[s].hard.bytes.ceiling)-state.used[s].bytes)));
  return {bound,budgetCoverage:bound<requested?'reduced':'completeBudget'};
}
const record=(bytes,slots,work,fanout)=>({hard:Object.fromEntries(dimensions.map((d,i)=>
  [d,{unit:units[d],kind:kinds[d],ceiling:[bytes,slots,work,fanout][i]}])),protectedSlots:1,softBytes:bytes});
function* records() {
  for(let b=0;b<=2;b++)for(let s=1;s<=3;s++)for(let w=0;w<=2;w++)for(let f=0;f<=2;f++)yield record(b,s,w,f);
}
const profile=(host,origin)=>({scope:{host:copy(host),origin:copy(origin)},deadlineTicks:1});
function ledger(p,mode) {
  return {used:Object.fromEntries(scopes.map(s=>{
    const c=p.scope[s];
    return [s,{bytes:mode===0?0:c.hard.bytes.ceiling,
      slots:mode===1?c.hard.slots.ceiling-c.protectedSlots:0,
      work:mode===0?0:c.hard.work.ceiling,fanout:mode===0?0:c.hard.fanout.ceiling,
      completions:mode===2?1:0}];
  })),cancelled:false};
}
const demand=(bytes=0,slots=0,work=0,fanout=0,completions=0)=>({bytes,slots,work,fanout,completions});
const requests=[demand(),demand(1),demand(0,1),demand(0,0,1),demand(0,0,0,1),
  demand(0,0,0,0,1),demand(1,1,1,1,1),demand(2,1,2,2,1)];
function candidates(p) {
  const all=[copy(p)];
  const grown=copy(p);for(const s of scopes)for(const d of dimensions)grown.scope[s].hard[d].ceiling++;
  all.push(grown);
  const byteShrink=copy(p);for(const s of scopes)byteShrink.scope[s].hard.bytes.ceiling=Math.max(0,byteShrink.scope[s].hard.bytes.ceiling-1);
  for(const s of scopes)byteShrink.scope[s].softBytes=Math.min(byteShrink.scope[s].softBytes,byteShrink.scope[s].hard.bytes.ceiling);
  all.push(byteShrink);
  const slotShrink=copy(p);for(const s of scopes)slotShrink.scope[s].hard.slots.ceiling--;
  all.push(slotShrink);
  const floorGrowth=copy(p);for(const s of scopes)floorGrowth.scope[s].protectedSlots++;
  all.push(floorGrowth);
  const softer=copy(p);for(const s of scopes)softer.scope[s].softBytes=0;
  all.push(softer);
  return all;
}
export function checkCapacityAdmission() {
  let profiles=0,admissionCases=0,activationCases=0,optionalCases=0;
  for(const host of records())for(const origin of records()) {
    const p=profile(host,origin);validateCapacityProfile(p);profiles++;
    for(let mode=0;mode<3;mode++) {
      const state=ledger(p,mode),before=copy({p,state});
      for(const request of requests) {
        const result=admitCapacity(p,state,request);
        // Independent prospective-ledger oracle; admission uses headroom subtraction.
        const expected=scopes.every(s=>{
          const c=p.scope[s],u=state.used[s];
          return dimensions.every(d=>u[d]+request[d]<=c.hard[d].ceiling)&&
            u.slots+request.slots<=c.hard.slots.ceiling-c.protectedSlots&&
            u.completions+request.completions<=c.protectedSlots;
        });
        assert.equal(result.status==='admitted',expected,'admission differs from prospective ledger');
        if(expected) {
          validateState(p,result.state);
          for(const s of scopes)assert(result.state.used[s].slots+result.state.used[s].completions<=p.scope[s].hard.slots.ceiling,'protected partition overcommitted');
        } else assert.deepEqual(result.state,state,'rejection changed obligations');
        admissionCases++;
      }
      for(const candidate of candidates(p)) {
        const result=activateCapacity(p,state,candidate);
        let shapeValid=true;try{validateCapacityProfile(candidate);}catch{shapeValid=false;}
        const compatible=shapeValid&&scopes.every(s=>{
          const c=candidate.scope[s],u=state.used[s];
          return dimensions.every(d=>u[d]<=c.hard[d].ceiling)&&
            u.slots+c.protectedSlots<=c.hard.slots.ceiling&&u.completions<=c.protectedSlots;
        });
        assert.equal(result.status,shapeValid?(compatible?'activated':'incompatible'):'invalidProfile','activation lacks compatibility disposition');
        if(result.status==='activated') {
          validateState(result.profile,result.state);assert.deepEqual(result.state,state,'activation rewrote ledger');
        } else {assert.deepEqual(result.state,state);assert.deepEqual(result.profile,p);}
        activationCases++;
      }
      for(const wanted of [0,1,3]) {
        const planned=planOptionalBytes(p,state,wanted);
        assert(planned.bound<=wanted&&natural(planned.bound));
        for(const s of scopes)assert(planned.bound<=p.scope[s].hard.bytes.ceiling-state.used[s].bytes);
        assert.equal(planned.budgetCoverage==='completeBudget',planned.bound===wanted);
        optionalCases++;
      }
      assert.deepEqual({p,state},before,'input profile or ledger mutated');
    }
  }
  const controls=[];
  const detect=(name,condition)=>{assert(condition,`control escaped: ${name}`);controls.push(name);};
  const p=profile(record(2,2,2,2),record(2,2,2,2)),empty=ledger(p,0);
  const rejectProfile=(name,mutate)=>{const invalid=copy(p);mutate(invalid);
    const before=copy(invalid);assert.throws(()=>validateCapacityProfile(invalid));assert.deepEqual(invalid,before);controls.push(name);};
  rejectProfile('missing dimension treated unlimited',x=>delete x.scope.host.hard.bytes);
  rejectProfile('infinite ceiling accepted',x=>x.scope.host.hard.bytes.ceiling=Infinity);
  rejectProfile('unit confusion',x=>x.scope.origin.hard.work.unit='accounted-byte');
  rejectProfile('consumed work treated occupancy',x=>x.scope.host.hard.work.kind='occupancy');
  rejectProfile('unbounded time descriptor',x=>x.deadlineTicks=NaN);
  rejectProfile('protected floor unavailable',x=>x.scope.host.protectedSlots=0);
  rejectProfile('soft target expands hard capacity',x=>x.scope.origin.softBytes=3);
  const originTight=copy(p);originTight.scope.origin.hard.work.ceiling=0;
  detect('host ceiling alone checked',admitCapacity(originTight,empty,demand(0,0,1)).status==='capacity');
  detect('ordinary work borrows completion reserve',admitCapacity(p,empty,demand(0,2)).status==='capacity');
  detect('unknown demand assumed zero',admitCapacity(p,empty,{...demand(),bytes:null}).status==='unknownDemand');
  const first=admitCapacity(p,empty,demand(0,0,2,1)).state;
  detect('children or retries mint work quota',admitCapacity(p,first,demand(0,0,1,1)).status==='capacity');
  const held=ledger(p,2),cancelled={...copy(held),cancelled:true};
  detect('cancel intent releases live obligations',JSON.stringify(cancelled.used)===JSON.stringify(held.used)&&cancelled.used.host.completions===1);
  const shrink=copy(p);shrink.scope.host.hard.bytes.ceiling=1;shrink.scope.host.softBytes=1;
  detect('profile shrink confiscates retained charge',activateCapacity(p,held,shrink).status==='incompatible');
  const floorUp=copy(p);floorUp.scope.host.protectedSlots=2;
  detect('floor growth hides ordinary commitments',activateCapacity(p,ledger(p,1),floorUp).status==='incompatible');
  detect('profile increase without authority',activateCapacity(p,empty,candidates(p)[1],false).status==='denied');
  const big=profile(record(Number.MAX_SAFE_INTEGER,2,2,2),record(Number.MAX_SAFE_INTEGER,2,2,2));
  const bigUsed=ledger(big,0);for(const s of scopes)bigUsed.used[s].bytes=Number.MAX_SAFE_INTEGER-1;
  detect('overflow hides excess reservation',admitCapacity(big,bigUsed,demand(2)).status==='capacity');
  const optional=planOptionalBytes(p,held,1);
  detect('reduced optional budget called complete',optional.bound===0&&optional.budgetCoverage==='reduced');
  const changed=copy(p);for(const s of scopes)changed.scope[s].softBytes=0;
  detect('adaptive target changes admission rights',admitCapacity(p,empty,demand(2)).status===admitCapacity(changed,empty,demand(2)).status);
  return {status:'finite-capacity-admission-only',profiles,admissionCases,activationCases,optionalCases,controls,
    scope:'Same-author finite declared-charge fixtures/two scopes; not measured defaults, actual allocator/RSS/clock/provider/fairness or implementation refinement'};
}
if(process.argv[1]?.endsWith('/capacity-admission.mjs'))console.log(JSON.stringify(checkCapacityAdmission(),null,2));
