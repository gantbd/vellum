import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import * as original from './boundary-commands.mjs';
import {acquireBoundaryBatch} from './unicode-boundaries.mjs';
import {unicodeCases} from './unicode-conformance.mjs';
import {normalizeSelection} from './selection-products.mjs';
import {prepareReversal,publishReversal} from './history-reversal.mjs';
const copy=structuredClone,bytes=s=>Array.from(Buffer.from(s,'utf8')),text=s=>Buffer.from(s).toString('utf8');
const limits={maxRootBytes:1024,maxBoundaries:2048,maxWords:512,maxBoundaryBytes:32768,maxSteps:32,maxDerivedBytes:32768};
const selectionLimits={maxRootBytes:1024,maxInputViews:8,maxViews:8,maxInputSelections:8,maxSelections:8,maxSelectionBytes:4096,maxTextBytes:256,maxEditBytes:4096};
const document={authority:'a',id:'A',incarnation:1},view=id=>({authority:'a',id,incarnation:1});
const sel=(anchor,head=anchor,affinity='after')=>({anchor,head,affinity});
const gateNames=['authorized','sourceReadable','viewWritable','continuous','live','bounded','notStopped','interpretationCurrent','companionsReady','boundaryLive','boundaryRetained'];
const good={...Object.fromEntries(gateNames.map(k=>[k,true])),sourceWritable:true,allTargetsWritable:true,versionsAvailable:true,viewVersionsAvailable:true,
  historyReady:true,completeIdentity:true,maxParticipants:8,targetRestorationValid:true,boundaryProfile:'unicode18-uax29/1'};
const corpus=unicodeCases('unicode18-grapheme-test.txt'),examples=['','e\u0301X','a🙂b','ab\r\ncd',"  can't...32.3 漢字 !",'ภาษาไทย','a🇦\u200d🇧🇨b','abc','a\u0308b'];
const allTexts=[...corpus.map(c=>c.text),...examples],evidence=acquireBoundaryBatch(allTexts.map(t=>({ref:{...document,revision:1},source:bytes(t)})),limits);
assert(evidence);
const fixture=(source,selections=[sel(0)])=>({authority:'a',publication:0,documents:[{document,source:bytes(source),revision:1,annotationSet:1,records:[]}],groups:[],frontiers:{},views:[
  {view:view('origin'),document,sourceRevision:1,selectionGeneration:1,profile:'explicit-positional-selection/1',...normalizeSelection(bytes(source),selections,0,selectionLimits),viewport:3},
  {view:view('peer'),document,sourceRevision:1,selectionGeneration:1,profile:'explicit-positional-selection/1',selections:[sel(bytes(source).length)],primary:0,viewport:17}]});
const request=(boundary,unit='grapheme',direction='forward',operation='logical-movement',extend=false,count=1)=>({contract:'boundary-command/1',operation,invocation:'logical-edit',view:view('origin'),document,
  sourceRevision:1,expectedSelection:1,profile:'unicode-logical-edit/1',boundary,unit,direction,count,extend});
