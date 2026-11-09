// Private finite mechanisms for §28; not production schemas/providers or defaults.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const bits=[false,true],clone=structuredClone;

export function retirementClosure(edges,seed) {
  let removed=seed;
  for(let rank=0;rank<3;rank++)for(const [prerequisite,dependent] of edges)
    if(removed&(1<<prerequisite))removed|=1<<dependent;
  return removed;
}
export function retireHistory(edges,seed,pins) {
  const removed=retirementClosure(edges,seed);
  return removed&pins?{kind:'refused',removed:0}:{kind:'retired',removed};
}
export function joinTyping(facts) {
  return ['open','sameOrigin','samePlan','sameParticipants','compatible','capacity'].every(k=>facts[k]);
}
export function progressBaseline(facts) {
  return ['permitted','attributed','uncontested','causal','sameDestination','sameContext','providerValid'].every(k=>facts[k])&&!facts.pinned;
}
export function encodeBaseUtf8(text,bom) {
  if(!bom&&text.startsWith('\uFEFF'))return null;
  return Buffer.from((bom?'\uFEFF':'')+text,'utf8');
}
export function decodeBaseUtf8(bytes) {
  // Preserve BOM in decoder output, then consume exactly one explicitly.
  const raw=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes);
  const bom=raw.startsWith('\uFEFF');return {text:bom?raw.slice(1):raw,bom};
}
export function navigate(matches,start,current,direction,wrap) {
  if(!['next','previous'].includes(direction))throw Error('Unsupported direction');
  if(current!==null&&(!Number.isInteger(current)||current<0||current>=matches.length))return {kind:'stale'};
  const index=current!==null?(direction==='next'?current+1:current-1):
    direction==='next'?matches.findIndex(m=>m>=start):matches.findLastIndex(m=>m<=start);
  if(index>=0&&index<matches.length)return {kind:'match',index,wrapped:false};
  if(wrap&&matches.length)return {kind:'match',index:direction==='next'?0:matches.length-1,wrapped:true};
  return {kind:'none'};
}
export function activateContext({valid,compatibleOld,heldOld,space}) {
  if(!valid)return compatibleOld?'retainedOld':'plaintext';
  return heldOld&&!space?'refused':'activated';
}
export function allowNativeCommit({sameSession,sameTarget,currentSource,grant,conflict,cancelled}) {
  return sameSession&&sameTarget&&currentSource&&grant&&!conflict&&!cancelled;
}
export function allowCut({acknowledged,currentSource,grant,unknown}) {
  return acknowledged&&currentSource&&grant&&!unknown;
}
export function structuredPaste(fragments,targets) {
  return fragments.length===targets.length?targets.map((target,i)=>({target,text:fragments[i]})):null;
}
export function captureDiagnostic(s,{enabled,failure,record},capacity,lossMax) {
  const n=clone(s);
  if(!enabled)return n;
  if(failure||n.queue.length>=capacity) {
    if(n.loss===lossMax)n.inexact=true;else n.loss++;
  }else n.queue.push(record);
  return n;
}
export function completeSemantic(f) {
  return f.grant&&f.current&&f.scopeMatches&&f.membershipCurrent&&f.complete;
}
export function commentUpdate(body,explicitDelete,grant) {
  return !grant?{kind:'denied'}:explicitDelete?{kind:'deleted'}:{kind:'retained',body};
}
export function childRange(bodyStart,bodyEnd,from,to) {
  return 0<=from&&from<=to&&to<=bodyEnd-bodyStart?{from:bodyStart+from,to:bodyStart+to}:null;
}
export function normalizeSelections(ranges,primary) {
  // Private fixture: nonempty ranges only; no collapsed affinity/tie claims.
  const sorted=ranges.map((r,i)=>({...r,direction:r.direction??'forward',original:i,primary:i===primary})).sort((a,b)=>a.from-b.from||a.to-b.to);
  const out=[];
  for(const r of sorted) {
    const last=out.at(-1);
    if(last&&r.from<last.to){
      last.to=Math.max(last.to,r.to);
      if(r.primary||(!last.primary&&r.original<last.original))last.direction=r.direction;
      last.original=Math.min(last.original,r.original);last.primary||=r.primary;
    }
    else out.push({...r});
  }
  return {ranges:out.map(({from,to,direction})=>({from,to,direction})),primary:out.findIndex(r=>r.primary)};
}
export function resolvePreference(entries,allowed) {
  if(!entries.length)return {kind:'absent'};
  const rank=Math.max(...entries.map(e=>e.rank));
  const top=entries.filter(e=>e.rank===rank);
  if(top.some(e=>e.value!==top[0].value))return {kind:'conflict'};
  return allowed.includes(top[0].value)?{kind:'value',value:top[0].value}:{kind:'denied'};
}

