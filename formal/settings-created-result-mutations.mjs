import assert from 'node:assert/strict';
import {createdSettingsTestGraph as load} from './settings-created-result-test-graph.mjs';
import {checkCreatedAtomic} from './settings-created-result-checks.mjs';
import {scopedLifecycleFixture} from './scoped-lifecycle-checks.mjs';
const mutants=[
 ['admission-ceiling-widens','state','g?.settingsCreatedResultReferencesAllowed===true','true'],
 ['committed-receipt-not-installed','kernel','if(settingsCreatedStage&&!commitCreatedSettingsPublication','if(false&&!commitCreatedSettingsPublication'],
 ['created-result-stage-skipped','kernel',"if(entry.settingsCreatedHold&&['applied','unchanged'].includes(result.status))","if(false&&['applied','unchanged'].includes(result.status))"],
 ['result-reservation-not-charged','state','owner.charge+=charge;','owner.charge+=0;'],
 ['protected-charge-not-refunded','state','owner.charge-=s.charge;','owner.charge-=0;'],
 ['captured-ceiling-not-enforced','state','!s.ceiling||','false||'],
 ['current-result-grants-truthy','state','keys.every(k=>g?.[k]===true)','keys.every(k=>g?.[k])'],
 ['result-envelope-preflight-ignored','state','size(data)>owner.limits.maxResultBytes||size(frame)>owner.limits.maxResultBytes','false'],
 ['parent-reservation-capacity-ignored','state','charge>owner.limits.maxHeldBytes-owner.charge','false'],
 ['created-indices-alias-first-layer','state','references.set(s.refs[index],{hold,index});','references.set(s.refs[index],{hold,index:0});'],
 ['reference-owner-binding-ignored','state','r?.hold!==hold','!r'],
 ['common-outcome-release-ignored','state','!summary(s,policy)||','false||'],
 ['basis-permission-manufactured','state','s.data.profile,g);','s.data.profile,{...g,settingsBasisAllowed:true});'],
];
try{const baseline=checkCreatedAtomic(await load()),killed=[];for(const m of mutants){const api=await load(m);let failed=false;try{checkCreatedAtomic(api);}catch(e){assert.equal(e.code,'ERR_ASSERTION',m[0]+': '+String(e.message).slice(0,170));failed=true;}assert(failed,m[0]+'/survived');killed.push(m[0]);}
 let faultCases=0;
 for(const [name,from,to] of [
  ['after-stage',"const current=publishNativeEngineSettings(before,s.plan,g,owner,domain,ticket);","throw Error('injected after stage');const current=publishNativeEngineSettings(before,s.plan,g,owner,domain,ticket);"],
  ['after-reference-registration','references.set(s.refs[index],{hold,index});return true;',"references.set(s.refs[index],{hold,index});throw Error('injected after private references');return true;"]
 ]){const api=await load([name,'state',from,to]),x=scopedLifecycleFixture(api,{host:{settingsCreatedResultReferencesAllowed:true,settingsCreatedResultReadable:true,settingsCreatedResultContextCurrent:true,historicalSettingsLayersReadable:true}}),before=x.root(),usage=api.invocationUsage(x.domain),hold=api.prepareInvocationSettingsCreatedResult(x.domain,x.ticket,x.c,x.h());assert(hold);assert.equal(x.submit().status,'accepted');assert.equal(x.run().status,'faulted');assert.equal(x.root(),before);assert.equal(x.read().outcome.publication,'unknown');assert.equal(api.observeSettingsCreatedResult(x.domain,x.ticket,x.c,x.policy,x.h()).status,'unavailable');assert.equal(api.invocationSettingsCreatedUsage(x.domain,x.ticket,x.c).references,0);assert.equal(api.issueInvocation(x.domain,x.c,x.h()),null);assert.equal(api.retireInvocation(x.domain,x.ticket,x.h({retentionReleased:false})).status,'unavailable');assert.equal(api.retireInvocation(x.domain,x.ticket,x.h()).status,'retired');assert.equal(api.invocationUsage(x.domain).declaredHeldBytes,usage.declaredHeldBytes);assert.equal(api.invocationUsage(x.domain).issued,usage.issued);faultCases++;}
 console.log(JSON.stringify({scope:'isolated-created-settings-atomic-mutations',baseline,constructionOutsideDetection:true,mutations:killed,afterStageAndPrivateRegistrationFaultCases:faultCases,physicalFaultContainmentOrAllFaultsProved:false}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
