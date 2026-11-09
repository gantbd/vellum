// Mutation detection requires an assertion failure after a coherent baseline.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {checkFormatBases} from './serialization-basis-checks.mjs';
import {checkScopedFormat} from './scoped-serialization-checks.mjs';
const stateURL=new URL('./source-serialization-state.mjs',import.meta.url),kernelURL=new URL('./invocation-publication.mjs',import.meta.url);
const state=fs.readFileSync(stateURL,'utf8'),kernel=fs.readFileSync(kernelURL,'utf8');
const moduleURL=(raw,base,overrides={})=>'data:text/javascript;base64,'+Buffer.from(raw.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,base).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,base).href)+',import.meta.url)')).toString('base64');
const mutants=[
 ['original-basis-identity-ignored','(!scoped||old.scoped.basis===scoped.basis)','true'],
 ['queued-basis-revalidation-ignored','if(p.scoped){const current=','if(false){const current='],
 ['truthy-basis-grants-accepted','keys.every(k=>g?.[k]===true)','keys.every(k=>g?.[k])'],
 ['actual-parent-reservation-ignored','||charge>owner.limits.maxHeldBytes-owner.charge',''],
 ['spent-basis-issuance-refunded','found.m.bases.delete(token);basisOwners.delete(token);return true;','found.m.bases.delete(token);found.m.issued--;basisOwners.delete(token);return true;'],
 ['foreign-host-context-can-poison-basis','!equal(g?.sourceSerializationContext,c)||',''],
 ['metadata-generation-not-rechecked','!d||d.serialization.generation!==s.value.expectedSerialization','!d'],
 ['malformed-semantics-can-observe-loss','if(!decideSerialization(owner.root,request,c,p,h)||!deps(request,h,p))return null;',''],
 ['nonlive-document-can-produce-basis',"d.live!==false&&(!d.lifecycle||d.lifecycle.phase==='live')&&",''],
];
async function load(change){let raw=state;if(change){assert.equal(raw.split(change[1]).length-1,1,change[0]+'/unique-preimage');raw=raw.replace(change[1],change[2]);}const s=moduleURL(raw,stateURL),editingURL=new URL('./editing-intents.mjs',import.meta.url),editing=moduleURL(fs.readFileSync(editingURL,'utf8'),editingURL,{'./source-serialization-state.mjs':s}),k=moduleURL(kernel,kernelURL,{'./source-serialization-state.mjs':s,'./editing-intents.mjs':editing});return import(k);}
try{
 const baseline=await load();checkFormatBases(baseline);checkScopedFormat(baseline);
 for(const from of ['not-a-source-preimage','return'])await assert.rejects(()=>load(['construction-control',from,'']),{code:'ERR_ASSERTION'});
 const killed=[];for(const m of mutants){const api=await load(m);let code=null;try{checkFormatBases(api);checkScopedFormat(api);}catch(e){code=e.code;assert.equal(code,'ERR_ASSERTION',m[0]+': '+String(e.message).slice(0,180));}assert.equal(code,'ERR_ASSERTION',m[0]+'/survived');killed.push(m[0]);}
 console.log(JSON.stringify({scope:'isolated-metadata-basis-and-scoped-publication-mutations',coherentBaselinePassed:true,constructionOutsideDetection:true,preimageControls:2,assertionOnlyKills:killed,productionOrIndependentProof:false}));
}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