export function checkBaselineMechanisms() {
  const counts={history:0,grouping:0,causality:0,bom:0,navigation:0,activation:0,native:0,cut:0,
    clipboard:0,diagnostics:0,semantic:0,configuration:0,comments:0,regions:0,selections:0};
  const arcs=[[0,1],[0,2],[1,0],[1,2],[2,0],[2,1]];
  for(let graph=0;graph<64;graph++) {
    const edges=arcs.filter((_,i)=>graph&(1<<i));
    for(let seed=0;seed<8;seed++)for(let pins=0;pins<8;pins++) {
      // Independent oracle: intersect all dependency-closed supersets of the seed.
      const supersets=Array.from({length:8},(_,i)=>i).filter(set=>(set&seed)===seed&&
        edges.every(([a,b])=>!(set&(1<<a))||Boolean(set&(1<<b))));
      const expected=supersets.reduce((a,b)=>a&b,7),actual=retireHistory(edges,seed,pins);
      assert.equal(retirementClosure(edges,seed),expected);
      assert.deepEqual(actual,expected&pins?{kind:'refused',removed:0}:{kind:'retired',removed:expected});counts.history++;
    }
  }
  const allFlags=(names,fn)=>{for(let m=0;m<(1<<names.length);m++)fn(Object.fromEntries(names.map((n,i)=>[n,Boolean(m&(1<<i))])));};
  allFlags(['open','sameOrigin','samePlan','sameParticipants','compatible','capacity'],f=>{
    assert.equal(joinTyping(f),Object.values(f).filter(Boolean).length===6);counts.grouping++;
  });
  allFlags(['permitted','attributed','uncontested','causal','sameDestination','sameContext','providerValid','pinned'],f=>{
    const expected=!f.pinned&&['permitted','attributed','uncontested','causal','sameDestination','sameContext','providerValid'].filter(k=>f[k]).length===7;
    assert.equal(progressBaseline(f),expected);counts.causality++;
  });
  const corpus=['','x','\uFEFF','\uFEFFx','\uFEFF\uFEFFx','a\r\nb\rc\n','é','👩‍💻','\0','x\uFEFF'];
  for(const text of corpus)for(const bom of bits) {
    const encoded=encodeBaseUtf8(text,bom);
    if(!bom&&text.startsWith('\uFEFF'))assert.equal(encoded,null);
    else {assert.deepEqual(decodeBaseUtf8(encoded),{text,bom});assert.deepEqual(encodeBaseUtf8(...Object.values(decodeBaseUtf8(encoded))),encoded);}
    counts.bom++;
  }
  for(const bytes of [[0xff],[0xc0,0xaf],[0xed,0xa0,0x80],[0xf4,0x90,0x80,0x80],[0xe2,0x82]]) {
    assert.throws(()=>decodeBaseUtf8(Buffer.from(bytes)));counts.bom++;
  }
  for(const matches of [[],[0],[1,3],[0,2,4]])for(let start=0;start<6;start++)
    for(const current of [null,...matches.map((_,i)=>i)])for(const direction of ['next','previous'])for(const wrap of bits) {
      const candidates=matches.map((position,index)=>({position,index}));
      const inDirection=candidates.filter(m=>current===null?(direction==='next'?m.position>=start:m.position<=start):
        direction==='next'?m.index>current:m.index<current);
      if(direction==='previous')inDirection.reverse();
      const expected=inDirection.length?{kind:'match',index:inDirection[0].index,wrapped:false}:
        wrap&&matches.length?{kind:'match',index:direction==='next'?0:matches.length-1,wrapped:true}:{kind:'none'};
      assert.deepEqual(navigate(matches,start,current,direction,wrap),expected);counts.navigation++;
    }
  allFlags(['valid','compatibleOld','heldOld','space'],f=>{
    const r=activateContext(f);assert.equal(r,!f.valid?(f.compatibleOld?'retainedOld':'plaintext'):
      f.heldOld&&!f.space?'refused':'activated');counts.activation++;
  });
  allFlags(['sameSession','sameTarget','currentSource','grant','conflict','cancelled'],f=>{
    assert.equal(allowNativeCommit(f),[f.sameSession,f.sameTarget,f.currentSource,f.grant,!f.conflict,!f.cancelled].filter(Boolean).length===6);counts.native++;
  });
  allFlags(['acknowledged','currentSource','grant','unknown'],f=>{
    assert.equal(allowCut(f),[f.acknowledged,f.currentSource,f.grant,!f.unknown].filter(Boolean).length===4);counts.cut++;
  });
  for(let fragments=0;fragments<4;fragments++)for(let targets=0;targets<4;targets++) {
    const f=Array.from({length:fragments},(_,i)=>`f${i}`),t=Array.from({length:targets},(_,i)=>i);
    assert.deepEqual(structuredPaste(f,t),fragments===targets?t.map(i=>({target:i,text:`f${i}`})):null);counts.clipboard++;
  }
  for(const capacity of [0,1,2])for(const lossMax of [1,2])for(let size=0;size<=capacity;size++)
    for(let loss=0;loss<=lossMax;loss++)for(const inexact of bits)for(const enabled of bits)for(const failure of bits) {
      const s={queue:Array(size).fill(0),loss,inexact,authoritative:'committed'},before=clone(s);
      const r=captureDiagnostic(s,{enabled,failure,record:1},capacity,lossMax);
      const dropped=enabled&&(failure||size===capacity);
      assert.equal(r.queue.length,size+(enabled&&!dropped?1:0));
      assert.equal(r.loss,Math.min(loss+(dropped?1:0),lossMax));
      assert.equal(r.inexact,inexact||(dropped&&loss===lossMax));
      assert.equal(r.authoritative,s.authoritative);assert.deepEqual(s,before);counts.diagnostics++;
    }
  allFlags(['grant','current','scopeMatches','membershipCurrent','complete'],f=>{
    assert.equal(completeSemantic(f),Object.values(f).filter(Boolean).length===5);counts.semantic++;
  });
  for(const body of ['','comment','\uFEFF'])for(const explicitDelete of bits)for(const grant of bits) {
    const r=commentUpdate(body,explicitDelete,grant);
    assert.deepEqual(r,!grant?{kind:'denied'}:explicitDelete?{kind:'deleted'}:{kind:'retained',body});counts.comments++;
  }
  for(let bodyStart=1;bodyStart<=3;bodyStart++)for(let length=0;length<=3;length++)
    for(let from=-1;from<=4;from++)for(let to=-1;to<=4;to++) {
      const mapped=childRange(bodyStart,bodyStart+length,from,to);
      const valid=from>=0&&to>=from&&to<=length;
      assert.deepEqual(mapped,valid?{from:bodyStart+from,to:bodyStart+to}:null);counts.regions++;
    }
  const ranges=[];for(let from=0;from<3;from++)for(let to=from+1;to<=3;to++)ranges.push({from,to});
  for(const baseA of ranges)for(const baseB of ranges)for(const primary of [0,1])
    for(const directionA of ['forward','backward'])for(const directionB of ['forward','backward']) {
    const a={...baseA,direction:directionA},b={...baseB,direction:directionB};
    const normalized=normalizeSelections([a,b],primary);
    // Independent two-range interval oracle; adjacent ranges remain separate.
    const overlap=Math.max(a.from,b.from)<Math.min(a.to,b.to);
    const order=a.from<b.from||(a.from===b.from&&a.to<=b.to)?[0,1]:[1,0];
    assert.deepEqual(normalized,overlap?{ranges:[{from:Math.min(a.from,b.from),to:Math.max(a.to,b.to),direction:[a,b][primary].direction}],primary:0}:
      {ranges:order.map(i=>[a,b][i]),primary:order.indexOf(primary)});counts.selections++;
  }
  for(const rankA of [0,1])for(const rankB of [0,1])for(const valueA of [0,1])for(const valueB of [0,1])
    for(const allowed of [[],[0],[1],[0,1]]) {
      const entries=[{rank:rankA,value:valueA},{rank:rankB,value:valueB}],before=clone(entries);
      const r=resolvePreference(entries,allowed);
      const conflict=rankA===rankB&&valueA!==valueB,value=rankB>rankA?valueB:valueA;
      assert.deepEqual(r,conflict?{kind:'conflict'}:allowed.includes(value)?{kind:'value',value}:{kind:'denied'});
      assert.deepEqual(resolvePreference([...entries].reverse(),allowed),r);assert.deepEqual(entries,before);counts.configuration++;
    }
  const controls=[];const control=(id,ok)=>{assert(ok);controls.push(id);};
  control('dependent-pin-refuses-whole-retirement',retireHistory([[0,1]],1,2).kind==='refused');
  control('unrelated-group-preserved',retireHistory([[0,1]],1,0).removed===3);
  control('origin-separates-typing',!joinTyping({open:true,sameOrigin:false,samePlan:true,sameParticipants:true,compatible:true,capacity:true}));
  control('ambiguous-bom-tuple-rejected',encodeBaseUtf8('\uFEFFx',false)===null);
  control('one-bom-not-two-consumed',decodeBaseUtf8(Buffer.from('\uFEFF\uFEFFx')).text==='\uFEFFx');
  control('one-match-wrap-is-one-return',navigate([2],2,0,'next',true).wrapped);
  control('stale-current-match-rejects',navigate([2],2,2,'next',false).kind==='stale');
  control('incompatible-old-service-not-current',activateContext({valid:false,compatibleOld:false,heldOld:true,space:true})==='plaintext');
  control('held-context-exhaustion-refuses',activateContext({valid:true,heldOld:true,space:false})==='refused');
  control('unknown-export-preserves-source',!allowCut({acknowledged:true,currentSource:true,grant:true,unknown:true}));
  control('mismatched-fragments-not-truncated',structuredPaste(['x'],[0,1])===null);
  control('incomplete-rename-not-complete',!completeSemantic({grant:true,current:true,scopeMatches:true,membershipCurrent:true,complete:false}));
  control('loss-does-not-wrap',captureDiagnostic({queue:[],loss:1,inexact:false},{enabled:true,failure:true,record:0},0,1).inexact);
  control('equal-rank-not-open-order',resolvePreference([{rank:0,value:0},{rank:0,value:1}],[0,1]).kind==='conflict');
  control('empty-comment-not-delete',commentUpdate('',false,true).kind==='retained');
  control('crossing-child-range-not-clipped',childRange(2,4,0,3)===null);
  control('adjacent-selections-not-merged',normalizeSelections([{from:0,to:1},{from:1,to:2}],1).ranges.length===2);
  control('nonprimary-union-uses-listed-direction',normalizeSelections([
    {from:2,to:3,direction:'backward'},{from:0,to:3,direction:'forward'},
    {from:4,to:5,direction:'forward'}],2).ranges[0].direction==='backward');
  return {status:'finite-baseline-mechanisms-only',counts,controls};
}

