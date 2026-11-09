import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import * as predicates from './read-dependencies.mjs';
import {invocationPublicationFixture as fixture,invocationCompoundJob as compound} from './invocation-publication-checks.mjs';
const original={...kernel,...predicates},copy=structuredClone;
const catalog=JSON.parse(fs.readFileSync(new URL('./read-dependencies.json',import.meta.url)));
const limits={maxDependencies:8,maxIdentifierBytes:128};
const gates=Object.fromEntries(['grant','currentScope','continuity','capacity','companions','notStopped','transferReady','uniqueDomainAttested','schedulerOwned','reservationHeld','ownerControlAllowed','retentionReleased','deliveryDrained'].map(k=>[k,true]));
// Use the actual owner's established gates, not a second approximation.
const invocationCatalog=JSON.parse(fs.readFileSync(new URL('./invocation-publication.json',import.meta.url)));
for(const k of [...invocationCatalog.admissionGates,...invocationCatalog.releaseGates])gates[k]=true;
const policy={publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:'reader'};
let seed;
function setup(api=original){
 if(!seed){const a=fixture();seed=copy(a.root());const c=copy(seed.documents[0]);c.document.id='C';seed.documents.push(c);}
 const z=fixture(api,copy(seed));
 const reads=()=>{const d=z.root().documents[2],document=copy(d.document);return [
  {kind:'source',document:copy(document),expectedSource:d.revision},
  {kind:'annotation-set',document:copy(document),expectedAnnotationSet:d.annotationSet},
  {kind:'annotation-body',document:copy(document),id:'note',incarnation:1,expectedBody:d.records[0].bodyVersion},
  {kind:'annotation-authored-target',document:copy(document),id:'note',incarnation:1,expectedTarget:d.records[0].targetGeneration}];};
 const host=rs=>({...z.h,readDependencyLimits:copy(limits),readDependencyAuthority:{contract:catalog.authorityContract,context:copy(z.c),grants:rs.map(dependency=>({dependency:copy(dependency),readAllowed:true,presenceAllowed:true,equalityAllowed:true}))}});
 const job=(rs=reads())=>{const j=compound(z.reference,z.root(),['A','B']);j.plan.contract=catalog.contract;j.plan.reads=copy(rs);return j;};
 return {...z,reads,host,job};
}
function* permutations(xs){if(!xs.length){yield [];return;}for(let i=0;i<xs.length;i++)for(const tail of permutations(xs.filter((_,j)=>j!==i)))yield [xs[i],...tail];}
export function checkReadDependencies(api=original){
 const controls=[];let predicateCases=0,permissionCases=0,permutationCases=0;
 const check=(name,f)=>{f();controls.push(name);};
 check('four-closed-current-predicates-and-stale-versions',()=>{for(let i=0;i<4;i++){const z=setup(api),r=z.reads()[i];assert(api.declaredReadsCurrent(z.root(),[r],z.host([r]).readDependencyAuthority,z.c,limits));const stale=copy(r);stale[catalog.versions[r.kind]]++;assert(!api.declaredReadsCurrent(z.root(),[stale],z.host([stale]).readDependencyAuthority,z.c,limits));predicateCases+=2;}});
 check('actual-native-compound-publishes-only-authored-participants',()=>{for(const route of ['typed','dynamic']){const z=setup(api),j=z.job(),before=copy(z.root().documents[2]);assert.equal(z.submit(j,route).status,'accepted');assert.equal(z.run(gates,z.host(j.plan.reads)).status,'settled');assert.equal(z.read().outcome.publication,'applied');assert.equal(z.root().groups.length,1);assert.equal(z.root().groups[0].scopes.length,2);assert.deepEqual(z.root().documents[2],before);assert(!JSON.stringify(z.root().groups).includes('"id":"C"'));}});
 check('conjunctive-read-permutations-share-owned-intent-on-both-routes',()=>{const z=setup(api),j=z.job();assert.equal(z.submit(j).status,'accepted');for(const rs of permutations(j.plan.reads)){assert.equal(z.submit({...j,plan:{...j.plan,reads:rs}},'dynamic').status,'pending');permutationCases++;}z.run(gates,z.host(j.plan.reads));assert.equal(z.read().outcome.publication,'applied');assert.equal(z.submit(j).status,'observe-existing');});
 check('admission-rejects-unknown-extra-missing-and-duplicate-read-products',()=>{for(const mode of ['kind','extra','missing','duplicate','conflicting','old-with-reads']){const z=setup(api),j=z.job();if(mode==='kind')j.plan.reads[0].kind='evaluate';if(mode==='extra')j.plan.reads[0].expression='true';if(mode==='missing')delete j.plan.reads;if(mode==='duplicate')j.plan.reads.push(copy(j.plan.reads[0]));if(mode==='conflicting')j.plan.reads.push({...copy(j.plan.reads[0]),expectedSource:2});if(mode==='old-with-reads')j.plan.contract=catalog.baseContract;const u=z.usage();assert.equal(z.submit(j).status,'unavailable',mode);assert.deepEqual(z.usage(),u);}});
 check('permission-facts-are-exact-true-complete-and-current-context-bound',()=>{for(let mask=0;mask<8;mask++){const z=setup(api),r=[z.reads()[0]],h=z.host(r);catalog.permissionFields.forEach((k,i)=>h.readDependencyAuthority.grants[0][k]=!!(mask&(1<<i)));assert.equal(api.declaredReadsCurrent(z.root(),r,h.readDependencyAuthority,z.c,limits),mask===7);permissionCases++;}for(const key of catalog.contextFields){const z=setup(api),r=z.reads(),h=z.host(r);h.readDependencyAuthority.context[key]='other';assert(!api.declaredReadsCurrent(z.root(),r,h.readDependencyAuthority,z.c,limits));}for(const mode of ['missing','extra','duplicate','predicate','truthy']){const z=setup(api),r=z.reads(),h=z.host(r),gs=h.readDependencyAuthority.grants;if(mode==='missing')gs.pop();if(mode==='extra')gs.push(copy(gs[0]));if(mode==='duplicate')gs[1]=copy(gs[0]);if(mode==='predicate')gs[0].dependency.expectedSource++;if(mode==='truthy')gs[0].readAllowed=1;assert(!api.declaredReadsCurrent(z.root(),r,h.readDependencyAuthority,z.c,limits),mode);}});
 check('missing-disposed-foreign-and-reincarnated-subjects-refuse',()=>{for(const mode of ['missing','disposed','foreign','document-incarnation','record-incarnation']){const z=setup(api),r=z.reads();if(mode==='missing')r[0].document.id='absent';if(mode==='disposed'){const s=copy(z.root());s.documents[2].live=false;assert(!api.declaredReadsCurrent(s,r,z.host(r).readDependencyAuthority,z.c,limits));continue;}if(mode==='foreign')r[0].document.authority='other';if(mode==='document-incarnation')r[0].document.incarnation++;if(mode==='record-incarnation')r[2].incarnation++;assert(!api.declaredReadsCurrent(z.root(),r,z.host(r).readDependencyAuthority,z.c,limits),mode);}});
 check('read-limits-and-identifiers-are-finite-exact-and-scalar-valid',()=>{const z=setup(api),r=z.reads();for(const l of [{...limits,maxDependencies:3},{...limits,maxIdentifierBytes:0},{...limits,maxDependencies:Infinity},{...limits,extra:1}])assert(!api.validReadDependencies(r,l));for(const value of ['', '\ud800']){const rs=copy(r);rs[0].document.id=value;assert(!api.validReadDependencies(rs,limits));}const rs=copy(r);delete rs[1];assert(!api.validReadDependencies(rs,limits));});
 check('stale-read-refuses-whole-root-even-for-a-noop',()=>{for(const noop of [false,true]){const z=setup(api),j=z.job();j.plan.reads[0].expectedSource++;if(noop){j.plan.sources=[];j.plan.annotations=[];}const before=z.root();assert.equal(z.submit(j).status,'accepted');z.run(gates,z.host(j.plan.reads));assert.equal(z.read().outcome.publication,'notApplied');assert.equal(z.root(),before);}});
 check('read-and-write-same-document-validates-before-candidate-change',()=>{const z=setup(api),j=z.job([{kind:'source',document:copy(z.root().documents[0].document),expectedSource:1}]);assert.equal(z.submit(j).status,'accepted');z.run(gates,z.host(j.plan.reads));assert.equal(z.read().outcome.publication,'applied');assert.equal(z.root().documents[0].revision,2);});
 check('empty-conjunction-preserves-legacy-behavior-without-read-grants',()=>{for(const empty of [false,true]){const z=setup(api),j=z.job([]);if(!empty){delete j.plan.reads;j.plan.contract=catalog.baseContract;}assert.equal(z.submit(j).status,'accepted');z.run();assert.equal(z.read().outcome.publication,'applied');}});
 check('frozen-read-intent-is-not-a-caller-alias',()=>{const z=setup(api),j=z.job(),rs=copy(j.plan.reads);assert.equal(z.submit(j).status,'accepted');j.plan.reads[0].expectedSource++;j.plan.reads.length=0;z.run(gates,z.host(rs));assert.equal(z.read().outcome.publication,'applied');});
 check('intervening-native-read-object-edit-is-rechecked-at-final-publication',()=>{const z=setup(api),j=z.job();assert.equal(z.submit(j).status,'accepted');const next=api.issueInvocation(z.domain,z.c,gates),other=compound(next.reference,z.root(),['C']);assert.equal(api.submitInvocation(z.domain,next.ticket,other,'typed',z.c,policy,gates).status,'accepted');assert.equal(api.runInvocation(z.domain,next.ticket,z.c,gates,z.h).status,'settled');const before=z.root();assert.equal(before.documents[2].revision,2);z.run(gates,z.host(j.plan.reads));assert.equal(z.root(),before);assert.equal(z.read().outcome.publication,'notApplied');assert.equal(z.root().documents[0].revision,1);});
 check('unrelated-native-publication-does-not-stale-a-fine-grained-body-read',()=>{const z=setup(api),j=z.job([z.reads()[2]]);assert.equal(z.submit(j).status,'accepted');const next=api.issueInvocation(z.domain,z.c,gates),other=compound(next.reference,z.root(),['C']);api.submitInvocation(z.domain,next.ticket,other,'dynamic',z.c,policy,gates);api.runInvocation(z.domain,next.ticket,z.c,gates,z.h);const c=copy(z.root().documents[2]);z.run(gates,z.host(j.plan.reads));assert.equal(z.read().outcome.publication,'applied');assert.equal(z.root().publication,2);assert.deepEqual(z.root().documents[2],c);});
 check('grant-order-is-not-semantic-and-host-cannot-substitute-owner-context',()=>{const z=setup(api),j=z.job(),h=z.host(j.plan.reads);h.readDependencyAuthority.grants.reverse();assert.equal(z.submit(j).status,'accepted');z.run(gates,h);assert.equal(z.read().outcome.publication,'applied');const a=setup(api),k=a.job(),bad=a.host(k.plan.reads);bad.readDependencyAuthority.context.recipient='attacker';bad.readContext=copy(bad.readDependencyAuthority.context);a.submit(k);a.run(gates,bad);assert.equal(a.read().outcome.publication,'notApplied');});
 return {scope:catalog.scope,predicateCases,permissionCases,permutationCases,controls,actualNativePublicationChecked:true,allSemanticAssumptionsDiscovered:false,actualPolicyTruthProved:false,publicCodecSelected:false,implementationRefinement:false,independentCleanroomCertification:false};
}
export async function checkReadDependencyMutations(){
 const names=['read-dependencies.mjs','annotation-compounds.mjs','invocation-publication.mjs'],raw=Object.fromEntries(names.map(n=>[n,fs.readFileSync(new URL(n,import.meta.url),'utf8')]));
 const mutations=[
  ['read-source-version-ignored',names[0],'d.revision!==r.expectedSource','false'],
  ['read-set-version-ignored',names[0],'d.annotationSet!==r.expectedAnnotationSet','false'],
  ['read-body-version-ignored',names[0],'row.bodyVersion!==r.expectedBody','false'],
  ['read-target-version-ignored',names[0],'row.targetGeneration!==r.expectedTarget','false'],
  ['read-grant-context-ignored',names[0],'!equal(authority.context,context)','false'],
  ['read-permissions-truthy',names[0],'g[k]===true','!!g[k]'],
  ['read-duplicate-subject-allowed',names[0],'new Set(reads.map(key)).size===reads.length','true'],
  ['read-count-bound-ignored',names[0],'reads.length<=l.maxDependencies','true'],
  ['read-predicate-grant-mismatch-allowed',names[0],'!equal(grants.get(key(r)),r)','false'],
  ['read-publication-check-omitted',names[1],'if(readPlan&&!declaredReadsCurrent','if(false&&!declaredReadsCurrent'],
  ['read-canonicalization-omitted',names[0],'.sort((a,b)=>Buffer.compare(Buffer.from(key(a)),Buffer.from(key(b))))',''],
  ['read-owner-context-not-forwarded',names[2],'compound(before,entry.plan,host,entry.context)','compound(before,entry.plan,host,null)']
 ];
 const load=async mutation=>{const source={...raw},urls={};if(mutation){assert(source[mutation[1]].includes(mutation[2]),mutation[0]);source[mutation[1]]=source[mutation[1]].replace(mutation[2],mutation[3]);}for(const n of names){const body=source[n].replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+(urls[p.slice(2)]??new URL(p,import.meta.url).href)+q);urls[n]='data:text/javascript;base64,'+Buffer.from(body+'\n// '+(mutation?.[0]??'baseline')).toString('base64');}return {...await import(urls[names[0]]),...await import(urls[names[2]])};};
 checkReadDependencies(await load());const killed=[];for(const m of mutations){let failed=false;const loadedMutationApi=await load(m);try{checkReadDependencies(loadedMutationApi);}catch(e){assert.equal(e.code,'ERR_ASSERTION',m[0]+': '+e.message);failed=true;}assert(failed,m[0]);killed.push(m[0]);}return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkReadDependencies(),mutations:await checkReadDependencyMutations(),mutationBaselineChecked:true},null,2));
