// Coherent reference graphs and assertion-only mutants, not production certification.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
const root=new URL('.',import.meta.url);
const toModule=(relative,overrides={},mutation=null,suffix='')=>{
 const file=new URL(relative,root);let source=fs.readFileSync(file,'utf8');
 if(mutation){assert.equal(source.split(mutation[1]).length,2,mutation[0]);source=source.replace(mutation[1],mutation[2]);}
 source=source.replace(/from '(\.\.?\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,file).href));
 source=source.replace(/new URL\('(\.\.?\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,file).href)+',import.meta.url)');
 return 'data:text/javascript;base64,'+Buffer.from(source+suffix).toString('base64');
};
const mutants=[
 ['declaration-basis-context-unbound',"v?.writeBasis?.kind==='definition'&&same(v.context,c)","v?.writeBasis?.kind==='definition'"],
 ['foreign-current-profile-can-poison-declaration-basis',"const held=retainedDefinitionBasis(owner,domain,token,c);if(!held||held.s.stopped||owner.faulted)return null;\n if(!same(g?.semanticSettingsProfile,held.m.profile)||!same(g?.semanticSettingsContext,c))return null;","const held=retainedDefinitionBasis(owner,domain,token,c);if(!held||held.s.stopped||owner.faulted)return null;\n if(false)return null;"],
 ['strict-declaration-basis-grants-ignored','if(!allowed(g,definitionBasisGates))return null;','if(false)return null;'],
 ['stale-declaration-triple-accepted','!same(definitionCondition(cfg),held.v.writeBasis.expected)','false'],
 ['declaration-discharge-guard-ignored',"if(!held||!allowed(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;held.m.leases.delete(token);definitionBases.delete(token)","if(!held)return false;held.m.leases.delete(token);definitionBases.delete(token)"],
 ['declaration-basis-issuance-refunded','held.m.leases.delete(token);definitionBases.delete(token);return true;','held.m.leases.delete(token);held.m.issued--;definitionBases.delete(token);return true;'],
 ['declaration-spent-capacity-refilled',"if(!m||m.leases.size>=p.maxContexts||m.issued>=p.maxIssuedContexts)return null;\n const token=Object.freeze({kind:'native-settings-definition-basis/1'})","if(!m||m.leases.size>=p.maxContexts)return null;\n const token=Object.freeze({kind:'native-settings-definition-basis/1'})"],
 ['loss-observed-before-complete-declaration-preflight','const request={...r,contract:definitionInput.contract,expected:held.v.writeBasis.expected};','resolveNativeEngineSettingsDefinitionBasis(owner,domain,basis,c,h);const request={...r,contract:definitionInput.contract,expected:held.v.writeBasis.expected};'],
 ['declaration-basis-use-not-revalidated','const current=resolveNativeEngineSettingsDefinitionBasis(owner,domain,basis,c,h);','const current=held.v.writeBasis.expected;'],
 ['retained-declaration-basis-reference-substituted','prior.scoped.bases.every((b,i)=>b===scoped.bases[i])','true'],
 ['native-scoped-declaration-intent-conflated','Boolean(prior.scoped)===Boolean(scoped)','true']
 ,['declaration-capture-byte-bound-ignored',"const capture=owned({context,writeBasis:{kind:'definition',expected:definitionCondition(cfg)}},p,p.maxContextBytes);if(!capture)return null;","const capture=owned({context,writeBasis:{kind:'definition',expected:definitionCondition(cfg)}},p,p.maxStateBytes);if(!capture)return null;"]
 ,['observed-declaration-authority-loss-not-absorbing',"if(['authorized','scope','continuous','live','interpretationCurrent','settingsWritable','settingsDefinitionBasisAllowed'].some(k=>g?.[k]===false)){held.s.stopped=true;return null;}","if(['authorized','scope','continuous','live','interpretationCurrent','settingsWritable','settingsDefinitionBasisAllowed'].some(k=>g?.[k]===false)){return null;}"]
 ,['interpretation-discharge-consumes-declaration-basis',"v&&!v.writeBasis&&same(v.context,c)&&allowed(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])","v&&same(v.context,c)&&allowed(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])"]
 ,['complete-definition-wrapper-budget-ignored',"if(bytes({kind:'settings',plan:value},p,owner.limits.maxIntentBytes)===null)return null;",'']
];
async function graph(mutation){
 const probe="\nexport function definitionTestPlanBytes(token){const p=plans.get(token);return p?{value:Buffer.byteLength(JSON.stringify(p.value)),wrapper:Buffer.byteLength(JSON.stringify({kind:'settings',plan:p.value}))}:null;}";
 const state=toModule('engine-settings-state.mjs',{},mutation,probe),editing=toModule('editing-intents.mjs',{'./engine-settings-state.mjs':state});
 const kernel=toModule('invocation-publication.mjs',{'./engine-settings-state.mjs':state,'./editing-intents.mjs':editing});
 const [basis,scoped]=await Promise.all([import(toModule('settings-definition-basis-checks.mjs',{'./invocation-publication.mjs':kernel})),import(toModule('scoped-definition-checks.mjs',{'./invocation-publication.mjs':kernel}))]);
 const api={...await import(state),...await import(kernel)};
 return {run:()=>({basis:basis.checkDefinitionBases(api),scoped:scoped.checkScopedDefinition(api),budgets:scoped.checkScopedDefinitionBudgets(api)})};
}
export const scopedDefinitionMutationNames=Object.freeze(mutants.map(m=>m[0]));
export async function checkScopedDefinitionEvidence(){
 assert.equal(Object.hasOwn(await import('./engine-settings-state.mjs'),'definitionTestPlanBytes'),false);
 const baseline=(await graph(null)).run(),killed=[];
 for(const mutation of mutants){const test=await graph(mutation);let failed=false;try{test.run();}catch(e){assert.equal(e.code,'ERR_ASSERTION',mutation[0]+': '+String(e.message).slice(0,150));failed=true;}assert(failed,mutation[0]);killed.push(mutation[0]);}
 return {scope:'owned-scoped-settings-definition-evidence-only',baseline,mutations:killed,coherentBaselineChecked:true,testProbeOnlyInGeneratedGraphs:true,hostPublicPhysicalOrIndependentTruthProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(await checkScopedDefinitionEvidence()));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<generated-module>'));process.exitCode=1;}