export function validateBaselineLedger(ledger) {
  assert(Array.isArray(ledger));
  assert.equal(ledger.length,48);assert.deepEqual(ledger.map(r=>r.id),Array.from({length:48},(_,i)=>`D${String(i+1).padStart(2,'0')}`));
  const spec=fs.readFileSync(path.join(root,'specification.md'),'utf8');
  const model=fs.readFileSync(path.join(root,'model.md'),'utf8');
  const sections=new Set([...spec.matchAll(/^#{2,4} (\d+(?:\.\d+)*)(?:[ .])/gm)].map(m=>m[1]));
  const observations=new Set([...model.matchAll(/^\| (INS-\d{2})(?:\s*\||:)/gm)].map(m=>m[1]));
  for(const r of ledger) {
    for(const key of ['id','status','contract','evidence','claim','gate','scenario','observation'])
      assert(typeof r[key]==='string'&&r[key].trim().length>0,r.id);
    assert(['baseline-specified','baseline-specified-profile-gated'].includes(r.status));
    assert(sections.has(r.contract),r.id);assert(/^formal\/[a-z0-9-]+\.md$/.test(r.evidence),r.id);
    assert(fs.existsSync(path.join(root,r.evidence)),r.id);assert(r.claim&&r.gate&&r.scenario&&r.observation);
    assert(observations.has(r.observation),r.id);
  }
}
export function checkBaselineLedger() {
  const ledger=JSON.parse(fs.readFileSync(path.join(root,'formal/baseline-decisions.json'),'utf8'));
  validateBaselineLedger(ledger);
  const controls=[];
  for(const [name,mutate] of [
    ['missing-decision',l=>l.pop()],['duplicate-decision',l=>{l[1].id=l[0].id;}],
    ['unknown-contract',l=>{l[0].contract='999.1';}],['unknown-evidence',l=>{l[0].evidence='formal/absent-proof.md';}],
    ['gate-silently-closed',l=>{l[0].status='fully-proven';}],['missing-residual',l=>{l[0].gate='';}],
    ['unknown-observation',l=>{l[0].observation='INS-99';}],['missing-scenario',l=>{l[0].scenario='';}],
    ['object-residual',l=>{l[0].gate={};}],['regex-observation',l=>{l[0].observation='INS-99|.*';}],
    ['regex-contract',l=>{l[0].contract='[0-9]+';}]
  ]){const changed=clone(ledger);mutate(changed);assert.throws(()=>validateBaselineLedger(changed));controls.push(name);}
  return {status:'baseline-ledger-integrity-only',decisions:48,controls,concreteSpecificationClosed:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(process.argv.includes('--ledger')?checkBaselineLedger():checkBaselineMechanisms(),null,2));
