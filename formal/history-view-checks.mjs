// Exercises actual native F14 admission, actual retained journals and current-view maps.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import * as nativeApi from './history-reversal.mjs';
import * as viewApi from './history-views.mjs';
import {publishAnnotationPlan,reverseAnnotationGroup} from './annotation-compounds.mjs';
import {prepareSelectionEdit,publishSelectionEdit,normalizeSelection} from './selection-products.mjs';
import {emitByOrigin} from './text-oracles.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./history-views.json',import.meta.url),'utf8'));
const selection=JSON.parse(fs.readFileSync(new URL('./selection-products.json',import.meta.url),'utf8'));
const copy=structuredClone,bytes=s=>Array.from(Buffer.from(s,'utf8'));
export function checkHistoryViews(native=nativeApi,views=viewApi){
  assert.equal(catalog.selectionProfile,selection.profile);
  const doc=id=>({authority:'a',id,incarnation:1}),ref=id=>({authority:'a',id,incarnation:1});
  const point=(p,affinity='after')=>({anchor:p,head:p,affinity});
  const viewLimits={maxRootBytes:128,maxInputViews:8,maxViews:8,maxInputSelections:8,maxSelections:8,maxSelectionBytes:2048,maxTextBytes:32,maxEditBytes:256};
  const host={owner:{},recipient:'caller',profile:'execution',authorized:true,bounded:true,notStopped:true,interpretationCurrent:true,companionsReady:true,
    versionsAvailable:true,historyReady:true,invocationFresh:true,targetRestorationValid:true,completeIdentity:true,maxParticipants:8,viewVersionsAvailable:true,
    viewLimits,limits:{maxDependencies:8,maxParticipants:8,maxChanges:8,maxPlanBytes:65536,maxViewBatches:32,maxViewEdits:64,maxViewBytes:32768,maxViewPlanBytes:32768},
    requiredDependencies:[],contexts:[],groups:[{reference:'g',internal:'G',authority:'a'},{reference:'body',internal:'Body',authority:'a'}],scopes:[]};
  const request=(operation='undo',group='g')=>({contract:'history-reversal/1',operation,invocation:`reversal-${operation}`,authority:'a',target:{kind:'group',group},profile:host.profile,dependencies:[]});
  const invoke=(s,operation='undo',h=host)=>native.publishReversal(s,native.prepareReversal(s,request(operation),h),h);
  const row={id:'note',incarnation:1,body:'old',bodyVersion:1,target:{sourceRevision:1,target:{kind:'range',start:3,end:4}},targetGeneration:1,
    placement:{state:'attached',sourceRevision:1,target:{kind:'range',start:3,end:4}}};
  const initial={authority:'a',publication:0,documents:[{document:doc('A'),source:bytes('abcd'),revision:1,annotationSet:1,records:[copy(row)]},
    {document:doc('B'),source:bytes('uvwx'),revision:1,annotationSet:1,records:[]},{document:doc('C'),source:bytes('notes'),revision:1,annotationSet:1,records:[]}],
    groups:[],frontiers:{},views:[]};
  const source=(id,lo,hi,text)=>({document:doc(id),expectedSource:1,expectedAnnotationSet:1,edits:[{lo,hi,expected:initial.documents.find(d=>d.document.id===id).source.slice(lo,hi),ins:bytes(text)}]});
  const authored=publishAnnotationPlan(initial,{contract:'owned-annotation-plan/1',invocation:'G',descriptor:'candidate-G',sources:[source('A',1,2,'XYZ'),source('B',1,2,'')],annotations:[]},host).state;
  // These views are attached AFTER the source group exists, not saved inside its journal.
  const view=(id,document,p)=>({view:ref(id),document:doc(document),sourceRevision:authored.documents.find(d=>d.document.id===document).revision,
    selectionGeneration:1,profile:selection.profile,selections:[point(p)],primary:0,viewport:7});
  authored.views=[view('origin','A',6),view('peer','A',1),view('second','B',2),view('unrelated','C',1)];
  authored.publication++; // One owner attach decision; no source/history mutation.
  const plan=native.prepareReversal(authored,request(),host),undone=native.publishReversal(authored,plan,host),redone=invoke(undone.state,'redo');
  assert.equal(undone.status,'applied');assert.equal(redone.status,'applied');
  const controls=[],control=(name,condition)=>{assert(condition,name);controls.push(name);};
  const coherent=s=>s.views.every(v=>{const d=s.documents.find(d=>equal(d.document,v.document));return d.revision===v.sourceRevision&&
    equal(normalizeSelection(d.source,v.selections,v.primary,viewLimits),{selections:v.selections,primary:v.primary});});
  let gateCases=0,oracleCases=0,sequenceCases=0;
  control('native-source-view-undo-redo-coherent',coherent(undone.state)&&coherent(redone.state)&&equal(redone.state.documents.map(d=>d.source),authored.documents.map(d=>d.source)));
  control('one-publication-and-fresh-source-revisions',undone.state.publication===authored.publication+1&&redone.state.publication===authored.publication+2&&
    undone.state.documents[0].revision===3&&redone.state.documents[0].revision===4);
  control('fresh-selection-generations-only-on-change',undone.state.views[0].selectionGeneration===2&&redone.state.views[0].selectionGeneration===3&&undone.state.views[1].selectionGeneration===1);
  control('no-new-authored-history',equal(redone.state.groups,authored.groups));
  control('unrelated-view-and-source-preserved',equal(undone.state.views[3],authored.views[3])&&equal(undone.state.documents[2],authored.documents[2]));
  control('raw-counterexample-now-native-bridge-covered',!coherent(reverseAnnotationGroup(authored,'G','undo',host).state)&&coherent(undone.state));
  const gateNames=['authorized','bounded','notStopped','interpretationCurrent','companionsReady','versionsAvailable','historyReady','invocationFresh','viewVersionsAvailable'];
  for(const [s,operation] of [[authored,'undo'],[undone.state,'redo']])for(let mask=0;mask<512;mask++){
    const h={...host,...Object.fromEntries(gateNames.map((k,i)=>[k,!!(mask&(1<<i))]))},before=copy(s),p=native.prepareReversal(s,request(operation),host);
    const out=native.publishReversal(s,p,h);assert.equal(out.status,mask===511?'applied':'not-published');
    if(mask!==511)assert.deepEqual(out.state,s);assert.deepEqual(s,before);gateCases++;
  }
  const selectionHost={...host,sourceReadable:true,sourceWritable:true,viewWritable:true,continuous:true,live:true,allTargetsWritable:true};
  const moveRequest={contract:'selection-edit/1',operation:'set-selection',invocation:'move',view:ref('origin'),document:doc('A'),sourceRevision:2,
    expectedSelection:1,profile:selection.profile,selections:[point(3,'before')],primary:0};
  const moved=publishSelectionEdit(authored,prepareSelectionEdit(authored,moveRequest,selectionHost,viewLimits),selectionHost).state;
  control('changed-current-selection-stales-preparation',native.publishReversal(moved,plan,host).status==='not-published');
  const remapped=invoke(moved);
  control('fresh-preparation-maps-current-not-old-selection',remapped.status==='applied'&&remapped.state.views[0].selections[0].head===1&&remapped.state.views[0].selections[0].affinity==='before'&&coherent(remapped.state));
  const opposing=copy(authored);Object.assign(opposing.views[0],normalizeSelection(opposing.documents[0].source,[point(3,'before'),point(3,'after')],1,viewLimits));
  const opposingUndo=invoke(opposing),opposingRedo=invoke(opposingUndo.state,'redo');
  control('opposing-carets-mapped-not-restored',equal(opposingUndo.state.views[0].selections,[point(1,'before'),point(2,'after')])&&
    equal(opposingRedo.state.views[0].selections,[point(1,'before'),point(4,'after')])&&opposingRedo.state.views[0].primary===1);
  const stationary=copy(authored);stationary.views=[{...copy(authored.views[0]),selections:[point(0)]}];
  const stationaryUndo=invoke(stationary,'undo',{...host,viewVersionsAvailable:false});
  control('source-context-changes-without-selection-version',stationaryUndo.status==='applied'&&stationaryUndo.state.views[0].sourceRevision===3&&stationaryUndo.state.views[0].selectionGeneration===1&&coherent(stationaryUndo.state));
  const added=copy(authored);added.views.push({...view('new','A',3),selections:[point(3,'before')]});
  control('new-view-stales-old-preparation',native.publishReversal(added,plan,host).status==='not-published');
  control('new-view-included-on-reprepare',invoke(added).state.views.at(-1).sourceRevision===3&&coherent(invoke(added).state));
  const removed=copy(authored);removed.views=removed.views.filter(v=>v.view.id!=='peer');
  control('removed-view-stales-plan',native.publishReversal(removed,plan,host).status==='not-published');
  control('reprepare-does-not-resurrect-closed-view',invoke(removed).state.views.length===removed.views.length);
  const cosmetic=copy(authored);cosmetic.views[0].viewport=99;
  control('late-viewport-not-restored',native.publishReversal(cosmetic,plan,host).state.views[0].viewport===99);
  const unrelatedRequest={...moveRequest,view:ref('unrelated'),document:doc('C'),sourceRevision:1,selections:[point(2)]};
  const changedUnrelated=publishSelectionEdit(authored,prepareSelectionEdit(authored,unrelatedRequest,selectionHost,viewLimits),selectionHost).state;
  control('unrelated-selection-not-a-dependency',native.publishReversal(changedUnrelated,plan,host).status==='applied');
  const duplicate=copy(authored);duplicate.views.push(copy(authored.views[1]));
  control('duplicate-view-identity-refused',native.prepareReversal(duplicate,request(),host)===null);
  const stale=copy(authored);stale.views[0].sourceRevision=1;
  control('stale-view-source-not-guessed',native.prepareReversal(stale,request(),host)===null);
  const unsupported=copy(authored);unsupported.views[0].profile='unknown-ui-profile';
  control('unsupported-required-view-not-skipped',native.prepareReversal(unsupported,request(),host)===null);
  const unknown=copy(authored);delete unknown.views;
  control('unknown-membership-not-empty',native.prepareReversal(unknown,request(),host)===null);
  control('missing-source-view-profile-refused',native.prepareReversal(authored,request(),{...host,viewLimits:undefined})===null);
  for(const key of catalog.workLimits){
    const h={...host,limits:{...host.limits,[key]:0}};
    control(`prepare-${key}`,native.prepareReversal(authored,request(),h)===null);
    control(`final-${key}`,native.publishReversal(authored,plan,h).status==='not-published');
  }
  control('captured-profile-not-reinterpreted',native.publishReversal(authored,plan,{...host,viewLimits:{...viewLimits,maxRootBytes:127}}).status==='not-published');
  control('view-membership-bounds',native.prepareReversal(authored,request(),{...host,viewLimits:{...viewLimits,maxViews:1}})===null);
  control('whole-owned-plan-budget-includes-view-part',native.prepareReversal(authored,request(),{...host,limits:{...host.limits,maxPlanBytes:1000}})===null);
  control('private-view-part-charged-exactly',plan.viewPart.charges.planBytes===Buffer.byteLength(JSON.stringify(plan.viewPart),'utf8'));
  control('copied-view-part-not-owned',!views.validateReversalViews(authored,copy(plan.viewPart),viewLimits,host.limits,true));
  control('copied-native-plan-not-owned',native.publishReversal(authored,copy(plan),host).status==='not-published');
  const wrongResult=reverseAnnotationGroup(authored,'G','undo',host);wrongResult.state.documents[0].revision--;
  control('mismatched-candidate-context-refused',views.completeReversalViews(authored,plan.viewPart,wrongResult,viewLimits,host.limits,true).status==='not-published');
  const bodyInput={contract:'owned-annotation-plan/1',invocation:'Body',descriptor:'body',sources:[],annotations:[{kind:'body',document:doc('A'),id:'note',incarnation:1,expectedBody:1,body:'later'}]};
  const laterBody=publishAnnotationPlan(authored,bodyInput,host).state;
  control('later-body-stales-old-source-mapping',native.publishReversal(laterBody,plan,host).status==='not-published');
  const preserveBody=invoke(laterBody);
  control('reprepare-preserves-later-body-and-maps-target',preserveBody.status==='applied'&&preserveBody.state.documents[0].records[0].body==='later'&&preserveBody.state.documents[0].records[0].bodyVersion===2&&
    equal(preserveBody.state.documents[0].records[0].placement.target,{kind:'range',start:3,end:4})&&coherent(preserveBody.state));
  const bodyOnlyState=copy(laterBody);delete bodyOnlyState.views;
  const noViewHost={...host,viewLimits:undefined,viewVersionsAvailable:false};
  const bodyOnly=native.publishReversal(bodyOnlyState,native.prepareReversal(bodyOnlyState,request('undo','body'),noViewHost),noViewHost);
  control('body-only-needs-no-source-view-profile',bodyOnly.status==='applied'&&bodyOnly.state.documents[0].revision===2&&bodyOnly.state.documents[0].records[0].body==='old');
  const compound=publishAnnotationPlan(initial,{contract:'owned-annotation-plan/1',invocation:'G',descriptor:'compound',sources:[source('A',1,2,'XYZ')],annotations:bodyInput.annotations},host).state;
  compound.views=[view('origin','A',6)];
  const compoundUndo=invoke(compound);
  control('compound-source-body-view-reversal',compoundUndo.status==='applied'&&compoundUndo.state.documents[0].records[0].body==='old'&&coherent(compoundUndo.state));
  // Independent sequential point oracle; no mapAttachment/mapSelection calls.
  const step=(p,e,affinity)=>p<e.lo?p:p>e.hi?p+e.ins.length-(e.hi-e.lo):e.lo<e.hi&&p===e.lo?e.lo:e.lo<e.hi&&p===e.hi?e.lo+e.ins.length:affinity==='before'?e.lo:e.lo+e.ins.length;
  for(const text of ['abcd','éab🙂','a\r\nb'])for(const ins of ['','X','🙂']){
    const src=bytes(text),positions=[0];for(const c of text)positions.push(positions.at(-1)+bytes(c).length);
    const r={...copy(initial.documents[0]),source:src,records:[]};
    for(let a=0;a<positions.length;a++)for(let b=a;b<positions.length;b++){
      const edits=[{lo:positions[a],hi:positions[b],expected:src.slice(positions[a],positions[b]),ins:bytes(ins)}];
      if(equal(emitByOrigin(src,edits),src))continue;
      const seed={authority:'a',publication:0,documents:[r],groups:[],frontiers:{},views:[]};
      const changed=publishAnnotationPlan(seed,{contract:'owned-annotation-plan/1',invocation:'G',descriptor:'finite',sources:[{document:doc('A'),expectedSource:1,expectedAnnotationSet:1,edits}],annotations:[]},host).state;
      const after=changed.documents[0].source,points=[0];for(const c of new TextDecoder('utf-8',{ignoreBOM:true}).decode(Uint8Array.from(after)))points.push(points.at(-1)+bytes(c).length);
      for(const p of points)for(const affinity of ['before','after']){
        const input=copy(changed);input.views=[{view:ref('finite'),document:doc('A'),sourceRevision:2,selectionGeneration:1,profile:selection.profile,selections:[point(p,affinity)],primary:0,viewport:12}];
        const u=invoke(input),d=invoke(u.state,'redo');assert.equal(u.status,'applied');assert.equal(d.status,'applied');
        let expected=p;for(const inverse of changed.groups[0].changes[0].inverseSequence)expected=step(expected,inverse,affinity);
        assert.equal(u.state.views[0].selections[0].head,expected);
        for(const e of [...edits].reverse())expected=step(expected,e,affinity);
        assert.equal(d.state.views[0].selections[0].head,expected);assert(coherent(u.state)&&coherent(d.state));oracleCases++;
      }
    }
  }
  // Actual two-edit inverses have sequential coordinates; undo must not sort them into a batch.
  const multiSeed={...copy(initial),documents:[{...copy(initial.documents[0]),source:bytes('abcdef'),records:[]}],views:[]};
  const multiEdits=[{lo:0,hi:1,expected:[97],ins:bytes('XYZ')},{lo:3,hi:5,expected:bytes('de'),ins:[]}];
  const multi=publishAnnotationPlan(multiSeed,{contract:'owned-annotation-plan/1',invocation:'G',descriptor:'multi',sources:[{document:doc('A'),expectedSource:1,expectedAnnotationSet:1,edits:multiEdits}],annotations:[]},host).state;
  for(let p=0;p<=multi.documents[0].source.length;p++)for(const affinity of ['before','after']){
    const s=copy(multi);s.views=[{view:ref('multi'),document:doc('A'),sourceRevision:2,selectionGeneration:1,profile:selection.profile,selections:[point(p,affinity)],primary:0}];
    const u=invoke(s),d=invoke(u.state,'redo');assert.equal(u.status,'applied');assert.equal(d.status,'applied');let expected=p;
    for(const e of multi.groups[0].changes[0].inverseSequence)expected=step(expected,e,affinity);
    assert.equal(u.state.views[0].selections[0].head,expected);for(const e of [...multiEdits].reverse())expected=step(expected,e,affinity);
    assert.equal(d.state.views[0].selections[0].head,expected);assert(coherent(u.state)&&coherent(d.state));sequenceCases++;
  }
  const ranges=copy(multi);ranges.views=[{view:ref('ranges'),document:doc('A'),sourceRevision:2,selectionGeneration:1,profile:selection.profile,
    selections:[{anchor:0,head:2,affinity:'after'},{anchor:4,head:2,affinity:'before'}],primary:1}];
  const rangeUndo=invoke(ranges);
  control('range-direction-primary-normalized-after-complete-transform',rangeUndo.status==='applied'&&equal(rangeUndo.state.views[0].selections,[{anchor:2,head:0,affinity:'before'}])&&rangeUndo.state.views[0].primary===0);
  control('multi-inverse-plan-retains-all-batches',native.prepareReversal({...multi,views:[]},request(),host).viewPart.sources[0].batches.length===2);
  return {status:'view-aware-reversal-bridge-only',gateCases,oracleCases,sequenceCases,controls,...catalog.scope};
}

