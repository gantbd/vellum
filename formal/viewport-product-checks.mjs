import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import * as original from './viewport-products.mjs';
import * as geometry from './geometry-products.mjs';
import {geometryFixtureForChecks,geometryLimitsForChecks as geometryLimits} from './geometry-product-checks.mjs';
import {prepareSelectionEdit,publishSelectionEdit} from './selection-products.mjs';
const copy=structuredClone,limits={maxCoordinate:10000,maxRequestBytes:16384,maxPlanBytes:32768};
const viewGates=['authorized','sourceReadable','viewWritable','continuous','live','bounded','notStopped','interpretationCurrent','companionsReady'];
const geometryGates=['authorized','scope','continuous','capacity','live','retained','notStopped'];
const metric=c=>Object.fromEntries(['unit','configurationGeneration','metricsGeneration','projectionGeneration'].map(k=>[k,c[k]]));
function fixture(){
  const f=geometryFixtureForChecks();f.state.views[0].viewportGeneration=1;f.state.views[0].viewportContext=metric(f.packet.context);
  f.good.viewportContext=copy(f.state.views[0].viewportContext);return f;
}
const request=(f,operation='scroll',fields={delta:{x:0,y:4}},s=f.state)=>({contract:'viewport-edit/1',operation,invocation:'viewport',view:copy(s.views[0].view),document:copy(s.views[0].document),
  sourceRevision:s.documents[0].revision,expectedViewport:s.views[0].viewportGeneration,profile:'integer-viewport/1',context:copy(s.views[0].viewportContext),...fields});
