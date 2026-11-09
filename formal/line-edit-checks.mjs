import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import * as original from './line-edits.mjs';
import {normalizeSelection} from './selection-products.mjs';
import {prepareReversal,publishReversal} from './history-reversal.mjs';
const copy=structuredClone,bytes=s=>Array.from(Buffer.from(s,'utf8')),text=s=>Buffer.from(s).toString('utf8');
export function checkLineEdits(api=original){
  const catalog=JSON.parse(fs.readFileSync(new URL('./line-edits.json',import.meta.url),'utf8'));
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  assert(families.find(f=>f.id===catalog.family&&f.kind==='localCommand')?.variants.includes(catalog.variant));
  const document={authority:'a',id:'A',incarnation:1},view=id=>({authority:'a',id,incarnation:1});
  const limits={maxRootBytes:512,maxInputViews:8,maxViews:8,maxInputSelections:8,maxSelections:8,maxSelectionBytes:4096,maxTextBytes:64,maxEditBytes:2048};
  const work={maxLines:16,maxPrefixBytes:256,maxIndentColumns:64,maxUnitBytes:8,maxDerivedBytes:8192};
  const gates=['authorized','sourceReadable','viewWritable','continuous','live','bounded','notStopped','interpretationCurrent','companionsReady'];
  const good={...Object.fromEntries(gates.map(k=>[k,true])),sourceWritable:true,allTargetsWritable:true,versionsAvailable:true,viewVersionsAvailable:true,
    historyReady:true,completeIdentity:true,maxParticipants:8,targetRestorationValid:true};
  const sel=(anchor,head=anchor,affinity='after')=>({anchor,head,affinity});
  function fixture(source,selections=[sel(0)]){
    const data=bytes(source),n=normalizeSelection(data,selections,0,limits);assert(n);
    return {authority:'a',publication:0,documents:[{document,source:data,revision:1,annotationSet:1,records:[]}],groups:[],frontiers:{},views:[
      {view:view('origin'),document,sourceRevision:1,selectionGeneration:1,profile:'explicit-positional-selection/1',...n,viewport:3},
      {view:view('peer'),document,sourceRevision:1,selectionGeneration:1,profile:'explicit-positional-selection/1',selections:[sel(data.length)],primary:0,viewport:17}]};
  }
  const request=(action,settings)=>({contract:'line-edit/1',operation:'ordinary-edit',action,invocation:'line-command',view:view('origin'),document,
    sourceRevision:1,expectedSelection:1,profile:'explicit-prefix-line-edit/1',settings});
  const settings={unit:'  ',tabWidth:4},newline={delimiter:'CRLF',autoIndent:'copy-prefix'};
  const g=r=>({...good,lineSettings:copy(r.settings)});
  const prepare=(s,r,h=g(r),l=limits,w=work)=>api.prepareLineEdit(s,r,h,l,w);
  const run=(s,r,h=g(r),l=limits,w=work)=>api.publishLineEdit(s,prepare(s,r,h,l,w),h,l,w);
  const controls=[],control=(name,ok)=>{assert(ok,name);controls.push(name);};
  let prefixCases=0,newlineCases=0,gateCases=0;
  const prefixes=['',' ','  ','\t',' \t','\t ','  \t ','\t\t'];
  // Independent string oracle: expand tabs to spaces, then measure string length.
  for(const prefix of prefixes)for(const tabWidth of [1,2,4,8])for(const unit of [' ','  ','    ','\t'])for(const action of ['indent-lines','outdent-lines']){
    const r=request(action,{unit,tabWidth}),s=fixture(prefix+'é\r\nTAIL'),out=run(s,r);
    let expanded='';for(const c of prefix)expanded+=c==='\t'?' '.repeat(tabWidth-expanded.length%tabWidth):c;
    const target=Math.max(0,expanded.length-(unit==='\t'?tabWidth:unit.length));
    const desired=action==='indent-lines'?unit+prefix:unit==='\t'?'\t'.repeat(Math.floor(target/tabWidth))+' '.repeat(target%tabWidth):' '.repeat(target);
    assert(['applied','unchanged'].includes(out.status));assert.equal(text(out.state.documents[0].source),desired+'é\r\nTAIL');
    assert.equal(out.state.views[1].sourceRevision,out.state.documents[0].revision);assert.equal(out.state.views[1].viewport,17);prefixCases++;
  }
  for(const prefix of prefixes)for(const delimiter of ['LF','CRLF','CR'])for(const autoIndent of ['none','copy-prefix'])for(const reversed of [false,true]){
    const source=prefix+'éX\r\nEND',lo=bytes(prefix+'é').length,hi=lo+1;
    const s=fixture(source,[reversed?sel(hi,lo):sel(lo,hi)]),r=request('insert-newline',{delimiter,autoIndent}),out=run(s,r);
    const insertion=({LF:'\n',CRLF:'\r\n',CR:'\r'})[delimiter]+(autoIndent==='copy-prefix'?prefix:'');
    assert.equal(out.status,'applied');assert.equal(text(out.state.documents[0].source),prefix+'é'+insertion+'\r\nEND');
    assert.equal(out.state.views[0].selections[0].head,lo+bytes(insertion).length);assert.equal(out.state.groups.length,1);newlineCases++;
  }
  const s=fixture(' a\r\n\tb\rc\nEND',[sel(0,4)]),r=request('indent-lines',settings),h=g(r),plan=prepare(s,r),out=api.publishLineEdit(s,plan,h,limits,work);
  for(let mask=0;mask<512;mask++){
    const now={...h,...Object.fromEntries(gates.map((k,i)=>[k,!!(mask&(1<<i))]))};
    assert.equal(api.publishLineEdit(s,plan,now,limits,work).status,mask===511?'applied':'not-published');gateCases++;
  }
  control('half-open-upper-line-excluded',text(out.state.documents[0].source)==='   a\r\n\tb\rc\nEND');
  const overlap=fixture('a\nb\nc',[sel(0,3),sel(2,4)]),dedup=run(overlap,r);
  control('each-target-line-once',text(dedup.state.documents[0].source)==='  a\n  b\nc');
  const carets=fixture('a\nb',[sel(0),sel(1)]);
  control('separate-carets-same-line-dedup',text(run(carets,r).state.documents[0].source)==='  a\nb');
  const eof=fixture('a\n',[sel(2)]);
  control('eof-empty-line-included',text(run(eof,r).state.documents[0].source)==='a\n  ');
  const reverse=fixture('a\nb\nc',[sel(4,0)]);
  control('reverse-range-same-lines',text(run(reverse,r).state.documents[0].source)==='  a\n  b\nc');
  const nr=request('insert-newline',newline),mid=fixture('   body',[sel(1)]);
  control('copy-prefix-stops-at-insertion',text(run(mid,nr).state.documents[0].source)===' \r\n   body');
  control('newlines-do-not-normalize-others',text(run(fixture('a\nb\rc\r\nd',[sel(1)]),nr).state.documents[0].source)==='a\r\n\nb\rc\r\nd');
  control('ascii-prefix-only',text(run(fixture('\u00a0body',[sel(2)]),nr).state.documents[0].source)==='\u00a0\r\nbody');
  control('outdent-empty-is-identity',run(fixture('body'),request('outdent-lines',settings)).status==='unchanged');
  control('tabs-use-next-stop',text(run(fixture(' \tbody'),request('outdent-lines',{unit:' ',tabWidth:4})).state.documents[0].source)==='   body');
  control('prefix-shift-tab-jump-bound',prepare(fixture('   \tbody'),r,g(r),limits,{...work,maxIndentColumns:6})===null);
  control('existing-unit-bytes-preserved',text(run(fixture('\t body'),r).state.documents[0].source)==='  \t body');
  const protectedPrefix=fixture('   body'),record={id:'indent-note',incarnation:1,body:'keep',bodyVersion:1,target:{sourceRevision:1,target:{kind:'range',start:0,end:1}},targetGeneration:1,placement:{state:'attached',sourceRevision:1,target:{kind:'range',start:0,end:1}}};
  protectedPrefix.documents[0].records=[record];const preserved=run(protectedPrefix,request('outdent-lines',{unit:' ',tabWidth:4}));
  control('unchanged-leading-byte-not-replaced',preserved.status==='applied'&&preserved.state.documents[0].records[0].placement.state==='attached'&&preserved.state.documents[0].records[0].placement.target.end===1);
  control('one-publication-and-history-group',out.status==='applied'&&out.state.publication===1&&out.state.groups.length===1);
  control('all-view-source-contexts-current',out.state.views.every(v=>v.sourceRevision===2)&&out.state.views[1].viewport===17);
  control('input-root-preserved',s.publication===0&&text(s.documents[0].source)===' a\r\n\tb\rc\nEND');
  control('copied-plan-not-owned',api.publishLineEdit(s,copy(plan),h,limits,work).status==='not-published');
  control('changed-setting-final-refuses',api.publishLineEdit(s,plan,{...h,lineSettings:{unit:'    ',tabWidth:4}},limits,work).state===s);
  control('changed-setting-prepare-refuses',prepare(s,r,{...h,lineSettings:{...settings,tabWidth:8}})===null);
  control('readonly-whole-refusal',run(s,r,{...h,allTargetsWritable:false}).state===s);
  const stale=copy(s);stale.views[1].selectionGeneration++;
  control('peer-selection-stales-plan',api.publishLineEdit(stale,plan,h,limits,work).state===stale);
  const added=copy(s);added.views.push({...copy(added.views[1]),view:view('new')});
  control('new-peer-stales-plan',api.publishLineEdit(added,plan,h,limits,work).state===added);
  const annotated=copy(s);annotated.documents[0].annotationSet++;
  control('annotation-set-stales-plan',api.publishLineEdit(annotated,plan,h,limits,work).state===annotated);
  const viewport=copy(s);viewport.views[1].viewport=99;
  control('current-viewport-preserved',api.publishLineEdit(viewport,plan,h,limits,work).state.views[1].viewport===99);
  control('crlf-interior-refused',prepare(fixture('a\r\nb',[sel(2)]),r)===null);
  const unknown=copy(s);delete unknown.views;
  control('unknown-inventory-prepare-refused',prepare(unknown,r)===null);
  control('unknown-inventory-final-refused',api.publishLineEdit(unknown,plan,h,limits,work).state===unknown);
  const multiple=run(fixture(' a\n\tb',[sel(2),sel(5)]),nr);
  control('per-selection-prefix-and-caret-after',text(multiple.state.documents[0].source)===' a\r\n \n\tb\r\n\t'&&equal(multiple.state.views[0].selections.map(s=>s.head),[5,11])&&multiple.state.groups.length===1);
  control('unresolved-newline-insertion-tie-refused',prepare(fixture('ab',[sel(1),sel(1,1,'before')]),nr)===null);
  for(const settings of [{unit:'',tabWidth:4},{unit:' \t',tabWidth:4},{unit:'x',tabWidth:4},{unit:' ',tabWidth:0},{unit:' ',tabWidth:1.5}])
    control(`invalid-settings-${JSON.stringify(settings)}`,prepare(s,request('indent-lines',settings))===null);
  control('invalid-delimiter-refused',prepare(s,request('insert-newline',{delimiter:'auto',autoIndent:'none'}))===null);
  control('grammar-mode-not-guessed',prepare(s,request('insert-newline',{delimiter:'LF',autoIndent:'syntax'}))===null);
  control('extra-field-refused',prepare(s,{...r,extra:true})===null);
  control('unsupported-profile-refused',prepare(s,{...r,profile:'smart-indent'})===null);
  for(const key of ['maxLines','maxPrefixBytes','maxIndentColumns','maxUnitBytes','maxDerivedBytes']){
    control(`prepare-limit-${key}`,prepare(s,r,h,limits,{...work,[key]:0})===null);
    control(`current-limit-${key}`,api.publishLineEdit(s,plan,h,limits,{...work,[key]:0}).state===s);
  }
  control('selection-limit-profile-stale',api.publishLineEdit(s,plan,h,{...limits,maxRootBytes:511},work).state===s);
  const viewLimits=limits,viewWorkLimits={maxViewBatches:32,maxViewEdits:128,maxViewBytes:16384,maxViewPlanBytes:65536};
  const hg={...h,viewLimits,limits:{...viewWorkLimits,maxDependencies:8,maxParticipants:8,maxChanges:32,maxPlanBytes:131072},invocationFresh:true,
    owner:{},recipient:'reviewer',profile:'history/1',requiredDependencies:[],contexts:[],groups:[{reference:'line-command',internal:'line-command',authority:'a'}],scopes:[]};
  const hp=prepareReversal(out.state,{contract:'history-reversal/1',operation:'undo',authority:'a',target:{kind:'group',group:'line-command'},
    invocation:'undo-line',profile:hg.profile,dependencies:[]},hg);
  const undo=publishReversal(out.state,hp,hg);
  control('actual-native-undo-restores-source',undo.status==='applied'&&equal(undo.state.documents[0].source,s.documents[0].source)&&undo.state.views.every(v=>v.sourceRevision===3));
  const redoRequest={...hp.request,operation:'redo',invocation:'redo-line'},redo=publishReversal(undo.state,prepareReversal(undo.state,redoRequest,hg),hg);
  control('actual-native-redo-restores-result',redo.status==='applied'&&equal(redo.state.documents[0].source,out.state.documents[0].source)&&redo.state.views.every(v=>v.sourceRevision===4));
  control('caller-settings-not-aliased',(()=>{const r=request('indent-lines',copy(settings)),h=g(r),p=prepare(s,r,h);r.settings.unit='    ';return api.publishLineEdit(s,p,h,limits,work).status==='applied';})());
  control('source-version-stales-plan',api.publishLineEdit(out.state,plan,h,limits,work).state===out.state);
  control('source-version-capacity-required',api.publishLineEdit(s,plan,{...h,versionsAvailable:false},limits,work).state===s);
  control('view-version-capacity-required',api.publishLineEdit(s,plan,{...h,viewVersionsAvailable:false},limits,work).state===s);
  return {status:'simple-line-edit-products-only',prefixCases,newlineCases,gateCases,controls,...JSON.parse(fs.readFileSync(new URL('./line-edits.json',import.meta.url),'utf8')).scope};
}
export async function checkLineMutations(){
  const source=fs.readFileSync(new URL('./line-edits.mjs',import.meta.url),'utf8'),killed=[];
  const mutations=[
    ['skip-current-settings','!equal(p.settings,g.lineSettings)','false'],
    ['include-upper-line','hi>lo&&hi===lines[to.line].start','false'],
    ['shift-lines-twice','[...selected].sort((a,b)=>a-b)','[...selected,...selected].sort((a,b)=>a-b)'],
    ['ignore-tabs','width-col%width','width'],
    ['linear-indent-column-bound','column([...unit,...source.slice(row.start,end)],0,unit.length+size,s.tabWidth)','current+step'],
    ['copy-past-caret','Math.min(row.bodyEnd,lo)','row.bodyEnd'],
    ['force-lf','...catalog.delimiters[request.settings.delimiter]','10'],
    ['replace-whole-prefix','let keep=0;while(keep<size&&keep<replacement.length&&source[row.start+keep]===replacement[keep])keep++;','let keep=0;'],
    ['skip-derived-quota','charges.derivedBytes>workLimits.maxDerivedBytes','false'],
    ['skip-current-unit-quota','p.charges.unitBytes>workLimits.maxUnitBytes','false'],
    ['no-publication','return publishSelectionEdit(state,p.core,g);',"return {status:'unchanged',state};"]
  ];
  for(const [name,from,to] of mutations){
    assert(source.includes(from),name);let code=source.replace(from,to);
    for(const file of ['reference-profile.mjs','selection-products.mjs'])code=code.replace(`'./${file}'`,JSON.stringify(new URL(file,import.meta.url).href));
    for(const file of ['line-edits.json','selection-products.json'])code=code.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(file,import.meta.url).href)})`);
    const api=await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`);
    let failed=false;try{checkLineEdits(api);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name);failed=true;}
    assert(failed,name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkLineEdits(),mutations:await checkLineMutations()},null,2));
