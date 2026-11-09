// Scoped native discovery lifetime reference; not host policy or production refinement.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as discovery from './native-discovery.mjs';
import {nativeDiscoveryFixture as fixture} from './native-discovery-checks.mjs';
export function checkDiscoveryInvalidation(api=discovery){
const q={operation:'list',limit:1,continuation:null},copy=structuredClone,flags=['authorized','scope','continuous','live','notStopped','discoveryAllowed','discoveryInterpretationCurrent','retained'];let gaps=0,unknowns=0,transients=0,isolations=0;
const seed=()=>{const x=fixture(api),r=x.open();assert.equal(x.query(r.context,q).status,'page');return {x,r};};
const exercise=(change,closed)=>{const {x,r}=seed(),before=copy(x.usage()),root=x.z.root(),producer=copy(x.z.usage());const options=change(x,r);assert.equal(x.query(r.context,options.request??q,options).status,'unavailable');const after=x.usage();assert.equal(after.contexts,before.contexts);assert.equal(after.issued,before.issued);assert.equal(after.declaredHeldBytes,before.declaredHeldBytes);assert.equal(after.stopped,closed?1:0);if(closed)assert.equal(after.selectors,0);assert.equal(x.query(r.context,q).status,closed?'unavailable':'page');assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),producer);return {x,r};};
for(const k of flags){exercise(x=>({queryG:x.g({[k]:false})}),true);gaps++;}
for(const k of flags)for(const value of [undefined,1,null]){exercise(x=>({queryG:x.g({[k]:value})}),false);unknowns++;}
for(const k of ['transferReady','replyReady','resourcesReady']){exercise(x=>({queryG:x.g({[k]:false})}),false);transients++;}
for(const mode of ['schema','presentation','metadata','entry','variant']){exercise(x=>{const f=x.visibility();if(mode==='schema')f[0].schema=false;if(mode==='presentation')f[0].presentation=false;if(mode==='metadata')x.rows[0].presentation.title='changed';if(mode==='entry'){Object.assign(f[0],{visible:false,schema:false,presentation:false,advice:false,order:null});f[0].variants.forEach(v=>Object.assign(v,{visible:false,order:null}));f[1].order=0;}if(mode==='variant')Object.assign(f[0].variants.at(-1),{visible:false,order:null});return {visibility:f};},true);gaps++;}
for(const k of ['authority','engine','recipient','purpose','destination','policy','continuity','profile']){exercise(x=>({queryScope:{...x.scope,[k]:'foreign'},queryG:x.g({authorized:false})}),false);isolations++;}
for(const value of [false,undefined,1]){exercise(x=>({queryG:x.g({outsidePublicationGuard:value,authorized:false})}),false);isolations++;}
exercise(x=>({request:{...q,extra:1},queryG:x.g({authorized:false})}),false);isolations++;
const {x,r}=exercise(x=>({queryG:x.g({retained:false})}),true),spent=x.usage().issued;
for(const k of ['ownerControlAllowed','retentionReleased','deliveryDrained'])for(const value of [false,1,undefined])assert.equal(api.retireNativeDiscovery(x.owner,r.context,x.g({[k]:value})).status,'unavailable');
assert.equal(api.retireNativeDiscovery(x.owner,r.context,x.g()).status,'retired');assert.equal(x.usage().issued,spent);assert.equal(x.open().status,'context');
const a=seed(),other=a.x.open();a.x.query(a.r.context,q,{queryG:a.x.g({retained:false})});assert.equal(a.x.query(other.context,q).status,'page');assert.equal(a.x.usage().stopped,1);
return {gaps,unknowns,transients,isolations,protectedDischargeCases:9,producerFramesChecked:true};
}
export async function checkDiscoveryInvalidationMutations(){
 const raw=fs.readFileSync(new URL('./native-discovery.mjs',import.meta.url),'utf8'),mutants=[
 ['observed-loss-ignored','.some(k=>g?.[k]===false)','.some(k=>false)'],
 ['unknown-loss-closes','if(!rootFor(m,g))return null;','if(!rootFor(m,g)){stop(m,s);return null;}'],
 ['publication-guard-ignored','if(g?.outsidePublicationGuard!==true||!request','if(!request'],
 ['foreign-scope-closes','!equal(s.scope,c)','false'],
 ['metadata-loss-not-absorbed','if(current===null||!equal(current,s.rows)){stop(m,s);return no();}','if(current===null||!equal(current,s.rows))return no();'],
 ['retention-loss-ignored',"'discoveryInterpretationCurrent','retained'","'discoveryInterpretationCurrent'"],
 ['retirement-without-discharge',"!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])",'false'],
 ['closure-refunds-issuance','s.stopped=true;s.rows=[]','m.issued=0;s.stopped=true;s.rows=[]']
 ];
 const load=s=>import('data:text/javascript;base64,'+Buffer.from(s.replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+new URL(p,new URL('./native-discovery.mjs',import.meta.url)).href+q)).toString('base64'));
 checkDiscoveryInvalidation(await load(raw));const killed=[];for(const [name,a,b]of mutants){assert(raw.includes(a),name);let failed=false;const loadedMutationApi=await load(raw.replace(a,b));try{checkDiscoveryInvalidation(loadedMutationApi);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name+': '+e.message);failed=true;}assert(failed,name);killed.push(name);}return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({scope:'owned-native-discovery-invalidation-only',...checkDiscoveryInvalidation(),mutations:await checkDiscoveryInvalidationMutations(),mutationBaselineChecked:true,actualHostPolicyTruthProved:false,allReaderFamiliesProved:false,implementationRefinement:false,independentCleanroomCertification:false},null,2));
