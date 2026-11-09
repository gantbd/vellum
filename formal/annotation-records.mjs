// Logical authored-field reference, not full history/mapping/persistence engine.
import assert from 'node:assert/strict';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
const copy=structuredClone;
const validBody=s=>typeof s==='string'&&[...s].every(c=>{const n=c.codePointAt(0);return n<0xd800||n>0xdfff;});
export function changeAnnotation(state,request,checks) {
  if(!checks.authorized||!checks.bounded||!checks.notStopped)return {status:'refused',state};
  const row=state.record;
  if(request.kind==='create') {
    if(row||!checks.targetCurrent||!validBody(request.body)||
      state.retired.some(r=>r.id===request.id&&r.incarnation===request.incarnation))return {status:'refused',state};
    return {status:'changed',state:{...state,record:{id:request.id,incarnation:request.incarnation,
      body:request.body,bodyVersion:1,target:copy(request.target),targetGeneration:1}}};
  }
  if(!row||row.id!==request.id||row.incarnation!==request.incarnation)return {status:'refused',state};
  if(request.kind==='body') {
    if(row.bodyVersion!==request.expectedBody||!validBody(request.body))return {status:'refused',state};
    if(row.body===request.body)return {status:'unchanged',state};
    return {status:'changed',state:{...state,record:{...row,body:request.body,bodyVersion:row.bodyVersion+1}}};
  }
  if(request.kind==='reattach') {
    if(row.targetGeneration!==request.expectedTarget||!checks.targetCurrent)return {status:'refused',state};
    if(equal(row.target,request.target))return {status:'unchanged',state};
    return {status:'changed',state:{...state,record:{...row,target:copy(request.target),targetGeneration:row.targetGeneration+1}}};
  }
  if(request.kind==='delete') {
    if(row.bodyVersion!==request.expectedBody||row.targetGeneration!==request.expectedTarget)return {status:'refused',state};
    return {status:'changed',state:{...state,record:null,retired:[...state.retired,copy(row)]}};
  }
  return {status:'refused',state};
}
const checks={authorized:true,bounded:true,notStopped:true,targetCurrent:true};
const state={source:'unchanged source',record:{id:'a',incarnation:1,body:'old',bodyVersion:1,
  target:{document:'d',incarnation:1,revision:1,lo:0,hi:3},targetGeneration:1},retired:[]};
const body={kind:'body',id:'a',incarnation:1,expectedBody:1,body:''};
const target={kind:'reattach',id:'a',incarnation:1,expectedTarget:1,
  target:{document:'d',incarnation:1,revision:2,lo:4,hi:7}};
export function checkAnnotationRecords() {
  let frameCases=0;const controls=[];
  const control=(name,ok)=>{assert(ok,name);controls.push(name);};
  for(const request of [body,target,{kind:'delete',id:'a',incarnation:1,expectedBody:1,expectedTarget:1}])
    for(let mask=0;mask<16;mask++) {
      const gate=Object.fromEntries(Object.keys(checks).map((k,i)=>[k,!!(mask&(1<<i))]));
      const before=copy({state,request,gate}),result=changeAnnotation(state,request,gate);
      const permitted=(mask&7)===7&&(request.kind!=='reattach'||!!(mask&8));
      assert.equal(result.status,permitted?'changed':'refused');
      assert.equal(result.state.source,state.source);
      if(!permitted)assert.deepEqual(result.state,state);
      assert.deepEqual({state,request,gate},before);frameCases++;
    }
  const erased=changeAnnotation(state,body,checks).state;
  control('empty-body-not-deletion',erased.record!==null&&erased.record.body==='');
  const created=changeAnnotation({...state,record:null},{kind:'create',id:'b',incarnation:1,body:'',target:state.record.target},checks);
  control('create-empty-body-is-authored-record',created.status==='changed'&&created.state.record.body==='');
  const left=changeAnnotation(changeAnnotation(state,body,checks).state,target,checks).state;
  const right=changeAnnotation(changeAnnotation(state,target,checks).state,body,checks).state;
  control('independent-body-and-target-fields-preserve-each-other',JSON.stringify(left)===JSON.stringify(right));
  control('target-change-does-not-overwrite-body',left.record.body===''&&left.record.bodyVersion===2);
  control('body-change-does-not-overwrite-target',left.record.target.revision===2&&left.record.targetGeneration===2);
  const refuse=(name,request,s=state)=>{
    const before=copy({s,request});const result=changeAnnotation(s,request,checks);
    assert.equal(result.status,'refused');assert.deepEqual(result.state,s);assert.deepEqual({s,request},before);controls.push(name);
  };
  refuse('stale-body-version',{...body,expectedBody:0});
  refuse('old-incarnation-not-recreated-record',{...body,incarnation:0});
  refuse('stale-target-generation',{...target,expectedTarget:0});
  refuse('delete-captures-current-body',{kind:'delete',id:'a',incarnation:1,expectedBody:1,expectedTarget:2},left);
  refuse('delete-captures-current-target',{kind:'delete',id:'a',incarnation:1,expectedBody:2,expectedTarget:1},left);
  const unchanged=(name,request)=>{
    const before=copy({state,request,checks}),result=changeAnnotation(state,request,checks);
    assert.equal(result.status,'unchanged');assert.deepEqual(result.state,before.state);
    assert.deepEqual({state,request,checks},before);controls.push(name);
  };
  unchanged('same-body-not-new-body-version',{...body,body:'old'});
  unchanged('same-target-not-new-target-generation',{...target,target:state.record.target});
  assert.throws(()=>assert.deepEqual({...copy(state),record:{...copy(state.record),bodyVersion:2}},state));
  controls.push('unchanged-version-mutation-sensitive');
  refuse('invalid-scalar-body-not-normalized',{...body,body:'\ud800'});
  const deleted=changeAnnotation(state,{kind:'delete',id:'a',incarnation:1,expectedBody:1,expectedTarget:1},checks).state;
  control('delete-keeps-required-retired-evidence',deleted.record===null&&deleted.retired.length===1&&deleted.retired[0].body==='old');
  refuse('deleted-incarnation-not-reused',{kind:'create',id:'a',incarnation:1,body:'new',target:state.record.target},deleted);
  control('fresh-incarnation-distinct-record',changeAnnotation(deleted,{kind:'create',id:'a',incarnation:2,body:'new',target:state.record.target},checks).status==='changed');
  refuse('unknown-verb-no-mutation',{...body,kind:'run-body'});
  return {status:'annotation-authored-field-only',frameCases,controls,
    fullHistoryMappingProved:false,persistenceFormatSelected:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(checkAnnotationRecords(),null,2));
