import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import * as original from './geometry-products.mjs';
const copy=structuredClone,bytes=s=>Array.from(Buffer.from(s,'utf8'));
const document={authority:'a',id:'A',incarnation:1},view={authority:'a',id:'origin',incarnation:1};
const point=(position,affinity='after')=>({position,affinity}),sel=(anchor,head=anchor,affinity='after')=>({anchor,head,affinity});
const geometryGates=['authorized','scope','continuous','capacity','live','retained','notStopped'];
const selectionGates=['authorized','sourceReadable','viewWritable','continuous','live','bounded','notStopped','interpretationCurrent','companionsReady'];
const limits={maxRootBytes:1024,maxRows:16,maxStops:128,maxHitRegions:128,maxTargets:32768,maxRequestBytes:32768,maxReplyBytes:32768,maxLayoutBytes:65536,maxSteps:32,maxValidationSteps:100000,maxCoordinate:10000};
const selectionLimits={maxRootBytes:1024,maxInputViews:8,maxViews:8,maxInputSelections:8,maxSelections:8,maxSelectionBytes:4096,maxTextBytes:256,maxEditBytes:4096};
function fixture(widths=[3,1,4]){
  const source=bytes(widths.map(w=>'a'.repeat(w)).join('\n')),viewport={x:0,y:0,width:32,height:8};let offset=0,index=0;
  const rows=widths.map((w,r)=>{const stops=Array.from({length:w+1},(_,c)=>({position:offset+c,affinity:'after',x:c*4,left:{kind:'edge'},right:{kind:'edge'}}));offset+=w+1;return {top:r*4,bottom:r*4+4,stops};});
  const flat=rows.flatMap(r=>r.stops);
  for(let i=0;i<flat.length;i++){if(i)flat[i].left={kind:'stop',index:i-1};if(i+1<flat.length)flat[i].right={kind:'stop',index:i+1};}
  const hits=rows.flatMap(r=>r.stops.map(s=>({rect:{left:s.x-1,right:s.x+1,top:r.top,bottom:r.bottom},target:{kind:'source',index:index++}})));
  const context={view,document,sourceRevision:1,annotationSet:1,layoutGeneration:1,decorationGeneration:1,projectionGeneration:1,configurationGeneration:1,
    metricsGeneration:1,viewportGeneration:1,viewport,unit:'synthetic-grid-unit/1',pageDistance:8,profile:'declared-caret-geometry/1'};
  const packet={context,source,rows,hits,atStart:true,atEnd:true};
  const state={authority:'a',publication:0,documents:[{document,source:copy(source),revision:1,annotationSet:1,records:[]}],groups:[],frontiers:{},views:[
    {view,document,sourceRevision:1,selectionGeneration:1,profile:'explicit-positional-selection/1',selections:[sel(0)],primary:0,viewport:copy(viewport)},
    {view:{...view,id:'peer'},document,sourceRevision:1,selectionGeneration:1,profile:'explicit-positional-selection/1',selections:[sel(source.length)],primary:0,viewport:copy(viewport)}]};
  const current={context:copy(context),...Object.fromEntries(geometryGates.map(k=>[k,true]))};
  const good={...Object.fromEntries(selectionGates.map(k=>[k,true])),versionsAvailable:true,viewVersionsAvailable:true,sourceWritable:false,geometry:current};
  return {packet,state,current,good};
}
const query=(operation,fields)=>({contract:'geometry-query/1',operation,...fields});
const nav=(origin,unit='row-down',count=1,goal=null,choices=Array(count).fill(null))=>query('visual-navigation-target',{origin,unit,count,goal,choices});
const caret=p=>query('caret-hit-conversion',{request:{kind:'caret',point:p}});
const hit=(x,y,choice=null)=>query('caret-hit-conversion',{request:{kind:'hit',x,y,choice}});
export {fixture as geometryFixtureForChecks,limits as geometryLimitsForChecks};
function command(layout,reference,mode='move-heads',extend=false){return {contract:'visual-selection/1',operation:'supported-visual-movement',invocation:'visual',view,document,
  sourceRevision:1,expectedSelection:1,profile:'declared-caret-geometry/1',layout,targets:[reference],mode,extend};}
