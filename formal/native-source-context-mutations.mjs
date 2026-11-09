import fs from 'node:fs';
import assert from 'node:assert/strict';
import {checkNativeSourceContexts} from './native-source-context-checks.mjs';
import {checkSourceContextBoundaries} from './native-source-context-boundaries.mjs';
const base=new URL('./',import.meta.url);
function url(file,m=null,overrides={}){
 let s=fs.readFileSync(new URL(file,base),'utf8');
 if(m){
  const queryRules=['closed-context-regranted','unknown-fact-invalidates','scope-binding-bypassed','context-compared-by-public-shape','malformed-query-can-poison','observed-loss-not-persisted'];
  const holdRules=['hold-capacity-not-enforced','hold-issuance-not-enforced'];
  const a=queryRules.includes(m[0])?s.indexOf('export function queryNativeSourceContext('):holdRules.includes(m[0])?s.indexOf('export function holdNativeSourceContext('):0,b=queryRules.includes(m[0])?s.indexOf('export function stopNativeSourceContext(',a):holdRules.includes(m[0])?s.indexOf('export function releaseNativeSourceContextHold(',a):s.length;
  assert(a>=0&&b>a,m[0]);const region=s.slice(a,b);assert.equal(region.split(m[1]).length-1,1,m[0]);s=s.slice(0,a)+region.replace(m[1],m[2])+s.slice(b);
 }
 s=s.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,base).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,base).href)+',import.meta.url)');return 'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
}
async function graph(m=null){const child=url('native-source-contexts.mjs',m);return import(url('invocation-publication.mjs',null,{'./native-source-contexts.mjs':child}));}
const mutations=[
 ['recursive-hold-release-forgets-original-identity','||m.holds.get(ticket)!==reference||m.contexts.get(reference)!==ctx',''],
 ['parent-charge-omitted','owner.charge+=charge;','void charge;'],
 ['parent-headroom-not-checked','charge>owner.limits.maxHeldBytes-owner.charge','false'],
 ['source-capture-not-from-actual-root','source:d.source','source:[0]'],
 ['unsupported-data-silently-narrowed',"!equal(r.data,['source'])",'false'],
 ['scope-binding-bypassed','if(!ctx||ctx.stopped||!s||!equal(s,ctx.scope)','if(!ctx||ctx.stopped||!s||false'],
 ['context-compared-by-public-shape','m.contexts.get(r.snapshot)','[...m.contexts].find(([ref])=>equal(ref,r.snapshot))?.[1]'],
 ['closed-context-regranted','if(!ctx||ctx.stopped||!s||!equal(s,ctx.scope)','if(!ctx||false||!s||!equal(s,ctx.scope)'],
 ['observed-loss-not-persisted','{close(ctx,true);return no();}','{return no();}'],
 ['unknown-fact-invalidates',"loss.some(k=>g?.[k]===false)","loss.some(k=>g?.[k]!==true)"],
 ['malformed-query-can-poison','||!queryShape(ctx,r)',''],
 ['operation-object-reaches-property-coercion',"typeof op.value!=='string'||",''],
 ['truthy-facts-accepted','ks.every(k=>g?.[k]===true)','ks.every(k=>g?.[k])'],
 ['stop-releases-held-root','if(ctx.holds===0)ctx.snapshot=null;','ctx.snapshot=null;'],
 ['held-context-can-retire','ctx.holds!==0','false'],
 ['hold-can-release-twice','m.holds.delete(ticket);','void ticket;'],
 ['retirement-refunds-issuance','return m.contexts.delete(reference);','m.issued=0;return m.contexts.delete(reference);'],
 ['pool-charge-not-released','owner.charge-=m.charge;','void m.charge;'],
 ['pool-can-reopen-with-fresh-issuance','pools.has(owner)','pools.get(owner)&&!pools.get(owner).closed'],
 ['hold-capacity-not-enforced','m.holds.size>=m.profile.maxHolds','false'],
 ['hold-issuance-not-enforced','m.issued>=m.profile.maxIssued','false'],
 ['local-pool-charge-off-by-one','charge>p.maxHeldBytes','charge>p.maxHeldBytes+1'],
 ['parent-headroom-off-by-one','charge>owner.limits.maxHeldBytes-owner.charge','charge>owner.limits.maxHeldBytes-owner.charge+1'],
 ['root-envelope-off-by-one','JSON.stringify(owner.root))>p.maxRootBytes','JSON.stringify(owner.root))>p.maxRootBytes+1'],
 ['capture-envelope-off-by-one','copy({ref,source:retained?original.ctx.snapshot.source:d.source,profile:s.profile},p,p.maxCaptureBytes)','copy({ref,source:retained?original.ctx.snapshot.source:d.source,profile:s.profile},p,p.maxCaptureBytes+1)'],
 ['scope-envelope-off-by-one','copy(c,p,p.maxContextBytes)','copy(c,p,p.maxContextBytes+1)'],
 ['context-selector-envelope-off-by-one','copy(context,p,p.maxSelectorBytes)','copy(context,p,p.maxSelectorBytes+1)'],
 ['acquisition-reply-envelope-off-by-one','copy(reply,p,p.maxReplyBytes)','copy(reply,p,p.maxReplyBytes+1)'],
 ['input-working-reservation-omitted','+2*p.maxRequestBytes+p.maxCaptureBytes+2*p.maxContextBytes+2*p.maxReplyBytes','']
];
try{const api=await graph(),baseline=checkNativeSourceContexts(api),boundaries=checkSourceContextBoundaries(api),killed=[];await assert.rejects(()=>graph(['syntax-construction','const pools=','const pools=;']),{name:'SyntaxError'});for(const m of mutations){const mutant=await graph(m);assert.throws(()=>{checkNativeSourceContexts(mutant);checkSourceContextBoundaries(mutant);},{code:'ERR_ASSERTION'},m[0]);killed.push(m[0]);}console.log(JSON.stringify({baseline,boundaries,mutations:killed,coherentNativeKernelBaseline:true,constructionOutsideDetection:true,constructionControls:1,nativeF05ResultLifetimeOrPhysicalIndependentProof:false}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
