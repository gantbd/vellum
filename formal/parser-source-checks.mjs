import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import * as original from './parser-source.mjs';
import * as compound from './annotation-compounds.mjs';
import * as selection from './selection-products.mjs';
import * as history from './history-reversal.mjs';
import {byteToLine} from './reference-profile.mjs';
const copy=structuredClone,bytes=s=>Array.from(Buffer.from(s,'utf8'));
const document={authority:'a',id:'A',incarnation:1},other={...document,id:'B'},view={authority:'a',id:'origin',incarnation:1};
const journalLimits={maxDocuments:4,maxRootBytes:2048,maxBatches:32,maxEdits:64,maxJournalBytes:65536};
const limits={maxRootBytes:2048,maxReceipts:16,maxBatches:32,maxEdits:64,maxCoordinate:4294967295,maxPlanBytes:131072};
const gates={authorized:true,sourceReadable:true,continuous:true,live:true,notStopped:true,captureReserved:true};
const host={authorized:true,bounded:true,notStopped:true,interpretationCurrent:true,companionsReady:true,versionsAvailable:true,historyReady:true,
  completeIdentity:true,targetRestorationValid:true,maxParticipants:8,parserSourceLimits:journalLimits,parserSourceReserved:true};
const initial=text=>({authority:'a',publication:0,documents:[{document:copy(document),revision:1,source:bytes(text),annotationSet:1,records:[]},
  {document:copy(other),revision:1,source:bytes('peer'),annotationSet:1,records:[]}],groups:[],frontiers:{},views:[]});
const snapshot=(root,doc=document)=>{const d=root.documents.find(d=>equal(d.document,doc));return {document:copy(d.document),revision:d.revision,source:copy(d.source)};};
const request=(root,edits,id='G',doc=document)=>{const d=root.documents.find(d=>equal(d.document,doc));return {contract:'owned-annotation-plan/1',invocation:id,descriptor:id,sources:[{document:copy(doc),expectedSource:d.revision,expectedAnnotationSet:d.annotationSet,
  edits:edits.map(e=>({...e,expected:d.source.slice(e.lo,e.hi)}))}],annotations:[]};};
