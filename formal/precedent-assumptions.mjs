// Structural assumption/handoff audit. Does NOT establish assumption truth,
// semantic completeness, induction correctness or concrete composition.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {parametricCases, precedentCases, sliceCases} from './slices.mjs';

const root=path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const read=f=>fs.readFileSync(path.join(root,f),'utf8');
const digest=s=>crypto.createHash('sha256').update(s).digest('hex');
const proofSources=['formal/precedent-proofs.md','formal/precedent-laws.smt2','formal/parametric.smt2','formal/ContextHandoff.tla'];
const claimIds=Array.from({length:9},(_,i)=>`PA-${String(i+1).padStart(2,'0')}`);
const handoffIds=Array.from({length:11},(_,i)=>`PH-${String(i+1).padStart(2,'0')}`);
const nonempty=(value,description)=>{
  if(typeof value!=='string'||!value.trim())throw Error(`Missing ${description}`);
};
const unique=(values,description)=>{
  if(!Array.isArray(values)||values.some(x=>typeof x!=='string')||new Set(values).size!==values.length)
    throw Error(`Invalid or duplicate ${description}`);
};

export function validatePrecedentAssumptions({
  registry=JSON.parse(read('formal/precedent-assumptions.json')),
  model=read('model.md'),
  sources=Object.fromEntries(proofSources.map(f=>[f,read(f)])),
}={}) {
  if(registry.version!==1)throw Error('Unsupported assumption registry version');
  nonempty(registry.scope,'registry scope');
  if(JSON.stringify(Object.keys(registry.sourceDigests||{}).sort())!==JSON.stringify([...proofSources].sort()))
    throw Error('Wrong proof source inventory');
  for(const f of proofSources)
    if(typeof sources[f]!=='string'||registry.sourceDigests[f]!==digest(sources[f]))
      throw Error(`Stale proof source fingerprint: ${f}`);
  const modelIds=new Set([...model.matchAll(/^\| ((?:CMP|ASM)-\d{2}) \|/gm)].map(m=>m[1]));
  const premises=new Map();
  if(!Array.isArray(registry.premises))throw Error('Missing premises');
  for(const p of registry.premises) {
    if(!/^AP-\d{2}$/.test(p.id)||premises.has(p.id))throw Error('Invalid or duplicate premise');
    for(const f of ['statement','boundary','failure','open'])nonempty(p[f],`${p.id} ${f}`);
    if(!['implementation','profile','environment','assurance'].includes(p.kind))throw Error('Unknown premise kind');
    for(const field of ['owners','assumptions'])unique(p[field],`${p.id} ${field}`);
    if(p.kind!=='assurance'&&!p.owners.length)throw Error(`Missing premise owner: ${p.id}`);
    for(const id of p.owners)
      if(!id.startsWith('CMP-')||!modelIds.has(id))throw Error(`Unknown premise owner: ${id}`);
    for(const id of p.assumptions)
      if(!id.startsWith('ASM-')||!modelIds.has(id))throw Error(`Unknown model assumption: ${id}`);
    premises.set(p.id,p);
  }
  if(!Array.isArray(registry.claims))throw Error('Missing claims');
  unique(registry.claims.map(c=>c.id),'claim inventory');
  if(JSON.stringify(registry.claims.map(c=>c.id).sort())!==JSON.stringify(claimIds))
    throw Error('Incomplete claim inventory');
  const cases=new Map([...parametricCases,...precedentCases].map(c=>[c.id,c]));
  const claims=new Map();
  const used=new Set();
  const cited=new Set();
  for(const c of registry.claims) {
    if(c.status!=='conditional-abstract')throw Error(`Unsupported claim promotion: ${c.id}`);
    if(!sources['formal/precedent-proofs.md'].includes(`## ${c.id}:`))throw Error(`Missing proof heading ${c.id}`);
    nonempty(c.limit,`${c.id} limit`);
    unique(c.premises,`${c.id} premises`);
    if(!c.premises.length)throw Error(`Missing claim premises: ${c.id}`);
    for(const id of c.premises){if(!premises.has(id))throw Error(`Unknown premise ${id}`);used.add(id);}
    for(const [field,expected] of [['positive',['sat','unsat']],['controls',['sat']]]) {
      unique(c[field],`${c.id} ${field}`);
      if(!c[field].length)throw Error(`Missing claim ${field}`);
      for(const id of c[field]) {
        if(!cases.has(id))throw Error(`Unknown condition ${id}`);
        if(JSON.stringify(cases.get(id).expected)!==JSON.stringify(expected))throw Error(`Wrong evidence role ${id}`);
        cited.add(id);
      }
    }
    claims.set(c.id,c);
  }
  for(const p of premises.keys())if(!used.has(p))throw Error(`Unused premise ${p}`);
  for(const c of precedentCases)if(!cited.has(c.id))throw Error(`Uncited precedent condition ${c.id}`);
  if(!Array.isArray(registry.handoffs))throw Error('Missing handoffs');
  unique(registry.handoffs.map(h=>h.id),'handoff inventory');
  if(JSON.stringify(registry.handoffs.map(h=>h.id).sort())!==JSON.stringify(handoffIds))
    throw Error('Incomplete handoff inventory');
  const adjacency=new Map(claimIds.map(id=>[id,[]]));
  const unboundSemanticHandoffs=[];
  const boundedHandoffs=[];
  const modelCases=new Map(sliceCases.map(c=>[c.id,c]));
  for(const h of registry.handoffs) {
    if(!claims.has(h.from)||!claims.has(h.to))throw Error(`Unknown handoff endpoint: ${h.id}`);
    for(const f of ['output','input','open'])nonempty(h[f],`${h.id} ${f}`);
    if(h.status!=='open-refinement')throw Error(`Unsupported handoff closure: ${h.id}`);
    unique(h.sharedPremises,`${h.id} shared premises`);
    for(const id of h.sharedPremises)
      if(!claims.get(h.from).premises.includes(id)||!claims.get(h.to).premises.includes(id))
        throw Error(`Unshared handoff premise ${h.id}/${id}`);
    if(!h.sharedPremises.some(id=>premises.get(id).kind!=='assurance'))unboundSemanticHandoffs.push(h.id);
    adjacency.get(h.from).push(h.to);
    if(h.id==='PH-09'&&!h.boundedEvidence)throw Error('Missing bounded handoff evidence');
    if(h.boundedEvidence) {
      const b=h.boundedEvidence,citedModel=new Set();
      nonempty(b.scope,`${h.id} bounded scope`);
      if(b.module!=='ContextHandoff'||!Number.isInteger(b.bound)||b.bound<1)
        throw Error('Unknown bounded model or bound');
      const modelCheck=(id,role)=>{
        const c=modelCases.get(id);
        if(!c||c.module!==b.module)throw Error(`Unknown bounded model check ${id}`);
        if(c.expected!==role)throw Error(`Wrong bounded evidence role ${id}`);
        if(c.constants.Bound!==b.bound)throw Error(`Bounded evidence bound mismatch ${id}`);
        citedModel.add(id);return c;
      };
      if(!Array.isArray(b.positive)||!b.positive.length)throw Error('Missing bounded positive evidence');
      unique(b.positive.map(e=>e.check),'bounded positive checks');
      for(const e of b.positive) {
        const c=modelCheck(e.check,'success');
        unique(e.properties,'bounded properties');
        if(!e.properties.length)throw Error('Missing bounded properties');
        for(const property of e.properties)
          if(![...c.invariants,...c.properties].includes(property))throw Error(`Unknown bounded property ${property}`);
      }
      for(const [field,role] of [['controls','counterexample'],['witnesses','witness']]) {
        unique(b[field],`bounded ${field}`);
        if(!b[field].length)throw Error(`Missing bounded ${field}`);
        for(const id of b[field])modelCheck(id,role);
      }
      for(const c of sliceCases.filter(c=>c.module===b.module))
        if(!citedModel.has(c.id))throw Error(`Uncited bounded model case ${c.id}`);
      boundedHandoffs.push({id:h.id,module:b.module,bound:b.bound,cases:citedModel.size});
    }
  }
  const visiting=new Set(),done=new Set(),order=[];
  const visit=id=>{
    if(visiting.has(id))throw Error(`Circular proof support at ${id}`);
    if(done.has(id))return;
    visiting.add(id);
    for(const child of adjacency.get(id))visit(child);
    visiting.delete(id);done.add(id);order.unshift(id);
  };
  for(const id of claimIds)visit(id);
  return {status:'structural-assumption-audit-only',claims:claims.size,premises:premises.size,
    selectedHandoffs:registry.handoffs.length,localConditions:precedentCases.length,
    proofSupportOrder:order,unboundSemanticHandoffs,boundedHandoffs,compositionClosed:false,
    scope:'Selected inventory, fingerprints, ownership, evidence roles and acyclic support links; not truth or semantic completeness.'};
}

