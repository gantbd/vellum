import fs from 'node:fs';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {checkEngineUIBoundary} from './engine-ui-checks.mjs';
const source=fs.readFileSync(new URL('./engine-facade.mjs',import.meta.url),'utf8');
const integrationSource=fs.readFileSync(new URL('./integration-facade.mjs',import.meta.url),'utf8');
const checks=fs.readFileSync(new URL('./engine-ui-checks.mjs',import.meta.url),'utf8');
const lower=code=>code.replace(/'\.\/([^']+\.mjs)'/g,(_,file)=>JSON.stringify(new URL(file,import.meta.url).href));
async function load(code,tag,integrationCode=integrationSource){
  code=lower(code).replace(/new URL\('\.\/([^']+\.json)',import.meta.url\)/g,(_,file)=>`new URL(${JSON.stringify(new URL(file,import.meta.url).href)})`);
  const url=`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${tag}`;
  integrationCode=lower(integrationCode.replace("'./engine-facade.mjs'",JSON.stringify(url))).replace(/new URL\('\.\/([^']+\.json)',import.meta.url\)/g,(_,file)=>`new URL(${JSON.stringify(new URL(file,import.meta.url).href)})`);
  const integrationURL=`data:text/javascript;base64,${Buffer.from(integrationCode).toString('base64')}#integration-${tag}`;
  const proof=lower(checks.replace("'./engine-facade.mjs'",JSON.stringify(url)).replace("'./integration-facade.mjs'",JSON.stringify(integrationURL)))
    .replace('const here=path.dirname(fileURLToPath(import.meta.url));',`const here=${JSON.stringify(fileURLToPath(new URL('.',import.meta.url)))};`);
  return import(`data:text/javascript;base64,${Buffer.from(proof).toString('base64')}#${tag}`);
}
const baseline=(await load(source,'baseline')).checkEngineUIBoundary();
assert.equal(baseline.engineVariants,71);assert.equal(baseline.integrationVariants,5);
const mutants=[
  ['allow-integration-through-engine',').engine.includes(decoded.variant)',').engine.concat(ownership.find(p=>p.family===decoded.descriptor.id).integration).includes(decoded.variant)'],
  ['allow-ui-through-engine',"return decoded&&ownership.find(p=>p.family===decoded.descriptor.id).engine.includes(decoded.variant)?decoded:null;",'return decoded;'],
  ['swap-engine-and-presentation',').engine.includes(decoded.variant)',').presentation.includes(decoded.variant)'],
  ['expose-ui-in-engine-discovery','const variants=ownership[index].engine;','const variants=f.variants;'],
  ['renumber-typed-family-after-filter','family:index,id:f.id','family:0,id:f.id']
];
const killed=[];
for(const [name,from,to] of mutants){assert(source.includes(from),name);const module=await load(source.replace(from,to),name);assert.throws(()=>module.checkEngineUIBoundary(),{code:'ERR_ASSERTION'},name);killed.push(name);}
const integrationMutants=[
 ['integration-descriptor-not-frozen','Object.freeze(v);',';'],
 ['integration-admits-engine',').integration.includes(decoded.variant)',').engine.includes(decoded.variant)'],
 ['integration-admits-presentation',').integration.includes(decoded.variant)',').presentation.includes(decoded.variant)'],
 ['integration-discovery-exposes-engine','const variants=ownership[index].integration;','const variants=f.variants;'],
 ['integration-renumbers-family','family:index,id:f.id','family:0,id:f.id']
];
for(const [name,from,to] of integrationMutants){assert(integrationSource.includes(from),name);const module=await load(source,name,integrationSource.replace(from,to));assert.throws(()=>module.checkEngineUIBoundary(),{code:'ERR_ASSERTION'},name);killed.push(name);}
console.log(JSON.stringify({...checkEngineUIBoundary(),mutationBaselineChecked:true,mutations:killed},null,2));
