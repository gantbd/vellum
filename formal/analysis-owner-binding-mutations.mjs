// Coherent actual-module mutations. Import/construction failures are not kills.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {checkAnalysisOwnerBinding} from './analysis-owner-binding-checks.mjs';
const source=fs.readFileSync(new URL('./analysis-work.mjs',import.meta.url),'utf8');
const mutations=[
 ['substitute-equal-root', 'state===invocationAuthorityRoot(o.nativeDomain)', 'true'],
 ['ignore-domain-health', 'invocationUsage(o.nativeDomain)?.faulted===false', 'true'],
 ['truthy-native-ordinary-grants', '&&strict(g,keys)', '&&keys.every(k=>g?.[k])'],
 ['replace-native-registry', 'nativeOwners.has(domain)', 'false'],
 ['infer-optional-analysis-inventory', '!nativeRootSupportsAnalysis(invocationAuthorityRoot(domain))', 'false'],
 ['unchecked-native-open', "!strict(g,['ownerControlAllowed','resourcesReady','preparationReserved'])", 'false'],
 ['truthy-native-ingress', "!strict(g,['authentic','evidenceAllowed','ingressReady'])", "!['authentic','evidenceAllowed','ingressReady'].every(k=>g?.[k])"],
 ['truthy-native-quiescence', 'g?.quiescenceAttested!==true', '!g?.quiescenceAttested'],
 ['truthy-native-control', 'g?.controlAllowed!==true', '!g?.controlAllowed'],
 ['truthy-native-retirement', "!strict(g,['retirementAllowed','retentionReleased','deliveryDrained'])", "!['retirementAllowed','retentionReleased','deliveryDrained'].every(k=>g?.[k])"],
 ['truthy-native-producer-binding', 'g?.nativeBindingAllowed!==true', '!g?.nativeBindingAllowed'],
 ['omit-query-root-guard', "if(!nativeState(owners.get(owner),state,g,[...release,'contextAllowed']))return {status:'unavailable'};", '']
];
async function load(change){
 let raw=source;
 if(change){assert.equal(raw.split(change[1]).length-1,1,change[0]);raw=raw.replace(change[1],change[2]);}
 raw=raw.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(new URL(p,import.meta.url).href))
  .replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,import.meta.url).href)+',import.meta.url)');
 return import('data:text/javascript;base64,'+Buffer.from(raw).toString('base64')+'#'+(change?.[0]??'baseline'));
}
export async function checkAnalysisOwnerBindingMutations(){
 const baseline=checkAnalysisOwnerBinding(await load()),killed=[];
 for(const change of mutations){
  const api=await load(change);let failed=false;
  try{checkAnalysisOwnerBinding(api);}catch(e){assert.equal(e.code,'ERR_ASSERTION',change[0]+': '+String(e.message).slice(0,200));failed=true;}
  assert(failed,change[0]);killed.push(change[0]);
 }
 return {scope:'actual-engine-analysis-owner-root-binding-mutations-only',baseline,mutations:killed,coherentModuleBaseline:true,constructionFailuresCounted:false,fullQualificationOrImplementationRefinement:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(await checkAnalysisOwnerBindingMutations()));}catch(e){console.error(e.code+': '+String(e.message).slice(0,500));process.exitCode=1;}
