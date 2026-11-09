// Actual F05 occurrence ownership -> F12 selection -> F11 complete view publication.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import * as selectionApi from './selection-products.mjs';
import * as searchApi from './literal-search.mjs';
import * as occurrenceApi from './search-occurrences.mjs';
import {publishAnnotationPlan} from './annotation-compounds.mjs';
const copy=structuredClone,bytes=s=>Array.from(Buffer.from(s,'utf8'));
const profile=JSON.parse(fs.readFileSync(new URL('./selection-products.json',import.meta.url),'utf8'));
export function checkSearchSelection(selection=selectionApi,search=searchApi,occurrences=occurrenceApi){
  assert(profile.families.F12.includes('select-search-match'));
  const document={authority:'a',id:'A',incarnation:1},view={authority:'a',id:'origin',incarnation:1};
  const point=p=>({anchor:p,head:p,affinity:'after'});
  const source=bytes('aa🙂aa\r\naa'),ref={authority:'a',document:'A',incarnation:1,revision:1};
  const query={contract:'literal-search/1',operation:'literal-matches',context:'source-context',ref,query:'aa',profile:'literal-exact/1',scopes:[{lo:0,hi:source.length}],generation:1,navigation:null};
  const searchCurrent={request:copy(query),grant:true,scope:true,continuity:true,capacity:true,retained:true,notStopped:true};
  const searchLimits={maxRootBytes:256,maxScopes:8,maxInputBytes:256,maxQueryBytes:32,maxComparisons:4096,maxMatches:64,maxReplyBytes:16384,maxReplacementBytes:32,maxEditBytes:1024,chunkBytes:1};
  const snapshot={ref:copy(ref),source:copy(source),annotationSet:1};
  const result=search.searchLiteral(snapshot,query,searchCurrent,searchLimits);
  const tickets=result.matches.map((_,index)=>search.bindSearchOccurrence(result,index,searchCurrent));
  const viewLimits={maxRootBytes:256,maxInputViews:8,maxViews:8,maxInputSelections:8,maxSelections:8,maxSelectionBytes:2048,maxTextBytes:32,maxEditBytes:1024};
  const viewWorkLimits={maxViewBatches:16,maxViewEdits:64,maxViewBytes:32768,maxViewPlanBytes:32768};
  const host={authorized:true,sourceReadable:true,viewWritable:true,continuous:true,live:true,bounded:true,notStopped:true,interpretationCurrent:true,companionsReady:true,
    versionsAvailable:true,viewVersionsAvailable:true,sourceWritable:true,allTargetsWritable:true,historyReady:true,completeIdentity:true,maxParticipants:8,
    allMatchesWritable:true,targetRestorationValid:true,searchCurrent,viewLimits,viewWorkLimits};
  const state={authority:'a',publication:0,documents:[{document,source:copy(source),revision:1,annotationSet:1,records:[]}],groups:[],frontiers:{},views:[
    {view,document,sourceRevision:1,selectionGeneration:1,profile:profile.profile,selections:[point(2),point(8)],primary:0,viewport:5},
    {view:{...view,id:'peer'},document,sourceRevision:1,selectionGeneration:1,profile:profile.profile,selections:[point(12)],primary:0,viewport:9}]};
  const request={contract:profile.contract,operation:'select-search-match',invocation:'select',view,document,sourceRevision:1,expectedSelection:1,profile:profile.profile,
    match:tickets[1],direction:'forward',affinity:'after',selectionMode:'replace-all'};
  const prepare=(r=request,s=state,h=host)=>selection.prepareSelectionEdit(s,r,h,viewLimits);
  const apply=(r=request,s=state,h=host)=>selection.publishSelectionEdit(s,prepare(r,s,h),h);
  const replace={operation:'replace-current',index:1,replacement:'Z',invocation:'replace'};
  const replacement=(s=state,r=replace,out=result,h=host,c=searchCurrent)=>search.prepareReplacement(out,r,c,s,h);
  const publish=(s=state,p=replacement(),h=host,c=searchCurrent)=>search.publishReplacement(s,p,c,h);
  const coherent=s=>s.views.every(v=>v.sourceRevision===s.documents[0].revision&&
    equal(selection.normalizeSelection(s.documents[0].source,v.selections,v.primary,viewLimits),{selections:v.selections,primary:v.primary}));
  const controls=[],control=(name,condition)=>{assert(condition,name);controls.push(name);};
  let selectionCases=0,gateCases=0,replacementCases=0;
  for(let index=0;index<tickets.length;index++)for(const selectionMode of profile.selectionModes)for(const direction of ['forward','backward'])for(const affinity of ['before','after']){
    const r={...request,match:tickets[index],selectionMode,direction,affinity},out=apply(r),range=result.matches[index];
    const match={anchor:direction==='forward'?range.lo:range.hi,head:direction==='forward'?range.hi:range.lo,affinity};
    const expected=selectionMode==='replace-all'?[match]:selectionMode==='replace-primary'?[match,point(8)]:[point(2),point(8),match];
    expected.sort((a,b)=>Math.min(a.anchor,a.head)-Math.min(b.anchor,b.head)||Math.max(a.anchor,a.head)-Math.max(b.anchor,b.head));
    assert.equal(out.status,'applied');assert.deepEqual(out.state.views[0].selections,expected);assert.equal(out.state.views[0].primary,expected.indexOf(match));
    assert.equal(out.state.documents[0].revision,1);assert.equal(out.state.groups.length,0);assert.deepEqual(out.state.views[1],state.views[1]);selectionCases++;
  }
  const plan=prepare(),viewGates=['authorized','sourceReadable','viewWritable','continuous','live','bounded','notStopped','interpretationCurrent','companionsReady'];
  const queryGates=['grant','scope','continuity','capacity','retained','notStopped'];
  for(let v=0;v<512;v++)for(let q=0;q<64;q++){
    const h={...host,...Object.fromEntries(viewGates.map((k,i)=>[k,!!(v&(1<<i))])),searchCurrent:{request:copy(query),...Object.fromEntries(queryGates.map((k,i)=>[k,!!(q&(1<<i))]))}};
    const out=selection.publishSelectionEdit(state,plan,h);assert.equal(out.status,v===511&&q===63?'applied':'not-published');if(v!==511||q!==63)assert.equal(out.state,state);gateCases++;
  }
  control('lookup-reuses-bounded-owned-ticket',result.matches[1].reference===tickets[1]&&tickets[1]===search.bindSearchOccurrence(result,1,searchCurrent)&&Object.isFrozen(tickets[1]));
  for(let i=0;i<128;i++)assert.equal(search.bindSearchOccurrence(result,1,searchCurrent),tickets[1]);
  control('copied-result-cannot-mint-occurrence',search.bindSearchOccurrence(copy(result),0,searchCurrent)===null);
  control('copied-ticket-not-owned',prepare({...request,match:copy(tickets[1])})===null);
  control('raw-range-not-an-occurrence',prepare({...request,match:{lo:6,hi:8}})===null);
  for(const index of [-1,3,1.5,'1'])control(`invalid-index-${String(index)}`,search.bindSearchOccurrence(result,index,searchCurrent)===null);
  control('revoked-query-cannot-reissue-cached-ticket',search.bindSearchOccurrence(result,1,{...searchCurrent,grant:false})===null);
  const disclosure=occurrences.resolveSearchOccurrence(tickets[1],searchCurrent);disclosure.range.lo=0;
  control('resolved-occurrence-is-owned-copy',occurrences.resolveSearchOccurrence(tickets[1],searchCurrent).range.lo===6);
  for(const [key,value] of [['query','other'],['profile','other'],['context','other'],['generation',2],['scopes',[{lo:0,hi:2}]]]){
    const h={...host,searchCurrent:{...searchCurrent,request:{...copy(query),[key]:value}}};
    control(`changed-search-${key}`,selection.publishSelectionEdit(state,plan,h).status==='not-published'&&prepare(request,state,h)===null);
  }
  const otherQuery={...copy(query),context:'other-context',ref:{...ref,document:'B'}};
  const otherCurrent={...searchCurrent,request:copy(otherQuery)};
  const otherResult=search.searchLiteral({...snapshot,ref:otherQuery.ref},otherQuery,otherCurrent,searchLimits);
  const foreign=search.bindSearchOccurrence(otherResult,1,otherCurrent);
  control('foreign-document-occurrence-not-retargeted',prepare({...request,match:foreign},state,{...host,searchCurrent:otherCurrent})===null);
  const reincarnated=copy(state);reincarnated.documents[0].document.incarnation=2;for(const v of reincarnated.views)v.document.incarnation=2;
  control('old-incarnation-not-reused',prepare({...request,document:{...document,incarnation:2}},reincarnated)===null);
  const changedView=selection.publishSelectionEdit(state,selection.prepareSelectionEdit(state,{...request,operation:'set-selection',selections:[point(0)],primary:0,
    match:undefined,direction:undefined,affinity:undefined,selectionMode:undefined},host,viewLimits),host);
  // Build the exact native set-selection shape; unknown fields never become ignored hints.
  const setRequest={contract:profile.contract,operation:'set-selection',invocation:'move',view,document,sourceRevision:1,expectedSelection:1,profile:profile.profile,selections:[point(0)],primary:0};
  assert.equal(changedView.status,'not-published');
  const moved=selection.publishSelectionEdit(state,selection.prepareSelectionEdit(state,setRequest,host,viewLimits),host).state;
  control('changed-view-generation-not-retargeted',selection.publishSelectionEdit(moved,plan,host).status==='not-published');
  const first=apply({...request,match:tickets[0]});
  const second=apply({...request,expectedSelection:2,match:tickets[2]},first.state);
  control('search-scope-independent-of-selected-match',second.status==='applied'&&equal(second.state.views[0].selections,[{anchor:10,head:12,affinity:'after'}])&&equal(searchCurrent.request.scopes,query.scopes));
  control('read-only-source-allows-search-selection',apply(request,state,{...host,sourceWritable:false,allMatchesWritable:false}).status==='applied');
  const same=apply({...request,expectedSelection:2},apply().state,{...host,versionsAvailable:false,viewVersionsAvailable:false});
  control('reselecting-same-match-is-validated-identity',same.status==='unchanged'&&same.state.publication===1&&same.state.views[0].selectionGeneration===2);
  const partial=search.searchLiteral(snapshot,query,searchCurrent,{...searchLimits,maxMatches:1}),known=search.bindSearchOccurrence(partial,0,searchCurrent);
  control('known-partial-occurrence-can-be-selected',partial.total===null&&apply({...request,match:known}).status==='applied');
  control('empty-query-cannot-mint-match',(()=>{const r={...copy(query),query:''},c={...searchCurrent,request:r},out=search.searchLiteral(snapshot,r,c,searchLimits);return search.bindSearchOccurrence(out,0,c)===null;})());
  for(const operation of ['replace-current','replace-all'])for(const replacementText of ['Z','$1','']){
    const r={...replace,operation,index:operation==='replace-all'?null:1,replacement:replacementText},out=publish(state,replacement(state,r));
    assert.equal(out.status,'applied');assert(coherent(out.state));assert.equal(out.state.publication,1);assert.equal(out.state.groups.length,1);replacementCases++;
  }
  const replacementPlan=replacement(),changed=publish();
  control('replacement-updates-every-view-context',changed.status==='applied'&&coherent(changed.state)&&changed.state.views[1].selections[0].head===11);
  control('stale-source-occurrence-not-mapped-to-new-revision',prepare({...request,sourceRevision:2,expectedSelection:changed.state.views[0].selectionGeneration},changed.state)===null);
  control('replacement-readonly-refuses-whole',publish(state,replacementPlan,{...host,allMatchesWritable:false}).status==='not-published');
  control('replacement-view-version-capacity-refuses-whole',publish(state,replacementPlan,{...host,viewVersionsAvailable:false}).status==='not-published');
  const newerPeerRequest={...setRequest,view:state.views[1].view,selections:[point(10)]};
  const newerPeer=selection.publishSelectionEdit(state,selection.prepareSelectionEdit(state,newerPeerRequest,host,viewLimits),host).state;
  control('replacement-current-peer-change-stales-plan',publish(newerPeer,replacementPlan).status==='not-published');
  const added=copy(state);added.views.push({...copy(state.views[1]),view:{...view,id:'new'}});
  control('replacement-new-view-not-skipped',publish(added,replacementPlan).status==='not-published');
  const noInventory=copy(state);delete noInventory.views;
  control('replacement-unknown-inventory-not-empty',replacement(noInventory)===null);
  const cosmetic=copy(state);cosmetic.views[1].viewport=99;
  control('replacement-current-viewport-preserved',publish(cosmetic,replacementPlan).state.views[1].viewport===99);
  const annotated=copy(state),target={kind:'range',start:2,end:6};
  annotated.documents[0].records=[{id:'note',incarnation:1,body:'old',bodyVersion:1,target:{sourceRevision:1,target:copy(target)},targetGeneration:1,
    placement:{state:'attached',sourceRevision:1,target:copy(target)}}];
  const laterBody=publishAnnotationPlan(annotated,{contract:'owned-annotation-plan/1',invocation:'body',descriptor:'body',sources:[],annotations:[
    {kind:'body',document,id:'note',incarnation:1,expectedBody:1,body:'later'}]},host).state;
  const freshAnnotationPlan=replacement(laterBody),freshAnnotation=publish(laterBody,freshAnnotationPlan);
  control('replacement-reprepares-current-annotations-without-search-rerun',freshAnnotation.status==='applied'&&freshAnnotation.state.documents[0].records[0].body==='later'&&freshAnnotation.state.documents[0].records[0].bodyVersion===2&&coherent(freshAnnotation.state));
  control('later-annotation-set-stales-prepared-replacement',publish(laterBody,replacement(annotated)).status==='not-published');
  const equalInput=copy(state);equalInput.views[0].selections=[point(1)];
  const equalRequest={...replace,index:0,replacement:'aa'},equalPlan=replacement(equalInput,equalRequest),equalOut=publish(equalInput,equalPlan);
  control('equal-byte-replacement-view-effect-not-noop',equalOut.status==='applied'&&equalOut.state.documents[0].revision===1&&equalOut.state.groups.length===0&&equalOut.state.publication===1&&equalOut.state.views[0].selections[0].head===2);
  control('equal-byte-view-publication-needs-version-capacity',publish(equalInput,equalPlan,{...host,versionsAvailable:false}).status==='not-published');
  const identityPlan=replacement(state,{...replace,replacement:'aa'}),identity=publish(state,identityPlan,{...host,versionsAvailable:false,viewVersionsAvailable:false});
  control('complete-source-view-identity-preserves-root',identity.status==='unchanged'&&identity.state===state);
  const wide=copy(state);wide.views[0].selections=[point(2),{anchor:0,head:12,affinity:'after'}];wide.views[0].primary=0;
  // Store canonical order while retaining the explicit primary point.
  Object.assign(wide.views[0],selection.normalizeSelection(source,wide.views[0].selections,0,viewLimits));
  const widened=apply({...request,selectionMode:'replace-primary'},wide);
  const exact=publish(widened.state,replacement(widened.state));
  control('selection-normalization-cannot-widen-replacement',widened.status==='applied'&&widened.state.views[0].selections[0].head===12&&equal(exact.state.documents[0].source,bytes('aa🙂Z\r\naa')));
  control('partial-replace-all-still-refused',replacement(state,{...replace,operation:'replace-all',index:null},partial)===null);
  const emptyRequest={...copy(query),query:''},emptyCurrent={...searchCurrent,request:emptyRequest},emptyResult=search.searchLiteral(snapshot,emptyRequest,emptyCurrent,searchLimits);
  control('empty-match-set-needs-no-view-map-inventory',search.publishReplacement(noInventory,search.prepareReplacement(emptyResult,{...replace,operation:'replace-all',index:null},emptyCurrent,noInventory,host),emptyCurrent,host).status==='unchanged');
  for(const field of ['maxViews','maxSelectionBytes'])control(`replacement-limit-${field}`,replacement(state,replace,result,{...host,viewLimits:{...viewLimits,[field]:0}})===null);
  control('replacement-work-bound',replacement(state,replace,result,{...host,viewWorkLimits:{...viewWorkLimits,maxViewEdits:0}})===null);
  return {status:'owned-search-selection-and-view-replacement-only',selectionCases,gateCases,replacementCases,controls,
    fullMovementCatalogProved:false,actualIssuerPolicyRetentionProved:false,allStateCompanionsProved:false,physicalAccountingProved:false,implementationRefinement:false};
}

