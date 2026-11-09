// Owned normalized search/replacement reference, not a native decoder or scheduler.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {readRange} from './source-edit-contracts.mjs';
import {navigate} from './baseline-contracts.mjs';
import {publishAnnotationPlan,reverseAnnotationGroup} from './annotation-compounds.mjs';
import {retainSearchResult,searchResultCapture,searchContextCurrent as currentAt,bindSearchOccurrence,discardSearchResultCapture} from './search-occurrences.mjs';
import {prepareForwardViews,validateReversalViews,completeForwardViews} from './history-views.mjs';
export {bindSearchOccurrence};
const catalog=JSON.parse(fs.readFileSync(new URL('./literal-search.json',import.meta.url),'utf8'));
const copy=structuredClone,plans=new WeakMap(),returnedResults=new WeakMap();
export function literalSearchInputForm(){return Object.freeze({family:catalog.family,contract:catalog.contract,fields:Object.freeze([...catalog.requestFields]),operations:Object.freeze([...catalog.variants]),profile:catalog.profile});}
const resultForm={statusOnly:['superseded','unavailable','exhausted','unsupported'],dataStatuses:['complete','partial'],fields:['status','context','ref','generation','coverage','matches','total'],matchFields:catalog.matchFields,navigation:{match:['kind','index','wrapped'],none:['kind'],stale:['kind'],unavailable:['kind']}};
export function literalSearchResultForms(){return freeze(copy(resultForm));}
const searchRefusal=status=>{assert(resultForm.statusOnly.includes(status),'undeclared search refusal');return {status};};
export function replacementInputForms(){return Object.freeze(catalog.replacementVariants.map(operation=>Object.freeze({family:catalog.replacementFamily,operation,variant:operation,fields:Object.freeze([...catalog.replacementFields])})));}
// Trusted metadata only: no traversal of unknown result objects or read grant.
export function replacementResultFootprint(result){return returnedResults.get(result)??null;}
export function discardLiteralSearchResult(result,g){
 if(!returnedResults.has(result)||!discardSearchResultCapture(result,g))return false;
 return returnedResults.delete(result);
}
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const bytes=s=>Array.from(Buffer.from(s,'utf8'));
const scalar=s=>typeof s==='string'&&new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(Uint8Array.from(bytes(s)))===s;
const boundary=(s,p)=>Number.isSafeInteger(p)&&p>=0&&p<=s.length&&(p===s.length||(s[p]&0xc0)!==0x80);
const releaseGates=['grant','scope','continuity','capacity','retained','notStopped'];
const unavailable=()=>searchRefusal('unavailable');
const validLimits=l=>l&&catalog.limits.every(k=>Number.isSafeInteger(l[k])&&l[k]>=0)&&l.chunkBytes>0;