export function checkBoundaryCommands(api=original){
  const catalog=JSON.parse(fs.readFileSync(new URL('./boundary-commands.json',import.meta.url),'utf8'));
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  for(const [id,variants] of Object.entries(catalog.families))assert(variants.every(v=>families.find(f=>f.id===id)?.variants.includes(v)));
  const prepare=(s,r,g=good,l=limits)=>api.prepareBoundaryCommand(s,r,g,selectionLimits,l);
  const run=(s,r,g=good,l=limits)=>api.publishBoundaryCommand(s,prepare(s,r,g,l),g,selectionLimits,l);
  const controls=[],control=(name,ok)=>{assert(ok,name);controls.push(name);};let movementCases=0,deletionCases=0,wordCases=0,wordDeletionCases=0,gateCases=0;
  for(let i=0;i<corpus.length;i++){
    const sample=corpus[i],last=sample.expected.length-1;
    for(let at=0;at<=last;at++)for(const direction of ['backward','forward']){
      const position=sample.expected[at],delta=direction==='forward'?1:-1,s=fixture(sample.text,[sel(position)]);
      for(const count of [1,2])for(const extend of [false,true]){
        const out=run(s,request(evidence[i],'grapheme',direction,'logical-movement',extend,count));
        assert(['applied','unchanged'].includes(out.status));const expected=sample.expected[Math.max(0,Math.min(last,at+count*delta))];
        assert.deepEqual(out.state.views[0].selections[0],sel(extend?position:expected,expected));
        assert.equal(out.state.documents[0].revision,1);assert.equal(out.state.groups.length,0);movementCases++;
      }
      const target=sample.expected[Math.max(0,Math.min(last,at+delta))],out=run(s,request(evidence[i],'grapheme',direction,'ordinary-edit'));
      assert(['applied','unchanged'].includes(out.status));const from=Math.min(position,target),to=Math.max(position,target);
      assert.deepEqual(out.state.documents[0].source,[...bytes(sample.text).slice(0,from),...bytes(sample.text).slice(to)]);
      assert.equal(out.state.views[0].selections[0].head,from);assert(out.state.views.every(v=>v.sourceRevision===out.state.documents[0].revision));deletionCases++;
    }
  }
  const index=n=>corpus.length+examples.indexOf(n),ticket=t=>evidence[index(t)];
  const phrase=examples[4],starts=[0,2,10,15,18,23],ends=[0,7,14,18,21,23];assert.equal(bytes(phrase).length,23);
  // Independent hand-written prose fixture endpoints: can't / 32.3 / 漢 / 字.
  for(const unit of ['word-start','word-end'])for(const position of [0,1,2,4,7,9,10,14,15,18,21,23])for(const direction of ['backward','forward'])for(const extend of [false,true]){
    const vector=unit==='word-start'?starts:ends,expected=direction==='forward'?Math.min(...vector.filter(p=>p>position),23):Math.max(...vector.filter(p=>p<position),0);
    const out=run(fixture(phrase,[sel(position)]),request(ticket(phrase),unit,direction,'logical-movement',extend));
    assert(['applied','unchanged'].includes(out.status));assert.equal(out.state.views[0].selections[0].head,expected);wordCases++;
    if(!extend){
      const deleted=run(fixture(phrase,[sel(position)]),request(ticket(phrase),unit,direction,'ordinary-edit'));
      const from=Math.min(position,expected),to=Math.max(position,expected),source=bytes(phrase);
      assert(['applied','unchanged'].includes(deleted.status));assert.deepEqual(deleted.state.documents[0].source,[...source.slice(0,from),...source.slice(to)]);wordDeletionCases++;
    }
  }
  const source='e\u0301X',s=fixture(source),r=request(ticket(source)),plan=prepare(s,r);
  for(let mask=0;mask<2048;mask++){
    const g={...good,...Object.fromEntries(gateNames.map((k,i)=>[k,!!(mask&(1<<i))]))};
    assert.equal(api.publishBoundaryCommand(s,plan,g,selectionLimits,limits).status,mask===2047?'applied':'not-published');gateCases++;
  }
  control('combining-cluster-not-scalar-step',run(s,r).state.views[0].selections[0].head===3);
  control('copied-boundary-not-owned',prepare(s,{...r,boundary:copy(r.boundary)})===null);
  control('foreign-source-not-retargeted',prepare(fixture('abc'),r)===null);
  const foreign=copy(s);foreign.documents[0].document.incarnation++;foreign.views.forEach(v=>v.document.incarnation++);
  control('incarnation-not-retargeted',prepare(foreign,{...r,document:foreign.documents[0].document})===null);
  control('copied-plan-not-owned',api.publishBoundaryCommand(s,copy(plan),good,selectionLimits,limits).state===s);
  control('revoked-boundary-at-prepare',prepare(s,r,{...good,boundaryRetained:false})===null);
  control('different-boundary-profile-refused',prepare(s,r,{...good,boundaryProfile:'unicode17'})===null);
  const inside=fixture(source,[sel(1)]);
  control('movement-leaves-cluster-interior',run(inside,r).state.views[0].selections[0].head===3);
  control('implicit-partial-cluster-delete-refused',prepare(inside,{...r,operation:'ordinary-edit'})===null);
  const explicit=fixture(source,[sel(0,1)]),deleted=run(explicit,{...r,operation:'ordinary-edit'});
  control('explicit-scalar-selection-remains-exact',text(deleted.state.documents[0].source)==='\u0301X');
  const span=fixture('abc',[sel(0,2)]),sr=request(ticket('abc'));
  control('nonextended-range-collapses-on-first-step',run(span,sr).state.views[0].selections[0].head===2);
  control('remaining-count-after-collapse',run(span,{...sr,count:2}).state.views[0].selections[0].head===3);
  control('extension-keeps-original-anchor',run(span,{...sr,extend:true}).state.views[0].selections[0].anchor===0);
  control('nonempty-delete-does-not-extra-step',text(run(span,{...sr,operation:'ordinary-edit',count:2}).state.documents[0].source)==='c');
  const cr=fixture('ab\r\ncd',[sel(2)]),crout=run(cr,request(ticket('ab\r\ncd'),'grapheme','forward','ordinary-edit'));
  control('crlf-is-one-character-delete',text(crout.state.documents[0].source)==='abcd');
  control('empty-source-identity',run(fixture(''),request(ticket(''))).status==='unchanged');
  control('read-only-navigation-allowed',run(s,r,{...good,sourceWritable:false}).status==='applied');
  control('read-only-delete-whole-refused',run(s,{...r,operation:'ordinary-edit'},{...good,allTargetsWritable:false}).state===s);
  control('delete-extend-not-reinterpreted',prepare(s,{...r,operation:'ordinary-edit',extend:true})===null);
  control('zero-count-refused',prepare(s,{...r,count:0})===null);
  control('unknown-unit-refused',prepare(s,{...r,unit:'identifier'})===null);
  control('unknown-field-refused',prepare(s,{...r,extra:1})===null);
  const many=fixture('abc',[sel(0),sel(2)]);
  control('total-step-cap-not-per-caret',prepare(many,{...sr,count:2},good,{...limits,maxSteps:3})===null);
  const collision=fixture('abc',[sel(1),sel(2)]);
  control('overlapping-derived-deletion-refuses',prepare(collision,{...sr,operation:'ordinary-edit',count:2})===null);
  for(const key of ['maxRootBytes','maxBoundaries','maxWords','maxBoundaryBytes','maxSteps','maxDerivedBytes']){
    control(`prepare-${key}`,prepare(s,r,good,{...limits,[key]:0})===null);
    control(`final-${key}`,api.publishBoundaryCommand(s,plan,good,selectionLimits,{...limits,[key]:0}).state===s);
  }
  const stale=copy(s);stale.views[0].selectionGeneration++;
  control('selection-stales-preparation',api.publishBoundaryCommand(stale,plan,good,selectionLimits,limits).state===stale);
  const editRequest={...r,operation:'ordinary-edit'},editPlan=prepare(s,editRequest),published=api.publishBoundaryCommand(s,editPlan,good,selectionLimits,limits);
  control('delete-one-root-and-group',published.status==='applied'&&published.state.publication===1&&published.state.groups.length===1&&text(published.state.documents[0].source)==='X');
  const peer=copy(s);peer.views[1].selectionGeneration++;
  control('delete-peer-stales-plan',api.publishBoundaryCommand(peer,editPlan,good,selectionLimits,limits).state===peer);
  const annotations=copy(s);annotations.documents[0].annotationSet++;
  control('delete-annotation-set-stales-plan',api.publishBoundaryCommand(annotations,editPlan,good,selectionLimits,limits).state===annotations);
  const viewport=copy(s);viewport.views[1].viewport=99;
  control('current-viewport-survives',api.publishBoundaryCommand(viewport,editPlan,good,selectionLimits,limits).state.views[1].viewport===99);
  control('published-source-retires-old-result-use',api.publishBoundaryCommand(published.state,editPlan,good,selectionLimits,limits).state===published.state);
  const host={...good,viewLimits:selectionLimits,limits:{maxDependencies:8,maxParticipants:8,maxChanges:32,maxPlanBytes:131072,maxViewBatches:32,maxViewEdits:128,maxViewBytes:16384,maxViewPlanBytes:65536},
    invocationFresh:true,owner:{},recipient:'reviewer',profile:'history/1',requiredDependencies:[],contexts:[],groups:[{reference:'logical-edit',internal:'logical-edit',authority:'a'}],scopes:[]};
  const undoRequest={contract:'history-reversal/1',operation:'undo',authority:'a',target:{kind:'group',group:'logical-edit'},invocation:'undo-boundary',profile:host.profile,dependencies:[]};
  const undone=publishReversal(published.state,prepareReversal(published.state,undoRequest,host),host);
  control('native-undo-recovers-exact-bytes',undone.status==='applied'&&equal(undone.state.documents[0].source,s.documents[0].source)&&undone.state.views.every(v=>v.sourceRevision===3));
  return {status:'logical-unicode-command-products-only',movementCases,deletionCases,wordCases,wordDeletionCases,gateCases,controls,...catalog.scope};
}
export async function checkBoundaryMutations(){
  const source=fs.readFileSync(new URL('./boundary-commands.mjs',import.meta.url),'utf8'),killed=[];
  const mutations=[
    ['skip-final-boundary-gate','!capture(state,p.request,g,limits)','false'],
    ['ignore-step-count','i<steps','i<Math.min(steps,1)'],
    ['extension-drops-anchor','anchor:request.extend?s.anchor:target','anchor:target'],
    ['collapse-also-moves','steps--;','steps=steps;'],
    ['partial-cluster-deletion','lo===hi&&!graphemes.includes(s.head)','false'],
    ['word-end-substitutes-start',"w[request.unit==='word-start'?0:1]",'w[1]'],
    ['ignore-total-step-bound','request.count*v.selections.length>limits.maxSteps','false'],
    ['word-grid-substitutes-graphemes',"request.unit==='grapheme'?graphemes:","request.unit==='grapheme'?table[1]:"]
  ];
  for(const [name,from,to] of mutations){
    assert(source.includes(from),name);let code=source.replace(from,to);
    for(const file of ['unicode-boundaries.mjs','selection-products.mjs'])code=code.replace(`'./${file}'`,JSON.stringify(new URL(file,import.meta.url).href));
    code=code.replace("new URL('./boundary-commands.json',import.meta.url)",`new URL(${JSON.stringify(new URL('./boundary-commands.json',import.meta.url).href)})`);
    const api=await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`);
    let failed=false;try{checkBoundaryCommands(api);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name);failed=true;}
    assert(failed,name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkBoundaryCommands(),mutations:await checkBoundaryMutations()},null,2));
