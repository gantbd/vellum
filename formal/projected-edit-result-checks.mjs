import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as original from './projected-edit-results.mjs';
import * as native from './annotation-editing.mjs';
import * as kernel from './invocation-publication.mjs';
import * as reversal from './history-reversal.mjs';
import * as observer from './publication-observation.mjs';
import * as histories from './history-contexts.mjs';
import * as annotationResults from './annotation-results.mjs';
import * as reversalResults from './reversal-results.mjs';
import {checkAnnotationResults} from './annotation-result-checks.mjs';
import {checkReversalResults} from './reversal-result-checks.mjs';
const copy=structuredClone,positiveVersion=[1,Number.MAX_SAFE_INTEGER],badVersion=[0,-1,1.5,Number.MAX_SAFE_INTEGER+1,NaN,Infinity,true,'1',null,undefined];
export function projectedEditResultCorpus(){
 const cases=[],samples=new Map(),observed=[],reads={count:0};let nativeCases=0,nativeFrameCases=0,shapeNonAuthority=0;
 const add=(value,family,valid,kind='malformed')=>cases.push({value,family,valid,kind});
 const api={...native,...kernel,...reversal,...observer,...histories,...annotationResults,...reversalResults};
 for(const [family,getter]of [['F13','observeAnnotationResult'],['F14','observeReversalResult']])api[getter]=(...args)=>{const root=kernel.invocationAuthorityRoot(args[0]),usage=kernel.invocationUsage(args[0]),value=kernel[getter](...args);assert.equal(kernel.invocationAuthorityRoot(args[0]),root);assert.deepEqual(kernel.invocationUsage(args[0]),usage);assert(original.projectedEditResultValid(value,family));nativeCases++;nativeFrameCases++;add(value,family,true,'native');if(value.status!=='unavailable'){const r=value.result,key=JSON.stringify([family,r.status,r.operation,Object.hasOwn(r,'contentVersion'),Object.hasOwn(r,'targetGeneration'),r.target?.kind??null]);if(!samples.has(key))samples.set(key,{family,value});if(!observed.some(x=>x.family===family))observed.push({family,args,value});}return value;};
 const annotation=checkAnnotationResults(api),history=checkReversalResults(api);
 for(const {family,value}of samples.values()){
  add(value,family==='F13'?'F14':'F13',false);add({...copy(value),extra:true},family,false);add({...copy(value),status:'surprise'},family,false);for(const key of ['status','result']){const bad=copy(value);delete bad[key];add(bad,family,false);}
  const r=value.result,optional=family==='F13'?['contentVersion','targetGeneration']:['target'];
  for(const key of Object.keys(r)){const bad=copy(value);delete bad.result[key];add(bad,family,optional.includes(key),optional.includes(key)?'structural':'malformed');}
  for(const key of ['authority','source','body','privateRoot','version'])add({status:value.status,result:{...copy(r),[key]:true}},family,false);
  for(const status of ['surprise','pending','unknown',null,1])add({status:value.status,result:{...copy(r),status}},family,false);
  for(const invocation of ['',null,1,'\ud800'])add({status:value.status,result:{...copy(r),invocation}},family,false);
  if(r.status==='applied')for(const publication of ['',null,1,'foreign'])add({status:value.status,result:{...copy(r),publication}},family,false);
  if(family==='F13'&&r.status!=='not-published'){
   for(const field of ['contentVersion','targetGeneration']){for(const v of positiveVersion)add({status:value.status,result:{...copy(r),[field]:v}},family,true,'structural');for(const v of badVersion)add({status:value.status,result:{...copy(r),[field]:v}},family,false);}
   for(const membership of ['surprise',r.membership==='present'?'retained-absent':'present',null,1])add({status:value.status,result:{...copy(r),membership}},family,false);
   for(const branch of ['annotation','document'])for(const key of branch==='annotation'?['id','incarnation','document']:['authority','id','incarnation']){const bad=copy(value),obj=branch==='annotation'?bad.result.annotation:bad.result.annotation.document;delete obj[key];add(bad,family,false);}
   for(const path of ['annotation','document'])for(const v of badVersion){const bad=copy(value),obj=path==='annotation'?bad.result.annotation:bad.result.annotation.document;obj.incarnation=v;add(bad,family,false);}
   for(const branch of ['annotation','document'])for(const key of branch==='annotation'?['id']:['authority','id'])for(const v of ['',null,1,'\ud800']){const bad=copy(value),obj=branch==='annotation'?bad.result.annotation:bad.result.annotation.document;obj[key]=v;add(bad,family,false);}
   for(const operation of ['create','delete']){const bad=copy(value);bad.result.status='unchanged';bad.result.operation=operation;bad.result.membership=operation==='delete'?'retained-absent':'present';delete bad.result.publication;add(bad,family,false);}
  }
  if(family==='F14'&&r.status==='applied'){
   add({status:value.status,result:{...copy(r),status:'unchanged'}},family,false);for(const target of [null,[],{}, {kind:'surprise'},{kind:'group',group:''},{kind:'group',group:'x',authority:'private'},{kind:'scope-frontier',scope:'x'},{kind:'scope-frontier',scope:'x',expectedGroup:'\ud800'},{kind:'scope-frontier',scope:'',expectedGroup:'x'}])add({status:value.status,result:{...copy(r),target}},family,false);
  }
  const nulls=Object.assign(Object.create(null),copy(value));nulls.result=Object.assign(Object.create(null),copy(r));add(nulls,family,true,'structural');
  for(const place of ['outer','result'])for(const key of Object.keys(place==='outer'?value:r)){const bad=copy(value),obj=place==='outer'?bad:bad.result,prior=obj[key];Object.defineProperty(obj,key,{enumerable:true,get(){reads.count++;return prior;}});add(bad,family,false);}
  for(const place of ['outer','result'])for(const kind of ['symbol','hidden','prototype']){const bad=copy(value),obj=place==='outer'?bad:bad.result;if(kind==='symbol')obj[Symbol('extra')]=true;else if(kind==='hidden')Object.defineProperty(obj,'hidden',{value:true});else Object.setPrototypeOf(obj,{inherited:true});add(bad,family,false);}
  for(const key of ['status','operation']){const bad=copy(value),prior=bad.result[key];bad.result[key]={[Symbol.toPrimitive](){reads.count++;return prior;}};add(bad,family,false);}
 }
 for(const family of ['F13','F14']){add({status:'unavailable'},family,true,'structural');for(const value of [null,[],{}, {status:'unavailable',result:{}},{status:'surprise'},{status:1}])add(value,family,false);add({status:'unavailable'},'unknown',false);}
 for(const {family,args,value}of observed){const getter=family==='F13'?'observeAnnotationResult':'observeReversalResult';assert(original.projectedEditResultValid(copy(value),family));assert.equal(kernel[getter](args[0],copy(args[1]),...args.slice(2)).status,'unavailable');shapeNonAuthority++;assert.equal(kernel[getter](args[0],args[1],args[2],args[3],{...args[4],authorized:false}).status,'unavailable');shapeNonAuthority++;}
 return {cases,reads,nativeCases,nativeFrameCases,shapeNonAuthority,samples:samples.size,nativeControls:{annotation:annotation.controls.length,reversal:history.controls.length}};
}
export function checkProjectedEditResults(api=original,corpus=projectedEditResultCorpus()){
 corpus.reads.count=0;for(const [index,c]of corpus.cases.entries())assert.equal(api.projectedEditResultValid(c.value,c.family),c.valid,'projected result case '+index+' '+c.kind);assert.equal(corpus.reads.count,0);
 const d=api.projectedEditResultDeclarations();assert(Object.isFrozen(d)&&Object.isFrozen(d.annotation.optionalVersions));assert.deepEqual(d.annotation.operations,['create','update-body','reattach','delete']);assert.deepEqual(d.reversal.operations,['undo','redo']);
 return {scope:'existing-projected-F13-F14-native-result-values',nativeCases:corpus.nativeCases,nativeFrameCases:corpus.nativeFrameCases,samples:corpus.samples,cases:corpus.cases.length,malformed:corpus.cases.filter(c=>!c.valid).length,structuralPositives:corpus.cases.filter(c=>c.valid&&c.kind==='structural').length,shapeNonAuthority:corpus.shapeNonAuthority,nativeControls:corpus.nativeControls,getterCalls:corpus.reads.count,publicRouteOrCodecSelected:false,nativePrecommitIntegrationOrRawIngressPhysicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkProjectedEditResults()));
