// Private reference for owned preparation and normalized report intake.
// No public handle codec, allocator, authentication or full companion engine.
import assert from 'node:assert/strict';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {decideExact} from './source-edit-contracts.mjs';
import {satisfaction} from './validation-reports.mjs';
const clone=structuredClone;
function freeze(value) {if(value&&typeof value==='object'){for(const child of Object.values(value))freeze(child);Object.freeze(value);}return value;}

export function prepare(registry,descriptor,plan,companions,context) {
  if(registry.entries.length>=registry.capacity||registry.next>registry.last)return {status:'capacity',registry};
  const selector={authority:registry.authority,session:registry.session,serial:registry.next};
  const value=freeze(clone({selector,descriptor,plan,companions,context}));
  return {status:'prepared',selector:clone(selector),registry:{...registry,next:registry.next+1,
    entries:[...registry.entries,value]}};
}
export function lookup(registry,selector) {
  return registry.entries.find(e=>equal(e.selector,selector))??null;
}
export function retire(registry,selector) {
  return {...registry,entries:registry.entries.filter(e=>!equal(e.selector,selector))};
}
export function assertionFor(candidate,claim) {
  return {...clone(claim),subject:{tag:'candidate',selector:clone(candidate.selector)},context:clone(candidate.context)};
}
export function submit(registry,selector,descriptor,context,state,ledger,gates) {
  const c=lookup(registry,selector);
  if(!c||!equal(c.descriptor,descriptor)||!equal(c.context,context)||!gates.interpretation||!gates.retained)
    return {status:'rejected',state};
  for(const claim of c.descriptor.claims)
    if(retainedSatisfaction(assertionFor(c,claim),ledger,context,ledger.issuers)!=='established')return {status:'rejected',state};
  // Complete companion identity/eligibility is supplied, not inferred from text.
  return decideExact(state,c.plan,gates);
}

export function ingestReport(ledger,report,authenticated) {
  if(!authenticated||!ledger.issuers.includes(report.issuer))return {status:'untrusted',ledger};
  // Shape/provenance are supplied; exact required-key membership is checked here.
  if(!ledger.keys.some(k=>equal(k,report.key)))return {status:'unrelated',ledger};
  const prior=ledger.records.find(r=>r.id===report.id&&r.issuer===report.issuer);
  if(prior&&equal(prior,report))return {status:'duplicate',ledger};
  if(prior)return {status:'protocolFault',ledger:{...ledger,fault:true}};
  if(ledger.records.length>=ledger.capacity)return {status:'retentionRequired',ledger:{...ledger,blocked:true}};
  return {status:'retained',ledger:{...ledger,records:[...ledger.records,freeze(clone(report))]}};
}
export function retainedSatisfaction(need,ledger,current,issuers) {
  return ledger.blocked||ledger.fault||!equal(ledger.context,current)?'insufficient':satisfaction(need,ledger.records,current,issuers);
}

const bytes=s=>Array.from(new TextEncoder().encode(s));
const ref={authority:'local',document:'A',incarnation:1,revision:1};
const state={authority:'local',generation:1,documents:[{ref,source:bytes('old')}]};
const plan={writes:[{ref,edits:[{context:1,lo:0,hi:3,expected:bytes('old'),ins:bytes('new')}]}],reads:[],fixtureEditLimit:2};
const context={providerGeneration:1,definitionClosure:1,regionMap:1,issuerPolicy:1};
const claim={kind:'schema',assertion:'fixture-conformance',definition:{name:'fixture',version:1,dialect:'fixture',options:[]},
  target:[['A',0,3]],scope:'fixture-scope',query:'fixture-query',partialCounterexample:false};
const descriptor={name:'fixture-exact',version:1,definitionGeneration:1,claims:[claim]};
const gates={grant:true,scope:true,continuity:true,capacity:true,companions:true,notStopped:true,interpretation:true,retained:true};
const empty=()=>({authority:'local',session:'session-1',capacity:2,next:1,last:3,entries:[]});