export async function checkSearchSelectionMutations(){
  const names={selection:'selection-products.mjs',search:'literal-search.mjs',occurrence:'search-occurrences.mjs',views:'history-views.mjs'};
  const originals=Object.fromEntries(Object.entries(names).map(([key,file])=>[key,fs.readFileSync(new URL(file,import.meta.url),'utf8')]));
  const mutants=[
    ['no-final-query-recheck','selection',"if(r.operation==='select-search-match'&&!resolveSearchOccurrence(r.match,g.searchCurrent))return refuse();",'/* omitted */'],
    ['ignore-occurrence-source-identity','selection',"!equal(occurrence.ref,{authority:request.document.authority,document:request.document.id,incarnation:request.document.incarnation,revision:request.sourceRevision})",'false'],
    ['painted-range-substitutes-match','selection','const {lo,hi}=occurrence.range','const {lo,hi}={lo:0,hi:d.source.length}'],
    ['no-occurrence-current-context','occurrence','!searchContextCurrent(owned.capture.request,current)','false'],
    ['unbounded-repeat-ticket-issuance','occurrence','if(capture.tickets.has(index))return capture.tickets.get(index);','/* issue again */'],
    ['replacement-omits-view-completion','search','return completeForwardViews(state,p.viewPart,published,checks.viewLimits,checks.viewWorkLimits,checks.viewVersionsAvailable);','return published;'],
    ['equal-source-drops-view-effect','views',"if(reversed.status==='unchanged'&&!part.viewChanged)","if(reversed.status==='unchanged')"],
    ['replacement-ignores-current-view-footprint','views','return equal(captureViews(state,part.sources,part.viewLimits),part.beforeViews);','return true;']
  ];
  const killed=[];
  for(const [name,target,from,to] of mutants){
    const code={...originals};assert(code[target].includes(from),name);code[target]=code[target].replace(from,to);
    const url=(source,key)=>`data:text/javascript;base64,${Buffer.from(source).toString('base64')}#${name}-${key}`;
    const occURL=url(code.occurrence,'occurrence');
    code.selection=code.selection.replace("'./search-occurrences.mjs'",JSON.stringify(occURL));
    for(const file of ['text-oracles.mjs','mapping-profiles.mjs','reference-profile.mjs','annotation-compounds.mjs','parser-source.mjs'])code.selection=code.selection.replace(`'./${file}'`,JSON.stringify(new URL(file,import.meta.url).href));
    for(const file of ['selection-products.json','operation-families.json'])code.selection=code.selection.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(file,import.meta.url).href)})`);
    const selURL=url(code.selection,'selection');
    code.views=code.views.replace("'./selection-products.mjs'",JSON.stringify(selURL)).replace("'./text-oracles.mjs'",JSON.stringify(new URL('./text-oracles.mjs',import.meta.url).href)).replace("'./parser-source.mjs'",JSON.stringify(new URL('./parser-source.mjs',import.meta.url).href));
    for(const file of ['history-views.json','selection-products.json'])code.views=code.views.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(file,import.meta.url).href)})`);
    const viewsURL=url(code.views,'views');
    code.search=code.search.replace("'./search-occurrences.mjs'",JSON.stringify(occURL)).replace("'./history-views.mjs'",JSON.stringify(viewsURL));
    for(const file of ['source-edit-contracts.mjs','baseline-contracts.mjs','annotation-compounds.mjs'])code.search=code.search.replace(`'./${file}'`,JSON.stringify(new URL(file,import.meta.url).href));
    for(const file of ['literal-search.json','operation-families.json'])code.search=code.search.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(file,import.meta.url).href)})`);
    const o=await import(occURL),s=await import(selURL),q=await import(url(code.search,'search'));
    let failed=false;try{checkSearchSelection(s,q,o);}catch(error){assert.equal(error.code,'ERR_ASSERTION',name);failed=true;}assert(failed,name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkSearchSelection(),mutations:await checkSearchSelectionMutations()},null,2));
