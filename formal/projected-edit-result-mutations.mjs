import assert from 'node:assert/strict';import fs from 'node:fs';
import {projectedEditResultCorpus,checkProjectedEditResults} from './projected-edit-result-checks.mjs';
const source=fs.readFileSync(new URL('./projected-edit-results.mjs',import.meta.url),'utf8'),rules=[
 ['unknown-family-admitted',"if(!['F13','F14'].includes(family))return false;",''],
 ['extra-fields-admitted','Object.keys(v).length===keys.length&&',''],
 ['unavailable-data-admitted',"if(v.status==='unavailable')return exact(v,['status']);","if(v.status==='unavailable')return true;"],
 ['annotation-envelope-unchecked','v.status===declarations.annotation.envelope&&annotationProjection(v.result)','annotationProjection(v.result)'],
 ['reversal-envelope-unchecked','v.status===declarations.reversal.envelope&&reversalProjection(v.result)','reversalProjection(v.result)'],
 ['annotation-operation-unchecked','const r=fields(v),d=declarations.annotation;if(!r||!label(r.invocation)||!d.operations.includes(r.operation))return false;','const r=fields(v),d=declarations.annotation;if(!r||!label(r.invocation))return false;'],
 ['reversal-operation-unchecked','const r=fields(v),d=declarations.reversal;if(!r||!label(r.invocation)||!d.operations.includes(r.operation))return false;','const r=fields(v),d=declarations.reversal;if(!r||!label(r.invocation))return false;'],
 ['membership-unchanged-admitted',"||r.status==='unchanged'&&['create','delete'].includes(r.operation)",''],
 ['annotation-publication-uncorrelated',"r.status!=='applied'||r.publication===r.invocation",'true'],
 ['reversal-publication-uncorrelated','&&r.publication===r.invocation&&','&&true&&'],
 ['zero-version-admitted','Number.isSafeInteger(v)&&v>0','Number.isSafeInteger(v)&&v>=0'],
 ['unsafe-integer-admitted','Number.isSafeInteger(v)&&v>0','Number.isInteger(v)&&v>0'],
 ['invalid-utf8-label-admitted',"&&Buffer.from(v,'utf8').toString('utf8')===v",''],
 ['empty-label-admitted',"typeof v==='string'&&v.length>0&&","typeof v==='string'&&"],
 ['document-identity-unchecked','&&document(a.document)&&','&&true&&'],
 ['annotation-id-unchecked','&&label(a.id)&&','&&true&&'],
 ['annotation-incarnation-unchecked','&&version(a.incarnation);','&&true;'],
 ['membership-unchecked',"&&r.membership===(r.operation==='delete'?'retained-absent':'present')",''],
 ['private-reversal-authority-admitted',"[...d.published,...(Object.hasOwn(r,'target')?['target']:[])]","[...d.published,...(Object.hasOwn(r,'target')?['target']:[]),...(Object.hasOwn(r,'authority')?['authority']:[])]"],
 ['withheld-reversal-target-rejected',"(!Object.hasOwn(r,'target')||target(r.target))",'target(r.target)'],
 ['withheld-annotation-versions-rejected','d.optionalVersions.filter(k=>Object.hasOwn(r,k))','d.optionalVersions'],
 ['empty-group-target-admitted','exact(t,d.groupTarget)&&label(t.group)','exact(t,d.groupTarget)'],
 ['frontier-group-label-unchecked','&&label(t.expectedGroup);','&&true;'],
 ['data-descriptors-bypassed',"const ds=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(ds);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(ds[k],'value')||ds[k].enumerable!==true))return null;\n return Object.fromEntries(keys.map(k=>[k,ds[k].value]));","return Object.fromEntries(Object.keys(value).map(k=>[k,value[k]]));"]
];
assert.equal(rules.length,24);assert.equal(new Set(rules.map(r=>r[0])).size,24);for(const[name,from]of rules)if(source.split(from).length-1!==1)throw Error('PROJECTED_EDIT_MUTATION_CONSTRUCTION: '+name);
const corpus=projectedEditResultCorpus(),baseline=checkProjectedEditResults(undefined,corpus),mutations=[];
try{for(const[name,from,to]of rules){const code=source.replace(from,to).replace("new URL('./annotation-results.json',import.meta.url)",'new URL('+JSON.stringify(new URL('./annotation-results.json',import.meta.url).href)+')'),api=await import('data:text/javascript;base64,'+Buffer.from(code+'\n// '+name).toString('base64'));let killed=false;try{checkProjectedEditResults(api,corpus);}catch(error){if(error.code!=='ERR_ASSERTION')throw error;killed=true;}assert(killed,'surviving projected edit result mutation '+name);mutations.push(name);}}catch(error){console.error(String(error.stack??error).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'[projected result checker]'));process.exit(1);}
console.log(JSON.stringify({scope:'existing-projected-edit-result-sensitivity',baseline,mutations,nativeCorpusProducedOnceForPureCheckerVariants:true,constructionOrToolFailuresCounted:false,publicRouteOrNativePrecommitIntegrationSelected:false,physicalOrIndependentProof:false}));
