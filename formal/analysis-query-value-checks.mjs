import assert from 'node:assert/strict';import {pathToFileURL} from 'node:url';
import fs from 'node:fs';
import * as native from './analysis-work.mjs';import {checkAnalysisWork,analysisWorkFixture} from './analysis-work-checks.mjs';
import * as original from './analysis-query-values.mjs';
const copy=structuredClone,operations=['supported-analysis','supplied-evidence-inspection'];let retained;
export function analysisQueryValueCorpus(){
 if(retained)return retained;const frames=[],authority=[];let unknownRequests=0;
 const api={...native,queryAnalysis(owner,state,request,g){
  const before=copy(state),usage=copy(native.analysisUsage(owner)),out=native.queryAnalysis(owner,state,request,g);
  assert.deepEqual(state,before);assert.deepEqual(native.analysisUsage(owner),usage);
  if(operations.includes(request?.operation))frames.push({operation:request.operation,value:copy(out)});else{assert.deepEqual(out,{status:'unavailable'});unknownRequests++;}
  if(!authority.length&&out.status==='captures'&&out.captures.length){
   for(const [r,h]of [[{...request,snapshot:copy(request.snapshot)},g],[request,{...g,authorized:false}]]){
    assert.deepEqual(native.queryAnalysis(owner,state,r,h),{status:'unavailable'});assert.deepEqual(state,before);assert.deepEqual(native.analysisUsage(owner),usage);authority.push(copy(out));
   }
  }
  return out;
 }};
 const producer=checkAnalysisWork(api);assert.equal(producer.controls.length,88);assert.equal(producer.emptyRangeCases,872);assert.equal(producer.scalarRangeCases,858);assert.equal(producer.interiorRangeRefusals,4);assert.equal(authority.length,2);
 const captures=copy(frames.find(f=>f.value.status==='captures'&&f.value.captures.length).value),summary=copy(frames.find(f=>f.value.status==='analysis-summary').value),samples=[...operations.map(operation=>({operation,value:{status:'unavailable'}})),{operation:operations[0],value:captures},{operation:operations[1],value:summary}],positive=[],bad=[];let getterCalls=0;
 const invalid=(operation,value)=>bad.push({operation,value}),valid=(operation,value)=>positive.push({operation,value});
 for(const sample of samples){
  const {operation,value}=sample;for(const key of Object.keys(value)){const v=copy(value);delete v[key];invalid(operation,v);}
  invalid(operation,{...copy(value),extra:true});invalid(operation,{...copy(value),status:'success'});invalid(operation,null);invalid(operation,[]);
  const accessor=copy(value);Object.defineProperty(accessor,'status',{enumerable:true,get(){getterCalls++;return value.status;}});invalid(operation,accessor);
  const hidden=copy(value);Object.defineProperty(hidden,'status',{enumerable:false,value:value.status});invalid(operation,hidden);
  const symbol=copy(value);symbol[Symbol('extra')]=true;invalid(operation,symbol);
 }
 for(const operation of operations){
  const base=operation===operations[0]?captures:summary;
  for(const parseState of ['parsed','syntax-errors'])for(const coverage of ['complete-within-profile','partial'])valid(operation,{...copy(base),parseState,coverage});
  for(const value of [0,Number.MAX_SAFE_INTEGER]){const v=copy(base);v.context.sourceRevision=value;for(const k of Object.keys(v.context).filter(k=>k.endsWith('Generation')))v.context[k]=value;v.context.document.incarnation=value;v.context.analysis.incarnation=value;valid(operation,v);}
  for(const key of Object.keys(base.context)){const v=copy(base);delete v.context[key];invalid(operation,v);}
  for(const key of ['sourceRevision',...Object.keys(base.context).filter(k=>k.endsWith('Generation'))])for(const value of [-1,1.5,Number.MAX_SAFE_INTEGER+1]){const v=copy(base);v.context[key]=value;invalid(operation,v);}
  for(const key of ['document','analysis'])for(const change of [v=>v.id='',v=>v.authority=0,v=>v.incarnation=-1,v=>v.incarnation=1.5,v=>v.incarnation=Number.MAX_SAFE_INTEGER+1,v=>v.extra=true,v=>delete v.id]){const v=copy(base);change(v.context[key]);invalid(operation,v);}
  for(const change of [v=>v.context.extra=true,v=>v.context.languageProfile='foreign',v=>v.parseState='complete',v=>v.coverage='complete',v=>v.context=null]){const v=copy(base);change(v);invalid(operation,v);}
  invalid(operation,copy(operation===operations[0]?summary:captures));
 }
 valid(operations[0],{...copy(captures),captures:[]});
 valid(operations[0],{...copy(captures),captures:[['host-block','',0,1],['host-inline','α',1,2],['child','child',Number.MAX_SAFE_INTEGER-1,Number.MAX_SAFE_INTEGER]]});
 // Identity strings follow this producer's nonempty-string rules; these fields
 // are not UTF-8 source text or an interchange codec selection.
 {const v=copy(captures);v.context.document.id='\ud800';valid(operations[0],v);}
 for(const change of [v=>v.captures=null,v=>v.captures={},v=>v.captures[0]=null,v=>v.captures[0]=[],v=>v.captures[0][0]='foreign',v=>v.captures[0][1]=0,v=>v.captures[0][1]='\ud800',v=>v.captures[0][2]=-1,v=>v.captures[0][2]=1.5,v=>v.captures[0][3]=Number.MAX_SAFE_INTEGER+1,v=>v.captures[0][3]=v.captures[0][2],v=>v.captures[0][2]=v.captures[0][3]+1,v=>v.captures[0].push('extra'),v=>v.captures.extra=true,v=>v.captures[0].extra=true]){
  const v=copy(captures);change(v);invalid(operations[0],v);
 }
 for(const rows of [[['host-block','one',1,3],['child','two',2,4]],[['host-block','one',3,4],['child','two',1,2]]])invalid(operations[0],{...copy(captures),captures:rows});
 {const v=copy(captures);delete v.captures[0];invalid(operations[0],v);}
 {const v=copy(captures);delete v.captures[0][1];invalid(operations[0],v);}
 {const v=copy(captures);Object.defineProperty(v.captures[0],'1',{enumerable:true,get(){getterCalls++;return 'not-read';}});invalid(operations[0],v);}
 {const v=copy(captures);Object.defineProperty(v.context.document,'id',{enumerable:true,get(){getterCalls++;return 'not-read';}});invalid(operations[0],v);}
 {const v=copy(captures);Object.defineProperty(v,'hidden',{value:true});invalid(operations[0],v);}
 retained={frames,producer,authority,unknownRequests,positive,bad,getterCalls:()=>getterCalls,resetGetterCalls:()=>{getterCalls=0;}};return retained;
}
export function checkAnalysisQueryValues(api=original,corpus=analysisQueryValueCorpus()){
 corpus.resetGetterCalls();
 const controls=[],check=(name,fn)=>{fn();controls.push(name);};
 check('actual-native-current-replies-preserve-root-and-reported-usage',()=>{assert(corpus.frames.length);for(const f of corpus.frames)assert.equal(api.analysisQueryValueValid(f.value,f.operation),true);});
 check('producer-compatible-values-not-new-universal-coordinate-or-codec-policy',()=>{for(const f of corpus.positive)assert.equal(api.analysisQueryValueValid(f.value,f.operation),true);});
 check('closed-fields-enums-scalar-types-ordered-positive-captures',()=>{for(const f of corpus.bad)assert.equal(api.analysisQueryValueValid(f.value,f.operation),false);assert.equal(corpus.getterCalls(),0);});
 check('operation-binds-captures-versus-summary-and-unavailable',()=>{for(const f of corpus.frames)for(const operation of [undefined,'foreign','shutdown'])assert.equal(api.analysisQueryValueValid(f.value,operation),false);});
 check('copied-snapshot-and-revoked-grant-refuse-despite-valid-return-shape',()=>{assert.equal(corpus.authority.length,2);for(const value of corpus.authority)assert.equal(api.analysisQueryValueValid(value,'supported-analysis'),true);});
 return {scope:'closed-existing-F08-analysis-query-values',controls,nativeReplies:corpus.frames.length,nativeProducerControls:corpus.producer.controls.length,nativePacketCases:corpus.producer.nativePacketCases,structuralPositives:corpus.positive.length,malformed:corpus.bad.length,cases:corpus.frames.length+corpus.positive.length+corpus.bad.length,authorityWitnesses:2,unknownNativeRequests:corpus.unknownRequests,getterCalls:corpus.getterCalls(),nativeReleaseIntegrationOrPublicCodecSelected:false,physicalGlobalRawIngressOrIndependentProof:false};
}
const nativeQuerySource=fs.readFileSync(new URL('./analysis-work.mjs',import.meta.url),'utf8');
async function loadNativeQuery(source,name){
 const text=source.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(new URL(p,import.meta.url).href)).replace(/new URL\('(\.\/[^']+)',import.meta.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,import.meta.url).href)+',import.meta.url)');
 return import('data:text/javascript;base64,'+Buffer.from(text).toString('base64')+'#'+name);
}
export async function checkAnalysisNativeQueryGuard(source=nativeQuerySource,packets=null){
 const api=await loadNativeQuery(source,'query-guard-base'),sample=analysisWorkFixture(api,packets??undefined),f=sample.packets,controls=[],check=async(name,fn)=>{await fn();controls.push(name);};let faultCases=0,reentryCases=0,capacityCases=0,constructionCases=0;
 const make=(a=api,l=sample.limits)=>analysisWorkFixture(a,f,l),workspace=l=>2*(l.maxEnvelopeBytes+Math.max(Buffer.byteLength(JSON.stringify({status:'captures'})),Buffer.byteLength(JSON.stringify({status:'analysis-summary'}))));
 // The summary has no range field in the native exact request declaration.
 const query=(a,x,snapshot,operation='supported-analysis',g=x.gates)=>{const r=x.query(snapshot,{operation});if(operation==='supplied-evidence-inspection')delete r.range;return a.queryAnalysis(x.owner,x.state,r,g);};
 await check('both-native-replies-refund-their-actual-owner-workspace',()=>{for(const operation of operations){const x=make(),a=x.ready(),before=api.analysisUsage(x.owner),root=copy(x.state);assert.equal(query(api,x,a.snapshot,operation).status,operation==='supported-analysis'?'captures':'analysis-summary');assert.deepEqual(api.analysisUsage(x.owner),before);assert.deepEqual(x.state,root);}});
 await check('two-construction-frame-headroom-is-exact-and-refusal-precedes-input-inspection',()=>{const x=make(),a=x.ready(),held=api.analysisUsage(x.owner).heldBytes,bytes=workspace(x.limits);for(const delta of [-1,0,1]){const y=make(api,{...x.limits,maxHeldBytes:held+bytes+delta}),b=y.ready(),before=api.analysisUsage(y.owner);let reads=0;const g=delta<0?new Proxy(y.gates,{get(t,k){reads++;return t[k];}}):y.gates;assert.equal(query(api,y,b.snapshot,'supported-analysis',g).status,delta<0?'unavailable':'captures');assert.equal(reads,0);assert.deepEqual(api.analysisUsage(y.owner),before);capacityCases++;}});
 const constructionMarker='  if(!(size(result)<=o.limits.maxReplyBytes))',constructionAPI=await loadNativeQuery(source.replace(constructionMarker,'  g.testOnlyConstructionProbe?.(size(result));\n'+constructionMarker),'query-construction');assert.equal(source.split(constructionMarker).length-1,1);for(const maxReplyBytes of [0,1])for(const operation of operations){const x=make(constructionAPI,{...sample.limits,maxReplyBytes}),ready=x.ready(),before=constructionAPI.analysisUsage(x.owner);let calls=0;const g={...x.gates,testOnlyConstructionProbe(bytes){calls++;const u=constructionAPI.analysisUsage(x.owner);assert.equal(u.queryBytes,workspace(x.limits));assert(bytes<=u.queryBytes/2,'assembled logical reply must fit its construction frame');assert(bytes>maxReplyBytes);}};assert.equal(query(constructionAPI,x,ready.snapshot,operation,g).status,'unavailable');assert.equal(calls,1);assert.deepEqual(constructionAPI.analysisUsage(x.owner),before);constructionCases++;}
 const marker='  if(!analysisQueryValueValid(result,r.operation)',corruptions={status:"result={...result,status:'unexpected'};",context:'result={...result,context:{...result.context,sourceRevision:result.context.sourceRevision+1}};',parse:"result={...result,parseState:result.parseState==='parsed'?'syntax-errors':'parsed'};",coverage:"result={...result,coverage:result.coverage==='partial'?'complete-within-profile':'partial'};",exception:"result=new Proxy(result,{getPrototypeOf(){throw Error('selected validator exception');}});"};
 assert.equal(source.split(marker).length-1,1,'native guard construction seam');
 await check('invalid-produced-values-fence-data-plane-but-retain-protected-drain',async()=>{for(const [name,body]of Object.entries(corruptions)){
  const a=await loadNativeQuery(source.replace(marker,'  if(g.testOnlyCorrupt===true){'+body+'}\n'+marker),'query-guard-'+name);
  for(const operation of operations){const x=make(a),ready=x.ready(),pending=x.admit('pending'),before=a.analysisUsage(x.owner),status=a.analysisTaskStatus(x.owner,ready.task),root=copy(x.state);assert.equal(query(a,x,ready.snapshot,operation,{...x.gates,testOnlyCorrupt:true}).status,'unavailable');assert.deepEqual(a.analysisUsage(x.owner),{...before,queryFaulted:true});assert.deepEqual(x.state,root);assert.deepEqual(a.analysisTaskStatus(x.owner,ready.task),status);assert.equal(query(a,x,ready.snapshot).status,'unavailable');assert.equal(a.admitAnalysisWork(x.owner,x.state,x.request('later'),x.gates).status,'unavailable');assert.equal(a.submitAnalysisWork(x.owner,pending,x.state,x.gates).status,'not-submitted');assert.equal(a.acceptAnalysisWork(x.owner,ready.task,x.state,x.gates).status,'unavailable');assert.equal(a.controlAnalysisWork(x.owner,pending,'cancel',x.gates).status,'stopped');assert.equal(a.observeAnalysisWork(x.owner,x.event(ready.task,'quiescence',null),x.ingress).status,'quiescent');assert.equal(a.retireAnalysisWork(x.owner,pending,x.retirement).status,'retired');assert.equal(a.retireAnalysisWork(x.owner,ready.task,x.retirement).status,'retired');const after=a.analysisUsage(x.owner);assert.equal(after.heldBytes,0);assert.equal(after.issued,before.issued);assert.equal(after.queryFaulted,true);faultCases++;}
 }});
 await check('busy-query-protects-all-owner-mutation-and-retirement-seams',async()=>{const a=await loadNativeQuery(source.replace(marker,'  g.testOnlyQueryProbe?.();\n'+marker),'query-reentry'),x=make(a),ready=x.ready(),pending=x.admit('pending');x.quiet(ready.task);const before=a.analysisUsage(x.owner);let calls=0;const g={...x.gates,testOnlyQueryProbe(){calls++;const u=a.analysisUsage(x.owner);assert.equal(u.queryBusy,true);assert.equal(u.queryBytes,workspace(x.limits));assert.equal(u.heldBytes,before.heldBytes+u.queryBytes);assert.equal(query(a,x,ready.snapshot).status,'unavailable');assert.equal(a.admitAnalysisWork(x.owner,x.state,x.request('nested'),x.gates).status,'unavailable');assert.equal(a.submitAnalysisWork(x.owner,pending,x.state,x.gates).status,'not-submitted');assert.equal(a.acceptAnalysisWork(x.owner,ready.task,x.state,x.gates).status,'unavailable');assert.equal(a.controlAnalysisWork(x.owner,pending,'cancel',x.gates).status,'unavailable');assert.equal(a.observeAnalysisWork(x.owner,x.event(ready.task,'quiescence',null),x.ingress).status,'unavailable');assert.equal(a.retireAnalysisWork(x.owner,ready.task,x.retirement).status,'unavailable');reentryCases+=7;}};assert.equal(query(a,x,ready.snapshot,'supported-analysis',g).status,'captures');assert.equal(calls,1);assert.deepEqual(a.analysisUsage(x.owner),before);});
 await check('ordinary-authority-and-alias-refusals-do-not-become-owner-faults',()=>{const x=make(),a=x.ready(),before=api.analysisUsage(x.owner);for(const [s,g]of [[copy(a.snapshot),x.gates],[a.snapshot,{...x.gates,authorized:false}]])assert.equal(query(api,x,s,'supported-analysis',g).status,'unavailable');assert.deepEqual(api.analysisUsage(x.owner),before);assert.equal(query(api,x,a.snapshot).status,'captures');});
 return {scope:'actual-F08-native-query-result-workspace-and-fault-guard',controls,faultCases,reentryCases,capacityCases,constructionCases,originalTaskAndSourceFramesChecked:true,protectedEvidenceAndRetirementRetained:true,declaredWorkspaceFrames:2,newPublicProfileOrVerb:false,physicalGlobalHostileIngressOrIndependentProof:false};
}
export async function checkAnalysisNativeQueryGuardMutations(){
 const sample=analysisWorkFixture(),packets=sample.packets,baseline=await checkAnalysisNativeQueryGuard(nativeQuerySource,packets),rules=[
  ['result-guard-bypassed','!analysisQueryValueValid(result,r.operation)','false'],
  ['context-correlation-ignored','!equal(result.context,s.packet.context)','false'],
  ['workspace-one-frame-short','workspace=2*queryFrameBytes','workspace=1*queryFrameBytes'],
  ['workspace-from-delivery-limit','workspace=2*queryFrameBytes','workspace=2*o.limits.maxReplyBytes'],
  ['query-reentry-allowed','if(!o||o.queryBusy||o.queryFaulted)return fail();','if(!o||o.queryFaulted)return fail();'],
  ['fault-not-latched','catch{o.queryFaulted=true;return fail();}','catch{o.queryFaulted=false;return fail();}'],
  ['workspace-not-refunded','finally{o.queryBytes=0;o.queryBusy=false;}','finally{o.queryBusy=false;}'],
  ['fault-blocks-protected-retirement','if(!t||o.queryBusy||!g?.retirementAllowed','if(!t||o.queryBusy||o.queryFaulted||!g?.retirementAllowed']
 ],mutations=[];
 for(const [name,from,to]of rules){const fn=name==='fault-blocks-protected-retirement'?'retireAnalysisWork':'queryAnalysis',a=nativeQuerySource.indexOf('export function '+fn+'('),next=nativeQuerySource.indexOf('\nexport function ',a+1),b=next<0?nativeQuerySource.length:next;assert(a>=0,name+': construction function');const region=nativeQuerySource.slice(a,b);assert.equal(region.split(from).length-1,1,name);const source=nativeQuerySource.slice(0,a)+region.replace(from,to)+nativeQuerySource.slice(b);let failed=false;try{await checkAnalysisNativeQueryGuard(source,packets);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name+': '+String(e.message).slice(0,120));failed=true;}assert(failed,name);mutations.push(name);}
 return {baseline,mutations,baselineBeforeMutationChecked:true,constructionFailuresCounted:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(checkAnalysisQueryValues()));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'[audit module]'));process.exitCode=1;}
