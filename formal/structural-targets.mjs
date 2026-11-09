// Private design fixtures: no grammar/schema engine or public request decoder.
import assert from 'node:assert/strict';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {decideExact} from './source-edit-contracts.mjs';

export function sufficientEvidence(operation,evidence,context,suppliers) {
  // required/target/plan/context are trusted captured operation intent, not a
  // caller/provider's authority to choose its own weaker admission descriptor.
  if(!operation.required.length)return true; // source-only work has no interpretation dependency
  if(!suppliers.available||!suppliers.attributed||!suppliers.mapsValid||!suppliers.withinBounds)return false;
  if(!equal(operation.context,context)||operation.required.some(n=>!['syntax','schema','binding'].includes(n.kind)))return false;
  return operation.required.every(need=>evidence.some(e=>e.kind===need.kind&&e.scope===need.scope&&
    e.query===need.query&&e.coverage==='complete'&&equal(e.context,context)&&
    equal(e.candidate,operation.plan)&&equal(e.target,operation.target)));
}
const bytes=s=>Array.from(new TextEncoder().encode(s));
const ref=name=>({authority:'local',document:name,incarnation:1,revision:1});
const gates={grant:true,scope:true,continuity:true,capacity:true,companions:true,notStopped:true};
const suppliers={available:true,attributed:true,mapsValid:true,withinBounds:true};
const context={providerGeneration:1,profileVersion:1,regionMapGeneration:1};
function fixture(text,lo,hi,replacement) {
  const state={authority:'local',generation:3,documents:[{ref:ref('A'),source:bytes(text)},
    {ref:ref('R'),source:bytes('read context')}]};
  const plan={writes:[{ref:ref('A'),edits:[{context:1,lo,hi,expected:state.documents[0].source.slice(lo,hi),
    ins:bytes(replacement)}]}],reads:[{ref:ref('R')}],fixtureEditLimit:2};
  return {state,operation:{plan,context:structuredClone(context),
    target:{ref:ref('A'),spans:[[lo,hi]],interpretation:'fixture-target'},required:[]}};
}
function record(operation,kind) {
  return {kind,scope:'target-scope',query:'fixture-query',coverage:'complete',
    context:structuredClone(context),candidate:structuredClone(operation.plan),target:structuredClone(operation.target)};
}
const need=kind=>({kind,scope:'target-scope',query:'fixture-query'});

