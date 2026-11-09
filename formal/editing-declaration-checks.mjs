// Selected first-party declaration evidence, not public decoding or enablement.
import assert from 'node:assert/strict';
import {validateStandardSchemas} from './standard-schema-validator.mjs';
import {pathToFileURL} from 'node:url';
import * as descriptor from './ordinary-edit-descriptor.mjs';
import {outerInputSchemaValid} from './input-schema-profiles.mjs';
import * as kernel from './invocation-publication.mjs';
import * as selection from './selection-products.mjs';
import * as line from './line-edits.mjs';
import * as boundary from './boundary-commands.mjs';
import * as configured from './configured-line-state.mjs';
import {editingInvocationFixture} from './editing-invocation-checks.mjs';
import {configuredLineFixture} from './configured-line-checks.mjs';
const defaultApi={...kernel,...selection,...line,...boundary,...configured,...descriptor},copy=r=>({...structuredClone(r),...(Object.hasOwn(r,'boundary')?{boundary:r.boundary}:{})});
export {defaultApi as editingDeclarationApi};
const expected={selection:['contract','operation','invocation','view','document','sourceRevision','expectedSelection','profile','text','selectionAfter'],line:['contract','operation','action','invocation','view','document','sourceRevision','expectedSelection','profile','settings'],boundary:['contract','operation','invocation','view','document','sourceRevision','expectedSelection','profile','boundary','unit','direction','count','extend'],configured:['contract','operation','action','invocation','document','view','sourceRevision','expectedSelection','profile','configuration']};
export function checkEditingDeclarations(api=defaultApi){
let forms=0,omissions=0,extras=0,contextCases=0,profileCases=0;const cases=[],d=api.ordinaryEditEngineDescriptor();assert(d);const schema=d.variants[0].input.schema;assert(api.ordinaryEditSchemaValid(schema));assert.equal(outerInputSchemaValid(schema),false);assert(Object.isFrozen(d)&&Object.isFrozen(schema.oneOf));assert.notEqual(api.ordinaryEditEngineDescriptor(),d);assert.deepEqual(api.ordinaryEditEngineDescriptor(),d);
for(const change of [v=>v.oneOf.pop(),v=>v.oneOf[0].additionalProperties=true,v=>v.oneOf[0].properties.extra={type:'string'},v=>v.$ref='https://invalid.example/schema']){const bad=structuredClone(schema);change(bad);assert.equal(api.ordinaryEditSchemaValid(bad),false);}
for(const kind of Object.keys(expected)){const x=kind==='configured'?configuredLineFixture(api):editingInvocationFixture(kind,api),root=x.root(),usage=api.invocationUsage(x.domain),h=kind==='configured'?x.host():x.h;
 const d=kind==='selection'?api.selectionInputForms().find(r=>r.family==='F11'):kind==='line'?api.lineInputForm():kind==='boundary'?api.boundaryInputForms().find(r=>r.family==='F11'):api.configuredLineInputForm();
 assert(Object.isFrozen(d)&&Object.isFrozen(d.fields));assert.equal(d.family,'F11');assert.equal(d.variant,'ordinary-edit');assert.equal(d.operation,x.request.operation);assert.equal(d.contract,x.request.contract);assert.equal(d.profileConstant,x.request.profile);assert.deepEqual([...d.fields].sort(),expected[kind].toSorted());assert.deepEqual(Object.keys(x.request).sort(),expected[kind].toSorted());
 const prepare=r=>kind==='selection'?api.prepareSelectionEdit(root,r,h,h.viewLimits):kind==='line'?api.prepareLineEdit(root,r,h,h.viewLimits,h.lineWorkLimits):kind==='boundary'?api.prepareBoundaryCommand(root,r,h,h.viewLimits,h.boundaryWorkLimits):api.prepareInvocationConfiguredLine(x.domain,x.ticket,{route:'typed',request:{family:10,variant:'ordinary-edit',version:1,arguments:r}},x.engineProfile,x.controlContext,h);
 assert(prepare(copy(x.request)),kind);cases.push({request:x.request,expected:true});
 for(const field of expected[kind]){const r=copy(x.request);delete r[field];assert.doesNotThrow(()=>assert.equal(prepare(r),null,kind+'/'+field));cases.push({request:r,expected:false});omissions++;}
 const r={...copy(x.request),extra:true};assert.doesNotThrow(()=>assert.equal(prepare(r),null));cases.push({request:r,expected:false});extras++;
const stale={...copy(x.request),sourceRevision:x.request.sourceRevision+1};assert.equal(prepare(stale),null);cases.push({request:stale,expected:true});contextCases++;
const wrongProfile={...copy(x.request),profile:'unknown-editing-profile'};assert.equal(prepare(wrongProfile),null);cases.push({request:wrongProfile,expected:false});profileCases++;
if(kind==='boundary'){const fake=structuredClone(x.request);assert.equal(prepare(fake),null);cases.push({request:fake,expected:true});contextCases++;}
 assert.equal(x.root(),root);assert.deepEqual(api.invocationUsage(x.domain),usage);forms++;
}
assert.equal(validateStandardSchemas(cases.map(c=>({schema,value:c.request,valid:c.expected}))),60);
return {scope:'isolated-ordinary-edit-handler-declarations',forms,omissions,extras,contextCases,profileCases,standardValidatorCases:cases.length,sharedPrivateGuardSourcesChecked:true,legacySchemaRecognitionUnchanged:true,publicDiscoveryOrCompleteValueCodecProved:false};}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkEditingDeclarations()));
