// Reference/partition validation, NOT proof of semantic adequacy or refinement.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {sourceUnits} from './specification-audit.mjs';
import {sliceCases} from './slices.mjs';

const root=path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const read=file=>fs.readFileSync(path.join(root,file),'utf8');
const hash=text=>crypto.createHash('sha256').update(text).digest('hex');
const insist=(ok,message)=>{if(!ok)throw Error(message);};
const nonempty=value=>typeof value==='string'&&value.trim().length>0;
const keys=(value,expected)=>value&&typeof value==='object'&&!Array.isArray(value)&&
  Object.keys(value).sort().join(',')===[...expected].sort().join(',');

export function validatePublicationClauses({
  ledger=JSON.parse(read('formal/publication-clauses.json')),
  requirementsText=read('requirements.md'),specificationText=read('specification.md'),
  decisionsText=read('specification-decisions.md'),
  observations=JSON.parse(read('formal/observability.json')),
}={}) {
  insist(keys(ledger,['status','sourceSha256','section','boundary','clauses']),'Invalid ledger shape');
  insist(ledger.status==='selected-clause-traceability-only','Unsupported evidence promotion');
  insist(ledger.sourceSha256===hash(requirementsText),'Stale source fingerprint');
  insist(ledger.section==='5.3','Wrong selected source section');
  insist(keys(ledger.boundary,['interface','point','profile']),'Invalid boundary shape');
  const {interface:iface,point,profile}=ledger.boundary;
  const plan=observations.coverage[iface];
  insist(iface==='IF-03'&&profile==='publication'&&point==='INS-02'&&
    plan?.profile===profile&&plan.points.includes(point),'Wrong observation boundary');
  const units=sourceUnits(requirementsText,new Set([ledger.section]));
  insist(units.size===10,'Selected source inventory changed');
  const owners=new Set([...specificationText.matchAll(/^#{2,4} (\d+(?:\.\d+)*)(?:\.| )/gm)].map(m=>m[1]));
  const decisions=new Set([...decisionsText.matchAll(/^\| (D\d{2})\b/gm)].map(m=>m[1]));
  const cases=new Map(sliceCases.map(c=>[c.id,c]));
  insist(Array.isArray(ledger.clauses)&&ledger.clauses.length===41,'Missing or extra selected clauses');
  const groups=new Map([...units.keys()].map(unit=>[unit,[]]));
  let previousUnit=-1,supported=0;
  for(const [index,c] of ledger.clauses.entries()) {
    insist(keys(c,['id','unit','fragment','contracts','support','scenario','residual','risk']),'Invalid clause shape');
    insist(c.id===`TXC-${String(index+1).padStart(2,'0')}`,'Invalid clause identity/order');
    insist(groups.has(c.unit),'Unknown source unit');
    const unitIndex=[...units.keys()].indexOf(c.unit);
    insist(unitIndex>=previousUnit,'Invalid source unit order');previousUnit=unitIndex;
    insist(nonempty(c.fragment)&&nonempty(c.scenario)&&nonempty(c.residual)&&nonempty(c.risk),'Missing clause obligation or limit');
    insist(Array.isArray(c.contracts)&&c.contracts.length>0&&new Set(c.contracts).size===c.contracts.length&&
      c.contracts.every(s=>owners.has(s)),'Missing or unknown specification owner');
    insist(Array.isArray(c.support)&&new Set(c.support).size===c.support.length,'Invalid support list');
    for(const ref of c.support) {
      insist(typeof ref==='string','Invalid support reference');
      const [id,property,...extra]=ref.split('/'),entry=cases.get(id);
      insist(entry&&extra.length===0,'Unknown support case');
      insist(entry.expected==='success','Control or witness is not positive support');
      insist([...(entry.invariants||[]),...(entry.properties||[])].includes(property),'Property not configured in support case');
    }
    for(const d of c.residual.matchAll(/\bD\d{2}\b/g))insist(decisions.has(d[0]),'Unknown residual decision');
    if(c.support.length)supported++;
    groups.get(c.unit).push(c.fragment);
  }
  for(const [unit,fragments] of groups) {
    insist(fragments.length>0,'Missing source unit partition');
    insist(fragments.join(' ')===units.get(unit).replace(/^[-*] /,''),`Non-exact source partition: ${unit}`);
  }
  return {status:ledger.status,section:ledger.section,sourceUnits:units.size,clauses:ledger.clauses.length,
    clausesWithScopedSupport:supported,clausesWithoutListedSupport:ledger.clauses.length-supported,
    completeSelectedTextPartition:true,allRefinementsClosed:false,
    scope:'Exact selected text partition and reference integrity; not semantic indivisibility, proof coverage, independent review or implementation refinement'};
}

if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const ledger=JSON.parse(read('formal/publication-clauses.json'));
  const report=validatePublicationClauses({ledger});
  if(process.argv.includes('--self-test')) {
    const controls=[];
    const reject=(name,mutate,reason)=>{
      const changed=structuredClone(ledger);mutate(changed);
      let error;try{validatePublicationClauses({ledger:changed});}catch(e){error=e;}
      insist(error instanceof Error&&error.message.includes(reason),`Sensitivity failure: ${name}`);
      controls.push(name);
    };
    reject('stale source',x=>x.sourceSha256='0'.repeat(64),'fingerprint');
    reject('missing clause',x=>x.clauses.pop(),'selected clauses');
    reject('duplicate identity',x=>x.clauses[1].id=x.clauses[0].id,'identity/order');
    reject('unknown source unit',x=>x.clauses[0].unit='5.3/b99','Unknown source unit');
    reject('altered source fragment',x=>x.clauses[0].fragment+=' invented','Non-exact source partition');
    reject('omitted clause owner',x=>x.clauses[0].contracts=[],'specification owner');
    reject('unknown clause owner',x=>x.clauses[0].contracts=['999'],'specification owner');
    reject('unknown support',x=>x.clauses[0].support=['missing/TypeOK'],'Unknown support case');
    reject('control promoted to proof',x=>x.clauses[0].support=['transactions-private-stage-control/NoPartialObservation'],'not positive support');
    reject('witness promoted to proof',x=>x.clauses[0].support=['Transactions-SuccessfulX/NotSuccessfulX'],'not positive support');
    reject('unconfigured property',x=>x.clauses[0].support=['transactions-safety/NotAProperty'],'not configured');
    reject('missing residual',x=>x.clauses[0].residual='','Missing clause obligation');
    reject('unknown decision',x=>x.clauses[0].residual='D99','Unknown residual decision');
    reject('wrong observation profile',x=>x.boundary.profile='release','observation boundary');
    reject('wrong observation point',x=>x.boundary.point='INS-03','observation boundary');
    reject('evidence promoted to closure',x=>x.status='proved','Unsupported evidence promotion');
    report.sensitivityControls=controls;
  }
  console.log(JSON.stringify(report,null,2));
}
