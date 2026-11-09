import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {registrationResultValueValid as valid} from './registration-result-values.mjs';
import {nativeSourceReadApi} from './native-source-read-checks.mjs';
import * as kernel from './invocation-publication.mjs';
import * as state from './registration-invocation-state.mjs';
import * as registry from './resource-registration-state.mjs';
import * as bases from './registration-bases.mjs';
import {checkRegistrationInvocations} from './registration-invocation-checks.mjs';
import {checkScopedRegistrations} from './registration-scoped-checks.mjs';
import {checkCreatedAtomic} from './settings-created-result-checks.mjs';
import {createdSettingsTestGraph} from './settings-created-result-test-graph.mjs';
export async function registrationResultValueCorpus(){
 const cases=[],counts={},controls={};let rootAndUsageReadbacks=0;
 const wrap=(api,ports)=>{const result={...api};for(const [name,kind] of ports)result[name]=(...args)=>{
  const root=api.invocationAuthorityRoot(args[0]),usage=api.invocationUsage(args[0]);
  const value=api[name](...args);assert.equal(valid(value,kind),true,name);
  assert.equal(api.invocationAuthorityRoot(args[0]),root);assert.deepEqual(api.invocationUsage(args[0]),usage);rootAndUsageReadbacks++;
  // Producer tests deliberately mutate returned aliases later. Retain this
  // release-time value, preserving opaque created references by identity.
  const captured=kind==='created-settings'&&value.status==='result'?{status:value.status,result:{invocation:value.result.invocation,status:value.result.status,created:value.result.created.map(row=>({index:row.index,reference:row.reference}))}}:structuredClone(value);
  cases.push({value:captured,kind,accepted:true});counts[kind]=(counts[kind]??0)+1;return value;
 };return result;};
 const api=wrap({...nativeSourceReadApi,...kernel,...state,...registry,...bases},[['observeRegistrationResult','host-registration'],['observeScopedRegistrationResult','scoped-registration']]);
 controls.host=checkRegistrationInvocations(api,true).controls;
 controls.scoped=checkScopedRegistrations(api,true).controls;
 const created=wrap(await createdSettingsTestGraph(),[['observeSettingsCreatedResult','created-settings']]);controls.created=checkCreatedAtomic(created).controls;
 const identity={authority:'a',id:'r',incarnation:0},frame=(kind,rows)=>({status:'result',result:{invocation:'i',status:'unchanged',[kind==='created-settings'?'created':'registrations']:rows}});
 const positives=[
  ['host-registration',frame('host-registration',[{table:'read',provider:identity,resource:identity,generation:0},{table:'write',provider:identity,resource:identity,generation:null}])],
  ['scoped-registration',frame('scoped-registration',[{basis:1,state:'absent'},{basis:0,state:'present'}])],
  ['created-settings',frame('created-settings',[{index:0,reference:Object.freeze({})},{index:1,reference:Object.freeze({})}])]
 ];
 for(const kind of ['host-registration','scoped-registration','created-settings'])positives.push([kind,frame(kind,[])]);
 let opaqueReads=0;const opaque=new Proxy({}, {ownKeys(){opaqueReads++;throw Error('opaque traversal');},get(){opaqueReads++;throw Error('opaque read');},getPrototypeOf(){opaqueReads++;throw Error('opaque prototype');}});
 positives.push(['created-settings',frame('created-settings',[{index:0,reference:opaque}])]);
 for(const [kind,value]of positives){assert(valid(value,kind));cases.push({kind,value,accepted:true});}
 const reject=(kind,value)=>{assert.equal(valid(value,kind),false);cases.push({kind,value,accepted:false});};
 let getters=0;const hostile=()=>{getters++;throw Error('value accessor');};
 for(const [kind,value]of positives){
  reject(kind,{...value,extra:true});reject(kind,{status:'unavailable',result:value.result});reject(kind,{...value,status:'other'});
  for(const status of ['not-published','notApplied','unknown',1,null])reject(kind,{...value,result:{...value.result,status}});
  for(const invocation of ['',1,null,'\ud800'])reject(kind,{...value,result:{...value.result,invocation}});
  for(const field of Object.keys(value.result)){const result={...value.result};delete result[field];reject(kind,{...value,result});}
  const hidden={...value};Object.defineProperty(hidden,'secret',{value:1});reject(kind,hidden);
  const hiddenStatus={...value};Object.defineProperty(hiddenStatus,'status',{value:value.status,enumerable:false});reject(kind,hiddenStatus);
  reject(kind,{...value,[Symbol('hidden')]:1});
  const accessor={...value};Object.defineProperty(accessor,'result',{get:hostile,enumerable:true});reject(kind,accessor);
  const key=kind==='created-settings'?'created':'registrations',array=value.result[key];
  const withRows=rs=>({...value,result:{...value.result,[key]:rs}});
  reject(kind,withRows({}));reject(kind,withRows(new Array(1)));
  const extended=[...array];extended.extra=1;reject(kind,withRows(extended));
  const getterArray=[];Object.defineProperty(getterArray,0,{get:hostile,enumerable:true});reject(kind,withRows(getterArray));
  if(array.length){
   const row=array[0];for(const field of Object.keys(row)){const next={...row};delete next[field];reject(kind,withRows([next,...array.slice(1)]));}
   reject(kind,withRows([{...row,extra:1},...array.slice(1)]));
   const next={...row};Object.defineProperty(next,Object.keys(row)[0],{get:hostile,enumerable:true});reject(kind,withRows([next,...array.slice(1)]));
  }
 }
 for(const number of [-1,0.5,Number.MAX_SAFE_INTEGER+1,'0',null]){
  const [kind,value]=positives[0],r=value.result.registrations[0];reject(kind,frame(kind,[{...r,provider:{...identity,incarnation:number}}]));
  if(number!==null)reject(kind,frame(kind,[{...r,generation:number}]));
  reject('scoped-registration',frame('scoped-registration',[{basis:number,state:'present'}]));
 }
 const host=positives[0][1].result.registrations[0];reject('host-registration',frame('host-registration',[host,host]));
 reject('host-registration',frame('host-registration',[{...host,table:'other'}]));
 reject('scoped-registration',frame('scoped-registration',[{basis:0,state:'present'},{basis:0,state:'absent'}]));
 reject('scoped-registration',frame('scoped-registration',[{basis:1,state:'present'}]));
 reject('scoped-registration',frame('scoped-registration',[{basis:0,state:'unknown'}]));
 const ref={};reject('created-settings',frame('created-settings',[{index:0,reference:ref},{index:1,reference:ref}]));
 for(const reference of [null,1,'ref',()=>{}])reject('created-settings',frame('created-settings',[{index:0,reference}]));
 reject('created-settings',frame('created-settings',[{index:1,reference:ref}]));
 assert.equal(getters,0);assert.equal(opaqueReads,0);
 // A copied opaque reference has the same shape, not the same native authority.
 const copied=frame('created-settings',[{index:0,reference:structuredClone(ref)}]);assert(valid(copied,'created-settings'));cases.push({kind:'created-settings',value:copied,accepted:true});
 for(const kind of ['host-registration','scoped-registration','created-settings']){
  for(const value of [null,[],1,'result',{status:'result'},{status:'unavailable',extra:true}])reject(kind,value);
 }
 for(const kind of ['unknown',null,1])reject(kind,{status:'unavailable'});
 return {cases,summary:{scope:'existing-native-F16-released-value-shapes',nativeReplies:rootAndUsageReadbacks,counts,controls,cases:cases.length,malformed:cases.filter(c=>!c.accepted).length,accessorCalls:getters,opaqueReads,rootAndReportedUsageUnchanged:true,shortNativeRegistrationCorpora:true,publicRouteSelected:false,nativePrecommitIntegrationSelected:false,physicalOrIndependentProof:false}};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify((await registrationResultValueCorpus()).summary));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'[audit module]'));process.exitCode=1;}
