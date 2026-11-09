// Owned history lifetime regression evidence, not actual policy or production refinement.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as native from './annotation-editing.mjs';
import * as kernel from './invocation-publication.mjs';
import * as reversal from './history-reversal.mjs';
import * as observer from './publication-observation.mjs';
import * as histories from './history-contexts.mjs';
import {historyContextFixture as fixture,historyContextAcquire as acquire,historyContextScope as c,historyContextGates as g} from './history-context-checks.mjs';
const original={...native,...kernel,...reversal,...observer,...histories};
export function checkHistoryInvalidation(api=original){
const setup=()=>{const z=fixture(api);z.execute(z.prepare({operation:'update-body'}));const r=acquire(api,z),q={contract:'history-query/1',operation:'frontier',context:r.context,scope:'scope-1',direction:'undo',profile:c.profile};assert.equal(api.queryHistoryInspection(z.history,q,c,g).status,'frontier');return {z,r,q};};let gaps=0,unknowns=0,transients=0;
for(const field of ['granted','retained','live','continuous','historyCurrent'])for(const value of [false,undefined,1,null]){const {z,r,q}=setup(),before=api.historyInspectionUsage(z.history),root=z.current();assert.equal(api.queryHistoryInspection(z.history,q,c,{...g,[field]:value}).status,'unavailable');assert.equal(api.queryHistoryInspection(z.history,q,c,g).status,value===false?'unavailable':'frontier');const after=api.historyInspectionUsage(z.history);assert.equal(after.contexts,before.contexts);assert.equal(after.issued,before.issued);assert.equal(after.declaredHeldBytes,before.declaredHeldBytes);assert.equal(z.current(),root);if(value===false)gaps++;else unknowns++;}
for(const field of ['transferReady','replyReady']){const {z,q}=setup();assert.equal(api.queryHistoryInspection(z.history,q,c,{...g,[field]:false}).status,'unavailable');assert.equal(api.queryHistoryInspection(z.history,q,c,g).status,'frontier');transients++;}
for(const field of Object.keys(c)){const {z,q}=setup();assert.equal(api.queryHistoryInspection(z.history,q,{...c,[field]:'foreign'},{...g,granted:false}).status,'unavailable');assert.equal(api.queryHistoryInspection(z.history,q,c,g).status,'frontier');}
for(const request of [q=>({...q,extra:1}),q=>({...q,direction:'surprise'}),q=>({...q,profile:'other'})]){const {z,q}=setup();assert.equal(api.queryHistoryInspection(z.history,request(q),c,{...g,granted:false}).status,'unavailable');assert.equal(api.queryHistoryInspection(z.history,q,c,g).status,'frontier');}
for(const value of [false,1,undefined]){const {z,r}=setup();assert.equal(api.closeHistoryInspection(z.history,r.context,c.recipient,{ownerControlAllowed:value}).status,'unavailable');assert.equal(api.stopHistoryInspection(z.history,c.recipient,{ownerControlAllowed:value}).status,'unavailable');assert.equal(api.historyInspectionUsage(z.history).live,1);}

let bridgeCases=0;
for(const field of ['granted','retained','live','continuous','historyCurrent']){const {z,r,q}=setup();assert.equal(api.historyInspectionReversalBindings(z.history,r.context,c,{...g,[field]:false}),null);assert.equal(api.queryHistoryInspection(z.history,q,c,g).status,'unavailable');bridgeCases++;}
const {z,r}=setup();assert.equal(api.closeHistoryInspection(z.history,r.context,c.recipient,{ownerControlAllowed:true}).status,'stopped');const before=api.historyInspectionUsage(z.history);
for(const field of ['ownerControlAllowed','retentionReleased','deliveryDrained'])for(const value of [false,1,undefined])assert.equal(api.retireHistoryInspection(z.history,r.context,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true,[field]:value}).status,'unavailable');
assert.deepEqual(api.historyInspectionUsage(z.history),before);assert.equal(api.retireHistoryInspection(z.history,r.context,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true}).status,'retired');assert.equal(api.historyInspectionUsage(z.history).issued,before.issued);
return {gaps,unknowns,transients,foreignScopes:Object.keys(c).length,malformedCases:3,exactControlCases:6,bridgeCases,protectedDischargeCases:9};
}
export async function checkHistoryInvalidationMutations(){
const raw=fs.readFileSync(new URL('./history-contexts.mjs',import.meta.url),'utf8'),mutants=[
 ['observed-loss-ignored','.some(k=>g?.[k]===false)','.some(k=>false)'],
 ['unknown-loss-closes','.some(k=>g?.[k]===false)','.some(k=>g?.[k]!==true)'],
 ['foreign-context-bypassed','!equal(o.context,c)','false'],
 ['nonboolean-control-accepted','g?.ownerControlAllowed!==true','!g?.ownerControlAllowed'],
 ['retirement-without-discharge','g.retentionReleased!==true||g.deliveryDrained!==true','false'],
 ['closed-context-revives','!e?.live','!e'],
 ['malformed-request-poisons','!exact(request,[...queries.common,...queries.variants[request.operation]])','false'],
 ['closure-refunds-issuance','{e.live=false;return null;}','{e.live=false;o.issued=0;return null;}']
];
const load=s=>import('data:text/javascript;base64,'+Buffer.from(s.replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+new URL(p,new URL('./history-contexts.mjs',import.meta.url)).href+q)).toString('base64'));
checkHistoryInvalidation({...original,...await load(raw)});const killed=[];for(const [name,a,b]of mutants){assert(raw.includes(a),name);const loadedMutationApi={...original,...await load(raw.replaceAll(a,b))};let failed=false;try{checkHistoryInvalidation(loadedMutationApi);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name+': '+e.message);failed=true;}assert(failed,name);killed.push(name);}return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({scope:'owned-native-history-invalidation-only',...checkHistoryInvalidation(),mutations:await checkHistoryInvalidationMutations(),mutationBaselineChecked:true,actualPolicyTruthProved:false,allReaderFamiliesProved:false,implementationRefinement:false,independentCleanroomCertification:false},null,2));
