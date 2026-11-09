// Logical measurement design reference only; no installed probe, owner or exporter.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
const catalog=JSON.parse(fs.readFileSync(new URL('./engine-measurements.json',import.meta.url)));
const copy=structuredClone,nat=n=>Number.isSafeInteger(n)&&n>=0;
const exact=(v,fields)=>v!==null&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...fields].sort());
const reusable=['normalized-bytes','source-octets','slots','completion-promises'],consumed=['work-steps','child-grants'];
const expected={
 'INS-01':['request.offers','request.dispositions','invocation.admissions','invocation.dispositions','admission.duration'],
 'INS-02':['preparation.duration','publication.dispositions','publication.duration'],
 'INS-03':['release.dispositions','release.duration'],
 'INS-04':['effect.dispatches','effect.evidence','save.baseline-adoptions','effect.queue.duration','effect.first-result.duration'],
 'INS-05':['derived.dispositions','derived.duration'],
 'INS-06':['history.dispositions'],
 'INS-07':['artifact.dispositions','artifact.duration','artifact.size'],
 'INS-08':['lifecycle.dispositions','resource.occupancy','resource.commitments','resource.consumed','resource.capacity'],
 'INS-09':['configuration.dispositions','configuration.duration'],
 'INS-12':['diagnostic.records'],
 'INS-13':['control.dispositions','control.duration','retention.duration']
};
export function checkMeasurementCatalog(c=catalog){
 assert(exact(c,['profile','scope','engineSites','excludedSites','pointFields','status','freshness','coverage','dimensions','types','measures']));
 assert.equal(c.profile,'engine-measurement/1');
 assert.equal(c.scope,'engine-measurement-design-values-only');
 assert.deepEqual(c.types,{counter:{valueFields:['total','exact'],aggregation:'cumulative-monotone-within-generation'},gauge:{valueFields:['amount'],aggregation:'current-value-no-implicit-sum'},duration:{valueFields:['seconds','endpoint'],aggregation:'individual-observations-no-default-buckets'}});
 assert(exact(c.dimensions,['admission','disposition','artifact','evidence','derived','diagnostic','owner','coordinate']));
 assert.deepEqual(c.dimensions.disposition,['success','unchanged','refused','stopped','unknown','fault']);
 assert.deepEqual(c.dimensions.artifact,['source','annotation']);
 assert.deepEqual(c.dimensions.evidence,['progress','acknowledgment','failure','quiescence','unknown','duplicate','contradiction']);
 assert.deepEqual(c.dimensions.diagnostic,['offered','captured','drained','dropped','invalid','discarded','preparation-refused']);
 assert.deepEqual(c.dimensions.owner,['invocation','publication','release','effect','analysis','history','artifact','lifecycle','configuration','diagnostics','control']);
 assert.deepEqual(c.engineSites,Object.keys(expected));assert.deepEqual(c.excludedSites,['INS-10','INS-11']);
 assert.deepEqual(c.pointFields,['profile','measure','generation','sequence','status','freshness','coverage','dimensions','value']);
 assert.equal(c.measures.length,32);assert.equal(new Set(c.measures.map(m=>m.id)).size,32);
 for(const [site,ids]of Object.entries(expected))assert.deepEqual(c.measures.filter(m=>m.site===site).map(m=>m.id),ids);
 for(const d of c.measures){
  assert(exact(d,['id','site','type','unit','population','boundary','dimensions']));
  assert(['counter','gauge','duration'].includes(d.type));
  assert.equal(d.type==='duration',d.id.endsWith('.duration'));
  assert.equal(d.unit==='coordinate',d.id.startsWith('resource.'));
  if(d.type==='duration')assert.equal(d.unit,'s');
  else if(d.id==='artifact.size')assert.equal(d.unit,'By');
  else if(!d.id.startsWith('resource.'))assert(/^\{[a-z-]+\}$/.test(d.unit));
  assert.equal(new Set(d.dimensions).size,d.dimensions.length);
  for(const field of d.dimensions)assert(Object.hasOwn(c.dimensions,field));
  for(const field of ['population','boundary'])assert(typeof d[field]==='string'&&d[field].trim());
 }
 assert.deepEqual(c.status,['observed','off','unavailable','unknown']);
 assert.deepEqual(c.freshness,['current','stale','unknown']);
 assert.deepEqual(c.coverage,['complete','sampled','inexact','unknown']);
 assert.deepEqual(c.dimensions.coordinate,[...reusable,...consumed]);
 assert.deepEqual(c.dimensions.admission,['first-admitted','replay-observed','refused','stopped','unavailable','unknown','fault']);
 assert.deepEqual(c.dimensions.derived,['accepted','partial','stale','superseded','unsupported','refused','stopped','unavailable','unknown','fault']);
 for(const values of Object.values(c.dimensions))assert(values.length>0&&new Set(values).size===values.length);
 return {descriptors:32,engineSites:11,uiSitesExcluded:2};
}
export function measurementPointValid(p,context,c=catalog){
 if(!context||!nat(context.maxInteger)||!Number.isFinite(context.maxSeconds)||context.maxSeconds<0||!context.generation||
 !exact(p,c.pointFields)||p.profile!==c.profile||p.generation!==context.generation||!nat(p.sequence)||p.sequence>context.maxInteger||
 !c.status.includes(p.status)||!c.freshness.includes(p.freshness)||!c.coverage.includes(p.coverage))return false;
 const d=c.measures.find(d=>d.id===p.measure);if(!d||!exact(p.dimensions,d.dimensions)||d.dimensions.some(k=>!c.dimensions[k].includes(p.dimensions[k])))return false;
 const coordinate=p.dimensions.coordinate;
 if(d.unit==='coordinate'&&!(d.id==='resource.consumed'?consumed:d.id==='resource.capacity'?[...reusable,...consumed]:reusable).includes(coordinate))return false;
 if(p.status!=='observed')return p.value===null&&p.freshness==='unknown'&&p.coverage==='unknown';
 const v=p.value;if(!exact(v,c.types[d.type].valueFields))return false;
 if(d.type==='counter')return nat(v.total)&&v.total<=context.maxInteger&&typeof v.exact==='boolean'&&(p.coverage==='complete'||v.exact===false);
 if(d.type==='gauge')return nat(v.amount)&&v.amount<=context.maxInteger;
 return Number.isFinite(v.seconds)&&v.seconds>=0&&v.seconds<=context.maxSeconds&&['completed','pending','censored'].includes(v.endpoint);
}
export function measurementCounterStep(v,amount,maximum){
 if(!exact(v,['total','exact'])||!nat(v.total)||typeof v.exact!=='boolean'||!nat(amount)||!nat(maximum)||v.total>maximum)return null;
 return amount<=maximum-v.total?{total:v.total+amount,exact:v.exact}:{total:maximum,exact:false};
}
export function measurementDelta(a,b,context){
 if(context?.continuous!==true||!measurementPointValid(a,context)||!measurementPointValid(b,context)||!['observed'].includes(a.status)||b.status!=='observed'||
 a.measure!==b.measure||!equal(a.dimensions,b.dimensions)||a.generation!==b.generation||b.sequence<=a.sequence||
 a.freshness!=='current'||b.freshness!=='current'||a.coverage!=='complete'||b.coverage!=='complete'||
 !exact(a.value,['total','exact'])||!exact(b.value,['total','exact'])||a.value.exact!==true||b.value.exact!==true||b.value.total<a.value.total)return null;
 return b.value.total-a.value.total;
}
export function measurementSeconds(start,end,clock,maximum){
 if(!start||!end||!clock||clock.valid!==true||!clock.domain||!clock.epoch||start.domain!==clock.domain||end.domain!==clock.domain||
 start.epoch!==clock.epoch||end.epoch!==clock.epoch||!nat(start.tick)||!nat(end.tick)||end.tick<start.tick||
 !Number.isFinite(clock.secondsPerTick)||clock.secondsPerTick<=0||!Number.isFinite(maximum)||maximum<0)return null;
 const seconds=(end.tick-start.tick)*clock.secondsPerTick;
 return Number.isFinite(seconds)&&seconds<=maximum?seconds:null;
}
export function measurementFirstResult(kind,attributed,alreadySeen){
 return attributed===true&&alreadySeen===false&&['acknowledgment','failure','unknown','contradiction'].includes(kind);
}
function fixture(d,context){
 const dimensions=Object.fromEntries(d.dimensions.map(k=>[k,k==='coordinate'?(d.id==='resource.consumed'?'work-steps':'slots'):catalog.dimensions[k][0]]));
 return {profile:catalog.profile,measure:d.id,generation:context.generation,sequence:1,status:'observed',freshness:'current',coverage:'complete',dimensions,
  value:d.type==='counter'?{total:0,exact:true}:d.type==='gauge'?{amount:0}:{seconds:0,endpoint:'completed'}};
}
export function checkMeasurementValues(api={valid:measurementPointValid,step:measurementCounterStep,delta:measurementDelta,seconds:measurementSeconds,firstResult:measurementFirstResult}){
 const context={generation:Object.freeze({}),maxInteger:10,maxSeconds:20,continuous:true},controls=[];let pointCases=0,coordinateCases=0;
 const check=(name,f)=>{f();controls.push(name);};
 check('closed-fields-values-and-observed-zero',()=>{
  for(const d of catalog.measures){
   const p=fixture(d,context);assert.equal(api.valid(p,context),true,d.id);pointCases++;
   for(const status of ['off','unavailable','unknown']){
    const q={...p,status,freshness:'unknown',coverage:'unknown',value:null};assert.equal(api.valid(q,context),true);pointCases++;
    assert.equal(api.valid({...q,value:p.value},context),false);pointCases++;
   }
   assert.equal(api.valid({...p,source:'private'},context),false);pointCases++;
   assert.equal(api.valid({...p,dimensions:{...p.dimensions,document:'private'}},context),false);pointCases++;
   for(const field of catalog.pointFields){const q={...p};delete q[field];assert.equal(api.valid(q,context),false);pointCases++;}
   assert.equal(api.valid({...p,sequence:11},context),false);pointCases++;
  }
 });
 check('bounded-values-and-inexact-coverage',()=>{
  for(const d of catalog.measures){const p=fixture(d,context),field=d.type==='counter'?'total':d.type==='gauge'?'amount':'seconds';
   for(const value of [-1,Infinity,NaN,d.type==='duration'?21:11]){assert.equal(api.valid({...p,value:{...p.value,[field]:value}},context),false);pointCases++;}
   if(d.type==='counter'){assert.equal(api.valid({...p,coverage:'sampled'},context),false);assert.equal(api.valid({...p,coverage:'sampled',value:{total:0,exact:false}},context),true);pointCases+=2;}
   if(d.type==='duration')for(const endpoint of ['pending','censored']){assert.equal(api.valid({...p,value:{seconds:1,endpoint}},context),true);pointCases++;}
  }
 });
 check('tagged-resource-units-do-not-interchange',()=>{
  for(const d of catalog.measures.filter(d=>d.unit==='coordinate'))for(const coordinate of [...reusable,...consumed]){
   const p=fixture(d,context);p.dimensions.coordinate=coordinate;
   const wanted=d.id==='resource.capacity'||(d.id==='resource.consumed'?consumed:reusable).includes(coordinate);
   assert.equal(api.valid(p,context),wanted,d.id+'/'+coordinate);coordinateCases++;
  }
 });
 check('current-derived-refusal-is-not-mislabeled-stale-or-fault',()=>{
  const d=catalog.measures.find(d=>d.id==='derived.dispositions');
  for(const [cause,classification]of [['authority-refused','refused'],['capacity-refused','refused'],['stopped','stopped'],['no-permitted-detail','unavailable'],['classification-not-established','unknown']]){
   const p=fixture(d,context);p.dimensions.derived=classification;
   assert.equal(api.valid(p,context),true,cause);pointCases++;
  }
 });
 check('per-offer-admission-does-not-infer-refusals-by-subtraction',()=>{
  const d=catalog.measures.find(d=>d.id==='request.dispositions');
  for(const admission of ['first-admitted','replay-observed','refused','stopped','unavailable','unknown','fault']){
   const p=fixture(d,context);p.dimensions.admission=admission;assert.equal(api.valid(p,context),true);pointCases++;
  }
 });
 check('first-result-is-once-per-attempt-not-quiescence-or-persistence',()=>{
  for(const kind of ['progress','acknowledgment','failure','quiescence','unknown','duplicate','contradiction'])for(const attributed of [false,true])for(const seen of [false,true]){
   const wanted=attributed&&!seen&&['acknowledgment','failure','unknown','contradiction'].includes(kind);
   assert.equal(api.firstResult(kind,attributed,seen),wanted);
  }
  for(const kind of ['acknowledgment','failure','unknown','contradiction'])for(const application of ['unknown','written','not-written'])for(const synchronization of ['none','failed','complete']){
   assert.equal(api.firstResult(kind,true,false),true,application+'/'+synchronization);
  }
  assert.equal(api.firstResult('quiescence',true,false),false);
  assert.equal(api.firstResult('acknowledgment',true,false),true);
  assert.equal(api.firstResult('acknowledgment',true,true),false);
  assert.equal(api.firstResult('unknown',true,true),false);
 });
 check('counter-saturation-is-sticky-not-wrap-or-reset',()=>{
  for(let total=0;total<=10;total++)for(let amount=0;amount<=12;amount++)for(const exactness of [true,false]){
   const actual=api.step({total,exact:exactness},amount,10),sum=total+amount;
   assert.deepEqual(actual,{total:Math.min(sum,10),exact:exactness&&sum<=10});
  }
 });
 check('delta-requires-same-series-exact-continuity',()=>{
  const a=fixture(catalog.measures[0],context),b={...a,sequence:2,value:{total:5,exact:true}};assert.equal(api.delta(a,b,context),5);
  for(const q of [{...b,generation:{}},{...b,sequence:1},{...b,coverage:'sampled',value:{total:5,exact:false}},{...b,freshness:'stale'},{...b,value:{total:5,exact:false}},{...b,status:'unknown',freshness:'unknown',coverage:'unknown',value:null}])assert.equal(api.delta(a,q,context),null);
  assert.equal(api.delta({...a,value:{total:6,exact:true}},b,context),null);
  for(const continuous of [false,undefined,1])assert.equal(api.delta(a,b,{...context,continuous}),null);
 });
 check('clock-domain-epoch-validity-and-cutoff-are-not-wall-time',()=>{
  const clock={domain:{},epoch:{},valid:true,secondsPerTick:0.5},start={domain:clock.domain,epoch:clock.epoch,tick:2},end={...start,tick:6};assert.equal(api.seconds(start,end,clock,20),2);
  for(const e of [{...end,domain:{}},{...end,epoch:{}},{...end,tick:1},{...end,tick:Infinity}])assert.equal(api.seconds(start,e,clock,20),null);
  for(const c of [{...clock,valid:false},{...clock,secondsPerTick:0},{...clock,secondsPerTick:Infinity}])assert.equal(api.seconds(start,end,c,20),null);
  assert.equal(api.seconds(start,end,clock,1),null);
 });
 return {controls,pointCases,coordinateCases,counterStepCases:286,providedScopeAndClockTruthProved:false};
}
export function checkEngineMeasurements(){
 const declarations=checkMeasurementCatalog(),baseline=checkMeasurementValues();
 const index=JSON.parse(fs.readFileSync(new URL('./observability.json',import.meta.url))).measurementProfile;
 assert.equal(index.profile,catalog.profile);assert.equal(index.descriptors,'engine-measurements.json');assert.equal(index.interpretation,'engine-measurements.md');
 assert.deepEqual(index.engineSites,catalog.engineSites);assert.deepEqual(index.uiSites,catalog.excludedSites);assert.equal(index.installedCaptureOrProviderClockTruth,false);
 const original={valid:measurementPointValid,step:measurementCounterStep,delta:measurementDelta,seconds:measurementSeconds,firstResult:measurementFirstResult};
 const mutations=[
  ['unknown-point-fields-allowed',{valid:(p,c)=>measurementPointValid(Object.fromEntries(Object.entries(p).filter(([k])=>catalog.pointFields.includes(k))),c)}],
  ['private-dimensions-allowed',{valid:(p,c)=>{const d=catalog.measures.find(d=>d.id===p.measure);return measurementPointValid({...p,dimensions:Object.fromEntries(Object.entries(p.dimensions).filter(([k])=>d?.dimensions.includes(k)))},c);}}],
  ['unobserved-values-treated-as-zero',{valid:(p,c)=>p.status!=='observed'&&p.value!==null?true:measurementPointValid(p,c)}],
  ['consumed-and-reusable-coordinates-mixed',{valid:(p,c)=>measurementPointValid(p.measure==='resource.consumed'?{...p,dimensions:{...p.dimensions,coordinate:'work-steps'}}:p,c)}],
  ['counter-wraps',{step:(v,n,m)=>({total:(v.total+n)%(m+1),exact:v.exact})}],
  ['cross-generation-delta',{delta:(a,b,c)=>measurementDelta({...a,generation:c.generation},{...b,generation:c.generation},c)}],
  ['sampled-delta-treated-as-exact',{delta:(a,b,c)=>measurementDelta({...a,coverage:'complete',value:{...a.value,exact:true}},{...b,coverage:'complete',value:{...b.value,exact:true}},c)}],
  ['continuity-evidence-ignored',{delta:(a,b,c)=>measurementDelta(a,b,{...c,continuous:true})}],
  ['quiescence-counted-as-first-result',{firstResult:(k,a,s)=>k==='quiescence'&&a===true&&s===false||measurementFirstResult(k,a,s)}],
  ['cross-clock-duration',{seconds:(s,e,c,m)=>measurementSeconds(s,{...e,domain:s.domain},c,m)}]
 ];
 const killed=[];
 for(const [name,change]of mutations){let detected=false;const api={...original,...change};try{checkMeasurementValues(api);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name+': '+e.message);detected=true;}assert(detected,name);killed.push(name);}
 return {scope:catalog.scope,declarations,baseline,mutations:killed,baselineBeforeMutationChecked:true,constructionFailuresCounted:false,
  installedCaptureAuthorityClockPhysicalOrIndependentProof:false,newFacadeWireExporterOrEngineRuntime:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkEngineMeasurements(),null,2));
