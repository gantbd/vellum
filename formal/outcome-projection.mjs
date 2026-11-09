// D05 finite reference. Private labels/precedence are not adopted public codes.
import assert from 'node:assert/strict';
const clone=structuredClone;
const flags=['publication','control','effect','reasons','details','recovery'];
const precedence=['invalid','denied','missing','stale','conflict','capacity','fault'];
const kinds=['applied','unchanged','notApplied','unknown'];
const dense=a=>Array.isArray(a)&&Array.from({length:a.length},(_,i)=>Object.hasOwn(a,i)).every(Boolean);
const freeze=value=>{
  if(value&&typeof value==='object'){for(const child of Object.values(value))freeze(child);Object.freeze(value);}
  return value;
};
const unavailable=()=>freeze({observation:'unavailable',publication:'withheld',control:'withheld',
  effect:'withheld',reasons:['unavailable'],participants:[],count:null,correlation:null,recovery:[]});
function validate(f) {
  assert(typeof f.publicCorrelation==='string','invalid private correlation');
  assert(kinds.includes(f.publication),'invalid private publication');
  assert(['done','stopped','rejected','faulted','observationEnded'].includes(f.control),'invalid private control');
  assert(['none','confirmed','failed','unknown'].includes(f.effect),'invalid private effect');
  assert(['none','dedup','nonapplication','conditional'].includes(f.basis),'invalid private basis');
  assert(!(f.publication==='applied'&&f.control==='rejected'),'contradictory private product');
  assert(f.basis!=='nonapplication'||(f.publication==='notApplied'&&f.effect==='none'),'unsupported private nonapplication basis');
  assert(dense(f.causes)&&f.causes.length<=4&&f.causes.every(c=>precedence.includes(c.code)&&typeof c.visible==='boolean'),'invalid private causes');
  assert(dense(f.participants)&&f.participants.length<=2&&f.participants.every(p=>typeof p.id==='string'&&typeof p.visible==='boolean'),'invalid private participants');
}
function canonicalReasons(f,p) {
  const permitted=new Set(p.reasons?f.causes.filter(c=>c.visible).map(c=>c.code):[]);
  const codes=precedence.filter(code=>permitted.has(code)).slice(0,2);
  return codes.length?codes:['unavailable'];
}
function recovery(f,p) {
  if(!p.recovery)return [];
  // A declared safe basis is an authoritative fixture premise, not a mere reason.
  switch(f.basis) {
    case 'dedup':return ['observe-existing'];
    case 'nonapplication':return ['prepare-new-proposal'];
    case 'conditional':return ['retry-protected-effect'];
    default:return f.effect==='unknown'||f.publication==='unknown'?['reconcile']:['inspect'];
  }
}
export function projectOutcome(f,p) {
  validate(f);
  const participants=p.details?f.participants.filter(x=>x.visible).map(x=>x.id):[];
  return freeze({observation:'available',publication:p.publication?f.publication:'withheld',
    control:p.control?f.control:'withheld',effect:p.effect?f.effect:'withheld',
    reasons:canonicalReasons(f,p),participants,count:p.details?participants.length:null,
    correlation:p.details?f.publicCorrelation:null,recovery:recovery(f,p)});
}
export function prepareOutcome(f,p) {
  validate(f);
  // Owned authoritative fixture facts, not a proposed telemetry capture buffer.
  return freeze({facts:clone(f),ceiling:clone(p),epoch:p.epoch,recipient:p.recipient});
}
export function releaseOutcome(prepared,current) {
  if(!prepared||!current.grant||!prepared.ceiling.grant||current.epoch!==prepared.epoch||
    current.recipient!==prepared.recipient)return unavailable();
  const p={...current};for(const flag of flags)p[flag]=current[flag]&&prepared.ceiling[flag];
  return projectOutcome(prepared.facts,p);
}
const templates=[
  ['applied','done','none'],['applied','done','confirmed'],['applied','done','failed'],
  ['applied','stopped','unknown'],['unchanged','done','none'],['notApplied','rejected','none'],
  ['unknown','observationEnded','unknown'],['unknown','faulted','unknown'],['applied','faulted','unknown']
];
const fact=(triple)=>({publication:triple[0],control:triple[1],effect:triple[2],basis:'none',
  causes:[{code:'capacity',visible:true},{code:'stale',visible:false},{code:'conflict',visible:true}],
  participants:[{id:'visible',visible:true},{id:'private-A',visible:false}],
  publicCorrelation:'allowed-local-correlation',internalTrace:'private-trace-A'});
const policy=mask=>Object.fromEntries([...flags.map((f,i)=>[f,Boolean(mask&(1<<i))]),
  ['grant',true],['epoch',0],['recipient','caller']]);