export function checkGeometryProducts(api=original,mutant=false){
  const catalog=JSON.parse(fs.readFileSync(new URL('./geometry-products.json',import.meta.url),'utf8'));
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  assert.deepEqual(Object.keys(catalog.variants).sort(),families.find(f=>f.id==='F09').variants.toSorted());
  assert(families.find(f=>f.id==='F12').variants.includes('supported-visual-movement'));
  const controls=[],control=(name,ok)=>{assert(ok,name);controls.push(name);};let oracleCases=0,queryGateCases=0,selectionGateCases=0;
  for(let rowCount=1;rowCount<=4;rowCount++)for(let width=0;width<=4;width++){
    const f=fixture(Array(rowCount).fill(width)),layout=api.acceptGeometry(f.state,f.packet,f.current,limits);assert(layout);
    let origin=0;
    for(let row=0;row<rowCount;row++)for(let col=0;col<=width;col++,origin++){
      const p=point(row*(width+1)+col),c=api.queryGeometry(f.state,layout,caret(p),f.current,limits);
      assert.equal(c.status,'carets');assert.equal(c.carets.length,1);assert.equal(c.carets[0].index,origin);
      const h=api.queryGeometry(f.state,layout,hit(col*4,row*4),f.current,limits);assert.equal(h.status,'source');assert.deepEqual(h.point,p);oracleCases++;
      for(const unit of catalog.units)for(const count of [1,2]){
        const out=api.queryGeometry(f.state,layout,nav(origin,unit,count),f.current,limits);assert.equal(out.status,'source');
        let expectedRow=row,expectedCol=col;
        if(unit==='left'||unit==='right'){
          const i=Math.max(0,Math.min(rowCount*(width+1)-1,origin+(unit==='right'?count:-count)));expectedRow=Math.floor(i/(width+1));expectedCol=i%(width+1);
        }else if(unit==='row-start')expectedCol=0;else if(unit==='row-end')expectedCol=width;
        else expectedRow=Math.max(0,Math.min(rowCount-1,row+(unit.endsWith('down')?1:-1)*count*(unit.startsWith('page')?2:1)));
        assert.deepEqual(out.point,point(expectedRow*(width+1)+expectedCol));assert.equal(out.goal.x,col*4);oracleCases++;
      }
    }
  }
  const f=fixture(),{state,packet,current,good}=f,layout=api.acceptGeometry(state,packet,current,limits),q=(r,l=limits,c=current,s=state,t=layout)=>api.queryGeometry(s,t,r,c,l);
  for(const mask of mutant?[0,127]:Array.from({length:128},(_,i)=>i)){
    const c={...current,...Object.fromEntries(geometryGates.map((k,i)=>[k,!!(mask&(1<<i))]))};
    assert.equal(q(caret(point(0)),limits,c).status,mask===127?'carets':'unavailable');
    assert.equal(!!api.acceptGeometry(state,packet,c,limits),mask===127);queryGateCases++;
  }
  const target=q(nav(0)),request=command(layout,target.reference),prepare=(r=request,s=state,g=good,l=limits)=>api.prepareVisualSelection(s,r,g,selectionLimits,l);
  const plan=prepare();assert(plan);
  for(const mask of mutant?[511,65535]:Array.from({length:65536},(_,i)=>i)){
    const g={...good,...Object.fromEntries(selectionGates.map((k,i)=>[k,!!(mask&(1<<i))])),geometry:{...current,...Object.fromEntries(geometryGates.map((k,i)=>[k,!!(mask&(1<<(9+i)))]))}};
    const out=api.publishVisualSelection(state,plan,g,selectionLimits,limits);assert.equal(out.status,mask===65535?'applied':'not-published');selectionGateCases++;
  }
  const out=api.publishVisualSelection(state,plan,good,selectionLimits,limits);
  control('pure-view-no-source-history',out.status==='applied'&&out.state.publication===1&&out.state.documents[0].revision===1&&out.state.groups.length===0&&equal(out.state.documents,state.documents));
  control('peer-and-viewport-preserved',equal(out.state.views[1],state.views[1])&&equal(out.state.views[0].viewport,state.views[0].viewport));
  const peerViewport=copy(state);peerViewport.views[1].viewport.y=99;
  control('unrelated-current-viewport-not-restored',api.publishVisualSelection(peerViewport,plan,good,selectionLimits,limits).state.views[1].viewport.y===99);
  control('inputs-not-mutated',state.publication===0&&state.views[0].selectionGeneration===1&&packet.context.layoutGeneration===1);
  const tracedPacket=copy(packet),rows=tracedPacket.rows;let rowReads=0;
  Object.defineProperty(tracedPacket,'rows',{enumerable:true,get(){rowReads++;return rows;}});
  control('admission-denies-before-relation-traversal',api.acceptGeometry(state,tracedPacket,{...current,scope:false},limits)===null&&rowReads===0);
  const tracedLimits={...limits};let workReads=0;
  Object.defineProperty(tracedLimits,'maxValidationSteps',{enumerable:true,get(){workReads++;return limits.maxValidationSteps;}});
  control('cached-denial-precedes-work-caps',q(caret(point(0)),tracedLimits,{...current,scope:false}).status==='unavailable'&&workReads===0);
  control('owned-cached-target-reused',q(nav(0)).reference===target.reference);
  control('copied-layout-not-owned',q(caret(point(0)),limits,current,state,copy(layout)).status==='unavailable');
  control('copied-target-not-owned',prepare({...request,targets:[copy(target.reference)]})===null);
  control('copied-plan-not-owned',api.publishVisualSelection(state,copy(plan),good,selectionLimits,limits).state===state);
  const foreign=api.acceptGeometry(state,packet,current,limits);
  control('target-from-other-layout-not-owned-here',prepare({...request,layout:foreign})===null);
  control('goal-from-other-layout-refused',q(nav(0,'row-down',1,{layout:foreign,x:4})).status==='unavailable');
  const goal=q(nav(3,'row-down'));
  control('short-row-does-not-reset-goal',goal.point.position===5&&goal.goal.x===12&&q(nav(goal.index,'row-down',1,goal.goal)).point.position===9);
  const variable=fixture([2,2,2]);variable.packet.rows.forEach((r,i)=>{r.top=[0,3,10][i];r.bottom=[3,10,11][i];});
  variable.packet.context.pageDistance=2;variable.current.context=copy(variable.packet.context);
  let hitIndex=0;variable.packet.hits=variable.packet.rows.flatMap(r=>r.stops.map(s=>({rect:{left:s.x-1,right:s.x+1,top:r.top,bottom:r.bottom},target:{kind:'source',index:hitIndex++}})));
  const variableLayout=api.acceptGeometry(variable.state,variable.packet,variable.current,limits);
  control('page-distance-accumulates-through-tall-row',api.queryGeometry(variable.state,variableLayout,nav(0,'page-down',2),variable.current,limits).point.position===3);
  const tie=q(nav(0,'row-down',1,{layout,x:2}));
  control('nearest-tie-is-ambiguous',tie.status==='ambiguous'&&tie.step===0&&tie.choices.length===2);
  const resolved=q(nav(0,'row-down',1,{layout,x:2},[tie.choices[1]]));
  control('explicit-tie-choice-resolves',resolved.status==='source'&&resolved.point.position===5);
  control('ambiguity-reply-is-bounded',q(nav(0,'row-down',1,{layout,x:2}),{...limits,maxReplyBytes:1}).status==='unavailable');
  control('choices-are-step-indexed',q(nav(0,'row-down',2,{layout,x:2},[tie.choices[0],null])).step===1);
  control('wrong-choice-does-not-pick-neighbor',q(nav(0,'row-down',1,{layout,x:2},[{kind:'source',index:0,point:point(0)}])).status==='unavailable');
  const partial=copy(packet);partial.atStart=false;partial.atEnd=false;partial.rows[0].stops[0].left={kind:'unavailable'};partial.rows.at(-1).stops.at(-1).right={kind:'unavailable'};
  const partialLayout=api.acceptGeometry(state,partial,current,limits);assert(partialLayout);
  control('partial-edge-not-document-end',q(nav(0,'row-up'),limits,current,state,partialLayout).status==='unavailable'&&q(nav(0,'left'),limits,current,state,partialLayout).status==='unavailable');
  control('complete-edge-can-saturate',q(nav(0,'row-up')).point.position===0);
  control('hit-gap-not-guessed',q(hit(2,0)).status==='unavailable');
  const virtual=copy(packet);virtual.hits.push({rect:{left:-1,right:1,top:0,bottom:4},target:{kind:'virtual',id:'indicator'}});
  const virtualLayout=api.acceptGeometry(state,virtual,current,limits),mixed=q(hit(0,0),limits,current,state,virtualLayout);
  control('virtual-overlap-is-ambiguous',mixed.status==='ambiguous'&&mixed.choices.length===2);
  const virtualOnly=q(hit(0,0,{kind:'virtual',id:'indicator'}),limits,current,state,virtualLayout);
  control('virtual-hit-not-source-reference',virtualOnly.status==='virtual'&&!Object.hasOwn(virtualOnly,'reference')&&prepare({...request,layout:virtualLayout,targets:[virtualOnly],mode:'replace-all'})===null);
  const hitSource=q(hit(0,0,mixed.choices.find(c=>c.kind==='source')),limits,current,state,virtualLayout);
  control('explicit-source-hit-usable',api.prepareVisualSelection(state,command(virtualLayout,hitSource.reference,'replace-all'),{...good,geometry:current},selectionLimits,limits)!==null);
  control('hit-is-not-origin-bound-navigation',prepare({...request,targets:[q(hit(4,0)).reference]})===null);
  const different=copy(state);different.views[0].selections=[sel(1)];
  control('navigation-origin-must-match-head',prepare(request,different)===null);
  const extend=copy(state);extend.views[0].selections=[sel(1,0)];const extended=api.publishVisualSelection(extend,prepare({...request,extend:true},extend),good,selectionLimits,limits);
  control('extension-preserves-anchor',extended.state.views[0].selections[0].anchor===1&&extended.state.views[0].selections[0].head===4);
  for(const key of ['layoutGeneration','decorationGeneration','projectionGeneration','configurationGeneration','metricsGeneration','viewportGeneration','unit','pageDistance']){
    const c=copy(current);c.context[key]=typeof c.context[key]==='number'?c.context[key]+1:'other-unit';
    control(`current-context-${key}`,q(caret(point(0)),limits,c).status==='unavailable'&&api.publishVisualSelection(state,plan,{...good,geometry:c},selectionLimits,limits).state===state);
  }
  const annotation=copy(state);annotation.documents[0].annotationSet++;
  control('annotation-context-stales',q(caret(point(0)),limits,current,annotation).status==='unavailable');
  const source=copy(state);source.documents[0].source[0]=98;
  control('same-revision-different-bytes-refused',q(caret(point(0)),limits,current,source).status==='unavailable');
  const viewport=copy(state);viewport.views[0].viewport.y++;
  control('viewport-change-stales-geometry',api.publishVisualSelection(viewport,plan,good,selectionLimits,limits).state===viewport);
  control('selection-generation-stales-preparation',api.publishVisualSelection(out.state,plan,good,selectionLimits,limits).state===out.state);
  const self=q(nav(0,'row-up')),identity=prepare({...request,targets:[self.reference]});
  control('identity-no-version-capacity-required',api.publishVisualSelection(state,identity,{...good,versionsAvailable:false,viewVersionsAvailable:false},selectionLimits,limits).status==='unchanged');
  control('changed-selection-needs-version-capacity',api.publishVisualSelection(state,plan,{...good,viewVersionsAvailable:false},selectionLimits,limits).state===state);
  const bounded=q(query('bounded-layout',{start:1,count:1}));
  control('bounded-layout-no-incidental-total',bounded.status==='layout'&&bounded.rows.length===1&&!Object.hasOwn(bounded,'total')&&!bounded.coverage.atStart&&!bounded.coverage.atEnd);
  control('layout-range-does-not-invent-rows',q(query('bounded-layout',{start:999,count:1})).status==='unavailable');
  const nonmonotone=fixture([4]);nonmonotone.packet.rows[0].stops.forEach((s,i)=>s.position=[0,3,2,1,4][i]);
  const nonmonotoneLayout=api.acceptGeometry(nonmonotone.state,nonmonotone.packet,nonmonotone.current,limits);
  control('visual-order-not-source-sort',api.queryGeometry(nonmonotone.state,nonmonotoneLayout,nav(0,'right'),nonmonotone.current,limits).point.position===3);
  const noHits=copy(packet);noHits.hits=[];const noHitLayout=api.acceptGeometry(state,noHits,current,limits);
  control('known-caret-does-not-fabricate-hit-coverage',q(caret(point(0)),limits,current,state,noHitLayout).status==='carets'&&q(hit(0,0),limits,current,state,noHitLayout).status==='unavailable');
  const fragment=copy(packet);fragment.rows=fragment.rows.slice(0,1);fragment.rows[0].stops.at(-1).right={kind:'unavailable'};fragment.hits=fragment.hits.filter(h=>h.target.index<fragment.rows[0].stops.length);fragment.atEnd=false;
  const fragmentLayout=api.acceptGeometry(state,fragment,current,limits);
  control('missing-caret-coverage-not-an-empty-complete-answer',q(caret(point(packet.source.length)),limits,current,state,fragmentLayout).status==='unavailable');
  const duplicate=fixture([1,1]);duplicate.packet.rows[1].stops[0].position=1;duplicate.state.views[0].selections=[sel(1)];
  const duplicateLayout=api.acceptGeometry(duplicate.state,duplicate.packet,duplicate.current,limits),duplicateTarget=api.queryGeometry(duplicate.state,duplicateLayout,nav(1,'right'),duplicate.current,limits);
  control('multiple-projections-are-reported',api.queryGeometry(duplicate.state,duplicateLayout,caret(point(1)),duplicate.current,limits).carets.length===2);
  control('unrepresentable-visual-caret-not-false-identity',api.prepareVisualSelection(duplicate.state,command(duplicateLayout,duplicateTarget.reference,'replace-all'),duplicate.good,selectionLimits,limits)===null);
  const multi=copy(state);multi.views[0].selections=[sel(0),sel(2)];multi.views[0].primary=1;
  const multipleRequest={...request,targets:[q(nav(0)).reference,q(nav(2)).reference]},multiplePlan=prepare(multipleRequest,multi);
  const multiple=api.publishVisualSelection(multi,multiplePlan,good,selectionLimits,limits);
  control('all-heads-and-primary-retained',equal(multiple.state.views[0].selections,[sel(4),sel(5)])&&multiple.state.views[0].primary===1&&multiple.state.groups.length===0);
  control('multi-head-step-budget-is-total',prepare(multipleRequest,multi,good,{...limits,maxSteps:1})===null);
  const replacePrimary=api.publishVisualSelection(multi,prepare({...request,mode:'replace-primary',targets:[q(hit(4,0)).reference]},multi),good,selectionLimits,limits);
  control('hit-replaces-only-explicit-primary',equal(replacePrimary.state.views[0].selections,[sel(0),sel(1)])&&replacePrimary.state.views[0].primary===1);
  const merged=api.publishVisualSelection(multi,prepare({...multipleRequest,extend:true},multi),good,selectionLimits,limits);
  control('extended-heads-normalize-with-primary',equal(merged.state.views[0].selections,[sel(0,5)])&&merged.state.views[0].primary===0);
  const hidden=fixture([4,4,4,4]),hiddenLayout=api.acceptGeometry(hidden.state,hidden.packet,hidden.current,limits);
  control('denied-geometry-does-not-distinguish-hidden-layout',equal(q(caret(point(0)),limits,{...current,scope:false}),api.queryGeometry(hidden.state,hiddenLayout,caret(point(0)),{...hidden.current,scope:false},limits)));
  const extreme=fixture([1]),exactLimits={...limits,maxCoordinate:Number.MAX_SAFE_INTEGER};extreme.packet.hits=[];
  extreme.packet.rows[0].stops[0].x=-Number.MAX_SAFE_INTEGER+1;extreme.packet.rows[0].stops[1].x=-Number.MAX_SAFE_INTEGER+2;
  const exactLayout=api.acceptGeometry(extreme.state,extreme.packet,extreme.current,exactLimits);
  const exactDistance=api.queryGeometry(extreme.state,exactLayout,nav(0,'row-down',1,{layout:exactLayout,x:Number.MAX_SAFE_INTEGER}),extreme.current,exactLimits);
  control('coordinate-distance-remains-exact',exactDistance.status==='source'&&exactDistance.point.position===1);
  const mutate=(fn)=>{const p=copy(packet);fn(p);return api.acceptGeometry(state,p,current,limits);};
  control('nonreciprocal-edge-refused',mutate(p=>p.rows[0].stops[1].left={kind:'edge'})===null);
  control('cyclic-neighbor-graph-refused',mutate(p=>{const flat=p.rows.flatMap(r=>r.stops);flat[0].left={kind:'stop',index:flat.length-1};flat.at(-1).right={kind:'stop',index:0};})===null);
  control('dangling-hit-refused',mutate(p=>p.hits[0].target.index=999)===null);
  control('contradictory-caret-hit-inverse-refused',mutate(p=>p.hits[0].target.index=1)===null);
  control('unsupported-page-distance-refused',mutate(p=>p.context.pageDistance=0)===null);
  control('unknown-query-field-refused',q({...caret(point(0)),extra:1}).status==='unavailable');
  control('unknown-unit-refused',q(nav(0,'guess-bidi')).status==='unavailable');
  control('zero-motion-count-refused',q({...nav(0),count:0,choices:[]}).status==='unavailable');
  const changedPacket=copy(packet),owned=api.acceptGeometry(state,changedPacket,current,limits);changedPacket.rows[0].stops[1].position=3;
  control('accepted-packet-not-aliased',q(nav(0,'right'),limits,current,state,owned).point.position===1);
  const quota={...limits,maxTargets:1},one=api.acceptGeometry(state,packet,current,quota);
  control('target-cache-is-bounded',q(nav(0),quota,current,state,one).status==='source'&&q(nav(0,'right'),quota,current,state,one).status==='unavailable'&&q(nav(0),quota,current,state,one).status==='source');
  for(const key of ['maxRootBytes','maxRows','maxStops','maxHitRegions','maxLayoutBytes','maxValidationSteps','maxCoordinate']){
    control(`admission-${key}`,api.acceptGeometry(state,packet,current,{...limits,[key]:0})===null);
    control(`current-${key}`,q(caret(point(0)),{...limits,[key]:0}).status==='unavailable');
  }
  for(const key of ['maxRequestBytes','maxReplyBytes','maxSteps'])control(`query-${key}`,q(nav(0),{...limits,[key]:0}).status==='unavailable');
  control('current-target-quota-refuses-prepared',api.publishVisualSelection(state,plan,good,selectionLimits,{...limits,maxTargets:0}).state===state);
  control('prepared-request-byte-cap-current',api.publishVisualSelection(state,plan,good,selectionLimits,{...limits,maxRequestBytes:0}).state===state);
  control('prepare-request-byte-cap',prepare(request,state,good,{...limits,maxRequestBytes:0})===null);
  const isolated=api.acceptGeometry(state,packet,current,limits),oneTarget=q(nav(0),limits,current,state,isolated),oneRequest=command(isolated,oneTarget.reference),onePlan=prepare(oneRequest);
  const queryBytes=Buffer.byteLength(JSON.stringify(nav(0)),'utf8');assert(Buffer.byteLength(JSON.stringify(oneRequest),'utf8')>queryBytes);
  control('consumer-byte-cap-is-independent-of-query-cap',api.publishVisualSelection(state,onePlan,good,selectionLimits,{...limits,maxRequestBytes:queryBytes}).state===state);
  control('current-step-cap-refuses-prepared',api.publishVisualSelection(state,onePlan,good,selectionLimits,{...limits,maxSteps:0}).state===state);
  return {status:'accepted-geometry-and-visual-selection-only',oracleCases,queryGateCases,selectionGateCases,controls,...catalog.scope};
}
export async function checkGeometryMutations(){
  const source=fs.readFileSync(new URL('./geometry-products.mjs',import.meta.url),'utf8'),killed=[];
  const mutations=[
    ['ignore-current-context','!equal(context,current.context)','false'],
    ['skip-final-geometry','!active(state,p.layout,g.geometry,limits)','false'],
    ['source-order-substitutes-visual',"packet.rows.flatMap((r,row)=>r.stops.map(s=>({...s,row})))","packet.rows.flatMap((r,row)=>r.stops.map(s=>({...s,row}))).sort((a,b)=>a.position-b.position)"],
    ['guess-unregistered-hit','if(!selected)return unavailable();',"if(!selected)return {status:'source',point:stopPoint(stops[0])};"],
    ['first-ambiguous-choice-wins','unique.length===1?unique[0]','unique.length>=1?unique[0]'],
    ['partial-is-complete-edge','if(!packet.atStart)return unavailable();','if(false)return unavailable();'],
    ['reset-preferred-position','goal=request.goal.x;','goal=origin.x;'],
    ['page-distance-rounds-each-step','pageY+=BigInt(down?1:-1)*BigInt(packet.context.pageDistance)','pageY=BigInt(row.top)+BigInt(down?1:-1)*BigInt(packet.context.pageDistance)'],
    ['omit-supported-inverse',"unique.length===1&&unique[0].kind==='source'&&!equal(stopPoint(stops[unique[0].index]),stopPoint(s))",'false'],
    ['ignore-reciprocal-neighbor',"!equal(stops[link.index][direction==='left'?'right':'left'],{kind:'stop',index:i})",'false'],
    ['unbounded-target-issuance','record.cache.size>=limits.maxTargets','false'],
    ['virtual-target-becomes-editable',"out=selected.kind==='source'?answerSource", "out=selected.kind==='source'||selected.kind==='virtual'?answerSource"],
    ['ignore-navigation-origin','!equal(t.origin,{position:selections[i].head,affinity:selections[i].affinity})','false'],
    ['consumer-quota-confused-with-query','p.requestBytes>limits.maxRequestBytes','false'],
    ['unrepresentable-caret-is-identity','resolved.some(t=>!uniquePoint(t.point))','false'],
    ['floating-distance-changes-ties','BigInt(s.x)-BigInt(goal)','BigInt(s.x-goal)'],
    ['missing-coverage-is-empty-success','if(!out.carets.length)return unavailable();','/* missing coverage mislabeled complete */'],
    ['denied-admission-still-traverses','!currentContext(state,packet.context,current)','!bounded(packet,limits)||!currentContext(state,packet.context,current)'],
    ['denied-query-still-traverses','p&&currentContext(state,p.packet.context,current)&&','p&&bounded(p.packet,limits)&&currentContext(state,p.packet.context,current)&&']
  ];
  for(const [name,from,to] of mutations){
    assert(source.includes(from),name);let code=source.replace(from,to);
    for(const file of ['text-oracles.mjs','selection-products.mjs'])code=code.replace(`'./${file}'`,JSON.stringify(new URL(file,import.meta.url).href));
    code=code.replace("new URL('./geometry-products.json',import.meta.url)",`new URL(${JSON.stringify(new URL('./geometry-products.json',import.meta.url).href)})`);
    const api=await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`);
    let failed=false;try{checkGeometryProducts(api,true);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name);failed=true;}
    assert(failed,name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkGeometryProducts(),mutations:await checkGeometryMutations()},null,2));