const query=(origin=0,unit='page-down')=>({contract:'geometry-query/1',operation:'visual-navigation-target',origin,unit,count:1,goal:null,choices:[null]});
const reveal=(layout,target,xAlign='nearest',yAlign='nearest',margin={x:0,y:0})=>({intent:{kind:'reveal',layout,target,xAlign,yAlign,margin}});
export function checkViewportProducts(api=original,geometryApi=geometry,mutant=false){
  const catalog=JSON.parse(fs.readFileSync(new URL('./viewport-products.json',import.meta.url),'utf8'));
  const family=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8')).find(f=>f.id==='F15');
  assert.deepEqual(Object.keys(catalog.variants).sort(),family.variants.toSorted());
  const f=fixture(),{state,packet,current,good}=f,layout=geometryApi.acceptGeometry(state,packet,current,geometryLimits);assert(layout);
  const target=geometryApi.queryGeometry(state,layout,query(),current,geometryLimits);assert.equal(target.status,'source');
  const scroll=request(f),rr=request(f,'viewport-intent',reveal(layout,target.reference));
  const prepare=(r=scroll,s=state,g=good,l=limits)=>api.prepareViewportEdit(s,r,g,l,geometryLimits);
  const run=(r=scroll,s=state,g=good,l=limits)=>api.publishViewportEdit(s,prepare(r,s,g,l),g,l,geometryLimits);
  const controls=[],control=(name,ok)=>{assert(ok,name);controls.push(name);};let alignmentCases=0,scrollGateCases=0,publicationGateCases=0;
  // Independent finite search oracle: enumerate viewports satisfying containment.
  for(const a of [-3,0,3])for(const length of [1,4,9])for(const extent of [3,4,8])for(const position of [-4,0,4])for(const margin of [0,1])for(const alignment of catalog.alignments){
    const sample=fixture();sample.packet.rows=[copy(sample.packet.rows[0])];sample.packet.rows[0].top=a;sample.packet.rows[0].bottom=a+length;
    sample.packet.rows[0].stops.at(-1).right={kind:'edge'};sample.packet.hits=[];
    sample.state.views[0].viewport.y=position;sample.state.views[0].viewport.height=extent;
    sample.packet.context.viewport=copy(sample.state.views[0].viewport);sample.packet.context.pageDistance=Math.min(extent,2);sample.current.context=copy(sample.packet.context);sample.good.geometry=sample.current;
    const token=geometryApi.acceptGeometry(sample.state,sample.packet,sample.current,geometryLimits);assert(token);
    const t=geometryApi.queryGeometry(sample.state,token,query(0,'row-start'),sample.current,geometryLimits);assert.equal(t.status,'source');
    const r=request(sample,'viewport-intent',reveal(token,t.reference,'nearest',alignment,{x:0,y:margin})),out=api.publishViewportEdit(sample.state,api.prepareViewportEdit(sample.state,r,sample.good,limits,geometryLimits),sample.good,limits,geometryLimits);
    let expected;
    if(alignment==='start')expected=a-margin;else if(alignment==='end')expected=a+length-extent+margin;else if(alignment==='center')expected=Math.floor((2*a+length-extent)/2);
    else{
      const candidates=Array.from({length:81},(_,i)=>i-40).filter(p=>length<=extent-2*margin?(p+margin<=a&&a+length<=p+extent-margin):(a<=p+margin&&p+extent-margin<=a+length));
      assert(candidates.length);expected=candidates.reduce((best,p)=>Math.abs(p-position)<Math.abs(best-position)?p:best);
    }
    assert(['applied','unchanged'].includes(out.status));assert.equal(out.state.views[0].viewport.y,expected);assert.equal(out.state.documents[0].revision,1);alignmentCases++;
  }
  const plan=prepare(),revealPlan=prepare(rr);assert(plan&&revealPlan);
  for(const mask of mutant?[0,511]:Array.from({length:512},(_,i)=>i)){
    const g={...good,...Object.fromEntries(viewGates.map((k,i)=>[k,!!(mask&(1<<i))]))};
    assert.equal(api.publishViewportEdit(state,plan,g,limits,geometryLimits).status,mask===511?'applied':'not-published');scrollGateCases++;
  }
  for(const mask of mutant?[511,65535]:Array.from({length:65536},(_,i)=>i)){
    const g={...good,...Object.fromEntries(viewGates.map((k,i)=>[k,!!(mask&(1<<i))])),geometry:{...current,...Object.fromEntries(geometryGates.map((k,i)=>[k,!!(mask&(1<<(9+i)))]))}};
    assert.equal(api.publishViewportEdit(state,revealPlan,g,limits,geometryLimits).status,mask===65535?'applied':'not-published');publicationGateCases++;
  }
  const moved=run(),revealed=run(rr);
  control('one-pure-viewport-publication',moved.status==='applied'&&moved.state.publication===1&&moved.state.views[0].viewportGeneration===2);
  control('source-history-selection-frame',equal(moved.state.documents,state.documents)&&equal(moved.state.groups,state.groups)&&equal(moved.state.frontiers,state.frontiers)&&equal(moved.state.views[0].selections,state.views[0].selections)&&moved.state.views[0].selectionGeneration===1);
  control('peer-and-input-preserved',equal(moved.state.views[1],state.views[1])&&state.publication===0&&state.views[0].viewportGeneration===1);
  control('reveal-uses-accepted-caret-band',revealed.status==='applied'&&revealed.state.views[0].viewport.y===4&&revealed.state.views[0].viewport.x===0);
  const back=run(request(f,'scroll',{delta:{x:0,y:-4}},moved.state),moved.state);
  control('aba-view-values-have-fresh-generation',equal(back.state.views[0].viewport,state.views[0].viewport)&&back.state.views[0].viewportGeneration===3);
  control('owned-generation-invalidates-old-geometry',geometryApi.queryGeometry(back.state,layout,{contract:'geometry-query/1',operation:'caret-hit-conversion',request:{kind:'caret',point:{position:0,affinity:'after'}}},current,geometryLimits).status==='unavailable');
  control('stale-viewport-plan-does-not-restore',api.publishViewportEdit(back.state,plan,good,limits,geometryLimits).state===back.state);
  const refresh=copy(packet);refresh.context.viewportGeneration=3;const freshCurrent={...current,context:copy(refresh.context)},fresh=geometryApi.acceptGeometry(back.state,refresh,freshCurrent,geometryLimits);assert(fresh);
  const ft=geometryApi.queryGeometry(back.state,fresh,query(),freshCurrent,geometryLimits),fr=request(f,'viewport-intent',reveal(fresh,ft.reference),back.state);
  control('fresh-geometry-after-aba-can-reveal',run(fr,back.state,{...good,geometry:freshCurrent}).status==='applied');
  control('raw-scroll-does-not-require-geometry',run(scroll,state,{...good,geometry:undefined}).status==='applied');
  control('raw-scrolls-can-progress-before-measurement',run(request(f,'scroll',{delta:{x:0,y:4}},moved.state),moved.state,{...good,geometry:undefined}).state.views[0].viewport.y===8);
  control('copied-plan-not-owned',api.publishViewportEdit(state,copy(plan),good,limits,geometryLimits).state===state);
  control('copied-reveal-target-not-owned',prepare(request(f,'viewport-intent',reveal(layout,copy(target.reference))))===null);
  control('foreign-reveal-layout-refused',prepare(request(f,'viewport-intent',reveal(geometryApi.acceptGeometry(state,packet,current,geometryLimits),target.reference)))===null);
  const missing=copy(state);delete missing.views[0].viewportGeneration;
  control('missing-generation-is-not-zero-default',prepare(scroll,missing)===null);
  const missingContext=copy(state);delete missingContext.views[0].viewportContext;
  control('missing-owned-metric-context-refused',prepare(scroll,missingContext)===null);
  const changedMetric=copy(state);changedMetric.views[0].viewportContext.metricsGeneration++;
  control('changed-owned-metric-context-refuses',api.publishViewportEdit(changedMetric,plan,good,limits,geometryLimits).state===changedMetric);
  control('changed-current-metric-context-refuses',api.publishViewportEdit(state,plan,{...good,viewportContext:{...good.viewportContext,metricsGeneration:2}},limits,geometryLimits).state===state);
  const selectionLimits={maxRootBytes:1024,maxInputViews:8,maxViews:8,maxInputSelections:8,maxSelections:8,maxSelectionBytes:4096,maxTextBytes:256,maxEditBytes:4096};
  const setter={contract:'selection-edit/1',operation:'set-selection',invocation:'later-selection',view:copy(state.views[0].view),document:copy(state.views[0].document),sourceRevision:1,expectedSelection:1,profile:'explicit-positional-selection/1',selections:[{anchor:2,head:2,affinity:'after'}],primary:0};
  const later=publishSelectionEdit(state,prepareSelectionEdit(state,setter,good,selectionLimits),good);assert.equal(later.status,'applied');
  const preserved=api.publishViewportEdit(later.state,plan,good,limits,geometryLimits);
  control('actual-later-selection-is-not-restored',preserved.status==='applied'&&preserved.state.views[0].selections[0].head===2&&preserved.state.views[0].selectionGeneration===2&&preserved.state.publication===2);
  const annotated=copy(state);annotated.documents[0].annotationSet++;
  control('raw-scroll-does-not-restore-comments',run(scroll,annotated).state.documents[0].annotationSet===2);
  control('reveal-annotation-dependency-refuses',api.publishViewportEdit(annotated,revealPlan,good,limits,geometryLimits).state===annotated);
  const source=copy(state);source.documents[0].revision++;source.views.forEach(v=>v.sourceRevision++);
  control('source-version-stales-viewport-plan',api.publishViewportEdit(source,plan,good,limits,geometryLimits).state===source);
  const peer=copy(state);peer.views[1].viewport.y=999;
  control('current-peer-viewport-preserved',api.publishViewportEdit(peer,plan,good,limits,geometryLimits).state.views[1].viewport.y===999);
  control('identity-validates-without-version-capacity',run(request(f,'scroll',{delta:{x:0,y:0}}),state,{...good,versionsAvailable:false}).status==='unchanged');
  control('changed-intent-requires-version-capacity',api.publishViewportEdit(state,plan,{...good,versionsAvailable:false},limits,geometryLimits).state===state);
  const noCounter=copy(state);noCounter.views[0].viewportGeneration=Number.MAX_SAFE_INTEGER;
  control('generation-overflow-refuses',run(request(f,'scroll',{delta:{x:0,y:1}},noCounter),noCounter).state===noCounter);
  const overflow=copy(state);overflow.publication=Number.MAX_SAFE_INTEGER;
  control('publication-overflow-refuses',run(scroll,overflow).state===overflow);
  control('invalid-target-identity-refuses',prepare({...scroll,view:null})===null);
  control('unknown-action-refuses',prepare(request(f,'viewport-intent',{intent:{kind:'animate-script'}}))===null);
  control('unknown-field-refuses',prepare({...scroll,extra:true})===null);
  control('nonintegral-delta-refuses',prepare(request(f,'scroll',{delta:{x:0,y:0.5}}))===null);
  control('invalid-dimensions-refuse',prepare(request(f,'viewport-intent',{intent:{kind:'set',viewport:{x:0,y:0,width:0,height:8}}}))===null);
  const set=run(request(f,'viewport-intent',{intent:{kind:'set',viewport:{x:-2,y:3,width:20,height:12}}}));
  control('explicit-set-replaces-owned-viewport',set.status==='applied'&&equal(set.state.views[0].viewport,{x:-2,y:3,width:20,height:12})&&set.state.views[0].viewportGeneration===2);
  control('explicit-set-identity-preserves-root',run(request(f,'viewport-intent',{intent:{kind:'set',viewport:copy(state.views[0].viewport)}})).state===state);
  control('unknown-content-is-not-silently-clamped',run(request(f,'scroll',{delta:{x:-5,y:500}})).state.views[0].viewport.y===500);
  control('overflow-is-not-rounded',prepare(request(f,'scroll',{delta:{x:0,y:10000}},moved.state),moved.state)===null);
  control('oversize-margin-refuses',prepare(request(f,'viewport-intent',reveal(layout,target.reference,'nearest','nearest',{x:0,y:4})))===null);
  control('unsupported-alignment-refuses',prepare(request(f,'viewport-intent',reveal(layout,target.reference,'nearest','guess')))===null);
  const duplicate=geometryFixtureForChecks([1,1]);duplicate.state.views[0].viewportGeneration=1;duplicate.state.views[0].viewportContext=metric(duplicate.packet.context);duplicate.good.viewportContext=copy(duplicate.state.views[0].viewportContext);
  duplicate.packet.rows[1].stops[0].position=1;
  const dl=geometryApi.acceptGeometry(duplicate.state,duplicate.packet,duplicate.current,geometryLimits),dt=geometryApi.queryGeometry(duplicate.state,dl,query(1,'right'),duplicate.current,geometryLimits);
  const dr=request(duplicate,'viewport-intent',reveal(dl,dt.reference,'start','start'));
  const dOut=api.publishViewportEdit(duplicate.state,api.prepareViewportEdit(duplicate.state,dr,duplicate.good,limits,geometryLimits),duplicate.good,limits,geometryLimits);
  control('explicit-projection-index-is-not-first-same-point',dOut.status==='applied'&&dOut.state.views[0].viewport.y===4&&dOut.state.views[0].selections[0].head===0);
  for(const key of ['maxCoordinate','maxRequestBytes','maxPlanBytes']){
    control(`prepare-${key}`,prepare(scroll,state,good,{...limits,[key]:0})===null);
    control(`final-${key}`,api.publishViewportEdit(state,plan,good,{...limits,[key]:0},geometryLimits).state===state);
  }
  return {status:'viewport-intent-and-geometry-reveal-only',alignmentCases,scrollGateCases,publicationGateCases,controls,...catalog.scope};
}
export async function checkViewportMutations(){
  const source=fs.readFileSync(new URL('./viewport-products.mjs',import.meta.url),'utf8'),killed=[];
  const mutations=[
    ['owned-context-ignored','!equal(v.viewportContext,r.context)','false'],
    ['final-reveal-not-rechecked','if(p.projection&&!equal(resolveReveal(state,p.request,g,geometryLimits),p.projection))','if(false)'],
    ['nearest-always-start','return p<lo?lo:p>hi?hi:p;','return first;'],
    ['center-truncates-negative','n>=0n?n/2n:-((-n+1n)/2n)','n/2n'],
    ['start-margin-ignored','first=a-m,last=b-e+m','first=a,last=b-e+m'],
    ['old-root-restored','const next=copy(state),v=next.views.find','const next=copy({...state,views:state.views.map(v=>equal(v.view,p.request.view)?{...v,selections:[{anchor:0,head:0,affinity:"after"}],primary:0}:v)}),v=next.views.find'],
    ['generation-not-incremented','v.viewportGeneration++;','/* stale generation */'],
    ['version-capacity-ignored','!g.versionsAvailable||','false||'],
    ['final-request-cap-ignored','p.requestBytes>limits.maxRequestBytes','false'],
    ['final-plan-cap-ignored','p.planBytes>limits.maxPlanBytes','false'],
    ['selection-mutated','v.viewport=copy(p.after);','v.viewport=copy(p.after);v.selections=[];']
  ];
  const loadViewport=async(code,name,geometryUrl=new URL('./geometry-products.mjs',import.meta.url).href)=>{
    code=code.replace("'./geometry-products.mjs'",JSON.stringify(geometryUrl)).replace("new URL('./viewport-products.json',import.meta.url)",`new URL(${JSON.stringify(new URL('./viewport-products.json',import.meta.url).href)})`);
    return import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`);
  };
  const kill=(api,g,name)=>{let failed=false;try{checkViewportProducts(api,g,true);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name);failed=true;}assert(failed,name);killed.push(name);};
  for(const [name,from,to] of mutations){assert(source.includes(from),name);kill(await loadViewport(source.replace(from,to),name),geometry,name);}
  const gs=fs.readFileSync(new URL('./geometry-products.mjs',import.meta.url),'utf8');
  for(const [name,from,to] of [
    ['owned-geometry-generation-ignored',"(!Object.hasOwn(views[0],'viewportGeneration')||views[0].viewportGeneration===context.viewportGeneration)",'true'],
    ['projection-index-replaced-by-first-source-point','const stops=flatten(p.packet),s=stops[t.index];','const stops=flatten(p.packet),s=stops.find(s=>equal(stopPoint(s),t.point));']
  ]){
    assert(gs.includes(from),name);let code=gs.replace(from,to);
    for(const file of ['text-oracles.mjs','selection-products.mjs'])code=code.replace(`'./${file}'`,JSON.stringify(new URL(file,import.meta.url).href));
    code=code.replace("new URL('./geometry-products.json',import.meta.url)",`new URL(${JSON.stringify(new URL('./geometry-products.json',import.meta.url).href)})`);
    const url=`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`,g=await import(url);
    kill(await loadViewport(source,name,url),g,name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkViewportProducts(),mutations:await checkViewportMutations()},null,2));
