// Existing native discovery metadata versus facade ownership, not execution.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {discoverySelfFixture} from './discovery-self-integration-checks.mjs';
import {engineInventory} from './engine-facade.mjs';
import {integrationInventory} from './integration-facade.mjs';
const copy=structuredClone;
const integrationPairs=['F11/accepted-paste','F11/validated-cut-deletion','F22/plain-or-supported-structured-read','F23/copy','F23/cut-export-stage'].sort();
function capture(){
 const x=discoverySelfFixture();
 return {engine:engineInventory(),integration:integrationInventory(),
  profiles:Array.from({length:13},(_,i)=>x.rows({...x.c,profile:'owned-contract-discovery/'+(i+1)}))};
}
function verify(data){
 const pairs=rows=>rows.flatMap(f=>f.variants.map(v=>f.id+'/'+v));
 const engine=pairs(data.engine),integration=pairs(data.integration);
 assert.equal(engine.length,71);assert.equal(new Set(engine).size,71);
 assert.deepEqual(integration.sort(),integrationPairs);
 for(const f of [...data.engine,...data.integration])assert.equal(f.family,Number(f.id.slice(1))-1);
 assert.equal(data.profiles.length,13);
 assert.deepEqual(data.profiles.map(rows=>rows.length),[2,3,4,5,7,7,8,9,10,11,12,13,14]);
 let metadataPairs=0;
 for(const rows of data.profiles){
  const seen=new Set();
  for(const row of rows){
   const family=JSON.parse(row.command.id)[2];
   for(const v of row.variants){
    const key=family+'/'+v.id;
    assert(engine.includes(key),key);assert(!integrationPairs.includes(key),key);
    assert(!seen.has(key),key);seen.add(key);metadataPairs++;
   }
  }
 }
 assert.equal(data.profiles[12].reduce((n,r)=>n+r.variants.length,0),43);
 return metadataPairs;
}
export function checkOwnershipDiscovery(){
 const observed=capture(),metadataPairs=verify(observed),sensitivities=[];
 const changeRow=(data,change)=>change(data.profiles[12].find(r=>JSON.parse(r.command.id)[2]==='F11'));
 const changes=[
  ['integration-workflow-in-engine-metadata',d=>changeRow(d,r=>{r.variants[0].id='accepted-paste';})],
  ['unknown-variant-in-engine-metadata',d=>{d.profiles[12][0].variants[0].id='unknown';}],
  ['duplicate-metadata-pair',d=>{const r=d.profiles[12][0];r.variants.push(copy(r.variants[0]));}],
  ['renumbered-engine-family',d=>{d.engine[0].family++;}],
  ['missing-current-metadata-variant',d=>{d.profiles[12][13].variants.pop();}],
  ['integration-owner-omitted',d=>{d.integration[0].variants.pop();}]
 ];
 for(const [name,change]of changes){const d=copy(observed);change(d);assert.throws(()=>verify(d),{code:'ERR_ASSERTION'},name);sensitivities.push(name);}
 return {scope:'existing-discovery-metadata-facade-ownership-only',profiles:13,currentRows:14,currentMetadataVariants:43,engineVariants:71,integrationVariants:5,metadataPairs,baselineChecked:true,oracleSensitivityControls:sensitivities,
  metadataHandlerTruthOrExecutionProved:false,integrationExecutionComplete:false,clipboardWorkflowKernelSeparationProved:false,implementationRefinement:false,independentCleanroomCertification:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkOwnershipDiscovery()));
