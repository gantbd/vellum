import assert from 'node:assert/strict';
import fs from 'node:fs';
import {checkMixedReads,checkMixedInterleavings,checkMixedReadLattice} from './mixed-reads-checks.mjs';
const base=new URL('./',import.meta.url),names=['read-dependencies.mjs','annotation-compounds.mjs','invocation-publication.mjs','proposal-validation.mjs'],raw=Object.fromEntries(names.map(n=>[n,fs.readFileSync(new URL(n,base),'utf8')]));
const mutants=[
 ['source-version-ignored','d.revision!==r.expectedSource','false'],
 ['annotation-set-version-ignored','d.annotationSet!==r.expectedAnnotationSet','false'],
 ['annotation-body-version-ignored','row.bodyVersion!==r.expectedBody','false'],
 ['annotation-target-version-ignored','row.targetGeneration!==r.expectedTarget','false'],
 ['source-configuration-domain-ignored','cfg.domains[k][r.domain]!==r.expectedGeneration',"r.domain!=='source'&&cfg.domains[k][r.domain]!==r.expectedGeneration"],
 ['serialization-configuration-domain-ignored','cfg.domains[k][r.domain]!==r.expectedGeneration',"r.domain!=='serialization'&&cfg.domains[k][r.domain]!==r.expectedGeneration"],
 ['analysis-configuration-domain-ignored','cfg.domains[k][r.domain]!==r.expectedGeneration',"r.domain!=='analysis'&&cfg.domains[k][r.domain]!==r.expectedGeneration"],
 ['configuration-state-version-ignored','cfg.generation!==r.expectedConfiguration','false'],
 ['workspace-generation-ignored','rows[0].generation!==r.expectedWorkspace','false'],
 ['first-current-source-satisfies-conjunction',"if(r.kind==='source'){if(d.revision!==r.expectedSource)return false;}","if(r.kind==='source'){if(d.revision!==r.expectedSource)return false;return true;}"],
 ['permission-facts-truthy','g[k]===true','!!g[k]'],
 ['native-publication-read-validation-omitted','if(readPlan&&!declaredReadsCurrent','if(false&&!declaredReadsCurrent','annotation-compounds.mjs'],
 ['native-read-canonicalization-omitted','.sort((a,b)=>Buffer.compare(Buffer.from(key(a)),Buffer.from(key(b))))',''],
];
async function graph(m){const source={...raw},urls={};if(m){const name=m[3]??names[0];assert.equal(source[name].split(m[1]).length-1,1,m[0]);source[name]=source[name].replace(m[1],m[2]);}for(const n of names){const body=source[n].replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+(urls[p.slice(2)]??new URL(p,base).href)+q);urls[n]='data:text/javascript;base64,'+Buffer.from(body+'\n// '+(m?.[0]??'baseline')).toString('base64');}return {...await import(urls[names[0]]),...await import(urls[names[2]]),...await import(urls[names[3]])};}
try{const api=await graph(),baseline=checkMixedReads(api),interleavings=checkMixedInterleavings(api),lattice=checkMixedReadLattice(api),killed=[];for(const m of mutants){const candidate=await graph(m);let detected=false;try{checkMixedReads(candidate);checkMixedInterleavings(candidate);checkMixedReadLattice(candidate);}catch(e){assert.equal(e.code,'ERR_ASSERTION',m[0]+': '+String(e.message).slice(0,120));detected=true;}assert(detected,m[0]+'/survived');killed.push(m[0]);}console.log(JSON.stringify({scope:'isolated-nine-predicate-conjunction-mutations',baseline,interleavings,lattice,mutations:killed,constructionOutsideDetection:true,coherentNativeBaselinePassed:true,allDependencyDiscoveryOrIndependentProof:false}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
