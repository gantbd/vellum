// Private normalized report fixtures, not a public decoder or real validator.
import assert from 'node:assert/strict';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';

export function satisfaction(need,reports,current,admissibleIssuers) {
  if(!equal(need.context,current))return 'insufficient';
  const applicable=reports.filter(r=>admissibleIssuers.includes(r.issuer)&&equal(r.key,need));
  const yes=applicable.some(r=>r.verdict==='conformant'&&r.coverage==='complete');
  const no=applicable.some(r=>r.verdict==='nonconformant'&&
    (r.coverage==='complete'||need.partialCounterexample));
  return yes?(no?'contested':'established'):(no?'refuted':'insufficient');
}

const context={providerGeneration:1,definitionClosure:1,regionMap:1,issuerPolicy:1};
function key(subject='candidate',definition='fixture-A') {
  return {kind:'schema',assertion:'fixture-conformance',definition:{name:definition,version:1,dialect:'fixture',options:[]},
    subject:subject==='source'?{tag:'source',sources:[['A',1,1]]}:
      {tag:'candidate',operation:'fixture-op',candidate:'prepared-1',bases:[['A',1,1]]},
    target:[['A',0,4]],scope:'fixture-scope',query:'fixture-query',context:structuredClone(context),partialCounterexample:false};
}
const report=(need,verdict,coverage='complete',issuer='accepted')=>({key:structuredClone(need),verdict,coverage,issuer});
function* permutations(xs) {
  if(!xs.length){yield [];return;}
  for(let i=0;i<xs.length;i++)for(const tail of permutations([...xs.slice(0,i),...xs.slice(i+1)]))yield [xs[i],...tail];
}

export function checkValidationReports() {
  let reportCases=0,orderCases=0;const controls=[];
  // Four independent facts: full positive, partial positive, full negative, unknown.
  // Explicit expected table by bit mask, independent of the matching implementation.
  const expected=['insufficient','established','insufficient','established',
    'refuted','contested','refuted','contested','insufficient','established',
    'insufficient','established','refuted','contested','refuted','contested'];
  for(const subject of ['source','candidate'])for(const definition of ['fixture-A','fixture-B']) {
    const need=key(subject,definition);
    const facts=[report(need,'conformant'),report(need,'conformant','partial'),
      report(need,'nonconformant'),report(need,'indeterminate')];
    for(let mask=0;mask<16;mask++) {
      const reports=facts.filter((_,i)=>mask&(1<<i));
      const before=structuredClone({need,reports,context});
      assert.equal(satisfaction(need,reports,context,['accepted']),expected[mask]);reportCases++;
      for(const ordered of permutations(reports)) {
        assert.equal(satisfaction(need,ordered,context,['accepted']),expected[mask]);
        assert.equal(satisfaction(need,[...ordered,...ordered],context,['accepted']),expected[mask]);orderCases++;
      }
      assert.deepEqual({need,reports,context},before);
    }
  }
  const need=key(),yes=report(need,'conformant'),no=report(need,'nonconformant');
  const control=(name,condition)=>{assert(condition,name);controls.push(name);};
  const unmatched=(name,mutate)=>{
    const r=structuredClone(yes);mutate(r);const before=structuredClone({need,r,context});
    assert.equal(satisfaction(need,[r],context,['accepted']),'insufficient',name);
    assert.deepEqual({need,r,context},before);controls.push(name);
  };
  unmatched('source-not-candidate',r=>{r.key.subject=key('source').subject;});
  unmatched('wrong-schema-identity',r=>{r.key.definition.name='fixture-B';});
  unmatched('wrong-schema-version',r=>{r.key.definition.version++;});
  unmatched('wrong-dialect',r=>{r.key.definition.dialect='other';});
  unmatched('wrong-validation-options',r=>{r.key.definition.options=['coerce'];});
  unmatched('wrong-assertion',r=>{r.key.assertion='another';});
  unmatched('wrong-kind',r=>{r.key.kind='syntax';});
  unmatched('wrong-query',r=>{r.key.query='another';});
  unmatched('wrong-scope',r=>{r.key.scope='another';});
  unmatched('wrong-target',r=>{r.key.target=[['A',0,5]];});
  unmatched('wrong-candidate',r=>{r.key.subject.candidate='prepared-2';});
  unmatched('wrong-base-revision',r=>{r.key.subject.bases[0][2]++;});
  unmatched('wrong-issuer',r=>{r.issuer='not-accepted';});
  const newer={...context,definitionClosure:2};
  control('stale-definition-closure',satisfaction(need,[yes],newer,['accepted'])==='insufficient');
  control('issuer-policy-change-not-conflict-erasure',satisfaction(need,[yes,no],
    {...context,issuerPolicy:2},[])==='insufficient');
  control('valid-then-invalid-not-success',satisfaction(need,[yes,no],context,['accepted'])==='contested');
  control('invalid-then-valid-not-success',satisfaction(need,[no,yes],context,['accepted'])==='contested');
  control('untrusted-negative-not-veto',satisfaction(need,[yes,{...no,issuer:'not-accepted'}],context,['accepted'])==='established');
  const otherNegative=structuredClone(no);otherNegative.key.definition.name='fixture-B';
  control('unrelated-negative-not-veto',satisfaction(need,[yes,otherNegative],context,['accepted'])==='established');
  const partialNo=report(need,'nonconformant','partial');
  control('unsupported-partial-counterexample-insufficient',satisfaction(need,[partialNo],context,['accepted'])==='insufficient');
  control('unsupported-partial-counterexample-not-veto',satisfaction(need,[yes,partialNo],context,['accepted'])==='established');
  const counterexampleNeed={...need,partialCounterexample:true};
  control('declared-partial-counterexample-refutes',satisfaction(counterexampleNeed,
    [report(counterexampleNeed,'nonconformant','partial')],context,['accepted'])==='refuted');
  control('indeterminate-not-nonconformance',satisfaction(need,[report(need,'indeterminate')],context,['accepted'])==='insufficient');
  control('partial-conformant-not-complete',satisfaction(need,[report(need,'conformant','partial')],context,['accepted'])==='insufficient');
  return {status:'normalized-validation-report-only',reportCases,orderCases,controls,
    publicSchemaClosed:false,providerTruthProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(checkValidationReports(),null,2));
