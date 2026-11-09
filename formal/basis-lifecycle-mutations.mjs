import assert from 'node:assert/strict';
import {checkBasisLifecycle,checkBasisLifecycleBudgets} from './basis-lifecycle-checks.mjs';
import {basisLifecycleTestGraph as graph} from './basis-lifecycle-test-graph.mjs';
const mutants=[
 ['layer-registry-authority-ignored','h.layerRegistryWritable!==true','false'],
 ['association-registry-authority-ignored','h.associationRegistryWritable!==true','false'],
 ['later-stale-layer-footprint-observed-too-late','!same(dependencies(cfg,[{layer:held.v.writeBasis.layer}]),held.v.writeBasis.dependencies)','false'],
 ['original-selector-reference-substituted','prior.scoped.bases.every((b,i)=>b===scoped.bases[i])','true'],
 ['unused-selector-bases-admitted','used.size!==writeBases.length','false'],
 ['separate-selector-vector-preflight-omitted','||writeBases.length>p.maxLayers',''],
 ['selector-index-range-ignored','||index>=writeBases.length',''],
 ['service-basis-revalidation-bypassed','if(p.scoped){const current=lowerScopedSettings',"if(p.scoped&&p.value.scoped.request.contract!==basisLifecycleInput.contract){const current=lowerScopedSettings"],
 ['selector-indices-collapse-to-first-layer','conditions[ref.basis].layer','conditions[0].layer'],
 ['declared-dependencies-not-revalidated','!same(extraDependencies(v.request,h,profile),v.extra)','false'],
 ['complete-parent-wrapper-budget-ignored',"if(bytes({kind:'settings',plan:value},p,owner.limits.maxIntentBytes)===null)return null;",''],
 ['known-candidate-root-budget-ignored','Buffer.byteLength(JSON.stringify(d.state))>owner.limits.maxRootBytes','false'],
];

try{const api=await graph(),baseline=checkBasisLifecycle(api),budgets=checkBasisLifecycleBudgets(api),killed=[];for(const m of mutants){const api=await graph(m);let failed=false;try{checkBasisLifecycle(api);checkBasisLifecycleBudgets(api);}catch(e){assert.equal(e.code,'ERR_ASSERTION',m[0]+': '+String(e.message).slice(0,170));failed=true;}assert(failed,m[0]+'/survived');killed.push(m[0]);}console.log(JSON.stringify({scope:'isolated-write-basis-lifecycle-mutations',baseline,budgets,coherentBaselinePassed:true,constructionOutsideDetection:true,testProbeOnlyInGeneratedGraphs:true,mutations:killed,allPublicProductsOrIndependentProof:false}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