const commit=(c,root,edits,id='G',h=host)=>c.publishAnnotationPlan(root,request(root,edits,id),h);
const pointOracle=(s,p)=>{const parts=Buffer.from(s.slice(0,p)).toString('utf8').split('\n');return {row:parts.length-1,column:Buffer.byteLength(parts.at(-1),'utf8')};};
function* words(n,prefix=''){yield prefix;if(n)for(const c of ['a','é','🙂','\r','\n'])yield* words(n-1,prefix+c);}
const positions=s=>{let p=0;return [0,...[...s].map(c=>(p+=bytes(c).length))];};
export function checkParserSource(api=original,c=compound,s=selection,h=history,mutant=false){
  const catalog=JSON.parse(fs.readFileSync(new URL('./parser-source.json',import.meta.url),'utf8'));
  let pointCases=0,editCases=0,multiEditCases=0,chainCases=0,gateCases=0;
  for(const text of mutant?['\r\né🙂\n']:words(3)){
    const source=bytes(text),valid=positions(text);
    for(let p=0;p<=source.length;p++){
      const out=api.parserPoint(source,p);if(valid.includes(p)){assert.deepEqual(out,pointOracle(source,p));assert.equal(api.parserByte(source,out),p);}else assert.equal(out,null);pointCases++;
    }
  }
  for(const text of mutant?['a\r\né🙂']:words(2))for(const lo of positions(text))for(const hi of positions(text).filter(p=>p>=lo))for(const insert of ['','X','é','🙂','\r\n','\uFEFF']){
    const root=initial(text),before=snapshot(root),ins=bytes(insert),out=commit(c,root,[{lo,hi,ins}]);
    assert(['applied','unchanged'].includes(out.status));const target=snapshot(out.state),refs=api.committedSourceTransforms(out.state),p=api.prepareParserSource(before,target,refs,gates,limits);assert(p);
    if(equal(before.source,target.source)){assert.equal(refs.length,0);assert.equal(p.mode,'unchanged');}
    else{
      assert.equal(refs.length,1);assert.equal(p.mode,'incremental');assert.deepEqual(Object.keys(p.edits[0]).sort(),catalog.editFields.toSorted());
      const start=pointOracle(before.source,lo),parts=insert.split('\n'),end={row:start.row+parts.length-1,column:parts.length===1?start.column+ins.length:Buffer.byteLength(parts.at(-1),'utf8')};
      assert.deepEqual(p.edits,[{start_byte:lo,old_end_byte:hi,new_end_byte:lo+ins.length,start_point:start,old_end_point:pointOracle(before.source,hi),new_end_point:end}]);
      assert.deepEqual(p.source,[...before.source.slice(0,lo),...ins,...before.source.slice(hi)]);
    }editCases++;
  }
  if(!mutant)for(const text of words(3)){
    const ps=positions(text);
    for(let i=0;i<ps.length;i++)for(let j=i;j<ps.length;j++)for(let k=j;k<ps.length;k++)for(let m=k;m<ps.length;m++){
      if(j===k&&(i===j||k===m))continue;
      for(const x of ['','X','🙂\n'])for(const y of ['','é','\r\n']){
        const root=initial(text),batch=[{lo:ps[i],hi:ps[j],ins:bytes(x)},{lo:ps[k],hi:ps[m],ins:bytes(y)}],o=commit(c,root,batch);assert(['applied','unchanged'].includes(o.status));
        const p=api.prepareParserSource(snapshot(root),snapshot(o.state),api.committedSourceTransforms(o.state),gates,limits);assert(p);
        if(o.status==='unchanged')assert.equal(p.mode,'unchanged');
        else{
          assert.equal(p.mode,'incremental');const source=snapshot(root).source,expected=[];
          for(const e of [...batch].reverse()){
            const start=pointOracle(source,e.lo),parts=Buffer.from(e.ins).toString('utf8').split('\n');
            expected.push({start_byte:e.lo,old_end_byte:e.hi,new_end_byte:e.lo+e.ins.length,start_point:start,old_end_point:pointOracle(source,e.hi),
              new_end_point:{row:start.row+parts.length-1,column:parts.length===1?start.column+e.ins.length:Buffer.byteLength(parts.at(-1),'utf8')}});
          }
          assert.deepEqual(p.edits,expected);
        }multiEditCases++;
      }
    }
  }
  for(const a of ['X','\n','é'])for(const b of ['Y','\r\n','🙂'])for(const d of ['Z','\r','\uFEFF']){
    const root=initial('abc'),one=commit(c,root,[{lo:0,hi:1,ins:bytes(a)}],'one'),two=commit(c,one.state,[{lo:bytes(a).length,hi:bytes(a).length,ins:bytes(b)}],'two'),three=commit(c,two.state,[{lo:0,hi:0,ins:bytes(d)}],'three');
    assert.equal(three.status,'applied');const chain=[one,two,three].flatMap(o=>api.committedSourceTransforms(o.state));
    const out=api.prepareParserSource(snapshot(root),snapshot(three.state),chain,gates,limits);assert.equal(out.mode,'incremental');assert.equal(out.edits.length,3);assert.deepEqual(out.source,snapshot(three.state).source);chainCases++;
  }
  const root=initial('a\r\né🙂\n'),before=snapshot(root),edits=[{lo:0,hi:1,ins:bytes('X\n')},{lo:5,hi:9,ins:bytes('Z')}],out=commit(c,root,edits);assert.equal(out.status,'applied');
  const target=snapshot(out.state),refs=api.committedSourceTransforms(out.state),plan=api.prepareParserSource(before,target,refs,gates,limits);assert.equal(plan.mode,'incremental');
  const controls=[],control=(name,ok)=>{assert(ok,name);controls.push(name);};
  control('batch-is-applied-high-to-low',plan.edits[0].start_byte===5&&plan.edits[1].start_byte===0);
  control('parser-lf-only-not-editor-line-profile',equal(api.parserPoint(bytes('a\rb'),2),{row:0,column:2})&&equal(byteToLine(bytes('a\rb'),2),{line:1,column:0}));
  control('parser-represents-crlf-interior',equal(api.parserPoint(bytes('\r\n'),1),{row:0,column:1})&&byteToLine(bytes('\r\n'),1)===null);
  control('point-does-not-use-utf16-columns',equal(api.parserPoint(bytes('🙂é'),6),{row:0,column:6}));
  control('point-coordinate-cap-refuses',api.parserPoint(bytes('abcd'),4,3)===null&&api.parserPoint([],0,4294967296)===null);
  control('point-inverse-no-cross-line-column',api.parserByte(bytes('a\nb'),{row:0,column:2})===null);
  control('point-inverse-no-scalar-interior',api.parserByte(bytes('🙂'),{row:0,column:1})===null);
  control('point-inverse-no-unknown-field',api.parserByte([],{row:0,column:0,extra:0})===null);
  control('published-root-has-owned-receipt',refs.length===1&&api.committedSourceTransforms(root).length===0);
  control('copied-root-does-not-forge-receipt',api.committedSourceTransforms(copy(out.state)).length===0);
  control('copied-receipt-is-not-owned',api.prepareParserSource(before,target,refs.map(r=>copy(r)),gates,limits).mode==='fresh');
  control('missing-chain-fresh-not-guessed-diff',api.prepareParserSource(before,target,[],gates,limits).mode==='fresh');
  control('duplicate-chain-falls-back',api.prepareParserSource(before,target,[...refs,...refs],gates,limits).mode==='fresh');
  control('fresh-source-exactly-preserved',equal(api.prepareParserSource(null,target,[],gates,limits).source,target.source));
  control('source-only-identity-preserves-version',api.prepareParserSource(before,before,[],gates,limits).mode==='unchanged');
  control('same-version-different-bytes-is-not-identity',api.prepareParserSource(before,{...before,source:bytes('different')},[],gates,limits)===null);
  control('invalid-utf8-target-refuses',api.prepareParserSource(null,{...target,source:[192,128]},[],gates,limits)===null);
  control('unknown-snapshot-field-refuses',api.prepareParserSource(null,{...target,extra:true},[],gates,limits)===null);
  control('missing-input-profile-coordinate-refuses',(()=>{const l=copy(limits);delete l.maxReceipts;return api.prepareParserSource(before,target,refs,gates,l)===null;})());
  control('foreign-base-is-fresh-not-cross-document-edit',api.prepareParserSource({...before,document:other},target,refs,gates,limits).mode==='fresh');
  control('future-base-is-fresh',api.prepareParserSource({...before,revision:9},target,refs,gates,limits).mode==='fresh');
  const unreserved=commit(c,root,edits,'no-reservation',{...host,parserSourceReserved:false});
  control('optional-journal-failure-does-not-stop-source-edit',unreserved.status==='applied'&&api.committedSourceTransforms(unreserved.state).length===0&&api.prepareParserSource(before,snapshot(unreserved.state),[],gates,limits).mode==='fresh');
  const denied=c.publishAnnotationPlan(root,request(root,edits),{...host,authorized:false});
  control('refused-source-has-no-new-receipt',denied.status==='not-published'&&api.committedSourceTransforms(denied.state).length===0);
  const undo=c.reverseAnnotationGroup(out.state,'G','undo',host),redo=c.reverseAnnotationGroup(undo.state,'G','redo',host);assert.equal(undo.status,'applied');assert.equal(redo.status,'applied');
  const chain=[...refs,...api.committedSourceTransforms(undo.state)];
  const round=api.prepareParserSource(before,snapshot(undo.state),chain,gates,limits);
  control('undo-to-equal-bytes-is-fresh-revision-not-identity',round.mode==='incremental'&&round.toRevision===3&&round.edits.length===4&&equal(round.source,before.source));
  control('redo-is-current-committed-transform',api.prepareParserSource(snapshot(undo.state),snapshot(redo.state),api.committedSourceTransforms(redo.state),gates,limits).mode==='incremental');
  control('reordered-source-receipts-do-not-coalesce',api.prepareParserSource(before,snapshot(undo.state),[...chain].reverse(),gates,limits).mode==='fresh');
  control('equal-byte-new-revision-without-journal-is-fresh',api.prepareParserSource(before,snapshot(undo.state),[],gates,limits).mode==='fresh');
  control('historical-loop-is-not-current-committed-chain',api.prepareParserSource(snapshot(undo.state),snapshot(redo.state),[...chain,...api.committedSourceTransforms(redo.state)],gates,limits).mode==='fresh');
  control('chain-cannot-substitute-target-bytes',api.prepareParserSource(before,{...target,source:bytes('different')},refs,gates,limits).mode==='fresh');
  for(let mask=0;mask<64;mask++){
    const g=Object.fromEntries(Object.keys(gates).map((k,i)=>[k,!!(mask&(1<<i))]));assert.equal(!!api.prepareParserSource(before,target,refs,g,limits),mask===63);gateCases++;
  }
  for(const key of ['maxReceipts','maxBatches','maxEdits'])control(`incremental-${key}-degrades-to-fresh`,api.prepareParserSource(before,target,refs,gates,{...limits,[key]:0}).mode==='fresh');
  for(const key of ['maxRootBytes','maxCoordinate','maxPlanBytes'])control(`input-${key}-refuses`,api.prepareParserSource(before,target,refs,gates,{...limits,[key]:0})===null);
  for(const key of Object.keys(journalLimits)){
    const applied=commit(c,root,edits,`limited-${key}`,{...host,parserSourceLimits:{...journalLimits,[key]:0}});
    control(`journal-${key}-does-not-block-edit`,applied.status==='applied'&&api.committedSourceTransforms(applied.state).length===0);
  }
  const mult=initial('ab'),r=request(mult,[{lo:0,hi:1,ins:bytes('x')}]);r.sources.push({...request(mult,[{lo:0,hi:1,ins:bytes('y')}],'G',other).sources[0]});
  const mo=c.publishAnnotationPlan(mult,r,host);assert.equal(mo.status,'applied');const mr=api.committedSourceTransforms(mo.state);
  control('multi-document-publication-keeps-separate-receipts',mr.length===2&&api.prepareParserSource(snapshot(mult),snapshot(mo.state),[mr[0]],gates,limits).mode==='incremental'&&api.prepareParserSource(snapshot(mult,other),snapshot(mo.state,other),[mr[1]],gates,limits).mode==='incremental');
  control('other-document-receipt-is-not-current-chain',api.prepareParserSource(snapshot(mult),snapshot(mo.state),[mr[1]],gates,limits).mode==='fresh');
  // Actual F11 and F14 wrappers must carry receipts through their private view roots.
  const viewed=initial('abcd');viewed.views=[{view:copy(view),document:copy(document),sourceRevision:1,selectionGeneration:1,profile:'explicit-positional-selection/1',selections:[{anchor:1,head:2,affinity:'after'}],primary:0,viewport:7}];
  const vl={maxRootBytes:2048,maxInputViews:8,maxViews:8,maxInputSelections:8,maxSelections:8,maxSelectionBytes:4096,maxTextBytes:128,maxEditBytes:512};
  const sg={...host,sourceReadable:true,viewWritable:true,continuous:true,live:true,sourceWritable:true,allTargetsWritable:true,viewVersionsAvailable:true};
  const sr={contract:'selection-edit/1',operation:'ordinary-edit',invocation:'ordinary',document:copy(document),view:copy(view),sourceRevision:1,expectedSelection:1,profile:'explicit-positional-selection/1',text:'X\n',selectionAfter:'carets-after'};
  const sp=s.prepareSelectionEdit(viewed,sr,sg,vl);assert(sp);const so=s.publishSelectionEdit(viewed,sp,sg);assert.equal(so.status,'applied');
  control('ordinary-view-publication-retains-actual-journal',api.committedSourceTransforms(so.state).length===1&&api.prepareParserSource(snapshot(viewed),snapshot(so.state),api.committedSourceTransforms(so.state),gates,limits).mode==='incremental');
  const hh={...sg,owner:{},recipient:'caller',profile:'history',retentionReady:true,viewLimits:vl,
    limits:{maxDependencies:8,maxParticipants:8,maxChanges:8,maxPlanBytes:32768,maxViewBatches:32,maxViewEdits:64,maxViewBytes:65536,maxViewPlanBytes:32768},
    requiredDependencies:[],contexts:[],groups:[{reference:'ordinary',internal:'ordinary',authority:'a'}],scopes:[]};
  // Required F14 gates are explicit; none are inferred from the editor being local.
  Object.assign(hh,{allTargetsWritable:true,current:true,scope:true,continuous:true,live:true,invocationFresh:true});
  const hr={contract:'history-reversal/1',operation:'undo',invocation:'undo-ordinary',authority:'a',profile:'history',dependencies:[],target:{kind:'group',group:'ordinary'}};
  const hp=h.prepareReversal(so.state,hr,hh);assert(hp);const ho=h.publishReversal(so.state,hp,hh);assert.equal(ho.status,'applied');
  control('native-history-view-root-retains-actual-journal',api.committedSourceTransforms(ho.state).length===1&&api.prepareParserSource(snapshot(so.state),snapshot(ho.state),api.committedSourceTransforms(ho.state),gates,limits).mode==='incremental');
  control('source-and-inputs-not-mutated',equal(snapshot(root),before)&&equal(snapshot(out.state),target)&&root.publication===0);
  const candidate=copy(root);candidate.documents[0].source=[...before.source,97];candidate.documents[0].revision++;
  const prepared=api.prepareSourceJournal(root,candidate,[{document,batches:[[{lo:before.source.length,hi:before.source.length,ins:[97]}]]}],journalLimits,true);assert(prepared);
  control('prepared-source-transform-not-yet-published',api.committedSourceTransforms(candidate).length===0);
  control('prepared-journal-binds-exact-root',!api.commitSourceJournal(copy(candidate),prepared));
  control('prepared-journal-binds-source-frame',(()=>{const changed=copy(candidate.documents[0].source);candidate.documents[0].source=[98];const refused=!api.commitSourceJournal(candidate,prepared);candidate.documents[0].source=changed;return refused;})());
  control('journal-commit-is-single-use',api.commitSourceJournal(candidate,prepared)&&!api.commitSourceJournal(candidate,prepared));
  const stale=copy(out.state);stale.documents[0].revision++;
  control('view-wrapper-cannot-carry-stale-source-frame',!api.carrySourceJournal(out.state,stale));
  control('view-wrapper-cannot-inherit-unrelated-publication',!api.carrySourceJournal(out.state,{...out.state,publication:out.state.publication+1}));
  return {status:'committed-parser-source-reference-only',pointCases,editCases,multiEditCases,chainCases,gateCases,controls,...catalog.scope};
}
export async function checkParserSourceMutations(){
  const files=Object.fromEntries(await Promise.all(['parser-source.mjs','annotation-compounds.mjs','selection-products.mjs','history-views.mjs','history-reversal.mjs'].map(async file=>[file,fs.readFileSync(new URL(file,import.meta.url),'utf8')]))),killed=[];
  const mutations=[
    ['cr-counted-as-parser-newline','parser-source.mjs','if(source[i]===10){row++;start=i+1;}','if(source[i]===10||source[i]===13){row++;start=i+1;}'],
    ['parser-columns-use-utf16','parser-source.mjs','const column=offset-start;','const column=Buffer.from(source.slice(start,offset)).toString(\'utf8\').length;'],
    ['parser-allows-scalar-interior','parser-source.mjs',"(p===s.length||(s[p]&0xc0)!==0x80)",'true'],
    ['batch-applied-low-to-high','parser-source.mjs','[...batch].reverse()','batch'],
    ['old-end-point-is-start','parser-source.mjs','old_end_point=parserPoint(source,e.hi,','old_end_point=parserPoint(source,e.lo,'],
    ['new-end-point-uses-old-source','parser-source.mjs','new_end_point=parserPoint(next,new_end_byte,','new_end_point=parserPoint(source,new_end_byte,'],
    ['equal-bytes-replace-source-version-identity','parser-source.mjs','if(base.revision===target.revision){','if(equal(base.source,target.source)){'],
    ['same-version-bytes-ignored','parser-source.mjs','if(!equal(base.source,target.source))return null;','/* bytes ignored */'],
    ['missing-chain-relabeled-incremental','parser-source.mjs',"if(revision!==target.revision||!equal(source,target.source))return fresh('missing-journal');","if(revision!==target.revision||!equal(source,target.source))return {...fresh('missing-journal'),mode:'incremental'};"],
    ['receipt-version-continuity-ignored','parser-source.mjs','r.from!==revision','false'],
    ['final-source-bytes-ignored','parser-source.mjs','revision!==target.revision||!equal(source,target.source)','revision!==target.revision'],
    ['request-authority-ignored','parser-source.mjs','!g?.authorized','false'],
    ['incremental-edit-cap-ignored','parser-source.mjs','steps.length>=l.maxEdits','false'],
    ['journal-reservation-ignored','parser-source.mjs','!reserved||','false||'],
    ['journal-byte-cap-ignored','parser-source.mjs','charged>limits.maxJournalBytes','false'],
    ['staged-root-binding-ignored','parser-source.mjs','p.after!==root','false'],
    ['committed-source-frame-ignored','parser-source.mjs','p.rows.some(r=>{const d=documentAt(root,r.document);return !d||d.revision!==r.to||!equal(d.source,r.after);})','false'],
    ['journal-not-single-use','parser-source.mjs','prepared.delete(token);','/* reusable token */'],
    ['source-publisher-drops-receipt','annotation-compounds.mjs','commitSourceJournal(n,journal);','/* source handoff lost */'],
    ['ordinary-view-root-drops-receipt','selection-products.mjs','carrySourceJournal(out.state,next);','/* ordinary handoff lost */'],
    ['history-view-root-drops-receipt','history-views.mjs','carrySourceJournal(reversed.state,next);','/* reversal handoff lost */']
  ];
  const data=(s,name)=>`data:text/javascript;base64,${Buffer.from(s).toString('base64')}#${name}`;
  for(const [name,file,from,to] of mutations){
    assert(files[file].includes(from),name);const code={...files};code[file]=code[file].replace(from,to);
    const urls={};
    for(const file of ['parser-source.mjs','annotation-compounds.mjs','selection-products.mjs','history-views.mjs','history-reversal.mjs']){
      let src=code[file];
      for(const dependency of ['parser-source.mjs','text-oracles.mjs','mapping-profiles.mjs','annotation-records.mjs','annotation-compounds.mjs','reference-profile.mjs','search-occurrences.mjs','selection-products.mjs','annotation-operations.mjs','history-views.mjs','annotation-targets.mjs','read-dependencies.mjs','source-scopes.mjs','annotation-membership.mjs','reversal-results.mjs']){
        if(src.includes(`'./${dependency}'`))src=src.replace(`'./${dependency}'`,JSON.stringify(urls[dependency]??new URL(dependency,import.meta.url).href));
      }
      for(const asset of ['parser-source.json','annotation-compounds.json','selection-products.json','operation-families.json','history-views.json','history-reversal.json'])src=src.replace(`new URL('./${asset}',import.meta.url)`,`new URL(${JSON.stringify(new URL(asset,import.meta.url).href)})`);
      urls[file]=data(src,`${name}-${file}`);
    }
    const [a,b,s,h]=await Promise.all(['parser-source.mjs','annotation-compounds.mjs','selection-products.mjs','history-reversal.mjs'].map(file=>import(urls[file])));
    let failed=false;try{checkParserSource(a,b,s,h,true);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name);failed=true;}assert(failed,name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkParserSource(),mutations:await checkParserSourceMutations()},null,2));