export async function checkHistoryViewMutations(){
  const core=fs.readFileSync(new URL('./history-views.mjs',import.meta.url),'utf8');
  const native=fs.readFileSync(new URL('./history-reversal.mjs',import.meta.url),'utf8');
  const mutations=[
    ['omit-view-part-at-publication','native','return completeReversalViews(state,plan.viewPart,reversed,host.viewLimits,host.limits,host.viewVersionsAvailable);','return reversed;'],
    ['skip-last-required-view','core','for(const view of beforeViews){','for(const view of beforeViews.slice(0,-1)){'],
    ['ignore-current-view-footprint','core','return equal(captureViews(state,part.sources,part.viewLimits),part.beforeViews);','return true;'],
    ['omit-fresh-view-source-context','core','v.sourceRevision=after.sourceRevision;','/* stale context */'],
    ['reuse-selection-generation','core','v.selectionGeneration++;','v.selectionGeneration+=0;'],
    ['reverse-retained-inverse-order','core','change.inverseSequence.map(e=>[e])','[...change.inverseSequence].reverse().map(e=>[e])'],
    ['ignore-view-version-capacity','core','part.viewChanged&&!viewVersionsAvailable','false'],
    ['restore-old-viewport','core','v.selections=copy(after.selections);','v.viewport=0;v.selections=copy(after.selections);']
  ];
  const killed=[];
  for(const [name,target,from,to] of mutations){
    const files={core,native};assert(files[target].includes(from),name);files[target]=files[target].replace(from,to);
    for(const file of ['selection-products.mjs','text-oracles.mjs','parser-source.mjs'])files.core=files.core.replace(`'./${file}'`,JSON.stringify(new URL(file,import.meta.url).href));
    for(const file of ['history-views.json','selection-products.json'])files.core=files.core.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(file,import.meta.url).href)})`);
    const coreURL=`data:text/javascript;base64,${Buffer.from(files.core).toString('base64')}#${name}`;
    files.native=files.native.replace("'./history-views.mjs'",JSON.stringify(coreURL));
    for(const file of ['annotation-compounds.mjs','annotation-operations.mjs','reversal-results.mjs'])files.native=files.native.replace(`'./${file}'`,JSON.stringify(new URL(file,import.meta.url).href));
    for(const file of ['history-reversal.json','operation-families.json'])files.native=files.native.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(file,import.meta.url).href)})`);
    const c=await import(coreURL),n=await import(`data:text/javascript;base64,${Buffer.from(files.native).toString('base64')}#${name}`);
    let failed=false;try{checkHistoryViews(n,c);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name);failed=true;}assert(failed,name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkHistoryViews(),mutations:await checkHistoryViewMutations()},null,2));
