import assert from 'node:assert/strict';
import fs from 'node:fs';
import {checkOrdinaryGateway,checkOrdinaryGatewayFault} from './ordinary-gateway-checks.mjs';
const url=new URL('./invocation-publication.mjs',import.meta.url),original=fs.readFileSync(url,'utf8');
async function load(mutation=null){let source=original+'\nexport const ordinaryFaultProbe={attempts:0};\n';if(mutation){assert(source.includes(mutation[1]),mutation[0]);source=source.replace(mutation[1],mutation[2]);}source=source.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(new URL(p,url).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,url).href)+',import.meta.url)');return import('data:text/javascript;base64,'+Buffer.from(source).toString('base64'));}
const mutations=[
 ['canonical-capture-exceeds-owner-headroom','capture.bytes>o.limits.maxHeldBytes-o.charge','false'],
 ['canonical-capture-reservation-not-recorded','entry.ordinaryCharge=capture.bytes;o.charge+=capture.bytes;','entry.ordinaryCharge=0;'],
 ['producer-refusal-keeps-canonical-charge','if(entry.ordinaryCharge&&!entry.ordinaryPrepared&&!entry.configuredLinePrepared)','if(false)'],
 ['preparation-allows-reentrant-retirement','if(entry?.ordinaryPreparing)return unavailable();if((entry?.ordinaryPrepared||entry?.ordinaryCharge)','if((entry?.ordinaryPrepared||entry?.ordinaryCharge)'],
 ['changed-arguments-reuse-old-job','sameOrdinaryEditArguments(original,normalized)&&equal(held.capture,capture.value)','true'],
 ['replay-reprepares-from-current-root','if(entry.ordinaryPrepared){const held=entry.ordinaryPrepared,original=','if(false){const held=entry.ordinaryPrepared,original='],
 ['foreign-domain-borrows-native-plan','if(ordinaryOwner&&(ordinaryOwner.retired||ordinaryOwner.domain!==domain||ordinaryOwner.ticket!==ticket||entry.ordinaryPrepared?.job.plan!==job.plan))','if(false)'],
 ['strict-permissions-ignored',"return gates(g,ordinaryPermissionFields)&&(kind!=='boundary'||gates(g,['boundaryLive','boundaryRetained']));",'return true;'],
 ['noop-requires-unused-source-version','return gates(g,ordinaryPermissionFields)&&',"return gates(g,[...ordinaryPermissionFields,'versionsAvailable'])&&"],
['publication-permissions-not-revalidated',"entry.ordinaryPrepared&&(!nativeSearchConsumerCurrent(o,domain,entry.ordinaryPrepared,host)||!(entry.ordinaryPrepared.inputFamily==='F12'?selectionPermissions(host,entry.ordinaryPrepared.job.kind,entry.ordinaryPrepared.capture.arguments.request):entry.ordinaryPrepared.job.kind==='replacement'?replacementPermissions(host):ordinaryPermissions(host,entry.ordinaryPrepared.job.kind)))",'false'],
 ['retirement-refunds-lifetime-issuance','o.slots.delete(ticket);','o.issued--;o.slots.delete(ticket);']
];
try{const baseline=checkOrdinaryGateway(await load()),killed=[];for(const mutation of mutations){const api=await load(mutation);assert.throws(()=>checkOrdinaryGateway(api),{code:'ERR_ASSERTION'},mutation[0]);killed.push(mutation[0]);}
const faults=[];for(const [name,marker] of [['after-lease-before-native-capture','  const native=captureEditingIntent(normalized.kind,plan,'],['after-capture-before-job-retention','  const job=Object.freeze({kind:normalized.kind,plan});']]){const injected='ordinaryFaultProbe.attempts++;throw Error("injected post-lease fault");',api=await load([name,marker,injected+marker]);faults.push(checkOrdinaryGatewayFault(api));assert.equal(api.ordinaryFaultProbe.attempts,1,name);}
console.log(JSON.stringify({scope:baseline.scope,baselineControls:baseline.controls.length,permissionCases:baseline.permissionCases,completeCaptureBoundaries:baseline.completeCaptureBoundaries,coherentBaseline:true,mutations:killed,lateFaults:faults,constructionErrorsAreKills:false}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
