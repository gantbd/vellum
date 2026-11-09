import assert from 'node:assert/strict';import fs from 'node:fs';import {pathToFileURL} from 'node:url';
import * as original from './advisory-engine-descriptor.mjs';
import {proposalValidationDeclaration} from './invocation-publication.mjs';
import {proposalValidationFixtureForChecks as fixture,checkProposalValidation} from './proposal-validation-checks.mjs';
import {validateStandardSchemas} from './standard-schema-validator.mjs';
const copy=structuredClone;
export function checkAdvisoryDescriptor(api=original){
 const d=api.advisoryEngineDescriptor(),forms=api.advisoryDescriptorInputForms(),native=proposalValidationDeclaration(),catalog=JSON.parse(fs.readFileSync(new URL('./proposal-validation.json',import.meta.url),'utf8')),cases=[],controls=[];let nativeProducts=0,syntaxDenials=0,recognitionDenials=0,runtimeCounterexamples=0;
 const check=(name,fn)=>{fn();controls.push(name);},add=(schema,value,valid)=>cases.push({schema,value:JSON.parse(JSON.stringify(value)),valid});
 assert(d&&forms);assert.equal(d.variants.length,1);const v=d.variants[0],input=v.input.schema,result=v.outcomes.result.schema;
 check('declarations-share-native-request-basis-profile-and-output-fields',()=>{
  assert.equal(d.family,'F10');assert.equal(d.contract,catalog.contract);assert.equal(d.kind,'query');assert.equal(d.version,1);assert.deepEqual(Object.keys(d.presentation),[]);
  for(const key of ['family','contract','operation','profile','requestFields','basisFields','replyFields','gates'])assert.deepEqual(native[key],catalog[key]);
  assert.deepEqual(forms,[{family:'F10',contract:catalog.contract,operation:catalog.operation,variant:catalog.operation,fields:catalog.requestFields}]);
  assert.equal(v.id,catalog.operation);assert.equal(v.input.coverage,'outer-structure-only');assert.deepEqual(input.required,catalog.requestFields);assert.equal(input.additionalProperties,false);assert.equal(v.outcomes.effect,'none');assert.deepEqual(v.outcomes.forms,['advice','unavailable']);
  assert(Object.isFrozen(native));assert(Object.isFrozen(native.requestFields));assert(Object.isFrozen(d));assert(Object.isFrozen(v.input.schema));assert(Object.isFrozen(v.outcomes.result.schema));
  assert(api.advisoryInputSchemaValid(input,d.contract,v.id));assert(api.advisoryOutcomesValid(v.outcomes,d.contract,v.id));
 });
 check('both-native-routes-preserve-current-root-and-reported-usage',()=>{
  for(const route of ['typed','dynamic']){
   const x=fixture(),root=x.root(),usage=copy(x.usage()),request=x.request,reply=x.query(undefined,undefined,undefined,route);
   assert.deepEqual(reply,{status:'advice',profile:catalog.profile,checksSatisfied:true});assert.equal(x.root(),root);assert.deepEqual(x.usage(),usage);add(input,request,true);add(result,reply,true);nativeProducts++;
   for(const key of catalog.requestFields){const bad=copy(request);delete bad[key];add(input,bad,false);syntaxDenials++;assert.equal(x.query(bad,undefined,undefined,route).status,'unavailable');}
   for(const bad of [{...copy(request),extra:true},{...copy(request),contract:'other'},{...copy(request),operation:'execute'},{...copy(request),plan:null},{...copy(request),plan:[]},{...copy(request),plan:1}]){add(input,bad,false);syntaxDenials++;assert.equal(x.query(bad,undefined,undefined,route).status,'unavailable');}
  }
 });
 check('schema-valid-data-does-not-grant-native-meaning-or-observation',()=>{
  for(const route of ['typed','dynamic']){
   for(const mode of ['empty-plan','empty-invocation','wrong-observation-basis']){
    const x=fixture(),request=copy(x.request),root=x.root(),usage=copy(x.usage());let host;
    if(mode==='empty-plan')request.plan={};if(mode==='empty-invocation')request.plan.invocation='';if(mode==='wrong-observation-basis')host={...x.h,proposalValidationBasis:{root:copy(root),context:copy(x.c),request:copy(request)}};
    add(input,request,true);const reply=x.query(request,host,undefined,route);assert.equal(reply.status,'unavailable');assert.equal(x.root(),root);assert.deepEqual(x.usage(),usage);add(result,reply,true);runtimeCounterexamples++;
   }
   const x=fixture(),request=copy(x.request);request.plan.sources[0].expectedSource++;const root=x.root(),usage=copy(x.usage()),reply=x.query(request,undefined,undefined,route);assert.deepEqual(reply,{status:'advice',profile:catalog.profile,checksSatisfied:false});assert.equal(x.root(),root);assert.deepEqual(x.usage(),usage);add(input,request,true);add(result,reply,true);nativeProducts++;
  }
 });
 check('closed-native-result-forms-never-promise-execution',()=>{
  for(const reply of [{status:'unavailable'},{status:'advice',profile:catalog.profile,checksSatisfied:true},{status:'advice',profile:catalog.profile,checksSatisfied:false}]){
   add(result,reply,true);
   for(const key of Object.keys(reply)){const bad=copy(reply);delete bad[key];add(result,bad,false);syntaxDenials++;}
   add(result,{...reply,extra:true},false);syntaxDenials++;
  }
  for(const reply of [{status:'success'},{status:'advice',profile:'other',checksSatisfied:true},{status:'advice',profile:catalog.profile,checksSatisfied:1},{status:'advice',profile:catalog.profile,checksSatisfied:'false'},{status:'advice',profile:catalog.profile,checksSatisfied:null},{status:'unavailable',checksSatisfied:false}]){add(result,reply,false);syntaxDenials++;}
  assert(v.outcomes.notEstablished.includes('execution-permission-or-reservation'));assert(v.outcomes.notEstablished.includes('future-currentness-or-success'));assert(v.requirements.notEstablished.includes('nested-plan-value-validation'));
 });
 check('exact-recognition-refuses-foreign-family-variant-and-mutated-meaning',()=>{
  for(const [contract,variant]of [['foreign',v.id],[d.contract,'exact-batch'],[d.contract,'status']]){assert.equal(api.advisoryInputSchemaValid(input,contract,variant),false);assert.equal(api.advisoryOutcomesValid(v.outcomes,contract,variant),false);recognitionDenials+=2;}
  for(const bad of [{...copy(input),additionalProperties:true},{...copy(input),extra:true}]){assert.equal(api.advisoryInputSchemaValid(bad,d.contract,v.id),false);recognitionDenials++;}
  for(const bad of [{...copy(v.outcomes),effect:'publish'},{...copy(v.outcomes),extra:true}]){assert.equal(api.advisoryOutcomesValid(bad,d.contract,v.id),false);recognitionDenials++;}
 });
 validateStandardSchemas(cases);const baseline=checkProposalValidation(undefined,true);assert.equal(baseline.controls.length,12);
 return {scope:'fixed-existing-F10-advisory-descriptor',controls,forms:forms.length,variants:d.variants.length,nativeProducts,validatorCases:cases.length,syntaxDenials,recognitionDenials,runtimeCounterexamples,nativeControls:baseline.controls.length,nativeGateCases:baseline.gateCases,nativeRouteCases:baseline.routeCases,discoveryRegistrationSelected:false,publicResultRouteOrNewExecutionSemanticsSelected:false,hostPhysicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(checkAdvisoryDescriptor()));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'[audit module]'));process.exitCode=1;}
