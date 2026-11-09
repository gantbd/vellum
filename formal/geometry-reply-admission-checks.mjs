import assert from 'node:assert/strict';import fs from 'node:fs';import {pathToFileURL} from 'node:url';
import * as original from './geometry-products.mjs';
import {geometryFixtureForChecks as fixture,geometryLimitsForChecks as caps} from './geometry-product-checks.mjs';
const navigation=origin=>({contract:'geometry-query/1',operation:'visual-navigation-target',origin,unit:'right',count:1,goal:null,choices:[null]});
const hit=x=>({contract:'geometry-query/1',operation:'caret-hit-conversion',request:{kind:'hit',x,y:0,choice:null}});
const frame=kind=>({status:'source',point:{position:1,affinity:'after'},index:1,...(kind==='navigation'?{goal:{layout:{profile:'declared-caret-geometry/1'},x:0}}:{}),reference:{kind:'owned-visual-target/1'}});
export function checkGeometryReplyAdmission(api=original){
 const controls=[],run=(name,fn)=>{fn();controls.push(name);};let boundaries=0,refusals=0;
 for(const kind of ['navigation','hit']){
  const request=kind==='navigation'?navigation(0):hit(4),other=kind==='navigation'?navigation(1):hit(8),expected=frame(kind),bytes=Buffer.byteLength(JSON.stringify(expected));
  for(const offset of [-1,0,1]){
   const f=fixture(),limits={...caps,maxTargets:1,maxReplyBytes:bytes+offset},layout=api.acceptGeometry(f.state,f.packet,f.current,limits);assert(layout);
   const before=structuredClone(f.state),reply=api.queryGeometry(f.state,layout,request,f.current,limits);
   assert.equal(reply.status,offset<0?'unavailable':'source');assert.deepEqual(f.state,before);assert.deepEqual(f.packet.source,f.state.documents[0].source);
   if(offset>=0){assert.deepEqual(reply,expected);assert.equal(api.queryGeometry(f.state,layout,request,f.current,limits).reference,reply.reference);assert.equal(api.queryGeometry(f.state,layout,other,f.current,{...limits,maxReplyBytes:caps.maxReplyBytes}).status,'unavailable');}
   else{assert.equal(api.queryGeometry(f.state,layout,other,f.current,{...limits,maxReplyBytes:caps.maxReplyBytes}).status,'source');refusals++;}boundaries++;
  }
  run(kind+'-repeated-withheld-replies-do-not-consume-target-slot',()=>{
   const f=fixture(),limits={...caps,maxTargets:1},layout=api.acceptGeometry(f.state,f.packet,f.current,limits);assert(layout);
   for(let i=0;i<3;i++){assert.equal(api.queryGeometry(f.state,layout,request,f.current,{...limits,maxReplyBytes:1}).status,'unavailable');refusals++;}
   const accepted=api.queryGeometry(f.state,layout,other,f.current,limits);assert.equal(accepted.status,'source');assert.equal(api.queryGeometry(f.state,layout,other,f.current,limits).reference,accepted.reference);assert.equal(api.queryGeometry(f.state,layout,request,f.current,limits).status,'unavailable');
  });
  run(kind+'-withholding-existing-reference-does-not-revoke-it',()=>{
   const f=fixture(),limits={...caps,maxTargets:1},layout=api.acceptGeometry(f.state,f.packet,f.current,limits),accepted=api.queryGeometry(f.state,layout,request,f.current,limits);assert.equal(accepted.status,'source');
   assert.equal(api.queryGeometry(f.state,layout,request,f.current,{...limits,maxReplyBytes:1}).status,'unavailable');refusals++;
   assert.equal(api.queryGeometry(f.state,layout,request,f.current,limits).reference,accepted.reference);
   assert.equal(api.queryGeometry(f.state,layout,request,{...f.current,scope:false},limits).status,'unavailable');
   assert(api.retireGeometry(layout));assert.equal(api.queryGeometry(f.state,layout,request,f.current,limits).status,'unavailable');
  });
 }
 return {scope:'native-geometry-source-reply-preinstallation-admission',controls,boundaries,withheldReplies:refusals,originalTargetIdentityAndQuotaPreserved:true,publicFacadeOrPhysicalOrIndependentProof:false};
}
export async function checkGeometryReplyAdmissionMutations(){
 const source=fs.readFileSync(new URL('./geometry-products.mjs',import.meta.url),'utf8'),guard='if(size({...result,reference:ref})>limits.maxReplyBytes)return unavailable();',install='targets.set(ref,{layout:ticket,index:result.index,point:copy(result.point),origin:copy(origin),steps});record.cache.set(key,ref);';
 assert.equal(source.split(guard).length-1,1);assert.equal(source.split(install).length-1,1);
 const variants=[['missing-reply-preflight',source.replace(guard,'')],['reply-preflight-after-target-installation',source.replace(guard,'').replace(install,install+guard)],['reply-preflight-uses-request-budget',source.replace(guard,guard.replace('maxReplyBytes','maxRequestBytes'))]];
 const load=async code=>import('data:text/javascript;base64,'+Buffer.from(code.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(new URL(p,import.meta.url).href)).replace("new URL('./geometry-products.json',import.meta.url)",'new URL('+JSON.stringify(new URL('./geometry-products.json',import.meta.url).href)+')')).toString('base64'));
 checkGeometryReplyAdmission(await load(source));const killed=[];
 for(const [name,code]of variants){const api=await load(code);let failed=false;try{checkGeometryReplyAdmission(api);}catch(e){assert.equal(e.code,'ERR_ASSERTION');failed=true;}assert(failed,name);killed.push(name);}return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify({baseline:checkGeometryReplyAdmission(),mutations:await checkGeometryReplyAdmissionMutations(),constructionOutsideDetection:true}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'[audit module]'));process.exitCode=1;}
