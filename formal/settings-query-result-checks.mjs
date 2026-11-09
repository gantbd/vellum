import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {settingsQueryResultValid as valid,settingsQueryResultDeclarations as declarations,settingsQueryOriginBindingsValid as originsValid} from './settings-query-results.mjs';
import {engineSettingsReadFixture as fixture} from './engine-settings-read-checks.mjs';
import {engineSettingsDefinition} from './engine-settings-checks.mjs';
const limits={maxRequestBytes:65536,maxInputNodes:4096,maxInputDepth:128},copy=structuredClone,operations=['effective-configuration','permitted-origin','capabilities'];
export function checkSettingsQueryResults(factory=fixture){
 const make=options=>factory(undefined,options);
 const controls=[],check=(name,f)=>{f();controls.push(name);};let nativeCases=0,denials=0,originBindingCases=0;
 const seed=op=>{const context=Object.freeze({kind:'context'}),target=Object.freeze({kind:'target'}),reference=Object.freeze({kind:'origin'}),declaration={key:'scalar',type:{kind:'integer',min:0,max:8},domain:'source',scopes:['host','workspace','document'],composition:'replace',sensitivity:true},value=op==='effective-configuration'?{status:'value',context,target,key:'scalar',value:{prose:'English Deutsch é',enabled:true,items:[null,1]}}:op==='permitted-origin'?{status:'origins',context,target,key:'scalar',coverage:'permitted-only',origins:[reference]}:{status:'capabilities',context,target,coverage:'permitted-only',capabilities:[declaration]};return {context,target,reference,declaration,value,ok:v=>valid(v,op,context,target,'scalar',limits)};};
 check('closed-native-result-and-declaration-forms',()=>{const d=declarations();assert(Object.isFrozen(d));assert.deepEqual(Object.keys(d.forms),['value','origins','capabilities','unavailable']);assert(Object.values(d.forms).every(Object.isFrozen));assert.deepEqual(d.declarationFields,['key','type','domain','scopes','composition','sensitivity']);assert(Object.isFrozen(d.declarationFields));});
 check('native-values-origins-and-declarations-keep-original-roles',()=>{
  for(const op of operations)for(const key of op==='capabilities'?['scalar']:['scalar','list','map'])for(const route of ['typed','dynamic']){const z=make(),c=z.acquire(),r=z.query(c,op,key,route);assert.equal(r.status,op==='effective-configuration'?'value':op==='permitted-origin'?'origins':'capabilities');assert.equal(valid(r,op,c.context,c.target,key,limits),true);if(r.status==='capabilities')assert.deepEqual(r.capabilities.map(d=>d.key),['scalar','list','map']);nativeCases++;}
 });
 check('independent-operation-fields-bindings-and-value-domains-refuse',()=>{
  for(const op of operations){const z=seed(op);assert(z.ok(z.value));const bad=[{...z.value,extra:true},{...z.value,status:'surprise'},{...z.value,status:1},{...z.value,context:{}},{...z.value,target:{}},...Object.keys(z.value).map(key=>{const v={...z.value};delete v[key];return v;})];if(op==='effective-configuration')bad.push({...z.value,key:'other'},{...z.value,value:undefined},{...z.value,value:Infinity},{...z.value,value:'\ud800'});if(op==='permitted-origin')bad.push({...z.value,key:'other'},{...z.value,coverage:'complete'},{...z.value,origins:null},{...z.value,origins:[1]},{...z.value,origins:Array(1)},{...z.value,origins:{left:'private-id'}});if(op==='capabilities')bad.push({...z.value,coverage:'complete'},{...z.value,capabilities:{}},{...z.value,capabilities:Array(1)},{...z.value,capabilities:[z.declaration,z.declaration]},...[
    {key:''},{type:'integer'},{domain:1},{composition:1},{scopes:'document'},{scopes:[1]},{sensitivity:1},{default:1}
   ].map(p=>({...z.value,capabilities:[{...z.declaration,...p}]})));
   for(const v of bad){assert.equal(z.ok(v),false,op);denials++;}
   for(const other of operations.filter(x=>x!==op)){assert.equal(valid(z.value,other,z.context,z.target,'scalar',limits),false);denials++;}
   const symbol={...z.value,[Symbol('private')]:1};assert.equal(z.ok(symbol),false);denials++;const hidden={...z.value};Object.defineProperty(hidden,'private',{value:1});assert.equal(z.ok(hidden),false);denials++;
   let calls=0;assert.doesNotThrow(()=>assert.equal(z.ok({...z.value,status:{toString(){calls++;throw Error('unexpected status coercion');}}}),false));assert.equal(calls,0);denials++;
   assert(z.ok({status:'unavailable'}));assert.equal(z.ok({status:'unavailable',reason:'private'}),false);denials++;
  }
 });
 check('bounded-json-values-and-declarations-not-a-new-type-compiler',()=>{
  const z=seed('effective-configuration');for(const value of [null,true,0,'',[],{},['prose'],{__proto__:null,'constructor':'literal'}])assert(z.ok({...z.value,value}));for(const field of ['maxRequestBytes','maxInputNodes','maxInputDepth'])assert.equal(valid(z.value,'effective-configuration',z.context,z.target,'scalar',{...limits,[field]:0}),false);const c=seed('capabilities');assert(c.ok({...c.value,capabilities:[]}));assert(c.ok({...c.value,capabilities:[{...c.declaration,type:{kind:'hypothetical'}}]}));for(const field of ['maxRequestBytes','maxInputNodes','maxInputDepth'])assert.equal(valid(c.value,'capabilities',c.context,c.target,'scalar',{...limits,[field]:0}),false);const o=seed('permitted-origin');assert(o.ok({...o.value,origins:[]}));assert(o.ok({...o.value,origins:{}}));assert(o.ok({...o.value,origins:{left:[{}]}}));assert.equal(originsValid([{}],['origin'],new Map([['origin',o.reference]])),false);
 });
 check('origin-membership-order-and-role-equality-are-not-structural',()=>{
  const a=Object.freeze({kind:'origin'}),b=Object.freeze({kind:'origin'}),refs=new Map([['a',a],['b',b]]);for(const [v,data,expected]of [[[a,b],['a','b'],true],[[b,a],['a','b'],false],[[copy(a),b],['a','b'],false],[[a],['a','b'],false],[[a,b,b],['a','b'],false],[[a,b],['a','missing'],false],[{left:[a],right:[b]},{left:['a'],right:['b']},true],[{left:[b],right:[a]},{left:['a'],right:['b']},false],[{left:[a],right:[b],extra:[]},{left:['a'],right:['b']},false]]){assert.equal(originsValid(v,data,refs),expected);originBindingCases++;}
 });
 check('current-native-disclosure-still-controls-empty-and-withheld-products',()=>{
  for(const facet of ['value','origins','declaration','capabilities']){const z=make(),f=copy(z.facts);if(facet==='capabilities')f.capabilities=false;else f.keys.forEach(k=>{k[facet]=false;if(facet==='origins')k.originIds=[];});const c=z.acquire(f);for(const op of operations){const r=z.query(c,op,'scalar','typed',f);assert(valid(r,op,c.context,c.target,'scalar',limits));if(op==='effective-configuration'&&facet==='value'||op==='permitted-origin'&&facet==='origins'||op==='capabilities'&&facet==='capabilities')assert.equal(r.status,'unavailable');if(op==='capabilities'&&facet==='declaration')assert.deepEqual(r.capabilities,[]);nativeCases++;}}
 });
 check('oversize-valid-reply-is-transient-pressure-not-an-invariant-fault',()=>{
  const definition=engineSettingsDefinition();definition.schemas[1].default=['x'.repeat(1024)];const z=make({definition,readProfile:{maxReplyBytes:256}}),c=z.acquire();assert.equal(c.status,'context');const root=z.root(),parent=z.parentUsage(),before=z.usage();assert.equal(z.query(c,'effective-configuration','list').status,'unavailable');assert.equal(z.usage().invalidated,0);assert.equal(z.usage().contexts,before.contexts);assert.equal(z.usage().declaredHeldBytes,before.declaredHeldBytes);assert.equal(z.query(c,'effective-configuration','scalar').value,1);assert.equal(z.root(),root);assert.deepEqual(z.parentUsage(),parent);nativeCases+=2;
 });
 return {scope:'closed-native-F07-settings-query-results',nativeCases,denials,originBindingCases,controls,shapeDoesNotCompileTypesOrEstablishPolicy:true,publicWirePhysicalGlobalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkSettingsQueryResults()));
