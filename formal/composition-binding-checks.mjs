import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {prepareSelectionEdit,selectionInvocationIntent,compositionInputForm} from './selection-products.mjs';
import {normalizeCompositionArguments} from './composition-arguments.mjs';
import {searchLiteral} from './literal-search.mjs';
import {editingInvocationFixture,editingInvocationContext as context,editingInvocationPolicy as policy} from './editing-invocation-checks.mjs';
const copy=structuredClone,bytes=s=>Array.from(Buffer.from(s,'utf8'));
const inputLimits={maxRequestBytes:65536,maxInputNodes:16384,maxInputDepth:64};
function fixture(options={}){
  const original=editingInvocationFixture('selection',{...kernel,prepareSelectionEdit,selectionInvocationIntent},options);
  const state=copy(original.root()),v=state.views[0];
  v.selections=options.selections??[{anchor:0,head:1,affinity:'after'},{anchor:2,head:3,affinity:'after'}];v.primary=options.primary??1;
  const host={...original.h,outsidePublicationGuard:true,preparationReserved:true},ownerLimits={...original.ls,maxIntentBytes:options.parentMaximum??original.ls.maxIntentBytes};
  if(options.headroom!==undefined)ownerLimits.maxHeldBytes=2*ownerLimits.maxRootBytes+Buffer.byteLength(JSON.stringify(context))+ownerLimits.maxSlots*(4*ownerLimits.maxIntentBytes+5*ownerLimits.maxResultBytes)+options.headroom;
  const domain=kernel.openInvocationDomain(state,context,ownerLimits,host);assert(domain);
  const issued=kernel.issueInvocation(domain,context,host);assert(issued);
  const request={contract:'composition-commit/1',operation:'accepted-composition-commit',invocation:issued.reference,document:copy(v.document),view:copy(v.view),sourceRevision:1,expectedSelection:1,profile:'primary-composition-replacement/1',replacement:options.replacement??{lo:2,hi:3,expected:bytes('c')},text:options.text??'X'};
  const envelope=(route='typed',r=request)=>({route,request:{...(route==='typed'?{family:10}:{name:'change-source'}),variant:'accepted-composition-commit',version:1,arguments:r}});
  return {domain,...issued,host,request,envelope,
    submit:(route='typed',r=request,h=host,limits=inputLimits)=>kernel.submitInvocationComposition(domain,issued.ticket,envelope(route,r),limits,context,policy,h),
    run:(h=host)=>kernel.runInvocation(domain,issued.ticket,context,h,h),
    root:()=>kernel.invocationAuthorityRoot(domain),usage:()=>kernel.invocationUsage(domain),
    read:()=>kernel.observeInvocation(domain,issued.ticket,context,policy,host)};
}
export function checkCompositionBinding(){
  const controls=[],control=(name,f)=>{f();controls.push(name);};let omissions=0,permissionCases=0;
  control('primary-only-native-publication-and-owned-replay',()=>{
    const z=fixture(),before=z.usage();assert.equal(z.submit().status,'accepted');const admitted=z.usage();assert(admitted.declaredHeldBytes>before.declaredHeldBytes);
    assert.equal(z.submit('dynamic').status,'pending');assert.deepEqual(z.usage(),admitted);
    assert.equal(z.run().status,'settled');const root=z.root();assert.equal(Buffer.from(root.documents[0].source).toString(),'abX');assert.equal(root.groups.length,1);
    assert.deepEqual(root.views[0].selections,[{anchor:0,head:1,affinity:'after'},{anchor:3,head:3,affinity:'after'}]);assert.equal(root.views[0].primary,1);assert.equal(root.views[1].viewport,17);
    assert.equal(z.submit('dynamic').status,'observe-existing');assert.equal(z.run().status,'unavailable');assert.equal(z.root(),root);assert.equal(z.read().outcome.publication,'applied');
    const held=z.usage().declaredHeldBytes;for(const key of ['retentionReleased','deliveryDrained','ownerControlAllowed'])assert.equal(kernel.retireInvocation(z.domain,z.ticket,{...z.host,[key]:false}).status,'unavailable');
    assert.equal(kernel.retireInvocation(z.domain,z.ticket,z.host).status,'retired');assert(z.usage().declaredHeldBytes<held);assert.equal(kernel.retireInvocation(z.domain,z.ticket,z.host).status,'unavailable');
  });
  control('explicit-surrounding-range-not-implicitly-current-selection',()=>{
    const z=fixture({replacement:{lo:1,hi:3,expected:bytes('bc')},text:'XY',selections:[{anchor:0,head:1,affinity:'before'},{anchor:3,head:3,affinity:'after'}]});
    assert.equal(z.submit().status,'accepted');z.run();assert.equal(Buffer.from(z.root().documents[0].source).toString(),'aXY');assert.equal(z.root().views[0].selections[1].head,3);
  });
  control('primary-index-zero-and-reversed-secondary-preserved',()=>{
    const z=fixture({primary:0,replacement:{lo:0,hi:1,expected:bytes('a')},selections:[{anchor:0,head:1,affinity:'before'},{anchor:3,head:2,affinity:'after'}],text:'YZ'});
    assert.equal(z.submit().status,'accepted');z.run();assert.equal(Buffer.from(z.root().documents[0].source).toString(),'YZbc');assert.deepEqual(z.root().views[0].selections,[{anchor:2,head:2,affinity:'before'},{anchor:4,head:3,affinity:'after'}]);assert.equal(z.root().views[0].primary,0);
  });
  control('primary-reconstructed-before-mapped-selection-merge',()=>{const z=fixture({source:'abcdef',selections:[{anchor:0,head:1,affinity:'after'},{anchor:2,head:3,affinity:'after'},{anchor:4,head:5,affinity:'after'}],primary:2,replacement:{lo:0,hi:5,expected:bytes('abcde')}});assert.equal(z.submit().status,'accepted');z.run();assert.equal(Buffer.from(z.root().documents[0].source).toString(),'Xf');assert.deepEqual(z.root().views[0].selections,[{anchor:0,head:1,affinity:'after'},{anchor:1,head:1,affinity:'after'}]);assert.equal(z.root().views[0].primary,1);});
  control('unicode-byte-range-and-literal-payload',()=>{
    const z=fixture({source:'a😀c',replacement:{lo:1,hi:5,expected:bytes('😀')},selections:[{anchor:0,head:1,affinity:'after'},{anchor:5,head:6,affinity:'after'}],text:'\nλ'});
    assert.equal(z.submit().status,'accepted');z.run();assert.equal(Buffer.from(z.root().documents[0].source).toString(),'a\nλc');assert.equal(z.root().views[0].selections[1].head,4);
  });
  control('empty-commit-deletes-explicit-range',()=>{const z=fixture({text:''});assert.equal(z.submit().status,'accepted');z.run();assert.equal(Buffer.from(z.root().documents[0].source).toString(),'ab');assert.equal(z.root().views[0].selections.at(-1).head,2);});
  control('equal-bytes-caret-effect-not-source-history',()=>{const z=fixture({text:'c'});assert.equal(z.submit().status,'accepted');z.run();assert.equal(z.root().documents[0].revision,1);assert.equal(z.root().groups.length,0);assert.equal(z.root().views[0].selections.at(-1).head,3);assert.equal(z.read().outcome.publication,'applied');});
  control('changed-intent-never-reuses-original-claim',()=>{const z=fixture();assert.equal(z.submit().status,'accepted');const before=z.usage();for(const r of [{...z.request,text:'Y'},{...z.request,replacement:{lo:1,hi:3,expected:bytes('bc')}},{...z.request,expectedSelection:2}])assert.equal(z.submit('dynamic',r).status,'unavailable');assert.deepEqual(z.usage(),before);z.request.text='mutated';z.run();assert.equal(Buffer.from(z.root().documents[0].source).toString(),'abX');});
  control('native-intent-retains-explicit-accepted-replacement',()=>{const z=fixture(),job=kernel.prepareInvocationComposition(z.domain,z.ticket,z.envelope(),inputLimits,context,z.host);assert(job);const captured=selectionInvocationIntent(job.plan,65536);assert(captured);assert.deepEqual(captured.value.compositionRequest,z.request);assert.deepEqual(captured.value.edits,[{...z.request.replacement,ins:bytes('X')}]);const changed=copy(z.request);changed.replacement.expected=bytes('b');assert.equal(kernel.prepareInvocationComposition(z.domain,z.ticket,z.envelope('dynamic',changed),inputLimits,context,z.host),null);});
  control('strict-fields-current-target-and-range-refusal',()=>{
    const z=fixture(),before=z.usage();for(const key of compositionInputForm().fields){const r=copy(z.request);delete r[key];assert.equal(z.submit('typed',r).status,'unavailable');omissions++;}
    for(const r of [{...z.request,extra:0},{...z.request,sourceRevision:2},{...z.request,expectedSelection:2},{...z.request,view:{...z.request.view,incarnation:2}},{...z.request,replacement:{lo:2,hi:3,expected:bytes('b')}},{...z.request,replacement:{lo:3,hi:2,expected:[]}},{...z.request,text:'\ud800'}])assert.equal(z.submit('typed',r).status,'unavailable');
    assert.deepEqual(z.usage(),before);assert.equal(z.root().publication,0);
  });
  control('current-permissions-rechecked-at-service',()=>{for(const key of ['sourceReadable','sourceWritable','allTargetsWritable','viewWritable','companionsReady','versionsAvailable','viewVersionsAvailable','notStopped']){const z=fixture();assert.equal(z.submit().status,'accepted');const before=z.root();assert.equal(z.run({...z.host,[key]:false}).status,'settled');assert.equal(z.root(),before);assert.equal(z.read().outcome.publication,'notApplied');permissionCases++;}});
  control('stale-original-view-after-intervening-real-command',()=>{
    const z=fixture();assert.equal(z.submit().status,'accepted');const issued=kernel.issueInvocation(z.domain,context,z.host),r={contract:'selection-edit/1',operation:'set-selection',invocation:issued.reference,document:z.request.document,view:z.request.view,sourceRevision:1,expectedSelection:1,profile:'explicit-positional-selection/1',selections:[{anchor:1,head:1,affinity:'after'}],primary:0};
    const plan=prepareSelectionEdit(z.root(),r,z.host,z.host.viewLimits);assert(plan);assert.equal(kernel.submitInvocation(z.domain,issued.ticket,{kind:'selection',plan},'typed',context,policy,z.host).status,'accepted');kernel.runInvocation(z.domain,issued.ticket,context,z.host,z.host);const before=z.root();z.run();assert.equal(z.root(),before);assert.equal(z.read().outcome.publication,'notApplied');
  });
  control('stop-does-not-publish-or-make-ticket-fresh',()=>{const z=fixture();z.submit();kernel.stopInvocation(z.domain,z.ticket,z.host);z.run();assert.equal(z.root().publication,0);assert.equal(z.read().outcome.control,'stopped');assert.equal(z.submit().status,'observe-existing');});
  control('argument-bounds-and-typed-dynamic-meaning',()=>{const z=fixture();const a=normalizeCompositionArguments(z.envelope(),context,inputLimits,65536),b=normalizeCompositionArguments(z.envelope('dynamic'),context,inputLimits,65536);assert(a&&b);assert.deepEqual(a.record,b.record);for(const limits of [{...inputLimits,maxRequestBytes:0},{...inputLimits,maxInputNodes:1},{...inputLimits,maxInputDepth:0}])assert.equal(z.submit('typed',z.request,z.host,limits).status,'unavailable');assert.equal(z.root().publication,0);});
  control('competing-native-facade-preparations-cannot-share-ticket',()=>{
    for(const kind of ['ordinary','selection','replacement'])for(const first of ['composition','other']){
      const z=fixture(),base={invocation:z.reference,document:z.request.document,view:z.request.view,sourceRevision:1,expectedSelection:1,profile:'explicit-positional-selection/1'};
      let r,prepare;
      if(kind==='ordinary'){r={...base,contract:'selection-edit/1',operation:'ordinary-edit',text:'X',selectionAfter:'carets-after'};prepare=()=>kernel.prepareInvocationOrdinaryEdit(z.domain,z.ticket,{route:'typed',request:{family:10,variant:'ordinary-edit',version:1,arguments:r}},inputLimits,context,z.host);}
      else if(kind==='selection'){r={...base,contract:'selection-edit/1',operation:'set-selection',selections:[{anchor:0,head:0,affinity:'after'}],primary:0};prepare=()=>kernel.prepareInvocationSelection(z.domain,z.ticket,{route:'typed',request:{family:11,variant:'set-selection',version:1,arguments:r}},inputLimits,context,z.host);}
      else{
        const document=z.root().documents[0],ref={authority:'a',document:'A',incarnation:1,revision:1},query={contract:'literal-search/1',operation:'literal-matches',context:'query-context',ref,query:'a',profile:'literal-exact/1',scopes:[{lo:0,hi:3}],generation:1,navigation:null},current={request:copy(query),grant:true,scope:true,continuity:true,capacity:true,retained:true,notStopped:true};
        const result=searchLiteral({ref,source:copy(document.source)},query,current,{maxRootBytes:4096,maxScopes:8,maxInputBytes:4096,maxQueryBytes:128,maxComparisons:32768,maxMatches:64,maxReplyBytes:32768,maxReplacementBytes:2048,maxEditBytes:8192,chunkBytes:2});assert.equal(result.status,'complete');z.host.searchCurrent=current;
        r={operation:'replace-all',index:null,replacement:'Y',invocation:z.reference};prepare=()=>kernel.prepareInvocationReplacement(z.domain,z.ticket,{route:'typed',request:{family:10,variant:'replace-all',version:1,arguments:r}},result,inputLimits,context,z.host);
      }
      const composition=()=>kernel.prepareInvocationComposition(z.domain,z.ticket,z.envelope(),inputLimits,context,z.host),a=first==='composition'?composition:prepare,b=first==='composition'?prepare:composition;
      assert(a(),kind+':'+first);const usage=z.usage();assert.equal(b(),null,kind+':'+first);assert.deepEqual(z.usage(),usage);
    }
    const a=fixture(),b=fixture(),job=kernel.prepareInvocationComposition(a.domain,a.ticket,a.envelope(),inputLimits,context,a.host);assert(job);assert.equal(kernel.submitInvocation(b.domain,b.ticket,job,'typed',context,policy,b.host).status,'unavailable');
  });
  control('exact-owner-headroom-native-envelope-and-refusal-refund',()=>{
    const seed=fixture();const job=kernel.prepareInvocationComposition(seed.domain,seed.ticket,seed.envelope(),inputLimits,context,seed.host);assert(job);const charge=seed.usage().ordinaryArgumentBytes,native=selectionInvocationIntent(job.plan,65536);assert(native);const valueBytes=Buffer.byteLength(JSON.stringify(native.value));assert(valueBytes>charge);
    const tight=fixture({parentMaximum:valueBytes}),before=tight.usage();assert.equal(tight.submit().status,'unavailable');assert.deepEqual(tight.usage(),before);assert.equal(tight.root().publication,0);
    const z=fixture(),initial=z.usage();assert.equal(z.submit('typed',{...z.request,replacement:{lo:2,hi:3,expected:bytes('b')}}).status,'unavailable');assert.deepEqual(z.usage(),initial);assert.equal(z.submit().status,'accepted');
    for(const headroom of [charge-1,charge,charge+1]){const q=fixture({headroom}),usage=q.usage();assert.equal(q.submit().status,headroom<charge?'unavailable':'accepted');if(headroom<charge)assert.deepEqual(q.usage(),usage);else assert.equal(q.usage().declaredHeldBytes,usage.declaredHeldBytes+charge);}
  });
  return {controls,omissions,permissionCases,primaryOnlyEngineBindingChecked:true,nativeIMEOrProductionImplementationProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkCompositionBinding()));