export function searchLiteral(snapshot,request,current,limits){
  // Source/scopes are normalized engine-owned values. Current recipient/purpose,
  // destination and capture-ceiling truth are supplier premises, not selectors.
  if(!exact(request,catalog.requestFields)||request.contract!==catalog.contract||!catalog.variants.includes(request.operation)||
      !currentAt(request,current)||!validLimits(limits))return unavailable();
  if(request.profile!==catalog.profile)return searchRefusal('unsupported');
  if(!scalar(request.query)||!Array.isArray(request.scopes)||request.scopes.length>limits.maxScopes||
      !snapshot||snapshot.source.length>limits.maxRootBytes||bytes(request.query).length>limits.maxQueryBytes)return searchRefusal('exhausted');
  if(!equal(snapshot.ref,request.ref))return searchRefusal('superseded');
  const navigation=request.operation!=='literal-matches';
  if(navigation?(!exact(request.navigation,catalog.navigationFields)||!boundary(snapshot.source,request.navigation.start)||
      typeof request.navigation.wrap!=='boolean'):request.navigation!==null)return unavailable();
  const scopes=[];
  for(const r of request.scopes.toSorted((a,b)=>a.lo-b.lo||a.hi-b.hi)){
    if(!exact(r,['lo','hi'])||!boundary(snapshot.source,r.lo)||!boundary(snapshot.source,r.hi)||r.lo>r.hi)return unavailable();
    const last=scopes.at(-1);
    if(last&&r.lo<last.hi)last.hi=Math.max(last.hi,r.hi);else scopes.push(copy(r));
  }
  if(scopes.reduce((n,r)=>n+r.hi-r.lo,0)>limits.maxInputBytes)return searchRefusal('exhausted');
  const query=bytes(request.query),matches=[];let comparisons=0,complete=true;
  outer:for(const scope of scopes){
    const read=readRange(snapshot,{ref:request.ref,...scope},current);if(read.status!=='content')return unavailable();
    // Byte chunks may split scalars/CRLF/query text; offsets remain source bytes.
    const chunks=[];for(let i=0;i<read.source.length;i+=limits.chunkBytes)chunks.push(read.source.slice(i,i+limits.chunkBytes));
    const at=i=>chunks[Math.floor(i/limits.chunkBytes)][i%limits.chunkBytes];
    if(!query.length)continue;
    for(let i=0;i+query.length<=read.source.length;){
      let same=true;
      for(let j=0;j<query.length;j++){
        if(comparisons===limits.maxComparisons){complete=false;break outer;}comparisons++;
        if(at(i+j)!==query[j]){same=false;break;}
      }
      if(same){
        if(matches.length===limits.maxMatches){complete=false;break outer;}
        matches.push({lo:scope.lo+i,hi:scope.lo+i+query.length});i+=query.length;
      }else i++;
    }
  }
  const out={status:complete?'complete':'partial',context:request.context,ref:copy(request.ref),generation:request.generation,
    coverage:complete?'complete-within-scope':'known-subset',matches,total:complete?matches.length:null};
  if(navigation){
    // An incomplete set cannot establish predecessor/successor or wrap semantics.
    out.navigation=complete?navigate(matches.map(m=>m.lo),request.navigation.start,request.navigation.current,
      request.operation==='next-match'?'next':'previous',request.navigation.wrap):{kind:'unavailable'};
  }
  retainSearchResult(out,{request:copy(request),snapshot:copy(snapshot),limits:copy(limits)});
  for(let index=0;index<matches.length;index++)matches[index].reference=bindSearchOccurrence(out,index,current);
  assert(resultForm.dataStatuses.includes(out.status)&&exact(out,[...resultForm.fields,...(navigation?['navigation']:[])])&&out.matches.every(m=>exact(m,resultForm.matchFields)),'undeclared search reply shape');
  if(navigation)assert(Object.hasOwn(resultForm.navigation,out.navigation.kind)&&exact(out.navigation,resultForm.navigation[out.navigation.kind]),'undeclared search navigation');
  const replyBytes=Buffer.byteLength(JSON.stringify(out),'utf8');
  if(replyBytes>limits.maxReplyBytes)return searchRefusal('exhausted');
  freeze(out);returnedResults.set(out,Object.freeze({replyBytes}));return out;
}

export function prepareReplacement(result,request,current,state,checks){
  const capture=searchResultCapture(result);
  if(!capture||!exact(request,catalog.replacementFields)||!catalog.replacementVariants.includes(request.operation)||
      !currentAt(capture.request,current)||!scalar(request.replacement)||typeof request.invocation!=='string'||!request.invocation)return null;
  if(request.operation==='replace-all'&&(result.status!=='complete'||request.index!==null))return null;
  if(request.operation==='replace-current'&&(!Number.isSafeInteger(request.index)||request.index<0||request.index>=result.matches.length))return null;
  const matches=request.operation==='replace-all'?result.matches:[result.matches[request.index]],ins=bytes(request.replacement);
  if(ins.length>capture.limits.maxReplacementBytes||matches.reduce((n,m)=>n+m.hi-m.lo+ins.length,0)>capture.limits.maxEditBytes)return null;
  const edits=matches.map(m=>({lo:m.lo,hi:m.hi,expected:capture.snapshot.source.slice(m.lo,m.hi),ins:copy(ins)}));
  const r=capture.snapshot.ref,document={authority:r.authority,id:r.document,incarnation:r.incarnation};
  const viewPart=prepareForwardViews(state,document,r.revision,edits,checks.viewLimits,checks.viewWorkLimits);
  if(!viewPart)return null;
  const annotationSet=state.documents.find(d=>equal(d.document,document)).annotationSet;
  const plan=freeze({operation:request.operation,count:matches.length});
  plans.set(plan,{request:copy(request),capture,invocation:request.invocation,edits,viewPart,annotationSet});
  return plan;
}