export function checkStructuralTargets() {
  let claimCases=0,loweringCases=0;const controls=[];
  // Independent truth-table expectation is set inclusion over three unrelated claims.
  const kinds=['syntax','schema','binding'];
  const f=fixture('name()',0,4,'title');
  for(let required=0;required<8;required++)for(let supplied=0;supplied<8;supplied++) {
    const operation={...f.operation,required:kinds.filter((_,i)=>required&(1<<i)).map(need)};
    const evidence=kinds.filter((_,i)=>supplied&(1<<i)).map(k=>record(operation,k));
    for(let mask=0;mask<16;mask++) {
      const flags=Object.fromEntries(Object.keys(suppliers).map((k,i)=>[k,!!(mask&(1<<i))]));
      const before=structuredClone({operation,evidence,context,flags});
      assert.equal(sufficientEvidence(operation,evidence,context,flags),
        required===0||(mask===15&&(required&supplied)===required));
      assert.deepEqual({operation,evidence,context,flags},before);claimCases++;
    }
  }
  // Literal expectations across prose/code fixtures, not real parser/validator evidence.
  for(const [text,lo,hi,replacement,expected] of [
    ['# Title\r\nbody',2,7,'Heading','# Heading\r\nbody'],
    ['name()',0,4,'title','title()'],
    ['```x\nname\n```',5,9,'title','```x\ntitle\n```']]) {
    const {state,operation}=fixture(text,lo,hi,replacement);operation.required=[need('schema')];
    const evidence=[record(operation,'schema')];
    const before=structuredClone({state,operation,evidence});
    assert(sufficientEvidence(operation,evidence,context,suppliers));
    const result=decideExact(state,operation.plan,gates);
    assert.equal(result.status,'applied');assert.deepEqual(result.state.documents[0].source,bytes(expected));
    assert.deepEqual(result.state.documents[1],state.documents[1]);
    assert.deepEqual({state,operation,evidence},before);loweringCases++;
  }
  const {state,operation}=fixture('# Title',2,7,'Heading');operation.required=[need('schema')];
  const evidence=[record(operation,'schema')];
  const control=(name,condition)=>{assert(condition,name);controls.push(name);};
  const refuse=(name,mutate)=>{
    const o=structuredClone(operation),e=structuredClone(evidence),c=structuredClone(context),s=structuredClone(suppliers);
    mutate(o,e,c,s);const before=structuredClone({o,e,c,s,state});
    assert.equal(sufficientEvidence(o,e,c,s),false,name);
    assert.deepEqual({o,e,c,s,state},before);controls.push(name);
  };
  refuse('syntax-not-schema',(o,e)=>{e[0].kind='syntax';});
  refuse('binding-not-schema',(o,e)=>{e[0].kind='binding';});
  refuse('unknown-claim-not-required-kind',(o,e)=>{e[0].kind='anything';});
  refuse('unknown-required-claim-not-enabled',(o,e)=>{o.required[0].kind='anything';e[0].kind='anything';});
  refuse('wrong-query',(o,e)=>{e[0].query='different';});
  refuse('wrong-scope',(o,e)=>{e[0].scope='elsewhere';});
  refuse('partial-not-complete',(o,e)=>{e[0].coverage='partial';});
  refuse('starting-source-not-candidate',(o,e)=>{e[0].candidate.writes[0].edits[0].ins=bytes('Title');});
  refuse('candidate-changed-after-validation',o=>{o.plan.writes[0].edits[0].ins=bytes('Other');});
  refuse('read-dependency-omitted-after-validation',o=>{o.plan.reads=[];});
  refuse('target-map-changed-after-validation',o=>{o.target.spans=[[0,7]];});
  for(const key of Object.keys(context))refuse(`changed-${key}`,(o,e,c)=>{c[key]++;});
  refuse('fresh-evidence-not-reinterpretation',(o,e,c)=>{c.profileVersion++;e[0].context.profileVersion++;});
  for(const key of Object.keys(suppliers))refuse(`missing-${key}`,(o,e,c,s)=>{s[key]=false;});
  const more=[...evidence,{...record(operation,'syntax'),scope:'broader',coverage:'partial'}];
  control('irrelevant-partial-analysis-not-global-refusal',sufficientEvidence(operation,more,context,suppliers));
  control('source-only-does-not-require-provider',sufficientEvidence({...operation,required:[]},[],context,
    {available:false,attributed:false,mapsValid:false,withinBounds:false}));
  for(const key of ['grant','scope','capacity','notStopped']) {
    const before=structuredClone({state,operation,evidence});
    assert(sufficientEvidence(operation,evidence,context,suppliers));
    const result=decideExact(state,operation.plan,{...gates,[key]:false});
    control(`evidence-not-${key}`,result.status===(key==='notStopped'?'stopped':'rejected'));
    assert.deepEqual(result.state,state);assert.deepEqual({state,operation,evidence},before);
  }
  const changed=structuredClone(state);changed.documents[1].ref.revision++;
  const before=structuredClone(changed);
  assert(sufficientEvidence(operation,evidence,context,suppliers));
  const result=decideExact(changed,operation.plan,gates);
  control('current-source-dependencies-still-required',result.status==='rejected');
  assert.deepEqual(changed,before);assert.deepEqual(result.state,before);
  return {status:'structural-evidence-boundary-only',claimCases,loweringCases,controls,
    providerTruthProved:false,fullOperationClosure:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(checkStructuralTargets(),null,2));
