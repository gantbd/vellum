// Private design reference. Not a public schema, policy evaluator or editor.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {normalizeBySort} from './batch-normalization.mjs';
import {validateCanonicalBatch,emitByOrigin,applyRightToLeft} from './text-oracles.mjs';

const bytes=s=>Array.from(new TextEncoder().encode(s));
const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
const identity=(a,b)=>a.authority===b.authority&&a.document===b.document&&a.incarnation===b.incarnation;
const exact=(a,b)=>identity(a,b)&&a.revision===b.revision;
const all=Object.freeze({grant:true,scope:true,continuity:true,capacity:true,companions:true,notStopped:true});

export function readRange(snapshot,request,release) {
  // Suppliers include current recipient/destination/purpose/capture-ceiling checks.
  if(!release.grant||!release.scope||!release.continuity||!release.capacity||!snapshot||
    !exact(snapshot.ref,request.ref))return {status:'unavailable'};
  try {validateCanonicalBatch(snapshot.source,[{lo:request.lo,hi:request.hi,ins:[]}]);}
  catch {return {status:'unavailable'};}
  return {status:'content',ref:structuredClone(snapshot.ref),range:[request.lo,request.hi],
    unit:'utf8-byte',source:snapshot.source.slice(request.lo,request.hi)};
}

export function decideExact(state,request,gates) {
  // State and participants are trusted finite fixtures, not hostile-input decoders.
  const refused=()=>({status:'rejected',state});
  if(!gates.notStopped)return {status:'stopped',state};
  if(!gates.grant||!gates.scope||!gates.continuity||!gates.capacity||!gates.companions)return refused();
  if(!request.writes.length||new Set(request.writes.map(w=>w.ref.document)).size!==request.writes.length)
    return refused();
  if([...request.reads,...request.writes].some(p=>p.ref.authority!==state.authority||
    !state.documents.some(d=>d.live!==false&&exact(d.ref,p.ref))))return refused();
  const prepared=[];
  try {
    for(const w of request.writes) {
      const d=state.documents.find(d=>exact(d.ref,w.ref));
      const edits=normalizeBySort(d.source,w.ref.revision,w.edits,request.fixtureEditLimit);
      prepared.push({d,source:emitByOrigin(d.source,edits)});
    }
  } catch {return refused();}
  // companions=true means validated eligibility AND semantic identity of companions
  // in this source-only fixture, not a generic all-participant identity classifier.
  if(prepared.every(p=>same(p.d.source,p.source)))return {status:'unchanged',state};
  const next=structuredClone(state);next.generation++;
  for(const p of prepared)if(!same(p.d.source,p.source)) {
    const d=next.documents.find(d=>identity(d.ref,p.d.ref));
    d.source=p.source;d.ref.revision++;
  }
  return {status:'applied',state:next};
}

const ref=(document,revision=1)=>({authority:'local',document,incarnation:1,revision});
const fixture=()=>({authority:'local',generation:7,documents:[
  {ref:ref('A'),source:bytes('A🙂B')},{ref:ref('B'),source:bytes('é\r\n')},
  {ref:ref('C'),source:bytes('context')},{ref:ref('U'),source:bytes('unrelated')} ]});
const write=(s,id,lo,hi,text)=>{
  const d=s.documents.find(d=>d.ref.document===id);
  return {ref:structuredClone(d.ref),edits:[{context:d.ref.revision,lo,hi,
    expected:d.source.slice(lo,hi),ins:bytes(text)}]};
};
const proposal=s=>({writes:[write(s,'A',1,5,'é'),write(s,'B',0,2,'X')],
  reads:[{ref:structuredClone(s.documents[2].ref)}],fixtureEditLimit:4});