export function replacementInvocationIntent(plan,maximum){
  const p=plans.get(plan);if(!p||!Number.isSafeInteger(maximum)||maximum<0)return null;
  const value={request:p.request,capture:{request:p.capture.request,snapshot:p.capture.snapshot,limits:p.capture.limits},edits:p.edits,viewPart:p.viewPart,annotationSet:p.annotationSet};
  return Buffer.byteLength(JSON.stringify(value),'utf8')<=maximum?{invocation:p.invocation,value:freeze(copy(value))}:null;
}
// Trusted protected owner cleanup, not a public replacement verb.
export function discardReplacement(plan,g){return ['ownerControlAllowed','retentionReleased','deliveryDrained'].every(k=>g?.[k]===true)&&plans.delete(plan);}
export function publishReplacement(state,plan,current,checks){
  const p=plans.get(plan),refuse=()=>({status:'not-published',state});
  if(!p||!currentAt(p.capture.request,current)||!checks.allMatchesWritable)return refuse();
  if(!validateReversalViews(state,p.viewPart,checks.viewLimits,checks.viewWorkLimits,checks.viewVersionsAvailable)||p.viewPart.viewChanged&&!checks.versionsAvailable)return refuse();
  const {snapshot}=p.capture,r=snapshot.ref,document={authority:r.authority,id:r.document,incarnation:r.incarnation};
  // The current annotation set is a publication dependency even for source-only edits.
  const published=publishAnnotationPlan(state,{contract:'owned-annotation-plan/1',invocation:p.invocation,descriptor:p.invocation,
    sources:[{document,expectedSource:r.revision,expectedAnnotationSet:p.annotationSet,edits:copy(p.edits)}],annotations:[]},checks);
  return completeForwardViews(state,p.viewPart,published,checks.viewLimits,checks.viewWorkLimits,checks.viewVersionsAvailable);
}

