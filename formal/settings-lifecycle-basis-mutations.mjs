// Isolated owned-helper graph; no actual project adoption or host-policy proof.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
const root=new URL('.',import.meta.url);
const mutants=[
 ['lifecycle-scope-unbound',"v?.writeBasis?.kind==='lifecycle'&&same(v.context,c)","v?.writeBasis?.kind==='lifecycle'"],
 ['foreign-profile-can-poison-lifecycle-basis','!same(g?.semanticSettingsProfile,held.m.profile)||!same(g?.semanticSettingsContext,c)','false'],
 ['strict-lifecycle-current-grants-ignored','if(!allowed(g,lifecycleBasisGates))return null;','if(false)return null;'],
 ['stale-configuration-condition-accepted','!same(lifecycleCondition(cfg),held.v.writeBasis.expected)','false'],
 ['protected-lifecycle-discharge-ignored',"if(!held||!allowed(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;held.m.leases.delete(token);lifecycleBases.delete(token)","if(!held)return false;held.m.leases.delete(token);lifecycleBases.delete(token)"],
 ['lifecycle-issuance-refunded','held.m.leases.delete(token);lifecycleBases.delete(token);return true;','held.m.leases.delete(token);held.m.issued--;lifecycleBases.delete(token);return true;'],
 ['spent-lifecycle-issuance-refilled',"if(!m||m.leases.size>=p.maxContexts||m.issued>=p.maxIssuedContexts)return null;\n const token=Object.freeze({kind:'native-settings-lifecycle-basis/1'})","if(!m||m.leases.size>=p.maxContexts)return null;\n const token=Object.freeze({kind:'native-settings-lifecycle-basis/1'})"],
 ['lifecycle-context-byte-bound-ignored',"const capture=owned({context,writeBasis:{kind:'lifecycle',expected:lifecycleCondition(cfg)}},p,p.maxContextBytes);if(!capture)return null;","const capture=owned({context,writeBasis:{kind:'lifecycle',expected:lifecycleCondition(cfg)}},p,p.maxStateBytes);if(!capture)return null;"],
 ['lifecycle-observed-loss-not-absorbing',"if(['authorized','scope','continuous','live','interpretationCurrent','settingsWritable','settingsLifecycleBasisAllowed'].some(k=>g?.[k]===false)){held.s.stopped=true;return null;}","if(['authorized','scope','continuous','live','interpretationCurrent','settingsWritable','settingsLifecycleBasisAllowed'].some(k=>g?.[k]===false)){return null;}"],
 ['first-party-lifecycle-condition-gate-ignored','if(g?.settingsHostBasisAllowed!==true)return null;',''],
 ['native-incarnation-condition-omitted',"['generation','layerIncarnation']","['generation']"],
 ['lifecycle-capture-guard-ignored',"[...lifecycleBasisGates,'outsidePublicationGuard','preparationReserved']","lifecycleBasisGates"],
 ['unnecessary-source-publication-dependency','!cfg||!same(lifecycleCondition(cfg),held.v.writeBasis.expected)','!cfg||owner.root.publication!==0||!same(lifecycleCondition(cfg),held.v.writeBasis.expected)']
];
function moduleCode(relative,overrides={},mutation=null){
 const file=new URL(relative,root);let code=fs.readFileSync(file,'utf8');
 if(mutation){const a=code.indexOf('const lifecycleBasisGates='),b=code.indexOf('function normalizeScopedSettings',a);assert(a>=0&&b>a);let region=code.slice(a,b);assert.equal(region.split(mutation[1]).length,2,mutation[0]);region=region.replace(mutation[1],mutation[2]);code=code.slice(0,a)+region+code.slice(b);}
 code=code.replace(/from '(\.\.?\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,file).href));
 code=code.replace(/new URL\('(\.\.?\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,file).href)+',import.meta.url)');
 return 'data:text/javascript;base64,'+Buffer.from(code).toString('base64');
}
async function graph(mutation){
 const state=moduleCode('engine-settings-state.mjs',{},mutation),editing=moduleCode('editing-intents.mjs',{'./engine-settings-state.mjs':state}),kernel=moduleCode('invocation-publication.mjs',{'./engine-settings-state.mjs':state,'./editing-intents.mjs':editing});
 return import(moduleCode('settings-lifecycle-basis-checks.mjs',{'./invocation-publication.mjs':kernel}));
}
export const lifecycleBasisMutationNames=Object.freeze(mutants.map(m=>m[0]));
export async function checkLifecycleBasisMutations(){
 const baseline=(await graph(null)).checkLifecycleBases(),killed=[];
 for(const m of mutants){const test=await graph(m);let failed=false;try{test.checkLifecycleBases();}catch(e){assert.equal(e.code,'ERR_ASSERTION',m[0]+': '+String(e.message).slice(0,100));failed=true;}assert(failed,m[0]);killed.push(m[0]);}
 return {scope:'owned-settings-lifecycle-basis-mutations-only',baseline,mutations:killed,coherentBaselineChecked:true,actualScopedMutationPublicWireOrHostTruthProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(await checkLifecycleBasisMutations()));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<generated-module>'));process.exitCode=1;}
