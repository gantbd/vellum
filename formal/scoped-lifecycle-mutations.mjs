// Isolated coherent owned-helper graphs; not public results or host conformance.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
const root=new URL('.',import.meta.url);
const mutations=[
 ['wrong-private-incarnation-offset','incarnation:expected.layerIncarnation+i+1','incarnation:expected.layerIncarnation+i+2'],
 ['created-ordinal-collapsed','incarnation:expected.layerIncarnation+i+1','incarnation:expected.layerIncarnation+1'],
 ['created-reference-index-ignored','create[ref.create].layer','create[0].layer'],
 ['removal-generation-guessed','expectedGeneration:old.generation','expectedGeneration:1'],
 ['early-lifecycle-loss-observation','const expected=held.v.writeBasis.expected,cfg=nativeEngineSettingsBasis(owner,p);','resolveNativeEngineSettingsLifecycleBasis(owner,domain,basis,c,h);const expected=held.v.writeBasis.expected,cfg=nativeEngineSettingsBasis(owner,p);'],
 ['lifecycle-basis-use-not-revalidated','const current=resolveNativeEngineSettingsLifecycleBasis(owner,domain,basis,c,h);','const current=held.v.writeBasis.expected;'],
 ['created-reference-bounds-ignored','&&ref.create<r.create.length',''],
 ['retained-lifecycle-reference-substituted','prior.scoped.bases.every((b,i)=>b===scoped.bases[i])','true','shared'],
 ['native-scoped-lifecycle-intent-conflated','Boolean(prior.scoped)===Boolean(scoped)','true','shared'],
 ['lifecycle-parent-wrapper-budget-ignored',"if(bytes({kind:'settings',plan:value},p,owner.limits.maxIntentBytes)===null)return null;",'','shared'],
 ['known-candidate-root-capacity-ignored','||Buffer.byteLength(JSON.stringify(d.state))>owner.limits.maxRootBytes','','shared']
];
function moduleCode(relative,overrides={},mutation=null,suffix=''){
 const file=new URL(relative,root);let source=fs.readFileSync(file,'utf8');
 if(mutation){const shared=mutation[3]==='shared',a=shared?0:source.indexOf('function normalizeScopedLifecycle'),b=shared?source.length:source.indexOf('function normalizeBasisLifecycle',a);assert(a>=0&&b>a);const region=source.slice(a,b);assert.equal(region.split(mutation[1]).length,2,mutation[0]);source=source.slice(0,a)+region.replace(mutation[1],mutation[2])+source.slice(b);}
 source=source.replace(/from '(\.\.?\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,file).href));
 source=source.replace(/new URL\('(\.\.?\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,file).href)+',import.meta.url)');
 return 'data:text/javascript;base64,'+Buffer.from(source+suffix).toString('base64');
}
async function graph(mutation){
 const probe="\nexport function lifecycleTestPlanMetrics(token,g){const p=plans.get(token);if(!p)return null;const d=chooseDecision(p.owner.root,p.value.request,p.value.context.authority,p.value.profile,g);return d?{value:Buffer.byteLength(JSON.stringify(p.value)),wrapper:Buffer.byteLength(JSON.stringify({kind:'settings',plan:p.value})),before:Buffer.byteLength(JSON.stringify(p.owner.root)),candidate:Buffer.byteLength(JSON.stringify(d.state))}:null;}";
 const state=moduleCode('engine-settings-state.mjs',{},mutation,probe),editing=moduleCode('editing-intents.mjs',{'./engine-settings-state.mjs':state}),kernel=moduleCode('invocation-publication.mjs',{'./engine-settings-state.mjs':state,'./editing-intents.mjs':editing});
 const test=await import(moduleCode('scoped-lifecycle-checks.mjs',{'./invocation-publication.mjs':kernel})),api={...await import(state),...await import(kernel)};
 return {run:()=>({reference:test.checkScopedLifecycle(api),budgets:test.checkScopedLifecycleBudgets(api)})};
}
export const scopedLifecycleMutationNames=Object.freeze(mutations.map(m=>m[0]));
export async function checkScopedLifecycleEvidence(){
 assert.equal(Object.hasOwn(await import('./engine-settings-state.mjs'),'lifecycleTestPlanMetrics'),false);
 const baseline=(await graph(null)).run(),killed=[];
 for(const mutation of mutations){const x=await graph(mutation);let failed=false;try{x.run();}catch(e){assert.equal(e.code,'ERR_ASSERTION',mutation[0]+': '+String(e.message).slice(0,160));failed=true;}assert(failed,mutation[0]);killed.push(mutation[0]);}
 return {scope:'owned-scoped-settings-lifecycle-evidence-only',baseline,mutations:killed,coherentBaselineChecked:true,testProbeOnlyInGeneratedGraphs:true,physicalPublicResultOrHostTruthProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(await checkScopedLifecycleEvidence()));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<generated-module>'));process.exitCode=1;}
