// Retained declaration conditions; not public wire or policy truth.
import assert from 'node:assert/strict';
import * as kernel from './invocation-publication.mjs';
import {pathToFileURL} from 'node:url';
import {engineSettingsFixture} from './engine-settings-checks.mjs';
import {engineSettingsDefinitionFixture} from './engine-settings-definition-checks.mjs';
const unavailable={status:'unavailable'},current={status:'settings-definition-basis',availability:'current'};
export function checkDefinitionBases(api=kernel){
function fixture(definition=false,options={}){
 const x=definition?engineSettingsDefinitionFixture(api):engineSettingsFixture(api,options),c=x.controlContext;
 const g=(more={})=>({...x.host(),settingsDefinitionBasisAllowed:true,settingsHostBasisAllowed:true,...more});
 const capture=(context=c,h=g(),p=x.p)=>api.captureInvocationSettingsDefinitionBasis(x.domain,context,p,h);
 return {...x,c,g,capture,inspect:(b,h=g(),context=c)=>api.inspectInvocationSettingsDefinitionBasis(x.domain,b,context,h),resolve:(b,h=g(),context=c)=>api.resolveInvocationSettingsDefinitionBasis(x.domain,b,context,h),retire:(b,h=g(),context=c)=>api.retireInvocationSettingsDefinitionBasis(x.domain,b,context,h),held:()=>api.invocationSemanticSettingsUsage(x.domain)};
}
let cases=0;const check=f=>{f();cases++;};
check(()=>{const x=fixture(),root=x.root(),usage=x.usage(),held=x.held(),b=x.capture();assert(b);assert.equal(x.root(),root);assert.deepEqual(x.usage(),usage);assert.equal(x.held().contexts,held.contexts+1);assert.equal(x.held().issued,held.issued+1);assert.deepEqual(x.inspect(b),current);assert.deepEqual(x.resolve(b),{schemaGeneration:1,restrictionGeneration:1,associationGeneration:1});});
check(()=>{const x=fixture(),b=x.capture();assert(b);assert.equal(api.readInvocationSemanticContext(x.domain,b,x.c,x.g()),null);assert.equal(api.releaseInvocationSemanticContext(x.domain,b,x.c,x.g()),false);assert.deepEqual(api.inspectInvocationSettingsWriteBasis(x.domain,b,x.c,x.g()),unavailable);assert.equal(api.resolveInvocationSettingsWriteBasis(x.domain,b,x.c,x.g()),null);assert.deepEqual(x.inspect(b),current);});
check(()=>{const x=fixture(),b=x.capture(),ordinary=api.captureInvocationSettingsWriteBasis(x.domain,{authority:'a',id:'w1',incarnation:1},x.c,x.p,x.g({settingsBasisAllowed:true}));assert(ordinary);assert.deepEqual(x.inspect(ordinary,x.g({authorized:false})),unavailable);assert.deepEqual(api.inspectInvocationSettingsWriteBasis(x.domain,ordinary,x.c,x.g({settingsBasisAllowed:true})),{status:'settings-write-basis',availability:'current'});assert.deepEqual(x.inspect(structuredClone(b)),unavailable);});
check(()=>{const x=fixture(),b=x.capture();assert.deepEqual(x.inspect(b,x.g({settingsHostBasisAllowed:false})),current);for(const value of [false,1,undefined])assert.equal(x.resolve(b,x.g({settingsHostBasisAllowed:value})),null);assert(x.resolve(b));});
check(()=>{const x=fixture(),b=x.capture();x.submit();x.run();assert.equal(x.root().engineSettings.generation,2);assert.deepEqual(x.inspect(b),current);assert(x.resolve(b));});
check(()=>{const x=fixture(true),b=x.capture();x.submit();x.run();assert.equal(x.root().engineSettings.schemaGeneration,2);assert.deepEqual(x.inspect(b),unavailable);assert.equal(x.resolve(b),null);assert(x.retire(b));});
for(const key of ['authorized','scope','continuous','live','interpretationCurrent','settingsWritable','settingsDefinitionBasisAllowed'])check(()=>{const x=fixture(),b=x.capture();assert.deepEqual(x.inspect(b,x.g({[key]:false})),unavailable);assert.equal(x.resolve(b),null);assert(x.retire(b));});
for(const value of [1,undefined])check(()=>{const x=fixture(),b=x.capture();assert.deepEqual(x.inspect(b,x.g({settingsDefinitionBasisAllowed:value})),unavailable);assert(x.resolve(b));});
for(const key of ['notStopped','resourcesReady','outsidePublicationGuard'])check(()=>{const x=fixture(),b=x.capture();assert.deepEqual(x.inspect(b,x.g({[key]:false})),unavailable);assert(x.resolve(b));});
for(const field of ['authority','engine','recipient','purpose','destination','profile','continuity'])check(()=>{const x=fixture(),b=x.capture(),c={...x.c,[field]:'foreign'};assert.deepEqual(x.inspect(b,x.g({authorized:false}),c),unavailable);assert(x.resolve(b));assert.equal(x.retire(b,x.g(),c),false);});
check(()=>{const x=fixture(),b=x.capture();assert.deepEqual(x.inspect(b,x.g({semanticSettingsProfile:{...x.p,maxKeys:0},authorized:false})),unavailable);assert(x.resolve(b));});
check(()=>{const x=fixture(),b=x.capture(),issued=x.held().issued;assert(api.stopInvocationSettingsDefinitionBasis(x.domain,b,x.c,x.g()));assert.deepEqual(x.inspect(b),unavailable);assert.equal(x.retire(b,x.g({retentionReleased:false})),false);assert(x.retire(b));assert.equal(x.held().issued,issued);assert.equal(x.retire(b),false);});
check(()=>{const x=fixture(),bs=[];for(let i=0;i<x.p.maxContexts;i++){const b=x.capture();assert(b);bs.push(b);}assert.equal(x.capture(),null);for(const b of bs)assert(x.retire(b));const remaining=x.p.maxIssuedContexts-x.held().issued;for(let i=0;i<remaining;i++){const b=x.capture();assert(b);assert(x.retire(b));}assert.equal(x.capture(),null);assert.equal(x.held().contexts,0);});
for(const profile of [{maxContexts:0},{maxIssuedContexts:0},{maxContextBytes:1},{maxHeldBytes:0}])check(()=>{const x=fixture(false,{profile}),held=x.held();assert.equal(x.capture(),null);assert.deepEqual(x.held(),held);});
check(()=>{const x=fixture(false,{profile:{maxContextBytes:512}}),c={...x.c};assert(Buffer.byteLength(JSON.stringify(x.p))<512);for(const k of ['recipient','purpose','destination','profile','continuity'])c[k]='x'.repeat(x.p.argumentLimits.maxIdentifierBytes);const held=x.held();assert.equal(x.capture(c,x.g({semanticSettingsContext:c})),null);assert.deepEqual(x.held(),held);});
check(()=>{const x=fixture(),y=fixture(),b=x.capture();assert.deepEqual(y.inspect(b,y.g({authorized:false})),unavailable);assert.equal(y.retire(b),false);assert.deepEqual(x.inspect(b),current);assert(x.retire(b));});
check(()=>{const seed=fixture(),c={...seed.c,recipient:'r'.repeat(seed.p.argumentLimits.maxIdentifierBytes)},expected={schemaGeneration:1,restrictionGeneration:1,associationGeneration:1},n=Buffer.byteLength(JSON.stringify({context:c,writeBasis:{kind:'definition',expected}}));for(const delta of [-1,0,1]){const x=fixture(false,{profile:{maxContextBytes:n+delta}}),held=x.held(),b=x.capture(c,x.g({semanticSettingsContext:c}));assert.equal(Boolean(b),delta>=0);if(!b)assert.deepEqual(x.held(),held);}});
return {scope:'owned-settings-definition-basis-only',cases,sharedPool:true,noConfigurationDisclosure:true,actualMutationLoweringProved:false,hostPolicyOrPublicWireProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkDefinitionBases()));
