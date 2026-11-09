import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as original from './analysis-work.mjs';
import {treeSitterCases,runTreeSitterReference} from './tree-sitter-checks.mjs';
import {publishAnnotationPlan} from './annotation-compounds.mjs';
import {committedSourceTransforms} from './parser-source.mjs';
const copy=structuredClone,bytes=s=>Array.from(Buffer.from(s,'utf8'));
const id=n=>({authority:'a',id:n,incarnation:1}),document=id('doc'),analysis=id('analysis'),provider=id('provider'),view=id('view');
const limits={maxTasks:16,maxOutstanding:8,maxIssued:32,maxCaptureBytes:262144,maxEnvelopeBytes:65536,maxHeldBytes:4194304,maxSourceBytes:32768,maxRegions:32,maxRanges:128,maxCaptures:4096,maxTagBytes:256,maxQueryBytes:4096,maxReplyBytes:65536};
const parserLimits={maxRootBytes:32768,maxReceipts:16,maxBatches:32,maxEdits:64,maxCoordinate:4294967295,maxPlanBytes:262144};
const launch=['authorized','sourceReadable','disclosureAllowed','continuous','live','notStopped','providerAvailable','capacity','reservationReady'];
const accept=['authorized','sourceReadable','disclosureAllowed','continuous','live','notStopped','acceptanceAllowed'];
const release=['authorized','scope','continuous','live','retained','notStopped','replyReady'];
const gates=Object.fromEntries([...launch,...accept,...release,'contextAllowed','controlAllowed'].map(k=>[k,true]));gates.provider=provider;
const ingress={authentic:true,evidenceAllowed:true,ingressReady:true,quiescenceAttested:true};
const retirement={retirementAllowed:true,retentionReleased:true,deliveryDrained:true};
const context=()=>({document:copy(document),sourceRevision:1,analysis:copy(analysis),grammarGeneration:1,highlightQueryGeneration:1,discoveryQueryGeneration:1,registryGeneration:1,configurationGeneration:1,languageProfile:'pinned-markdown-javascript-reference/1'});
function fixtures(){const request={operation:'run',cases:[],mixed:treeSitterCases().mixed};return {request,native:runTreeSitterReference(request)};}
export function analysisWorkFixture(api=original,f=fixtures(),l=limits,index=0){
    const c=context(),row=f.native.mixed[index],source=copy(f.request.mixed[index].source),binding=copy(c);delete binding.sourceRevision;binding.provider=copy(provider);
    const state={authority:'a',publication:0,documents:[{document:copy(document),revision:1,source,annotationSet:1,records:[]}],analysisContexts:[binding],views:[view,id('peer-view')].map(v=>({view:copy(v),document:copy(document),analysis:copy(analysis),sourceRevision:1})),groups:[],frontiers:{}};
    const owner=api.newAnalysisRegistry(l,parserLimits);assert(owner);
    const request=(n='one',extra={})=>({contract:'analysis-work/1',operation:'enabled-parse-query-work',profile:'captured-analysis/1',origin:{operation:n,effect:'parse',attempt:'1'},provider:copy(provider),context:copy(c),mode:'parse-query',base:null,journals:[],...extra});
    const packet=()=>({context:copy(c),source:copy(source),parseState:row.hostHasError?'syntax-errors':'parsed',coverage:row.partial?'partial':'complete-within-profile',regions:copy(row.regions),captures:copy(row.resolved)});
    const admit=(n='one',extra={})=>{const r=api.admitAnalysisWork(owner,state,request(n,extra),gates);assert.equal(r.status,'admitted');return r.task;};
    const submit=t=>{const r=api.submitAnalysisWork(owner,t,state,gates);assert.equal(r.status,'submitted');return r;};
    const event=(task,kind='result',p=packet(),n='one')=>({contract:'analysis-evidence/1',task,origin:{operation:n,effect:'parse',attempt:'1'},provider:copy(provider),kind,...(kind==='result'?{packet:p}:kind==='failure'?{reason:'worker-fault'}:{})});
    const record=(t,p=packet(),n='one')=>{assert.equal(api.observeAnalysisWork(owner,event(t,'result',p,n),ingress).status,'recorded');};
    const accepted=(t)=>{const r=api.acceptAnalysisWork(owner,t,state,gates);assert.equal(r.status,'analysis');return r.snapshot;};
    const ready=()=>{const t=admit();submit(t);record(t);return {task:t,snapshot:accepted(t)};};
    const query=(snapshot,extra={})=>({contract:'analysis-query/1',operation:'supported-analysis',snapshot,view:copy(view),range:[0,source.length],...extra});
    const quiet=(t,n='one')=>api.observeAnalysisWork(owner,event(t,'quiescence',null,n),ingress);
    return {c,state,owner,request,packet,admit,submit,event,record,accepted,ready,query,quiet,limits:copy(l),packets:f,gates:copy(gates),ingress:copy(ingress),retirement:copy(retirement)};
}
export function checkAnalysisWork(api=original,f=fixtures()){
  const controls=[],control=(n,fn)=>{fn();controls.push(n);};let admissionGateCases=0,submissionGateCases=0,acceptanceGateCases=0,releaseGateCases=0,emptyRangeCases=0,scalarRangeCases=0,interiorRangeRefusals=0;
  const setup=(l=limits,index=0)=>analysisWorkFixture(api,f,l,index);
  for(let mask=0;mask<2**launch.length;mask++){
    const x=setup(),g={...gates,...Object.fromEntries(launch.map((k,i)=>[k,!!(mask&(1<<i))]))};
    assert.equal(api.admitAnalysisWork(x.owner,x.state,x.request(),g).status,mask===511?'admitted':'unavailable');admissionGateCases++;
    const y=setup(),t=y.admit();assert.equal(api.submitAnalysisWork(y.owner,t,y.state,g).status,mask===511?'submitted':'not-submitted');submissionGateCases++;
  }
  for(let mask=0;mask<128;mask++){
    const x=setup(),t=x.admit();x.submit(t);x.record(t);const g={...gates,...Object.fromEntries(accept.map((k,i)=>[k,!!(mask&(1<<i))]))};
    assert.equal(api.acceptAnalysisWork(x.owner,t,x.state,g).status,mask===127?'analysis':'unavailable');acceptanceGateCases++;
    const y=setup(),a=y.ready(),h={...gates,...Object.fromEntries(release.map((k,i)=>[k,!!(mask&(1<<i))]))};
    assert.equal(api.queryAnalysis(y.owner,y.state,y.query(a.snapshot),h).status,mask===127?'captures':'unavailable');releaseGateCases++;
  }
  control('actual-native-packets-through-current-owner',()=>{for(let i=0;i<f.native.mixed.length;i++){const x=setup(limits,i),before=copy(x.state),a=x.ready();assert.equal(api.queryAnalysis(x.owner,x.state,x.query(a.snapshot),gates).status,'captures');assert.deepEqual(x.state,before);}});
  control('opaque-owner-task-snapshot-not-copyable',()=>{const x=setup(),t=x.admit();assert.equal(api.submitAnalysisWork(copy(x.owner),t,x.state,gates).status,'not-submitted');assert.equal(api.submitAnalysisWork(x.owner,copy(t),x.state,gates).status,'not-submitted');x.submit(t);x.record(t);const s=x.accepted(t);assert.equal(api.queryAnalysis(x.owner,x.state,x.query(copy(s)),gates).status,'unavailable');});
  control('single-submission-and-origin-replay',()=>{const x=setup(),t=x.admit();x.submit(t);assert.equal(api.submitAnalysisWork(x.owner,t,x.state,gates).status,'not-submitted');assert.equal(api.admitAnalysisWork(x.owner,x.state,x.request(),gates).status,'unavailable');});
  for(const operation of ['cancel','supersede'])control(`${operation}-holds-lease-until-quiescence`,()=>{
    const x=setup(),t=x.admit();x.submit(t);const n=x.admit('two'),held=api.analysisUsage(x.owner).heldBytes;
    assert.equal(api.controlAnalysisWork(x.owner,t,operation,gates).status,'stopped');assert.equal(api.controlAnalysisWork(x.owner,t,operation,gates).status,'unchanged');
    assert.equal(api.submitAnalysisWork(x.owner,n,x.state,gates).status,'not-submitted');assert.equal(api.retireAnalysisWork(x.owner,t,retirement).status,'unavailable');
    assert.equal(x.quiet(t).status,'quiescent');assert.equal(api.analysisUsage(x.owner).heldBytes,held);assert.equal(api.analysisUsage(x.owner).outstanding,1);x.submit(n);
    assert.equal(x.quiet(t).status,'duplicate');assert.equal(api.analysisUsage(x.owner).leases,1);assert.equal(api.retireAnalysisWork(x.owner,t,retirement).status,'retired');
    assert.equal(api.admitAnalysisWork(x.owner,x.state,x.request(),gates).status,'unavailable');
  });
  control('ready-cancel-closes-without-executor',()=>{const x=setup(),t=x.admit();api.controlAnalysisWork(x.owner,t,'cancel',gates);assert.equal(api.analysisUsage(x.owner).outstanding,0);assert.equal(api.analysisUsage(x.owner).leases,0);assert.equal(api.retireAnalysisWork(x.owner,t,retirement).status,'retired');});
  control('independent-analysis-identities-progress',()=>{const x=setup(),t=x.admit();x.submit(t);const c=copy(x.c);c.analysis=id('other-context');const row={...copy(x.state.analysisContexts[0]),analysis:c.analysis};x.state.analysisContexts.push(row);const n=x.admit('two',{context:c});x.submit(n);assert.equal(api.analysisUsage(x.owner).leases,2);});
  control('current-context-before-source-traversal',()=>{const x=setup();assert.equal(api.admitAnalysisWork(x.owner,x.state,x.request(),{...gates,contextAllowed:false}).status,'unavailable');});
  for(const k of ['sourceRevision','grammarGeneration','highlightQueryGeneration','discoveryQueryGeneration','registryGeneration','configurationGeneration','analysis','document','languageProfile'])control(`stale-${k}-at-submit-accept-read`,()=>{
    const mutate=x=>{if(k==='sourceRevision')x.state.documents[0].revision++;else if(['analysis','document'].includes(k))x.state.analysisContexts[0][k]=id(`other-${k}`);else x.state.analysisContexts[0][k]=typeof x.c[k]==='number'?x.c[k]+1:'unknown';};
    const x=setup(),t=x.admit();mutate(x);assert.equal(api.submitAnalysisWork(x.owner,t,x.state,gates).status,'not-submitted');
    const y=setup(),u=y.admit();y.submit(u);y.record(u);mutate(y);assert.equal(api.acceptAnalysisWork(y.owner,u,y.state,gates).status,'unavailable');
    const z=setup(),a=z.ready();mutate(z);assert.equal(api.queryAnalysis(z.owner,z.state,z.query(a.snapshot),gates).status,'unavailable');
  });
  control('same-version-altered-bytes-rejected',()=>{const x=setup(),t=x.admit();x.submit(t);x.record(t);x.state.documents[0].source[0]=32;assert.equal(api.acceptAnalysisWork(x.owner,t,x.state,gates).status,'unavailable');});
  control('same-version-altered-bytes-before-submission',()=>{const x=setup(),t=x.admit();x.state.documents[0].source[0]=32;assert.equal(api.submitAnalysisWork(x.owner,t,x.state,gates).status,'not-submitted');});
  control('provider-currentness-not-caller-only',()=>{const x=setup(),a=x.ready();x.state.analysisContexts[0].provider=id('replacement');assert.equal(api.queryAnalysis(x.owner,x.state,x.query(a.snapshot),gates).status,'unavailable');});
  control('two-views-share-one-accepted-analysis',()=>{const x=setup(),a=x.ready();const one=api.queryAnalysis(x.owner,x.state,x.query(a.snapshot),gates),two=api.queryAnalysis(x.owner,x.state,x.query(a.snapshot,{view:id('peer-view')}),gates);assert.deepEqual(one,two);assert.equal(api.analysisUsage(x.owner).tasks,1);});
  for(const k of ['view','document','analysis','sourceRevision'])control(`actual-view-${k}-revalidated`,()=>{const x=setup(),a=x.ready();x.state.views[0][k]=k==='sourceRevision'?2:id('other');assert.equal(api.queryAnalysis(x.owner,x.state,x.query(a.snapshot),gates).status,'unavailable');});
  control('summary-not-ast-or-permission',()=>{const x=setup(),a=x.ready(),out=api.queryAnalysis(x.owner,x.state,x.query(a.snapshot,{operation:'supplied-evidence-inspection',range:undefined}),gates);assert.equal(out.status,'unavailable');const r=x.query(a.snapshot,{operation:'supplied-evidence-inspection'});delete r.range;const q=api.queryAnalysis(x.owner,x.state,r,gates);assert.deepEqual(Object.keys(q).sort(),['context','coverage','parseState','status']);});
  control('display-fragments-clipped-without-source-edits',()=>{const x=setup(),a=x.ready();const q=api.queryAnalysis(x.owner,x.state,x.query(a.snapshot,{range:[14,16]}),gates);assert.deepEqual(q.captures,[['host-inline','text.strong',14,16]]);assert.equal(api.queryAnalysis(x.owner,x.state,x.query(a.snapshot,{range:[0,0]}),gates).captures.length,0);});
  control('half-open-empty-and-adjacent-scalar-query-intersections',()=>{
    for(let index=0;index<f.native.mixed.length;index++){
      const x=setup(limits,index),a=x.ready(),before=copy(x.state),usage=copy(api.analysisUsage(x.owner)),source=x.state.documents[0].source,packet=x.packet(),positions=Array.from({length:source.length+1},(_,p)=>p).filter(p=>p===source.length||(source[p]&0xc0)!==0x80);
      for(let p=0;p<=source.length;p++){
        const out=api.queryAnalysis(x.owner,x.state,x.query(a.snapshot,{range:[p,p]}),gates);
        if(!positions.includes(p)){assert.deepEqual(out,{status:'unavailable'});interiorRangeRefusals++;continue;}
        assert.deepEqual(out,{status:'captures',context:packet.context,parseState:packet.parseState,coverage:packet.coverage,captures:[]});emptyRangeCases++;
      }
      for(let i=1;i<positions.length;i++){
        const lo=positions[i-1],hi=positions[i],expected=[];
        for(const [layer,name,start,end]of packet.captures){const a=Math.max(lo,start),b=Math.min(hi,end);if(a<b)expected.push([layer,name,a,b]);}
        const out=api.queryAnalysis(x.owner,x.state,x.query(a.snapshot,{range:[lo,hi]}),gates);
        assert.equal(out.status,'captures');assert.deepEqual(out.captures,expected);assert(out.captures.every(([, ,a,b])=>a<b));scalarRangeCases++;
      }
      for(const key of [...release,'contextAllowed'])assert.deepEqual(api.queryAnalysis(x.owner,x.state,x.query(a.snapshot,{range:[0,0]}),{...gates,[key]:false}),{status:'unavailable'});
      assert.deepEqual(x.state,before);assert.deepEqual(api.analysisUsage(x.owner),usage);
    }
  });
  control('utf8-range-interior-not-position',()=>{const x=setup(),a=x.ready();const p=x.state.documents[0].source.indexOf(0xc3)+1;assert.equal(api.queryAnalysis(x.owner,x.state,x.query(a.snapshot,{range:[p,p+1]}),gates).status,'unavailable');});
  for(const k of ['authentic','evidenceAllowed','ingressReady'])control(`intake-${k}`,()=>{const x=setup(),t=x.admit();x.submit(t);assert.equal(api.observeAnalysisWork(x.owner,x.event(t),{...ingress,[k]:false}).status,'unavailable');assert.equal(api.analysisTaskStatus(x.owner,t).witnesses,0);});
  for(const k of ['origin','provider','task','contract','extra'])control(`exact-correlated-${k}`,()=>{const x=setup(),t=x.admit();x.submit(t);const e=x.event(t);e[k]=k==='origin'?{...e.origin,attempt:'stale'}:k==='provider'?id('other'):k==='task'?copy(t):'unknown';assert.equal(api.observeAnalysisWork(x.owner,e,ingress).status,'unavailable');assert.equal(api.analysisUsage(x.owner).leases,1);});
  control('quiescence-is-attested-not-result',()=>{const x=setup(),t=x.admit();x.submit(t);x.record(t);assert.equal(api.analysisUsage(x.owner).leases,1);assert.equal(api.observeAnalysisWork(x.owner,x.event(t,'quiescence'),{...ingress,quiescenceAttested:false}).status,'unavailable');assert.equal(api.analysisUsage(x.owner).leases,1);});
  control('duplicate-conflict-third-witness-closed-cap',()=>{const x=setup(),t=x.admit();x.submit(t);x.record(t);const s=x.accepted(t);assert.equal(api.observeAnalysisWork(x.owner,x.event(t),ingress).status,'duplicate');const e=x.event(t,'failure');assert.equal(api.observeAnalysisWork(x.owner,e,ingress).status,'conflict');assert.equal(api.queryAnalysis(x.owner,x.state,x.query(s),gates).status,'unavailable');assert.equal(api.acceptAnalysisWork(x.owner,t,x.state,gates).status,'unavailable');e.reason='exhausted';assert.equal(api.observeAnalysisWork(x.owner,e,ingress).status,'evidence-full');assert.equal(api.analysisTaskStatus(x.owner,t).witnesses,2);});
  for(const op of ['cancel','supersede'])control(`${op}-disables-already-accepted-query`,()=>{const x=setup(),a=x.ready();api.controlAnalysisWork(x.owner,a.task,op,gates);assert.equal(api.queryAnalysis(x.owner,x.state,x.query(a.snapshot),gates).status,'unavailable');});
  const badPackets=[
    ['wrong-source',p=>p.source[0]=32],['wrong-context',p=>p.context.grammarGeneration++],['unknown-field',p=>p.extra=true],['bad-parse-state',p=>p.parseState='conforms'],['bad-coverage',p=>p.coverage='universal'],
    ['overlap-display',p=>p.captures.push(copy(p.captures[0]))],['invalid-layer',p=>p.captures[0][0]='execute'],['invalid-utf8-position',p=>{const n=p.source.indexOf(0xc3)+1;p.captures=[['child','variable',n,n+1]];}],
    ['invalid-tag-scalar',p=>p.captures[0][1]='\ud800'],['invalid-region-kind',p=>p.regions[0].kind='evaluate'],['child-outside-range',p=>p.regions[0].captures[0][1]=0],['body-outside-source',p=>p.regions[0].body[1]=p.source.length+1],
    ['unordered-included-ranges',p=>p.regions[0].ranges.push(copy(p.regions[0].ranges[0]))],['fallback-has-captures',p=>p.regions[0].kind='unsupported-language'],['parsed-without-ranges',p=>p.regions[0].ranges=[]],['region-budget-not-full-coverage',p=>p.regions[2].kind='region-budget']
  ];
  for(const [n,change] of badPackets)control(`packet-${n}`,()=>{const x=setup(),t=x.admit();x.submit(t);const p=x.packet();change(p);x.record(t,p);assert.equal(api.acceptAnalysisWork(x.owner,t,x.state,gates).status,'unavailable');});
  control('child-capture-cannot-cross-host-prefix-hole',()=>{const x=setup(limits,2),t=x.admit();x.submit(t);const p=x.packet(),r=p.regions[0];assert(r.ranges.length>1);r.captures=[['variable',r.ranges[0][0],r.ranges[1][1]]];x.record(t,p);assert.equal(api.acceptAnalysisWork(x.owner,t,x.state,gates).status,'unavailable');});
  control('resolved-child-cannot-cross-host-prefix-hole',()=>{const x=setup(limits,2),t=x.admit();x.submit(t);const p=x.packet(),r=p.regions[0];p.captures=[['child','variable',r.ranges[0][0],r.ranges[1][1]]];x.record(t,p);assert.equal(api.acceptAnalysisWork(x.owner,t,x.state,gates).status,'unavailable');});
  control('resolved-child-cannot-claim-host-prose',()=>{const x=setup(),t=x.admit();x.submit(t);const p=x.packet();p.captures=[['child','variable',0,1]];x.record(t,p);assert.equal(api.acceptAnalysisWork(x.owner,t,x.state,gates).status,'unavailable');});
  for(const k of ['maxTasks','maxOutstanding','maxIssued','maxCaptureBytes','maxEnvelopeBytes','maxHeldBytes','maxSourceBytes'])control(`admission-budget-${k}`,()=>{const x=setup({...limits,[k]:0});assert.equal(api.admitAnalysisWork(x.owner,x.state,x.request(),gates).status,'unavailable');});
  for(const k of ['maxRegions','maxRanges','maxCaptures','maxTagBytes'])control(`packet-budget-${k}`,()=>{const x=setup({...limits,[k]:0}),t=x.admit();x.submit(t);x.record(t);assert.equal(api.acceptAnalysisWork(x.owner,t,x.state,gates).status,'unavailable');});
  for(const k of ['maxQueryBytes','maxReplyBytes'])control(`query-budget-${k}`,()=>{const x=setup({...limits,[k]:0}),a=x.ready();assert.equal(api.queryAnalysis(x.owner,x.state,x.query(a.snapshot),gates).status,'unavailable');});
  control('invalid-profile-limits-closed',()=>{assert.equal(api.newAnalysisRegistry({...limits,extra:1},parserLimits),null);assert.equal(api.newAnalysisRegistry({...limits,maxTasks:-1},parserLimits),null);assert.equal(api.newAnalysisRegistry(limits,{...parserLimits,maxCoordinate:4294967296}),null);});
  control('query-only-owned-base-highlight-only-change',()=>{
    const x=setup(),a=x.ready();x.quiet(a.task);x.c.highlightQueryGeneration++;x.state.analysisContexts[0].highlightQueryGeneration++;
    const t=x.admit('two',{mode:'query-only',base:a.snapshot}),out=x.submit(t);assert.equal(out.capture.input.mode,'reuse-parse');assert.equal(out.base,a.snapshot);x.record(t,x.packet(),'two');const s=x.accepted(t);
    assert.equal(api.queryAnalysis(x.owner,x.state,x.query(s),gates).status,'captures');assert.equal(api.queryAnalysis(x.owner,x.state,x.query(a.snapshot),gates).status,'unavailable');
    assert.equal(api.retireAnalysisWork(x.owner,a.task,retirement).status,'unavailable');x.quiet(t,'two');assert.equal(api.retireAnalysisWork(x.owner,t,retirement).status,'retired');assert.equal(api.retireAnalysisWork(x.owner,a.task,retirement).status,'retired');
  });
  for(const k of ['grammarGeneration','discoveryQueryGeneration','registryGeneration','configurationGeneration'])control(`query-reuse-rejects-${k}`,()=>{const x=setup(),a=x.ready();x.quiet(a.task);x.c[k]++;x.state.analysisContexts[0][k]++;assert.equal(api.admitAnalysisWork(x.owner,x.state,x.request('two',{mode:'query-only',base:a.snapshot}),gates).status,'unavailable');});
  control('query-only-rejects-null-or-copied-base-and-journals',()=>{const x=setup(),a=x.ready();for(const extra of [{base:null},{base:copy(a.snapshot)},{base:a.snapshot,journals:[{}]}])assert.equal(api.admitAnalysisWork(x.owner,x.state,x.request('two',{mode:'query-only',...extra}),gates).status,'unavailable');});
  control('base-contradiction-reaches-dependent-snapshot',()=>{const x=setup(),a=x.ready();x.quiet(a.task);x.c.highlightQueryGeneration++;x.state.analysisContexts[0].highlightQueryGeneration++;const t=x.admit('two',{mode:'query-only',base:a.snapshot});x.submit(t);x.record(t,x.packet(),'two');const s=x.accepted(t);assert.equal(api.observeAnalysisWork(x.owner,x.event(a.task,'failure'),ingress).status,'conflict');assert.equal(api.queryAnalysis(x.owner,x.state,x.query(s),gates).status,'unavailable');});
  control('cancelled-query-disabled-base-reusable-with-new-authority',()=>{const x=setup(),a=x.ready();api.controlAnalysisWork(x.owner,a.task,'cancel',gates);x.quiet(a.task);const t=x.admit('two',{mode:'query-only',base:a.snapshot});assert.equal(x.submit(t).capture.input.mode,'reuse-parse');});
  control('owned-source-journal-incremental-and-missing-journal-fresh',()=>{
    const x=setup(),a=x.ready();x.quiet(a.task);const d=x.state.documents[0],out=publishAnnotationPlan(x.state,{contract:'owned-annotation-plan/1',invocation:'edit',descriptor:'edit',sources:[{document:copy(document),expectedSource:1,expectedAnnotationSet:1,edits:[{lo:2,hi:7,ins:bytes('Revised'),expected:d.source.slice(2,7)}]}],annotations:[]},
      {authorized:true,bounded:true,notStopped:true,interpretationCurrent:true,companionsReady:true,versionsAvailable:true,historyReady:true,completeIdentity:true,maxParticipants:4,parserSourceReserved:true,parserSourceLimits:{maxDocuments:4,maxRootBytes:32768,maxBatches:32,maxEdits:64,maxJournalBytes:262144}});
    assert.equal(out.status,'applied');Object.assign(x.state,out.state);x.c.sourceRevision=x.state.documents[0].revision;x.state.views.forEach(v=>v.sourceRevision=x.c.sourceRevision);
    const t=x.admit('two',{base:a.snapshot,journals:committedSourceTransforms(out.state)}),submitted=x.submit(t);assert.equal(submitted.capture.input.mode,'incremental');assert.deepEqual(submitted.capture.input.edits[0].start_point,{row:0,column:2});
    api.controlAnalysisWork(x.owner,t,'cancel',gates);x.quiet(t,'two');const n=x.admit('three',{base:a.snapshot});assert.equal(x.submit(n).capture.input.mode,'fresh');assert.equal(api.retireAnalysisWork(x.owner,a.task,retirement).status,'unavailable');assert.equal(api.retireAnalysisWork(x.owner,t,retirement).status,'retired');assert.equal(api.retireAnalysisWork(x.owner,a.task,retirement).status,'retired');
  });
  control('stale-identical-bytes-new-revision-not-current',()=>{const x=setup(),a=x.ready();x.state.documents[0].revision++;assert.equal(api.queryAnalysis(x.owner,x.state,x.query(a.snapshot),gates).status,'unavailable');});
  control('query-reuse-cannot-equate-bytes-and-revision',()=>{const x=setup(),a=x.ready();x.quiet(a.task);x.state.documents[0].revision++;x.c.sourceRevision++;assert.equal(api.admitAnalysisWork(x.owner,x.state,x.request('two',{mode:'query-only',base:a.snapshot}),gates).status,'unavailable');});
  control('retirement-gates-all-required',()=>{for(const k of Object.keys(retirement)){const x=setup(),a=x.ready();x.quiet(a.task);assert.equal(api.retireAnalysisWork(x.owner,a.task,{...retirement,[k]:false}).status,'unavailable');}});
  control('aggregate-task-issued-work-budgets-not-one-task-only',()=>{for(const k of ['maxTasks','maxOutstanding','maxIssued']){const x=setup({...limits,[k]:1});x.admit();assert.equal(api.admitAnalysisWork(x.owner,x.state,x.request('two'),gates).status,'unavailable');}});
  control('oversized-result-envelope-not-retained',()=>{const x=setup(),t=x.admit();x.submit(t);const p=x.packet();p.padding='x'.repeat(limits.maxEnvelopeBytes);assert.equal(api.observeAnalysisWork(x.owner,x.event(t,'result',p),ingress).status,'unavailable');assert.equal(api.analysisTaskStatus(x.owner,t).witnesses,0);});
  return {scope:'owned-analysis-work-reference-only',admissionGateCases,submissionGateCases,acceptanceGateCases,releaseGateCases,emptyRangeCases,scalarRangeCases,interiorRangeRefusals,nativePacketCases:f.native.mixed.length,controls,...JSON.parse(fs.readFileSync(new URL('./analysis-work.json',import.meta.url),'utf8')).scope};
}
export async function checkAnalysisWorkMutations(f=fixtures()){
  const source=fs.readFileSync(new URL('./analysis-work.mjs',import.meta.url),'utf8'),mutations=[
    ['empty-range-is-not-an-intersection','r.range[0]<r.range[1]&&lo<r.range[1]','lo<r.range[1]'],
    ['submit-without-lease','||o.leases.has(key(t.context.analysis))','||false'],
    ['cancel-frees-lease','t[field]=true;','owners.get(owner).leases.delete(key(t.context.analysis));t[field]=true;'],
    ['quiescence-unattested','if(!g.quiescenceAttested)','if(false)'],
    ['source-version-not-current','docs[0].revision!==c.sourceRevision','false'],
    ['source-bytes-not-current','!equal(d.source,t.input.source)','false'],
    ['query-generation-in-parse-key','catalog.parseKeyFields.map(k=>','[...catalog.parseKeyFields,"highlightQueryGeneration"].map(k=>'],
    ['parse-key-ignores-grammar','catalog.parseKeyFields.map(k=>','catalog.parseKeyFields.filter(k=>k!=="grammarGeneration").map(k=>'],
    ['conflict-query-survives','invalidate(t,true);return {status:\'conflict\'}','/* invalidation lost */return {status:\'conflict\'}'],
    ['copied-task-correlation','o.tasks.get(event?.task)','[...o.tasks.values()][0]'],
    ['view-analysis-not-current','!equal(views[0].analysis,s.packet.context.analysis)','false'],
    ['retire-held-base','[...o.tasks.values()].some(p=>p.base!==null&&p.base===t.snapshot)','false'],
    ['utf8-interior-accepted',"(p===s.length||(s[p]&0xc0)!==0x80)",'true'],
    ['capture-hole-not-checked','!r.ranges.some(([a,b])=>a<=row[1]&&row[2]<=b)','false'],
    ['resolved-child-hole-not-checked',"p.captures.every(([layer,,lo,hi])=>layer!=='child'||p.regions.some(r=>r.kind==='parsed'&&r.ranges.some(([a,b])=>a<=lo&&hi<=b)))",'true'],
    ['acceptance-current-authority-ignored','!acceptance.every(k=>g?.[k])','false'],
    ['cancelled-snapshot-readable','s.queryEnabled=false;','/* query remains enabled */'],
    ['source-revision-in-query-reuse-ignored','s.packet.context.sourceRevision===c.sourceRevision&&equal(s.packet.source,source)','equal(s.packet.source,source)'],
    ['base-chain-invalidity-ignored','!s.parseValid||','false||'],
    ['envelope-cap-ignored','size(event)>o.limits.maxEnvelopeBytes','false'],
    ['reply-cap-ignored','size(result)<=o.limits.maxReplyBytes','true']
  ],killed=[];
  for(const [name,from,to] of mutations){
    assert(source.includes(from),name);let src=source.replace(from,to);
    for(const dep of ['text-oracles.mjs','parser-source.mjs','analysis-query-values.mjs','semantic-settings.mjs','analysis-query-arguments.mjs','invocation-publication.mjs'])src=src.replace(`'./${dep}'`,JSON.stringify(new URL(dep,import.meta.url).href));
    for(const asset of ['analysis-work.json','parser-source.json'])src=src.replace(`new URL('./${asset}',import.meta.url)`,`new URL(${JSON.stringify(new URL(asset,import.meta.url).href)})`);
    const api=await import(`data:text/javascript;base64,${Buffer.from(src).toString('base64')}#${name}`);
    let failed=false;try{checkAnalysisWork(api,f);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name);failed=true;}assert(failed,name);killed.push(name);
  }return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){const f=fixtures();console.log(JSON.stringify({...checkAnalysisWork(original,f),mutations:await checkAnalysisWorkMutations(f)},null,2));}