function* permutations(xs) {
  if(!xs.length){yield [];return;}
  for(let i=0;i<xs.length;i++)for(const tail of permutations([...xs.slice(0,i),...xs.slice(i+1)]))yield [xs[i],...tail];
}
export function checkOutcomeProjection() {
  let pairs=0,orderCases=0,releaseCases=0;const policies=64;
  for(const triple of templates)for(let mask=0;mask<policies;mask++) {
    const p=policy(mask),f=fact(triple),original=clone(f),baseline=projectOutcome(f,p);
    for(const hiddenCause of precedence)for(const hiddenId of ['private-A','private-B']) {
      const other={...clone(f),internalTrace:'private-trace-B'};
      other.causes[1].code=hiddenCause;other.participants[1].id=hiddenId;
      assert.deepEqual(projectOutcome(other,p),baseline,'protected explanation leaked');pairs++;
    }
    // Hidden cause multiplicity/presence must not affect reason counts or kind.
    for(const causes of [f.causes.filter(c=>c.visible),[...f.causes,{code:'missing',visible:false}]])
      {assert.deepEqual(projectOutcome({...f,causes},p),baseline);pairs++;}
    for(const causes of permutations(f.causes)) {
      assert.deepEqual(projectOutcome({...f,causes},p),baseline,'first failure determines output');orderCases++;
    }
    const prepared=prepareOutcome(f,p),saved=clone(prepared);
    assert.deepEqual(releaseOutcome(prepared,p),baseline);
    for(const current of [{...p,grant:false},{...p,epoch:1},{...p,recipient:'other'}]) {
      assert.deepEqual(releaseOutcome(prepared,current),unavailable(),'queued release gate bypass');releaseCases++;
    }
    assert.deepEqual(releaseOutcome(null,p),unavailable(),'absent status guessed as nonapplication');releaseCases++;
    const broad=policy(63),narrow=policy(0);
    assert.deepEqual(releaseOutcome(prepared,broad),baseline,'queued ceiling enlarged');releaseCases++;
    assert.deepEqual(releaseOutcome(prepared,narrow),projectOutcome(f,narrow),'rich queue leaked on narrowing');releaseCases++;
    assert.deepEqual(prepared,saved,'release mutated retained facts');
    assert.deepEqual(f,original,'projection rewrote authoritative facts');
    if(p.publication)assert.equal(baseline.publication,f.publication);
    if(p.effect)assert.equal(baseline.effect,f.effect);
    assert(Object.isFrozen(baseline)&&Object.isFrozen(baseline.reasons)&&Object.isFrozen(baseline.recovery));
  }
  const controls=[];
  const detect=(name,condition)=>{assert(condition,`control escaped: ${name}`);controls.push(name);};
  const f=fact(templates[0]),p=policy(63);
  detect('raw first-failure code leaks',f.causes[1].code!==({...f.causes[1],code:'denied'}).code);
  const onlyHidden={...f,causes:[{code:'missing',visible:false}]};
  const hiddenOther={...onlyHidden,causes:[{code:'denied',visible:false}]};
  detect('top-level hidden reason differs',JSON.stringify(projectOutcome(onlyHidden,p))===JSON.stringify(projectOutcome(hiddenOther,p)));
  detect('internal reason count leaks',projectOutcome(onlyHidden,p).reasons.length===projectOutcome({...onlyHidden,causes:[...onlyHidden.causes,...onlyHidden.causes]},p).reasons.length);
  detect('private participant count leaks',projectOutcome(f,p).count===1&&f.participants.length===2);
  detect('withheld count fabricated as zero',projectOutcome(f,policy(0)).count===null&&unavailable().count===null);
  detect('raw correlation leaks',projectOutcome(f,p).correlation!==f.internalTrace);
  const later=fact(templates[3]),report=projectOutcome(later,p),snapshot=clone(report);later.effect='confirmed';
  detect('later evidence rewrites delivered report',JSON.stringify(report)===JSON.stringify(snapshot)&&
    report.publication==='applied'&&report.effect==='unknown'&&projectOutcome(later,p).effect==='confirmed');
  const committed=fact(templates[2]);
  detect('failed save relabels committed edit',projectOutcome(committed,p).publication==='applied'&&projectOutcome(committed,p).effect==='failed');
  const unknown=fact(templates[6]);
  detect('unknown outcome called not applied',projectOutcome(unknown,p).publication==='unknown');
  detect('absent status invents nonapplication',releaseOutcome(null,p).publication==='withheld'&&releaseOutcome(null,p).observation==='unavailable');
  detect('unknown write blindly retried',projectOutcome(unknown,p).recovery[0]==='reconcile');
  detect('visible reason grants retry',projectOutcome({...unknown,causes:[{code:'capacity',visible:true}]},p).recovery[0]==='reconcile');
  const dedup={...unknown,basis:'dedup'};
  detect('dedup confused with new execution',projectOutcome(dedup,p).recovery[0]==='observe-existing');
  const notApplied={...fact(templates[5]),basis:'nonapplication'};
  detect('known nonapplication confused with same invocation restart',projectOutcome(notApplied,p).recovery[0]==='prepare-new-proposal');
  detect('hidden recovery basis leaks',projectOutcome({...unknown,basis:'conditional'},policy(0)).recovery.length===0);
  assert.throws(()=>projectOutcome({...unknown,basis:'nonapplication'},p),/unsupported private nonapplication basis/);
  controls.push('unsupported nonapplication basis rejected');
  assert.throws(()=>projectOutcome({...unknown,causes:[{code:'arbitrary',visible:true}]},p),/invalid private causes/);
  controls.push('unknown internal code rejected');
  assert.throws(()=>projectOutcome({...committed,control:'rejected'},p),/contradictory private product/);
  controls.push('contradictory admitted product rejected');
  const alias={private:'must remain mutable'};
  assert.throws(()=>projectOutcome({...committed,publicCorrelation:alias},p),/invalid private correlation/);
  detect('correlation alias freezes authoritative input',!Object.isFrozen(alias));
  return {status:'finite-outcome-projection-only',templates:templates.length,policies,pairs,orderCases,releaseCases,controls,
    scope:'Same-author nine fact templates/64 fixed Boolean disclosure profiles; private codes/ranking, oracle policy and retry bases, not full API/provider/security/clock implementation refinement'};
}
if(process.argv[1]?.endsWith('/outcome-projection.mjs'))console.log(JSON.stringify(checkOutcomeProjection(),null,2));
