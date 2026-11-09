import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import {checkObserverBinding} from './observer-binding-checks.mjs';
const mutations=[
 ['event-claim-guard-omitted',"export function releaseBoundPublicationEvent(owner,claimed,currentContext,g){\n if(!originalClaim(owners.get(owner),claimed))return unavailable();","export function releaseBoundPublicationEvent(owner,claimed,currentContext,g){"],
 ['outcome-claim-guard-omitted',"export function releaseBoundPublicationOutcome(owner,handle,claimed,currentContext,g){\n if(!originalClaim(owners.get(owner),claimed))return unavailable();","export function releaseBoundPublicationOutcome(owner,handle,claimed,currentContext,g){"],
 ['original-epoch-unbound','||!(claimed.outcomePolicy.epoch===o.context.outcomePolicy.epoch)',''],
 ['original-scope-unbound',"return catalog.contextFields.every(key=>key==='outcomePolicy'||equal(claimed[key],o.context[key]));",'return true;'],
 ['event-current-scope-substituted','return releasePublicationEvent(owner,currentContext,g);','return releasePublicationEvent(owner,claimed,g);'],
 ['outcome-current-scope-substituted','return releasePublicationOutcome(owner,handle,currentContext,g);','return releasePublicationOutcome(owner,handle,claimed,g);'],
 ['event-current-authorization-widened','return releasePublicationEvent(owner,currentContext,g);','return releasePublicationEvent(owner,currentContext,{...g,authorized:true});'],
 ['outcome-current-authorization-widened','return releasePublicationOutcome(owner,handle,currentContext,g);','return releasePublicationOutcome(owner,handle,currentContext,{...g,authorized:true});']
];
export async function checkObserverBindingMutations(){
 const file=new URL('./publication-observation.mjs',import.meta.url),source=fs.readFileSync(file,'utf8');
 const load=async code=>{code=code.replace(/from '(\.\/[^']+)'/g,(_,s)=>'from '+JSON.stringify(new URL(s,file).href));code=code.replace("new URL('./publication-observation.json',import.meta.url)",'new URL('+JSON.stringify(new URL('./publication-observation.json',file).href)+')');return import('data:text/javascript;base64,'+Buffer.from(code).toString('base64'));};
 const baseline=checkObserverBinding(await load(source)),killed=[];
 for(const [name,from,to] of mutations){assert.equal(source.split(from).length-1,1,name);const api=await load(source.replace(from,to));let detected=false;try{checkObserverBinding(api);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name+': '+e.message);detected=true;}assert(detected,name);killed.push(name);}
 return {scope:'native-publication-observation-binding-sensitivity',baseline,mutations:killed,coherentObserverBaseline:true,constructionFailuresCounted:false,fullQualificationOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(await checkObserverBindingMutations()));}catch(e){console.error(e.message);process.exitCode=1;}