export function checkLiteralSearch(){
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  assert.deepEqual(catalog.variants.toSorted(),families.find(f=>f.id==='F05').variants.toSorted());
  assert(catalog.replacementVariants.every(v=>families.find(f=>f.id==='F11').variants.includes(v)));
  const limits={maxRootBytes:256,maxScopes:8,maxInputBytes:256,maxQueryBytes:32,maxComparisons:4096,maxMatches:64,maxReplyBytes:16384,
    maxReplacementBytes:32,maxEditBytes:1024,chunkBytes:1};
  const snapshot={ref:{authority:'a',document:'A',incarnation:1,revision:1},source:bytes('aa🙂aa\r\naa'),annotationSet:1};
  const request={contract:catalog.contract,operation:'literal-matches',context:'read-context',ref:copy(snapshot.ref),query:'aa',profile:catalog.profile,
    scopes:[{lo:0,hi:snapshot.source.length}],generation:1,navigation:null};
  const current={request:copy(request),...Object.fromEntries(releaseGates.map(k=>[k,true]))};
  const run=(s=snapshot,r=request,l=limits,c={...current,request:copy(r)})=>searchLiteral(s,r,c,l);
  const result=run(),controls=[],control=(name,condition)=>{assert(condition,name);controls.push(name);};
  const replacement={operation:'replace-all',index:null,replacement:'$1',invocation:'R'};
  const prepare=(out=result,r=replacement,c=current)=>prepareReplacement(out,r,c,state,checks);
  const state={authority:'a',publication:0,documents:[{document:{authority:'a',id:'A',incarnation:1},source:copy(snapshot.source),revision:1,annotationSet:1,records:[]}],groups:[],frontiers:{},views:[]};
  const checks={authorized:true,bounded:true,notStopped:true,interpretationCurrent:true,companionsReady:true,versionsAvailable:true,historyReady:true,
    maxParticipants:8,completeIdentity:true,targetRestorationValid:true,allMatchesWritable:true,viewVersionsAvailable:true,
    viewLimits:{maxRootBytes:256,maxInputViews:8,maxViews:8,maxInputSelections:8,maxSelections:8,maxSelectionBytes:2048,maxTextBytes:32,maxEditBytes:1024},
    viewWorkLimits:{maxViewBatches:16,maxViewEdits:64,maxViewBytes:32768,maxViewPlanBytes:32768}};
  let oracleCases=0,releaseCases=0,navigationCases=0;
  // Independent scalar-string oracle with independently accumulated byte offsets.
  const alphabet=['a','🙂','e\u0301','\r\n'];
  const texts=['','\uFEFF','aaaa','é e\u0301',...alphabet,...alphabet.flatMap(a=>alphabet.map(b=>a+b)),...alphabet.flatMap(a=>alphabet.flatMap(b=>alphabet.map(c=>a+b+c)))];
  for(const text of texts)for(const query of ['',...alphabet,'aa','🙂a','é','\n'])for(const chunkBytes of [1,2,3,5]){
    const source=bytes(text),scalars=[...text],positions=[0];for(const c of scalars)positions.push(positions.at(-1)+bytes(c).length);
    const expected=[];if(query)for(let i=0;i<scalars.length;){
      const tail=scalars.slice(i).join('');if(tail.startsWith(query)){expected.push({lo:positions[i],hi:positions[i]+bytes(query).length});i+=[...query].length;}else i++;
    }
    const out=run({...snapshot,source},{...request,query,scopes:[{lo:0,hi:source.length}]},{...limits,chunkBytes});
    assert.equal(out.status,'complete');assert.deepEqual(out.matches.map(({lo,hi})=>({lo,hi})),expected);assert.equal(out.total,expected.length);oracleCases++;
  }
  for(let mask=0;mask<64;mask++){
    const c={request:copy(request),...Object.fromEntries(releaseGates.map((k,i)=>[k,!!(mask&(1<<i))]))};
    assert.equal(run(snapshot,request,limits,c).status,mask===63?'complete':'unavailable');releaseCases++;
  }
  for(const operation of ['next-match','previous-match'])for(const currentIndex of [null,0,1,2])for(const start of [0,2,6,10])for(const wrap of [false,true]){
    const r={...request,operation,navigation:{start,current:currentIndex,wrap}},out=run(snapshot,r);
    assert.deepEqual(out.navigation,navigate([0,6,10],start,currentIndex,operation==='next-match'?'next':'previous',wrap));navigationCases++;
  }
  control('leftmost-nonoverlap',equal(run({...snapshot,source:bytes('aaaa')},{...request,scopes:[{lo:0,hi:4}]}).matches.map(({lo,hi})=>({lo,hi})),[{lo:0,hi:2},{lo:2,hi:4}]));
  control('no-normalization',run({...snapshot,source:bytes('é')},{...request,query:'e\u0301',scopes:[{lo:0,hi:2}]}).total===0);
  control('empty-query-no-matches',run(snapshot,{...request,query:''}).total===0);
  control('overlapping-scopes-normalized',run(snapshot,{...request,scopes:[{lo:0,hi:8},{lo:6,hi:12}]}).total===3);
  control('disjoint-scopes-not-joined',run({...snapshot,source:bytes('aXa')},{...request,scopes:[{lo:0,hi:1},{lo:2,hi:3}]}).total===0);
  control('adjacent-selection-scopes-kept',run({...snapshot,source:bytes('aa')},{...request,scopes:[{lo:0,hi:1},{lo:1,hi:2}]}).total===0);
  control('invalid-scalar-query',run(snapshot,{...request,query:'\ud800'}).status==='exhausted');
  control('invalid-boundary',run(snapshot,{...request,scopes:[{lo:3,hi:8}]}).status==='unavailable');
  control('stale-source',run(snapshot,{...request,ref:{...request.ref,revision:2}}).status==='superseded');
  control('unsupported-profile',run(snapshot,{...request,profile:'caseless'}).status==='unsupported');
  control('changed-generation',run(snapshot,request,limits,{...current,request:{...request,generation:2}}).status==='unavailable');
  for(const key of ['maxRootBytes','maxScopes','maxInputBytes','maxQueryBytes','maxReplyBytes'])control(`limit-${key}`,run(snapshot,request,{...limits,[key]:0}).status==='exhausted');
  const partial=run(snapshot,request,{...limits,maxMatches:1}),work=run(snapshot,request,{...limits,maxComparisons:0});
  control('partial-is-not-total',partial.status==='partial'&&partial.total===null&&partial.matches.length===1);
  control('work-budget-stops',work.status==='partial'&&work.total===null&&work.matches.length===0);
  control('exact-match-limit-completes',run(snapshot,request,{...limits,maxMatches:3}).status==='complete');
  control('partial-not-replace-all',prepare(partial)===null);
  control('copied-result-not-owned',prepare(copy(result))===null);
  control('copied-plan-not-owned',publishReplacement(state,copy(prepare()),current,checks).status==='not-published');
  control('stale-query-not-replaced',prepare(result,replacement,{...current,request:{...request,query:'x'}})===null);
  control('stale-scope-not-replaced',prepare(result,replacement,{...current,request:{...request,scopes:[]}})===null);
  control('stale-profile-not-replaced',prepare(result,replacement,{...current,request:{...request,profile:'other'}})===null);
  control('selected-index-required',prepare(result,{...replacement,operation:'replace-current',index:3})===null);
  control('literal-replacement-capacity',prepare(result,{...replacement,replacement:'x'.repeat(33)})===null);
  const plan=prepare(),before=copy(state),out=publishReplacement(state,plan,current,checks);
  control('one-publication-one-group',out.status==='applied'&&out.state.publication===1&&out.state.groups.length===1&&out.state.documents[0].revision===2);
  control('replacement-is-literal',equal(out.state.documents[0].source,bytes('$1🙂$1\r\n$1')));
  control('source-input-unchanged',equal(state,before));
  control('readonly-refuses-whole',publishReplacement(state,plan,current,{...checks,allMatchesWritable:false}).state===state);
  for(const key of ['authorized','bounded','notStopped','interpretationCurrent','companionsReady','versionsAvailable','historyReady'])
    control(`publication-${key}`,publishReplacement(state,plan,current,{...checks,[key]:false}).status==='not-published');
  const stale=copy(state);stale.documents[0].annotationSet++;
  control('annotation-dependency-current',publishReplacement(stale,plan,current,checks).status==='not-published');
  control('source-dependency-current',publishReplacement(out.state,plan,current,checks).status==='not-published');
  control('publication-rechecks-generation',publishReplacement(state,plan,{...current,request:{...request,generation:2}},checks).status==='not-published');
  const undo=reverseAnnotationGroup(out.state,'R','undo',checks),redo=reverseAnnotationGroup(undo.state,'R','redo',checks);
  control('actual-inverse-undo',undo.status==='applied'&&equal(undo.state.documents[0].source,snapshot.source));
  control('actual-forward-redo',redo.status==='applied'&&equal(redo.state.documents[0].source,out.state.documents[0].source));
  const same=publishReplacement(state,prepare(result,{...replacement,replacement:'aa'}),current,checks);
  control('equal-replacement-no-publication',same.status==='unchanged'&&same.state===state);
  const empty=run(snapshot,{...request,query:''}),emptyCurrent={...current,request:{...request,query:''}};
  control('empty-query-replacement-no-publication',publishReplacement(state,prepare(empty,replacement,emptyCurrent),emptyCurrent,checks).status==='unchanged');
  const one=publishReplacement(state,prepare(result,{...replacement,operation:'replace-current',index:1}),current,checks);
  control('replace-current-only-selected',one.status==='applied'&&equal(one.state.documents[0].source,bytes('aa🙂$1\r\naa')));
  const nr={...request,operation:'next-match',navigation:{start:0,current:null,wrap:true}};
  control('partial-navigation-honest',run(snapshot,nr,{...limits,maxMatches:1}).navigation.kind==='unavailable');
  const annotated=copy(state),target={kind:'range',start:2,end:6};
  annotated.documents[0].records=[{id:'note',incarnation:1,body:'authored',bodyVersion:1,target:{sourceRevision:1,target:copy(target)},targetGeneration:1,
    placement:{state:'attached',sourceRevision:1,target:copy(target)}}];
  const mapped=publishReplacement(annotated,prepare(result,{...replacement,replacement:'X'}),current,checks);
  control('replacement-maps-current-annotation',mapped.status==='applied'&&equal(mapped.state.documents[0].records[0].placement,
    {state:'attached',sourceRevision:2,target:{kind:'range',start:1,end:5}})&&mapped.state.documents[0].annotationSet===2);
  control('replacement-preserves-authored-body',mapped.state.documents[0].records[0].body==='authored'&&mapped.state.documents[0].records[0].targetGeneration===1);
  const mappedUndo=reverseAnnotationGroup(mapped.state,'R','undo',checks);
  control('replacement-undo-maps-not-restores-row',mappedUndo.status==='applied'&&equal(mappedUndo.state.documents[0].records[0].placement,
    {state:'attached',sourceRevision:3,target:copy(target)}));
  const overlapping=copy(annotated);overlapping.documents[0].records[0].placement.target={kind:'range',start:0,end:2};
  overlapping.documents[0].records[0].target.target=copy(overlapping.documents[0].records[0].placement.target);
  control('equal-byte-nonidentity-companion-published',publishReplacement(overlapping,prepare(result,{...replacement,replacement:'aa'}),current,checks).status==='applied');
  const rlimit=run(snapshot,request,{...limits,maxEditBytes:1});
  control('aggregate-edit-byte-limit',prepare(rlimit)===null);
  control('stale-current-navigation-identity',run(snapshot,{...nr,navigation:{...nr.navigation,current:3}}).navigation.kind==='stale');
  return {status:'literal-search-products-only',oracleCases,releaseCases,navigationCases,controls,...catalog.scope};
}

