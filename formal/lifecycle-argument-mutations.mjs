import assert from 'node:assert/strict';
import fs from 'node:fs';
import {checkLifecycleArguments} from './lifecycle-argument-checks.mjs';
const source=fs.readFileSync(new URL('./lifecycle-arguments.mjs',import.meta.url),'utf8');
const rules=[
 ['extra-envelope-accepted',"!exact(e,['route','request'])",'!e'],
 ['extra-wrapper-accepted','!exact(a,wrapper.fields)','!a'],
 ['extra-lower-accepted','!exact(r,form.fields)','!r'],
 ['cross-variant-accepted','form.variant!==decoded.variant','false'],
 ['foreign-profile-accepted','a.profile!==wrapper.profile','false'],
 ['foreign-contract-accepted','a.contract!==wrapper.contract','false'],
 ['scope-extra-accepted','!exact(scope.value,scopeFields)','false'],
 ['scope-empty-accepted','!Object.values(scope.value).every(text)','false'],
 ['maximum-not-intersected','Math.min(maximum,limits.maxRequestBytes)','limits.maxRequestBytes'],
 ['caller-record-retained','record:captured.value','record:{...captured.value,request:a}'],
 ['ordinary-meaning-ignored','&&equal(a.record,b.record)',''],
 ['wrong-producer-family','{family:16}','{family:17}']
];
const load=async text=>import('data:text/javascript;base64,'+Buffer.from(text.replace(/from '(\.\/[^']+)'/g,(_,relative)=>'from '+JSON.stringify(new URL(relative,import.meta.url).href))).toString('base64'));
const baseline=checkLifecycleArguments(await load(source)),mutations=[];
for(const [name,before,after]of rules){assert.equal(source.split(before).length-1,1,name);const api=await load(source.replace(before,after));let detected=false;try{checkLifecycleArguments(api);}catch(error){assert.equal(error.code,'ERR_ASSERTION',name+': '+error.message);detected=true;}assert(detected,name);mutations.push(name);}
console.log(JSON.stringify({scope:'selected-F17-lifecycle-argument-mutations',baseline,mutations,constructionFailuresCounted:false,fullF17OrImplementationOrIndependentProof:false}));
