import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as original from './selection-engine-descriptor.mjs';
import * as kernel from './invocation-publication.mjs';
import {selectionGatewayFixture} from './selection-gateway-checks.mjs';
import {validateStandardSchemas} from './standard-schema-validator.mjs';
const kinds=['set','logical','match','boundary'],copy=structuredClone;
export function checkSelectionDescriptor(api=original){
 const descriptor=api.selectionEngineDescriptor(),forms=api.selectionDescriptorInputForms(),controls=[],cases=[];let nativeProducts=0,syntaxDenials=0,recognitionDenials=0,runtimeCounterexamples=0;
 const check=(n,f)=>{f();controls.push(n);},row=id=>descriptor.variants.find(v=>v.id===id),add=(schema,value,valid)=>cases.push({schema,value:JSON.parse(JSON.stringify(value)),valid});assert(descriptor&&forms);
 check('exact-three-engine-variants-four-lazy-native-forms',()=>{
  assert.equal(descriptor.family,'F12');assert.equal(descriptor.contract,'engine-selection-declaration/1');assert.equal(descriptor.version,1);assert.equal(descriptor.kind,'localCommand');assert.deepEqual(descriptor.variants.map(v=>v.id),['set-selection','logical-movement','select-search-match']);assert.deepEqual(descriptor.variants.map(v=>v.input.schema.oneOf.length),[1,2,1]);assert.equal(forms.length,4);assert.deepEqual(forms.map(f=>f.contract+':'+f.operation),['selection-edit/1:set-selection','selection-edit/1:logical-movement','selection-edit/1:select-search-match','boundary-command/1:logical-movement']);
  for(const v of descriptor.variants){assert(Object.isFrozen(v.input.schema));assert.equal(v.input.coverage,'outer-structure-only');assert(api.selectionInputSchemaValid(v.input.schema,descriptor.contract,v.id));assert(api.selectionOutcomesValid(v.outcomes,descriptor.contract,v.id));assert.equal(v.requirements.sourceWriteGrant,'not-required-for-view-only-selection');assert.equal(v.outcomes.effect,'view-selection-only; no-source-or-annotation-mutation');assert.equal(Object.keys(descriptor.presentation).length,0);}
 });
 check('both-facades-execute-original-view-only-products-with-standard-syntax',()=>{
  for(const kind of kinds)for(const route of ['typed','dynamic']){
   const x=selectionGatewayFixture(kernel,kind),r=x.request,v=row(r.operation),before=x.root();add(v.input.schema,r,true);assert.equal(x.h.sourceWritable,false);assert.equal(x.h.allTargetsWritable,false);assert.equal(x.submit(r,route).status,'accepted');assert.equal(x.run().status,'settled');assert.equal(x.read().outcome.publication,'applied');assert.deepEqual(x.root().documents,before.documents);assert.deepEqual(x.root().groups,before.groups);nativeProducts++;
   for(const field of Object.keys(r)){const bad=copy(r);delete bad[field];add(v.input.schema,bad,false);syntaxDenials++;}
   for(const bad of [{...copy(r),extra:true},{...copy(r),contract:'surprise'},{...copy(r),profile:'surprise'},{...copy(r),sourceRevision:-1},{...copy(r),sourceRevision:1.5},{...copy(r),expectedSelection:Number.MAX_SAFE_INTEGER+1},{...copy(r),invocation:''},{...copy(r),view:null},{...copy(r),document:[]}]){add(v.input.schema,bad,false);syntaxDenials++;}
   for(const other of descriptor.variants.filter(v=>v.id!==r.operation)){add(other.input.schema,r,false);syntaxDenials++;}
   for(const [field,values]of Object.entries({...(kind==='set'?{selections:[{},[null]],primary:[-1,1.5]}:{}),...(['logical','boundary'].includes(kind)?{unit:[0,''],extend:['false',null]}:{}),...(kind==='boundary'?{boundary:[null,[]],direction:[0,''],count:[0,1.5,Number.MAX_SAFE_INTEGER+1]}:{}),...(kind==='match'?{match:[null,[]],direction:[0,''],affinity:[0,''],selectionMode:[0,'']}: {})}))for(const value of values){add(v.input.schema,{...copy(r),[field]:value},false);syntaxDenials++;}
  }
 });
 check('structural-data-acceptance-never-substitutes-native-values-or-authority',()=>{
  for(const kind of kinds){const x=selectionGatewayFixture(kernel,kind),r=x.request,v=row(r.operation),before=x.root(),usage=x.usage(),bad={...r,sourceRevision:r.sourceRevision+1};add(v.input.schema,bad,true);assert.equal(x.prepare(bad),null);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);runtimeCounterexamples++;}
  for(const kind of ['boundary','match']){const x=selectionGatewayFixture(kernel,kind),field=kind==='boundary'?'boundary':'match',bad={...x.request,[field]:copy(x.request[field])};add(row(bad.operation).input.schema,bad,true);assert.equal(x.prepare(bad),null);runtimeCounterexamples++;}
  for(const kind of ['logical','boundary']){const x=selectionGatewayFixture(kernel,kind),bad={...x.request,unit:'surprise'};add(row(bad.operation).input.schema,bad,true);assert.equal(x.prepare(bad),null);runtimeCounterexamples++;}
  const x=selectionGatewayFixture(kernel,'set'),bad={...x.request,selections:[{anchor:-1,head:0,affinity:'after'}]};add(row(bad.operation).input.schema,bad,true);assert.equal(x.prepare(bad),null);runtimeCounterexamples++;
 });
 check('finite-recognizers-refuse-widening-loader-code-and-false-effect-claims',()=>{
  for(const v of descriptor.variants){
   for(const value of [{...copy(v.input.schema),oneOf:[]},{...copy(v.input.schema),$ref:'https://not-loaded.invalid/schema'},{...copy(v.input.schema),execute:()=>{throw Error('must not execute');}},{...copy(v.input.schema),additionalProperties:true}]){assert.equal(api.selectionInputSchemaValid(value,descriptor.contract,v.id),false);recognitionDenials++;}
   for(const value of [{...copy(v.outcomes),effect:'source-mutation'},{...copy(v.outcomes),notEstablished:[]},{...copy(v.outcomes),release:'unconditional'},{...copy(v.outcomes),execution:'new-network-service'}]){assert.equal(api.selectionOutcomesValid(value,descriptor.contract,v.id),false);recognitionDenials++;}
  }
  for(const v of descriptor.variants)for(const [c,id]of [['foreign',v.id],[descriptor.contract,'ordinary-edit'],[descriptor.contract,'supported-visual-movement']]){assert.equal(api.selectionInputSchemaValid(v.input.schema,c,id),false);assert.equal(api.selectionOutcomesValid(v.outcomes,c,id),false);recognitionDenials+=2;}
 });
 validateStandardSchemas(cases);
 return {scope:'fixed-existing-F12-selection-descriptor',controls,forms:4,variants:3,nativeProducts,validatorCases:cases.length,syntaxDenials,recognitionDenials,runtimeCounterexamples,discoveryRegistrationSelected:false,noNewBehaviorUiPolicyOrPublicReceipt:true,hostPhysicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkSelectionDescriptor()));