export async function checkSearchMutations(){
  const source=fs.readFileSync(new URL('./literal-search.mjs',import.meta.url),'utf8');
  const occurrenceSource=fs.readFileSync(new URL('./search-occurrences.mjs',import.meta.url),'utf8');
  const mutants=[
    ['allow-overlapping-occurrences','i+=query.length','i++'],
    ['join-adjacent-selections','r.lo<last.hi','r.lo<=last.hi'],
    ['partial-replacement','result.status!==\'complete\'','false'],
    ['skip-readonly','!checks.allMatchesWritable','false'],
    ['omit-annotation-dependency','expectedAnnotationSet:p.annotationSet','expectedAnnotationSet:state.documents[0].annotationSet'],
    ['ignore-request-generation','equal(current.request,request)',"equal({...current.request,generation:0},{...request,generation:0})"],
    ['replace-current-widens',"request.operation==='replace-all'?result.matches:[result.matches[request.index]]",'result.matches']
  ];
  const killed=[];
  for(const [name,from,to] of mutants){
    const occurrenceMutation=name==='ignore-request-generation';
    assert((occurrenceMutation?occurrenceSource:source).includes(from),name);
    let code=occurrenceMutation?source:source.replace(from,to),occ=occurrenceMutation?occurrenceSource.replace(from,to):occurrenceSource;
    const occurrenceURL=`data:text/javascript;base64,${Buffer.from(occ).toString('base64')}#${name}`;
    code=code.replace("'./search-occurrences.mjs'",JSON.stringify(occurrenceURL));
    for(const file of ['source-edit-contracts.mjs','baseline-contracts.mjs','annotation-compounds.mjs','history-views.mjs'])code=code.replace(`'./${file}'`,JSON.stringify(new URL(file,import.meta.url).href));
    for(const file of ['literal-search.json','operation-families.json'])code=code.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(file,import.meta.url).href)})`);
    const module=await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`);
    let failed=false;try{module.checkLiteralSearch();}catch(e){assert.equal(e.code,'ERR_ASSERTION',name);failed=true;}
    assert(failed,name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkLiteralSearch(),mutations:await checkSearchMutations()},null,2));