export function checkSourceEditContracts() {
  let readCases=0,decisionCases=0;
  const controls=[];
  const control=(name,condition)=>{assert(condition,name);controls.push(name);};
  // Independent scalar-slice expectations for every valid range, not byte slicing
  // of the candidate read's output. All release supplier combinations are checked.
  for(const text of ['', 'A🙂B','e\u0301','\uFEFFé\r\n']) {
    const scalars=[...text],positions=[0];
    for(const c of scalars)positions.push(positions.at(-1)+bytes(c).length);
    const snap={ref:ref('A'),source:bytes(text)};
    for(let lo=0;lo<positions.length;lo++)for(let hi=lo;hi<positions.length;hi++)
      for(let mask=0;mask<16;mask++) {
        const release=Object.fromEntries(['grant','scope','continuity','capacity'].map((k,i)=>[k,!!(mask&(1<<i))]));
        const request={ref:ref('A'),lo:positions[lo],hi:positions[hi]};
        const before=structuredClone({snap,request,release});
        const result=readRange(snap,request,release);
        assert.deepEqual(result,mask===15?{status:'content',ref:ref('A'),range:[positions[lo],positions[hi]],
          unit:'utf8-byte',source:bytes(scalars.slice(lo,hi).join(''))}:{status:'unavailable'});
        if(result.status==='content')result.source.push(88); // owned result cannot mutate retained source
        assert.deepEqual({snap,request,release},before);readCases++;
      }
  }
  const state=fixture(),request=proposal(state);
  for(let mask=0;mask<64;mask++) {
    const gates=Object.fromEntries(Object.keys(all).map((k,i)=>[k,!!(mask&(1<<i))]));
    const before=structuredClone({state,request,gates});
    const result=decideExact(state,request,gates);
    const expectedStatus=!(mask&32)?'stopped':mask===63?'applied':'rejected';
    assert.equal(result.status,expectedStatus);
    if(expectedStatus!=='applied')assert.deepEqual(result.state,state);
    else {
      assert.deepEqual(result.state.documents.map(d=>d.source),[bytes('AéB'),bytes('X\r\n'),bytes('context'),bytes('unrelated')]);
      assert.equal(result.state.generation,8);
      assert.deepEqual(result.state.documents.map(d=>d.ref.revision),[2,2,1,1]);
      for(const w of request.writes) {
        const d=state.documents.find(d=>exact(d.ref,w.ref));
        assert.deepEqual(result.state.documents.find(d=>identity(d.ref,w.ref)).source,
          applyRightToLeft(d.source,w.edits));
      }
    }
    assert.deepEqual({state,request,gates},before);decisionCases++;
  }
  const snap=state.documents[0],read={ref:structuredClone(snap.ref),lo:1,hi:5};
  control('read-useful-success',same(readRange(snap,read,all).source,bytes('🙂')));
  for(const [name,changed] of [['revision',{revision:2}],['incarnation',{incarnation:2}],['authority',{authority:'remote'}],['document',{document:'B'}]])
    control(`wrong-${name}-not-retargeted`,readRange(snap,{...read,ref:{...read.ref,...changed}},all).status==='unavailable');
  control('retention-loss-not-empty',readRange(null,read,all).status==='unavailable');
  control('split-scalar-not-rounded',readRange(snap,{...read,lo:2},all).status==='unavailable');
  control('out-of-range-not-clamped',readRange(snap,{...read,hi:99},all).status==='unavailable');
  const checkRefusalFrame=(result,before,s,q)=>{
    assert.equal(result.status,'rejected');
    assert.deepEqual({state:s,request:q,gates:all},before);
    assert.deepEqual(result.state,before.state);
  };
  const invalid=(name,mutate)=>{
    const q=structuredClone(request);mutate(q);
    const before=structuredClone({state,request:q,gates:all});
    checkRefusalFrame(decideExact(state,q,all),before,state,q);controls.push(name);
  };
  invalid('expected-bytes-required',q=>{q.writes[0].edits[0].expected=[];});
  invalid('read-only-dependency-stale',q=>{q.reads[0].ref.revision++;});
  invalid('equal-bytes-not-equal-revision',q=>{q.writes[0].ref.revision++;});
  invalid('cross-authority-not-local',q=>{q.writes[1].ref.authority='remote';});
  invalid('second-participant-invalid-no-partial-write',q=>{q.writes[1].edits[0].ins=[255];});
  invalid('ambiguous-insertion-not-ordered',q=>{q.writes[0].edits.push({context:1,lo:1,hi:1,expected:[],ins:[88]});});
  invalid('duplicate-document-not-separate-transactions',q=>{q.writes.push(structuredClone(q.writes[0]));});
  invalid('exhausted-normalization-no-publication',q=>{q.fixtureEditLimit=0;});
  invalid('edit-incarnation-not-reused',q=>{q.writes[0].ref.incarnation++;});
  invalid('missing-edit-document-not-created',q=>{q.writes[0].ref.document='missing';});
  for(const [name,mutate] of [['generation',s=>{s.generation++;}],['source',s=>{s.documents[0].source.push(88);}]]) {
    const s=structuredClone(state),q=structuredClone(request),before=structuredClone({state:s,request:q,gates:all});
    mutate(s);
    assert.throws(()=>checkRefusalFrame({status:'rejected',state:s},before,s,q));
    controls.push(`refusal-${name}-mutation-detected`);
  }
  const unsorted=proposal(state);
  unsorted.writes[0].edits=[
    {context:1,lo:5,hi:6,expected:bytes('B'),ins:bytes('!')},
    {context:1,lo:0,hi:1,expected:bytes('A'),ins:bytes('X')}];
  const unsortedBefore=structuredClone({state,unsorted});
  const unsortedResult=decideExact(state,unsorted,all);
  assert.deepEqual({state,unsorted},unsortedBefore);
  control('unsorted-original-position-edits-succeed',unsortedResult.status==='applied'&&
    same(unsortedResult.state.documents[0].source,bytes('X🙂!')));
  const later=structuredClone(state);later.generation++;later.documents[3].source=bytes('new unrelated');later.documents[3].ref.revision++;
  const committed=decideExact(later,request,all);
  control('unrelated-publication-not-stale',committed.status==='applied');
  control('unrelated-current-state-preserved',same(committed.state.documents[3],later.documents[3]));
  const stale=structuredClone(later);stale.documents[2].ref.revision++;
  control('read-then-edit-race-rejected',decideExact(stale,request,all).status==='rejected');
  control('historical-read-not-forced-to-current',
    committed.state.documents[0].ref.revision!==snap.ref.revision&&
    same(readRange(snap,read,all).source,bytes('🙂'))&&
    decideExact(committed.state,request,all).status==='rejected');
  const noop={...request,writes:[write(state,'A',1,5,'🙂')]};
  control('validated-noop-no-generation',same(decideExact(later,noop,all),{status:'unchanged',state:later}));
  control('noop-still-authorized',decideExact(later,noop,{...all,grant:false}).status==='rejected');
  control('noop-still-cancellable',decideExact(later,noop,{...all,notStopped:false}).status==='stopped');
  control('companion-proof-not-optional',decideExact(state,request,{...all,companions:false}).status==='rejected');
  return {status:'source-edit-semantic-boundary-only',readCases,decisionCases,controls,
    fullFamilyClosure:false,implementationRefinement:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(checkSourceEditContracts(),null,2));
