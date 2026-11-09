import assert from 'node:assert/strict';
import fs from 'node:fs';
import {checkLifecycleActivationArguments} from './lifecycle-activation-argument-checks.mjs';
const source=fs.readFileSync(new URL('./lifecycle-activation-arguments.mjs',import.meta.url),'utf8'),rules=[
 ['extra-envelope-accepted',"!exact(e,['route','request'])",'!e'],
 ['unknown-route-accepted',"!['typed','dynamic'].includes(e.route)",'false'],
 ['foreign-family-selected',"decoded?.descriptor.id!=='F17'","decoded?.descriptor.id!=='F18'"],
 ['contract-unbound','a.contract===f.contract','true'],
 ['operation-unbound','a.operation===f.operation','true'],
 ['profile-unbound','a.profile===f.profile','true'],
 ['variant-unbound','f.variant===decoded.variant','true'],
 ['outer-fields-open','&&exact(a,f.fields)',''],
 ['nested-fields-open','exact(r,f.nested.fields)&&',''],
 ['nested-discriminator-unbound','&&matches(r,f.nested.matches)',''],
 ['container-action-unbound','matches(a,f.matches)','true'],
 ['caller-maximum-ignored','Math.min(maximum,limits.maxRequestBytes)','limits.maxRequestBytes'],
 ['capture-retains-caller-request','record:captured.value','record:{...captured.value,request:a}'],
 ['meaning-comparison-ignored','&&equal(a.record,b.record)',''],
 ['producer-family-changed','{family:16}','{family:17}']
];
const load=async text=>import('data:text/javascript;base64,'+Buffer.from(text.replace(/from '(\.\/[^']+)'/g,(_,relative)=>'from '+JSON.stringify(new URL(relative,import.meta.url).href))).toString('base64'));
const baseline=checkLifecycleActivationArguments(await load(source)),mutations=[];
for(const [name,before,after]of rules){assert.equal(source.split(before).length-1,1,name+': construction target');const api=await load(source.replace(before,after));let detected=false;try{checkLifecycleActivationArguments(api);}catch(error){assert.equal(error.code,'ERR_ASSERTION',name+': '+error.message);detected=true;}assert(detected,name);mutations.push(name);}
console.log(JSON.stringify({scope:'selected-F17-activation-argument-sensitivity',baseline,mutations,constructionFailuresCounted:false,fullF17OrGatewayOrIndependentProof:false}));
