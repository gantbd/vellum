import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath,pathToFileURL} from 'node:url';
import * as original from './native-analysis.mjs';
import * as core from './analysis-work.mjs';
import {treeSitterCases,runTreeSitterReference} from './tree-sitter-checks.mjs';
import {publishAnnotationPlan} from './annotation-compounds.mjs';
import {committedSourceTransforms} from './parser-source.mjs';
const copy=structuredClone,bytes=s=>Array.from(Buffer.from(s,'utf8')),id=n=>({authority:'a',id:n,incarnation:1});
const document=id('doc'),provider=id('native'),analysis=id('analysis'),view=id('view');
const limits={maxTasks:16,maxOutstanding:8,maxIssued:64,maxCaptureBytes:262144,maxEnvelopeBytes:65536,maxHeldBytes:4194304,maxSourceBytes:32768,maxRegions:32,maxRanges:128,maxCaptures:4096,maxTagBytes:256,maxQueryBytes:4096,maxReplyBytes:65536};
const parserLimits={maxRootBytes:32768,maxReceipts:16,maxBatches:32,maxEdits:64,maxCoordinate:4294967295,maxPlanBytes:262144};
const gates={authorized:true,sourceReadable:true,disclosureAllowed:true,continuous:true,live:true,notStopped:true,providerAvailable:true,capacity:true,reservationReady:true,scope:true,retained:true,replyReady:true,acceptanceAllowed:true,contextAllowed:true,controlAllowed:true,nativeBindingAllowed:true,provider};
const retire={retirementAllowed:true,retentionReleased:true,deliveryDrained:true,nativeBindingAllowed:true};
function setup(source,l=limits){
  const context={document:copy(document),sourceRevision:1,analysis:copy(analysis),grammarGeneration:1,highlightQueryGeneration:1,discoveryQueryGeneration:1,registryGeneration:1,configurationGeneration:1,languageProfile:'pinned-markdown-javascript-reference/1'},binding={...copy(context),provider:copy(provider)};delete binding.sourceRevision;
  const state={authority:'a',publication:0,documents:[{document:copy(document),revision:1,source:copy(source),annotationSet:1,records:[]}],analysisContexts:[binding],views:[view,id('peer')].map(v=>({view:copy(v),document:copy(document),analysis:copy(analysis),sourceRevision:1})),groups:[],frontiers:{}};
  const owner=core.newAnalysisRegistry(l,parserLimits);
  const admit=(n,extra={})=>{const r=core.admitAnalysisWork(owner,state,{contract:'analysis-work/1',operation:'enabled-parse-query-work',profile:'captured-analysis/1',origin:{operation:n,effect:'parse',attempt:'1'},provider:copy(provider),context:copy(context),mode:'parse-query',base:null,journals:[],...extra},gates);assert.equal(r.status,'admitted');return r.task;};
  const query=(snapshot,v=view)=>core.queryAnalysis(owner,state,{contract:'analysis-query/1',operation:'supported-analysis',snapshot,view:copy(v),range:[0,state.documents[0].source.length]},gates);
  const bumpQuery=()=>{context.highlightQueryGeneration=2;state.analysisContexts[0].highlightQueryGeneration=2;};
  return {context,state,owner,admit,query,bumpQuery};
}
export async function checkNativeAnalysis(api=original,sourceFile=new URL('./native-analysis.py',import.meta.url),short=false){
  const controls=[],control=async(n,fn)=>{await fn();controls.push(n);},corpus=treeSitterCases().mixed;
  const withSession=async(x,fn)=>{const s=await api.newNativeAnalysisSession(x.owner,provider,sourceFile);try{return await fn(s);}finally{await api.abortNativeAnalysis(s);}};
  let nativeCases=0;
  for(const sample of short?[corpus[0]]:corpus){const x=setup(sample.source);await withSession(x,async s=>{
    const before=copy(x.state),t=x.admit('fresh'),r=await api.executeNativeAnalysis(s,t,x.state,gates);assert.equal(r.status,'recorded');assert.equal(r.quiescence,'quiescent');assert.equal(core.analysisUsage(x.owner).leases,0);
    const a=api.acceptNativeAnalysis(s,t,x.state,gates);assert.equal(a.status,'analysis');const q=x.query(a.snapshot);assert.equal(q.status,'captures');assert.deepEqual(q,x.query(a.snapshot,id('peer')));assert.deepEqual(q.captures,runTreeSitterReference({operation:'run',cases:[],mixed:[{source:sample.source,available:['javascript']}]}).mixed[0].resolved);
    assert.deepEqual(x.state,before);assert.equal((await api.closeNativeAnalysis(s)).status,'unavailable');assert.equal((await api.retireNativeAnalysis(s,t,retire)).status,'retired');assert.equal(x.query(a.snapshot).status,'unavailable');assert.equal((await api.closeNativeAnalysis(s)).status,'closed');nativeCases++;
  });}
  await control('query-only-retains-exact-native-forest-and-new-query',async()=>{
    const x=setup(corpus[0].source);await withSession(x,async s=>{
      const t=x.admit('one');await api.executeNativeAnalysis(s,t,x.state,gates);const a=api.acceptNativeAnalysis(s,t,x.state,gates);assert.equal(a.status,'analysis');const old=x.query(a.snapshot);x.bumpQuery();
      const u=x.admit('two',{mode:'query-only',base:a.snapshot}),r=await api.executeNativeAnalysis(s,u,x.state,gates);assert.equal(r.status,'recorded');assert.equal(r.facts.exactQueryReuse,true);assert(r.facts.forestTrees>=4);const b=api.acceptNativeAnalysis(s,u,x.state,gates);assert.equal(b.status,'analysis');assert(x.query(b.snapshot).captures.some(([,n])=>n==='symbol'));assert.notDeepEqual(old.captures,x.query(b.snapshot).captures);
      assert.equal(x.query(a.snapshot).status,'unavailable');assert.equal((await api.retireNativeAnalysis(s,t,retire)).status,'unavailable');assert.equal(api.nativeAnalysisUsage(s).records,2);
      await api.retireNativeAnalysis(s,u,retire);await api.retireNativeAnalysis(s,t,retire);assert.equal(api.nativeAnalysisUsage(s).records,0);assert.equal((await api.closeNativeAnalysis(s)).status,'closed');
    });
  });
  await control('actual-committed-input-updates-copy-and-preserves-old-native-forest',async()=>{
    const x=setup(corpus[2].source);await withSession(x,async s=>{
      const t=x.admit('one');await api.executeNativeAnalysis(s,t,x.state,gates);const a=api.acceptNativeAnalysis(s,t,x.state,gates);assert.equal(a.status,'analysis');
      const out=publishAnnotationPlan(x.state,{contract:'owned-annotation-plan/1',invocation:'edit',descriptor:'edit',sources:[{document,expectedSource:1,expectedAnnotationSet:1,edits:[{lo:0,hi:0,ins:bytes('# Added é🙂\n\n'),expected:[]}]}],annotations:[]},
        {authorized:true,bounded:true,notStopped:true,interpretationCurrent:true,companionsReady:true,versionsAvailable:true,historyReady:true,completeIdentity:true,maxParticipants:4,parserSourceReserved:true,parserSourceLimits:{maxDocuments:4,maxRootBytes:32768,maxBatches:32,maxEdits:64,maxJournalBytes:262144}});
      assert.equal(out.status,'applied');Object.assign(x.state,out.state);x.context.sourceRevision=x.state.documents[0].revision;x.state.views.forEach(v=>v.sourceRevision=x.context.sourceRevision);
      const u=x.admit('two',{base:a.snapshot,journals:committedSourceTransforms(out.state)}),r=await api.executeNativeAnalysis(s,u,x.state,gates);assert.equal(r.status,'recorded');assert.equal(r.facts.inputMode,'incremental');assert.equal(r.facts.basePreserved,true);const b=api.acceptNativeAnalysis(s,u,x.state,gates);assert.equal(b.status,'analysis');assert.equal(x.query(b.snapshot).status,'captures');
      await api.retireNativeAnalysis(s,u,retire);await api.retireNativeAnalysis(s,t,retire);assert.equal((await api.closeNativeAnalysis(s)).status,'closed');
    });
  });
  await control('native-record-cannot-bind-a-different-retained-packet',async()=>{
    const x=setup(corpus[0].source,{...limits,maxEnvelopeBytes:1200});await withSession(x,async s=>{
      const t=x.admit('one'),r=await api.executeNativeAnalysis(s,t,x.state,gates);assert.equal(r.evidence,'unavailable');assert.equal(r.status,'unavailable');
      const packet={context:copy(x.context),source:copy(x.state.documents[0].source),parseState:'parsed',coverage:'partial',regions:[],captures:[]};
      assert.equal(core.observeAnalysisWork(x.owner,{contract:'analysis-evidence/1',task:t,origin:{operation:'one',effect:'parse',attempt:'1'},provider,kind:'result',packet},{authentic:true,evidenceAllowed:true,ingressReady:true}).status,'recorded');
      assert.equal(core.analysisResultMatches(x.owner,t,packet,{nativeBindingAllowed:false}),false);assert.equal(api.acceptNativeAnalysis(s,t,x.state,gates).status,'unavailable');await api.retireNativeAnalysis(s,t,retire);assert.equal((await api.closeNativeAnalysis(s)).status,'closed');
    });
  });
  await control('unbound-cross-session-base-does-not-substitute-a-native-tree',async()=>{
    const x=setup(corpus[0].source);await withSession(x,async s=>{
      const t=x.admit('one');await api.executeNativeAnalysis(s,t,x.state,gates);const a=api.acceptNativeAnalysis(s,t,x.state,gates);assert.equal(a.status,'analysis');x.bumpQuery();
      await withSession(x,async peer=>{const seed=x.admit('seed');assert.equal((await api.executeNativeAnalysis(peer,seed,x.state,gates)).status,'recorded');const u=x.admit('two',{mode:'query-only',base:a.snapshot}),r=await api.executeNativeAnalysis(peer,u,x.state,gates);assert.equal(r.status,'failure');assert.equal(r.quiescence,'quiescent');assert.equal(api.acceptNativeAnalysis(peer,u,x.state,gates).status,'unavailable');await api.retireNativeAnalysis(peer,u,retire);await api.retireNativeAnalysis(peer,seed,retire);assert.equal((await api.closeNativeAnalysis(peer)).status,'closed');});
      await api.retireNativeAnalysis(s,t,retire);assert.equal((await api.closeNativeAnalysis(s)).status,'closed');
    });
  });
  await control('cancel-before-native-return-still-retains-handle-and-drains-work',async()=>{
    const x=setup(corpus[0].source);await withSession(x,async s=>{const t=x.admit('one'),p=api.executeNativeAnalysis(s,t,x.state,gates);assert.equal(core.analysisUsage(x.owner).leases,1);core.controlAnalysisWork(x.owner,t,'cancel',gates);assert.equal(core.analysisUsage(x.owner).leases,1);assert.equal((await api.closeNativeAnalysis(s)).status,'unavailable');const r=await p;assert.equal(r.status,'recorded');assert.equal(r.quiescence,'quiescent');assert.equal(api.acceptNativeAnalysis(s,t,x.state,gates).status,'unavailable');assert.equal(api.nativeAnalysisUsage(s).records,1);await api.retireNativeAnalysis(s,t,retire);assert.equal((await api.closeNativeAnalysis(s)).status,'closed');});
  });
  await control('same-context-lease-fences-second-native-submission',async()=>{
    const x=setup(corpus[0].source);await withSession(x,async s=>{const t=x.admit('one'),u=x.admit('two'),p=api.executeNativeAnalysis(s,t,x.state,gates);assert.equal((await api.executeNativeAnalysis(s,u,x.state,gates)).status,'not-submitted');await p;core.controlAnalysisWork(x.owner,u,'cancel',gates);await api.retireNativeAnalysis(s,t,retire);core.retireAnalysisWork(x.owner,u,retire);assert.equal((await api.closeNativeAnalysis(s)).status,'closed');});
  });
  await control('native-host-gates-precede-submission',async()=>{
    const x=setup(corpus[0].source);await withSession(x,async s=>{const t=x.admit('one');assert.equal((await api.executeNativeAnalysis(s,t,x.state,{...gates,nativeBindingAllowed:false})).status,'unavailable');assert.equal((await api.executeNativeAnalysis(s,t,x.state,{...gates,provider:id('other')})).status,'unavailable');assert.equal(core.analysisTaskStatus(x.owner,t).phase,'ready');assert.equal(api.nativeAnalysisUsage(s).records,0);core.controlAnalysisWork(x.owner,t,'cancel',gates);core.retireAnalysisWork(x.owner,t,retire);assert.equal((await api.closeNativeAnalysis(s)).status,'closed');});
  });
  await control('stale-after-native-launch-cannot-become-current',async()=>{
    const x=setup(corpus[0].source);await withSession(x,async s=>{const t=x.admit('one'),p=api.executeNativeAnalysis(s,t,x.state,gates);x.bumpQuery();assert.equal((await p).status,'recorded');assert.equal(api.acceptNativeAnalysis(s,t,x.state,gates).status,'unavailable');assert.equal(api.nativeAnalysisUsage(s).records,1);await api.retireNativeAnalysis(s,t,retire);assert.equal((await api.closeNativeAnalysis(s)).status,'closed');});
  });
  await control('retirement-gates-preserve-core-and-native-holds',async()=>{
    const x=setup(corpus[0].source);await withSession(x,async s=>{const t=x.admit('one');await api.executeNativeAnalysis(s,t,x.state,gates);for(const key of ['nativeBindingAllowed','retentionReleased','deliveryDrained','retirementAllowed'])assert.equal((await api.retireNativeAnalysis(s,t,{...retire,[key]:false})).status,'unavailable');assert.equal(api.nativeAnalysisUsage(s).records,1);assert.equal(core.analysisUsage(x.owner).tasks,1);await api.retireNativeAnalysis(s,t,retire);assert.equal((await api.closeNativeAnalysis(s)).status,'closed');});
  });
  await control('distinct-analysis-contexts-progress-with-separate-native-bindings',async()=>{
    const x=setup(corpus[0].source);await withSession(x,async s=>{const c={...copy(x.context),analysis:id('other')};x.state.analysisContexts.push({...copy(x.state.analysisContexts[0]),analysis:c.analysis});x.state.views.push({view:id('other-view'),document,analysis:c.analysis,sourceRevision:1});const t=x.admit('one'),u=x.admit('two',{context:c}),p=api.executeNativeAnalysis(s,t,x.state,gates),q=api.executeNativeAnalysis(s,u,x.state,gates);assert.equal(core.analysisUsage(x.owner).leases,2);assert.equal((await p).status,'recorded');assert.equal((await q).status,'recorded');const a=api.acceptNativeAnalysis(s,t,x.state,gates),b=api.acceptNativeAnalysis(s,u,x.state,gates);assert.equal(a.status,'analysis');assert.equal(b.status,'analysis');assert.equal(x.query(a.snapshot).status,'captures');assert.equal(x.query(b.snapshot,id('other-view')).status,'captures');assert.equal(x.query(b.snapshot).status,'unavailable');await api.retireNativeAnalysis(s,u,retire);await api.retireNativeAnalysis(s,t,retire);assert.equal((await api.closeNativeAnalysis(s)).status,'closed');});
  });
  await control('invalid-query-is-failure-not-conformance-or-native-base-replacement',async()=>{
    const x=setup(corpus[0].source);await withSession(x,async s=>{const t=x.admit('one');await api.executeNativeAnalysis(s,t,x.state,gates);const a=api.acceptNativeAnalysis(s,t,x.state,gates);assert.equal(a.status,'analysis');x.context.highlightQueryGeneration=3;x.state.analysisContexts[0].highlightQueryGeneration=3;const u=x.admit('two',{mode:'query-only',base:a.snapshot}),r=await api.executeNativeAnalysis(s,u,x.state,gates);assert.equal(r.status,'failure');assert.equal(r.quiescence,'quiescent');assert.equal(api.acceptNativeAnalysis(s,u,x.state,gates).status,'unavailable');assert.equal((await api.retireNativeAnalysis(s,t,retire)).status,'unavailable');await api.retireNativeAnalysis(s,u,retire);await api.retireNativeAnalysis(s,t,retire);assert.equal((await api.closeNativeAnalysis(s)).status,'closed');});
  });
  await control('close-fences-new-native-admission-before-await',async()=>{
    const x=setup(corpus[0].source);await withSession(x,async s=>{const closing=api.closeNativeAnalysis(s).catch(e=>e),t=x.admit('one'),p=api.executeNativeAnalysis(s,t,x.state,gates).catch(e=>e);assert.equal(core.analysisTaskStatus(x.owner,t).phase,'ready');assert.equal((await p).status,'unavailable');assert.equal((await closing).status,'closed');core.controlAnalysisWork(x.owner,t,'cancel',gates);core.retireAnalysisWork(x.owner,t,retire);});
  });
  await control('retirement-fences-duplicate-native-drop-before-await',async()=>{
    const x=setup(corpus[0].source);await withSession(x,async s=>{const t=x.admit('one');await api.executeNativeAnalysis(s,t,x.state,gates);const p=api.retireNativeAnalysis(s,t,retire).catch(e=>e),q=api.retireNativeAnalysis(s,t,retire).catch(e=>e);assert.equal(api.nativeAnalysisUsage(s).pending,1);assert.equal(api.nativeAnalysisUsage(s).records,1);assert.equal((await q).status,'retirement-pending');assert.equal((await p).status,'retired');assert.equal((await api.closeNativeAnalysis(s)).status,'closed');});
  });
  await control('old-core-snapshot-refuses-after-native-retirement',async()=>{
    const x=setup(corpus[0].source);await withSession(x,async s=>{const t=x.admit('one');await api.executeNativeAnalysis(s,t,x.state,gates);const a=api.acceptNativeAnalysis(s,t,x.state,gates);assert.equal(a.status,'analysis');assert.equal(api.acceptNativeAnalysis(s,copy(t),x.state,gates).status,'unavailable');await api.retireNativeAnalysis(s,t,retire);x.bumpQuery();const request={contract:'analysis-work/1',operation:'enabled-parse-query-work',profile:'captured-analysis/1',origin:{operation:'two',effect:'parse',attempt:'1'},provider,context:x.context,mode:'query-only',base:a.snapshot,journals:[]};assert.equal(core.admitAnalysisWork(x.owner,x.state,request,gates).status,'unavailable');assert.equal((await api.closeNativeAnalysis(s)).status,'closed');});
  });
  if(!short)await control('process-death-is-not-quiescence-or-retirement',async()=>{
    const x=setup(corpus[0].source),s=await api.newNativeAnalysisSession(x.owner,provider,sourceFile),t=x.admit('one'),p=api.executeNativeAnalysis(s,t,x.state,gates),caught=p.catch(e=>e);await api.abortNativeAnalysis(s);const e=await caught;assert(e instanceof Error);assert.equal(core.analysisUsage(x.owner).leases,1);assert.equal(core.analysisTaskStatus(x.owner,t).quiescent,false);assert.equal(api.nativeAnalysisUsage(s).faulted,true);assert.equal(api.nativeAnalysisUsage(s).records,1);assert.equal((await api.retireNativeAnalysis(s,t,retire)).status,'unavailable');
  });
  return {scope:'live-native-analysis-binding-reference-only',nativeCases,controls,...JSON.parse(fs.readFileSync(new URL('./native-analysis.json',import.meta.url),'utf8')).scope};
}
export function checkNativeProtocol(sourceFile=new URL('./native-analysis.py',import.meta.url)){
  const x=setup(treeSitterCases().mixed[0].source),t=x.admit('one'),capture=core.submitAnalysisWork(x.owner,t,x.state,gates).capture;
  const reuse=copy(capture);reuse.mode='query-only';reuse.input={profile:'tree-sitter-utf8-source/1',mode:'reuse-parse',fromRevision:1,toRevision:1,source:copy(capture.input.source),edits:[]};reuse.context.highlightQueryGeneration=2;
  const invalidProvider=copy(reuse);invalidProvider.provider=id('other');const invalidGrammar=copy(reuse);invalidGrammar.context.grammarGeneration=2;const invalidSource=copy(reuse);invalidSource.input.source[0]=32;
  const cases=[{operation:'execute',capture,base:null},{operation:'execute',capture:invalidProvider,base:1},{operation:'execute',capture:invalidGrammar,base:1},{operation:'execute',capture:invalidSource,base:1},{operation:'execute',capture:reuse,base:99},{operation:'execute',capture:reuse,base:1},{operation:'drop',handle:1},{operation:'execute',capture:reuse,base:1},{operation:'drop',handle:2},{operation:'drop',handle:1},{operation:'close'}];
  const p=spawnSync(process.env.VELLUM_TREE_SITTER_PYTHON,[fileURLToPath(sourceFile),fileURLToPath(new URL('./',import.meta.url))],{input:cases.map((r,i)=>JSON.stringify({...r,serial:i+1})).join('\n')+'\n',encoding:'utf8',timeout:15000,maxBuffer:1048576});
  if(p.error||p.signal||p.status!==0){if(!p.error&&!p.signal&&p.status===2&&p.stderr.startsWith('REFERENCE_ASSERTION:'))assert.fail(p.stderr);throw Error(`native protocol runner failure: ${p.stderr||p.error}`);}
  const rows=p.stdout.trim().split('\n').map(s=>JSON.parse(s));assert.equal(rows.length,cases.length);assert.deepEqual(rows.map(r=>r.kind),['result','failure','failure','failure','failure','result','dropped','failure','dropped','unavailable','closed']);assert.equal(rows[5].handle,2);assert.equal(rows[5].facts.exactQueryReuse,true);return rows.length;
}
export async function checkNativeAnalysisMutations(){
  const js=fs.readFileSync(new URL('./native-analysis.mjs',import.meta.url),'utf8'),py=fs.readFileSync(new URL('./native-analysis.py',import.meta.url),'utf8'),directory=fs.mkdtempSync(path.join(os.tmpdir(),'vellum-native-binding-mutants-')),killed=[];
  const mutations=[
    ['native-edits-published-base','py','base["forest"]["block"].copy()','base["forest"]["block"]'],
    ['native-input-edits-omitted','py','edited.edit(**edit)','pass # edit omitted'],
    ['query-only-reparses-children','py','base["forest"] if mode == "reuse-parse" else None','None'],
    ['query-generation-does-not-select-new-query','py','if context["highlightQueryGeneration"] == 2:','if False:'],
    ['provider-handle-binding-ignored','py','base["provider"] != capture["provider"]','False'],
    ['source-handle-binding-ignored','py','base["source"] != source','False'],
    ['retired-native-handle-kept','py','del records[handle]','pass # handle retained'],
    ['retained-result-native-packet-match-ignored','js','!analysisResultMatches(s.owner,task,record.packet,g)','false'],
    ['host-native-drop-omitted','js','if(record.handle!==null){record.dropping=true;','if(false){record.dropping=true;'],
    ['host-binding-permission-ignored','js','!g?.nativeBindingAllowed','false'],
    ['unbound-native-base-guessed','js','s.snapshots.get(base)','s.snapshots.get(base)??[...s.records.values()][0]'],
    ['close-does-not-fence-admission','js','s.closing=true;const r=await pending;','const r=await pending;'],
    ['duplicate-drop-not-fenced','js',"if(record.dropping)return {status:'retirement-pending'};",'/* duplicate drop permitted */'],
    ['process-loss-fabricates-quiescence','js','s.exited=true;','s.exited=true;for(const task of s.records.keys())observeAnalysisWork(s.owner,{contract:"analysis-evidence/1",task,origin:{operation:"one",effect:"parse",attempt:"1"},provider:s.provider,kind:"quiescence"},ingress);']
  ];
  for(const [name,kind,from,to] of mutations){assert((kind==='py'?py:js).includes(from),name);let api=original,worker=new URL('./native-analysis.py',import.meta.url);
    if(kind==='py'){const target=path.join(directory,name+'.py');fs.writeFileSync(target,py.replace(from,to));worker=pathToFileURL(target);}
    else{let src=js.replace(from,to);src=src.replace("'./analysis-work.mjs'",JSON.stringify(new URL('./analysis-work.mjs',import.meta.url).href));for(const asset of ['native-analysis.json','tree-sitter-reference.json','native-analysis.py','./'])src=src.replaceAll(`new URL('${asset.startsWith('.')?asset:'./'+asset}',import.meta.url)`,`new URL(${JSON.stringify(new URL(asset,import.meta.url).href)})`);api=await import(`data:text/javascript;base64,${Buffer.from(src).toString('base64')}#${name}`);}
    let failed=false;try{if(kind==='py'&&['provider-handle-binding-ignored','source-handle-binding-ignored'].includes(name))checkNativeProtocol(worker);else await checkNativeAnalysis(api,worker,name!=='process-loss-fabricates-quiescence');}catch(e){assert(['ERR_ASSERTION','NATIVE_REFERENCE_ASSERTION'].includes(e.code),`${name}: ${e.message}`);failed=true;}assert(failed,name);killed.push(name);
  }return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...await checkNativeAnalysis(),protocolCases:checkNativeProtocol(),mutations:await checkNativeAnalysisMutations()},null,2));
