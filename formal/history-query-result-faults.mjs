// Fixed verification probes compiled before native execution, not runtime hooks.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {historyResultFixture} from './history-query-result-checks.mjs';
import {historyContextScope as context,historyContextGates as gates} from './history-context-checks.mjs';
const raw=fs.readFileSync(new URL('./history-contexts.mjs',import.meta.url),'utf8'),lower="transferReady:true,historyCurrent:true});",copy='function copyResult(result){',counter='__vellum_history_result_probe';
assert.equal(raw.split(lower).length-1,1);assert.equal(raw.split(copy).length-1,1);
const probes=[
 ['lower-extra',lower,lower+'out.extra="private";'],
 ['lower-context',lower,lower+'out.context="foreign";'],
 ['lower-status-getter',lower,lower+`Object.defineProperty(out,'status',{enumerable:true,get(){globalThis.${counter}++;throw Error('unexpected getter');}});`],
 ['lower-status-coercion',lower,lower+`out.status={toString(){globalThis.${counter}++;throw Error('unexpected coercion');}};`],
 ['copied-extra',copy,copy+'return {...result,extra:true};'],
 ['copied-context-cloned',copy,copy+'return structuredClone(result);'],
 ['lower-throw',lower,lower+"throw Error('fixed lower fault');"],
 ['copied-throw',copy,copy+"throw Error('fixed copy fault');"],
 ['frontier-private-alias',lower,lower+"out.group='private-unprojected-group';",['frontier']],
 ['detail-private-scope',lower,lower+"out.blockers[0].scope='private-unprojected-scope';",['permitted-blocking-detail']],
 ['detail-private-group',lower,lower+"out.blockers[0].group='private-unprojected-group';",['permitted-blocking-detail']],
 ['availability-unknown',lower,lower+"out.value='unknown';",['availability']],
 ['copied-meaning-changed',copy,copy+"return {...result,value:result.value==='blocked'?'eligible':'blocked'};",['availability']]
];
const prepared=[];for(const[name,from,to,operations=['frontier','availability','permitted-blocking-detail']]of probes){let body=raw.replace(from,to);body=body.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(new URL(p,import.meta.url).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>`new URL(${JSON.stringify(new URL(p,import.meta.url).href)},import.meta.url)`);prepared.push({name,operations,api:await import('data:text/javascript;base64,'+Buffer.from(body).toString('base64'))});}
const results=[];for(const probe of prepared){let cases=0;for(const operation of probe.operations){const x=historyResultFixture(operation,'undo',operation==='permitted-blocking-detail',probe.api),root=x.z.current(),before=x.usage();assert.equal(before.faulted,false);globalThis[counter]=0;assert.deepEqual(x.query(),{status:'unavailable'},probe.name);assert.equal(globalThis[counter],0,probe.name+': product code executed');delete globalThis[counter];assert.equal(x.z.current(),root);assert.deepEqual(x.usage(),{...before,faulted:true});assert.deepEqual(x.acquire(),{status:'unavailable'});assert.deepEqual(x.query(),{status:'unavailable'});assert.equal(probe.api.historyInspectionReversalBindings(x.z.history,x.request.context,context,gates),null);assert.deepEqual(x.usage(),{...before,faulted:true});
 for(const key of ['ownerControlAllowed','retentionReleased','deliveryDrained'])assert.deepEqual(probe.api.retireHistoryInspection(x.z.history,x.request.context,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true,[key]:false}),{status:'unavailable'});
 assert.equal(probe.api.closeHistoryInspection(x.z.history,x.request.context,context.recipient,{ownerControlAllowed:true}).status,'stopped');assert.equal(probe.api.retireHistoryInspection(x.z.history,x.request.context,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true}).status,'retired');assert.equal(x.usage().contexts,before.contexts-1);assert.equal(x.usage().issued,before.issued);assert.equal(x.usage().declaredHeldBytes,before.declaredHeldBytes);assert.equal(x.usage().faulted,true);cases++;}
 results.push({name:probe.name,cases});}
console.log(JSON.stringify({scope:'fixed-native-F04-history-result-invariant-probes',results,totalCases:results.reduce((n,r)=>n+r.cases,0),originalRootsAndHeldObligationsPreserved:true,protectedRetirementAndNonrefundingIssuanceChecked:true,productAccessorsOrCoercionExecuted:0,allFaultsPhysicalCrashOrIndependentProof:false}));