if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const base=JSON.parse(read('formal/precedent-assumptions.json'));
  const result=validatePrecedentAssumptions({registry:base});
  if(process.argv.includes('--self-test')) {
    const mutated=fn=>{const r=structuredClone(base);fn(r);return {registry:r};};
    const tests=[
      ['stale proof fingerprint',mutated(r=>r.sourceDigests[proofSources[0]]='0'.repeat(64)),'Stale proof source'],
      ['missing premise',mutated(r=>r.premises.shift()),'Unknown premise'],
      ['unknown owner',mutated(r=>r.premises[0].owners=['CMP-99']),'Unknown premise owner'],
      ['unknown environmental assumption',mutated(r=>r.premises[0].assumptions=['ASM-99']),'Unknown model assumption'],
      ['missing boundary',mutated(r=>r.premises[0].boundary=''),'Missing AP-01 boundary'],
      ['missing failure behavior',mutated(r=>r.premises[0].failure=''),'Missing AP-01 failure'],
      ['missing residual',mutated(r=>r.premises[0].open=''),'Missing AP-01 open'],
      ['missing claim',mutated(r=>r.claims.pop()),'Incomplete claim inventory'],
      ['duplicate claim',mutated(r=>r.claims.push(r.claims[0])),'duplicate claim inventory'],
      ['unknown condition',mutated(r=>r.claims[0].positive=['PRE-MISSING']),'Unknown condition'],
      ['control used as proof',mutated(r=>r.claims[0].positive=r.claims[0].controls),'Wrong evidence role'],
      ['uncited local condition',mutated(r=>r.claims[8].positive.pop()),'Uncited precedent condition'],
      ['unknown handoff endpoint',mutated(r=>r.handoffs[0].to='PA-99'),'Unknown handoff endpoint'],
      ['missing handoff',mutated(r=>r.handoffs.pop()),'Incomplete handoff inventory'],
      ['unshared premise',mutated(r=>r.handoffs[0].sharedPremises=['AP-18']),'Unshared handoff premise'],
      ['circular proof support',mutated(r=>{
        Object.assign(r.handoffs[10],{from:'PA-09',to:'PA-01',sharedPremises:['AP-01','AP-02','AP-03']});
      }),'Circular proof support'],
      ['claim silently promoted',mutated(r=>r.claims[0].status='proved'),'Unsupported claim promotion'],
      ['handoff silently closed',mutated(r=>r.handoffs[0].status='proved'),'Unsupported handoff closure'],
      ['unused premise',mutated(r=>r.premises.push({...r.premises[0],id:'AP-99'})),'Unused premise'],
      ['missing bounded handoff evidence',mutated(r=>delete r.handoffs[8].boundedEvidence),'Missing bounded handoff evidence'],
      ['witness used as bounded proof',mutated(r=>r.handoffs[8].boundedEvidence.positive[0].check='ContextHandoff-FreshApplied'),'Wrong bounded evidence role'],
      ['missing bounded property',mutated(r=>r.handoffs[8].boundedEvidence.positive[0].properties=['NotAProperty']),'Unknown bounded property'],
      ['wrong bounded model',mutated(r=>r.handoffs[8].boundedEvidence.positive[0].check='transactions-safety'),'Unknown bounded model check'],
      ['uncited bounded model case',mutated(r=>r.handoffs[8].boundedEvidence.witnesses.pop()),'Uncited bounded model case'],
      ['wrong bounded horizon',mutated(r=>r.handoffs[8].boundedEvidence.bound=7),'Bounded evidence bound mismatch'],
    ];
    for(const [name,inputs,expected] of tests) {
      let caught;
      try {validatePrecedentAssumptions(inputs);}catch(error){caught=error;}
      if(!caught?.message.includes(expected))throw Error(`Sensitivity control failed: ${name}: ${caught?.message}`);
    }
    // Bounded reference support does not close the actual schema handoff.
    if(JSON.stringify(result.unboundSemanticHandoffs)!==JSON.stringify(['PH-09']))
      throw Error('Registry-context gap was silently hidden');
    result.sensitivityControls=tests.map(([name])=>name);
  }
  console.log(JSON.stringify(result,null,2));
}