export function checkPreparationBoundary() {
  let admissionCases=0;const controls=[];
  const input={descriptor:clone(descriptor),plan:clone(plan),companions:{identity:true},context:clone(context)};
  const issued=prepare(empty(),input.descriptor,input.plan,input.companions,input.context);
  const c=lookup(issued.registry,issued.selector),need=assertionFor(c,claim);
  const yes={id:'r1',key:need,issuer:'accepted',verdict:'conformant',coverage:'complete'};
  const no={id:'r2',key:need,issuer:'accepted',verdict:'nonconformant',coverage:'complete'};
  const asLedger=records=>({keys:[need],context:clone(context),issuers:['accepted','other-accepted'],
    capacity:3,records,fault:false,blocked:false});
  for(let mask=0;mask<256;mask++) {
    const flags=Object.fromEntries(Object.keys(gates).map((k,i)=>[k,!!(mask&(1<<i))]));
    const before=clone({registry:issued.registry,selector:issued.selector,state,yes,flags});
    const result=submit(issued.registry,issued.selector,descriptor,context,state,asLedger([yes]),flags);
    const expected=mask===255?'applied':!(mask&64)||!(mask&128)?'rejected':!(mask&32)?'stopped':'rejected';
    assert.equal(result.status,expected);
    if(expected==='applied')assert.deepEqual(result.state.documents[0].source,bytes('new'));
    else assert.deepEqual(result.state,state);
    assert.deepEqual({registry:issued.registry,selector:issued.selector,state,yes,flags},before);admissionCases++;
  }
  const control=(name,ok)=>{assert(ok,name);controls.push(name);};
  const immutableBefore=clone(c);
  input.plan.writes[0].edits[0].ins[0]=88;input.descriptor.claims=[];input.context.definitionClosure++;
  control('caller-mutation-cannot-change-prepared-value',equal(c,immutableBefore));
  assert.throws(()=>{c.plan.writes[0].edits[0].ins.push(88);});controls.push('owned-value-is-frozen');
  const rejection=(name,registry,selector,desc=descriptor,ctx=context,reports=[yes])=>{
    const before=clone({registry,selector,desc,ctx,state,reports});
    const result=submit(registry,selector,desc,ctx,state,asLedger(reports),gates);
    assert.equal(result.status,'rejected',name);assert.deepEqual(result.state,state);
    assert.deepEqual({registry,selector,desc,ctx,state,reports},before);controls.push(name);
  };
  rejection('unknown-selector',issued.registry,{...issued.selector,serial:99});
  rejection('cross-authority-selector',issued.registry,{...issued.selector,authority:'other'});
  rejection('cross-session-selector',issued.registry,{...issued.selector,session:'other'});
  rejection('redefined-descriptor',issued.registry,issued.selector,{...descriptor,definitionGeneration:2});
  rejection('removed-required-claims',issued.registry,issued.selector,{...descriptor,claims:[]});
  rejection('changed-definition-context',issued.registry,issued.selector,descriptor,{...context,definitionClosure:2});
  rejection('retired-selector',retire(issued.registry,issued.selector),issued.selector);
  rejection('conflicting-validation',issued.registry,issued.selector,descriptor,context,[yes,no]);
  rejection('missing-required-report',issued.registry,issued.selector,descriptor,context,[]);
  const retired=retire(issued.registry,issued.selector),replacement=prepare(retired,descriptor,plan,{},context);
  control('retirement-does-not-reuse-selector',!equal(replacement.selector,issued.selector));
  control('old-selector-cannot-select-replacement',lookup(replacement.registry,issued.selector)===null);
  const full={...issued.registry,capacity:1};
  control('capacity-no-new-candidate',prepare(full,descriptor,plan,{},context).status==='capacity');
  control('serial-exhaustion-no-wrap',prepare({...empty(),next:4},descriptor,plan,{},context).status==='capacity');
  const plain=prepare(empty(),{...descriptor,claims:[]},plan,{},context);
  control('ordinary-source-needs-no-provider-report',submit(plain.registry,plain.selector,
    {...descriptor,claims:[]},context,state,asLedger([]),gates).status==='applied');
  const unrelatedCatalog={unused:{generation:9},selected:descriptor};
  control('unrelated-catalog-change-not-a-dependency',submit(issued.registry,issued.selector,
    unrelatedCatalog.selected,context,state,asLedger([yes]),gates).status==='applied');
  let ledger={...asLedger([]),capacity:2};
  ledger=ingestReport(ledger,yes,true).ledger;
  control('equivalent-duplicate-idempotent',ingestReport(ledger,clone(yes),true).status==='duplicate');
  control('untrusted-cannot-poison-ledger',equal(ingestReport(ledger,no,false).ledger,ledger));
  control('authenticated-inadmissible-issuer-cannot-poison',equal(ingestReport(ledger,{...no,issuer:'excluded'},true).ledger,ledger));
  const clash=ingestReport(ledger,{...yes,verdict:'nonconformant'},true);
  control('same-report-id-different-payload-is-fault',clash.status==='protocolFault'&&clash.ledger.fault);
  control('fault-cannot-leave-apparently-established',retainedSatisfaction(need,clash.ledger,context,['accepted'])==='insufficient');
  ledger=ingestReport(ledger,no,true).ledger;
  control('contradictory-report-retained',ledger.records.length===2&&retainedSatisfaction(need,ledger,context,['accepted'])==='contested');
  const tight=ingestReport({...asLedger([yes]),capacity:1},no,true);
  control('capacity-cannot-hide-negative-report',tight.status==='retentionRequired'&&tight.ledger.blocked&&
    retainedSatisfaction(need,tight.ledger,context,['accepted'])==='insufficient');
  for(const [name,held] of [['conflict',ledger],['capacity',tight.ledger],['identity-fault',clash.ledger]]) {
    const before=clone({held,state});
    const result=submit(issued.registry,issued.selector,descriptor,context,state,held,gates);
    control(`intake-${name}-blocks-publication`,result.status==='rejected');
    assert.deepEqual(result.state,state);assert.deepEqual({held,state},before);
  }
  const unrelated=clone(no);unrelated.key.subject.selector.serial++;
  control('authenticated-wrong-candidate-cannot-poison',equal(ingestReport(ledger,unrelated,true).ledger,ledger));
  const named=ingestReport({...ledger,capacity:3},{...yes,issuer:'other-accepted'},true);
  control('report-identity-namespaced-by-issuer',named.status==='retained');
  const original=clone(ledger);const mutable=clone(yes);mutable.id='r3';
  const retained=ingestReport({...ledger,capacity:3},mutable,true);mutable.key.scope='tampered';
  control('report-payload-captured-owned',retained.ledger.records.at(-1).key.scope===need.scope);
  assert.deepEqual(ledger,original);
  return {status:'owned-preparation-boundary-only',admissionCases,controls,
    publicEncodingClosed:false,fullCompanionRefinement:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(checkPreparationBoundary(),null,2));
