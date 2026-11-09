import assert from 'node:assert/strict';
import * as kernel from './invocation-publication.mjs';
import {lifecycleInvocationFixture} from './lifecycle-invocation-checks.mjs';
import {engineSettingsProfile} from './engine-settings-checks.mjs';
const z=lifecycleInvocationFixture(kernel,'dispose'),api=z.api,c=z.controlContext,document=z.lower.document;
const p={argumentLimits:Object.fromEntries(['maxDependencies','maxRequestBytes','maxIdentifierBytes','maxInputNodes','maxInputDepth'].map(k=>[k,engineSettingsProfile.argumentLimits[k]])),maxPlanBytes:engineSettingsProfile.maxPlanBytes};
const bp={serializationProfile:p,maxContexts:2,maxIssuedContexts:4,maxContextBytes:4096,maxHeldBytes:16384};
const h=(more={})=>({...z.g(),ownerControlAllowed:true,resourcesReady:true,outsidePublicationGuard:true,preparationReserved:true,settingsWritable:true,serializationBasisAllowed:true,serializationHostBasisAllowed:true,retentionReleased:true,deliveryDrained:true,sourceSerializationBasis:z.root(),sourceSerializationContext:c,sourceSerializationProfile:p,serializationWriteContext:{context:c,document,writeAllowed:true},...more});
try{
 const initial=api.invocationUsage(z.domain);assert(api.openInvocationSerializationBasisPool(z.domain,bp,h()));
 const b=api.captureInvocationSerializationBasis(z.domain,document,c,h());assert(b);assert(api.resolveInvocationSerializationBasis(z.domain,b,c,h()));
 assert.equal(z.submit().status,'accepted');assert.equal(z.run().status,'settled');assert.equal(z.outcome().outcome.publication,'applied');
 assert.equal(z.root().documents.find(d=>JSON.stringify(d.document)===JSON.stringify(document)).live,false);
 assert.equal(api.resolveInvocationSerializationBasis(z.domain,b,c,h()),null);
 assert.equal(api.captureInvocationSerializationBasis(z.domain,document,c,h()),null);
 assert.equal(api.retireInvocationSerializationBasis(z.domain,b,c,h({retentionReleased:false})),false);
 assert(api.retireInvocationSerializationBasis(z.domain,b,c,h()));assert.equal(api.invocationSerializationBasisUsage(z.domain).issued,1);
 assert(api.retireInvocationSerializationBasisPool(z.domain,h()));assert.equal(api.invocationUsage(z.domain).declaredHeldBytes,initial.declaredHeldBytes);
 console.log(JSON.stringify({scope:'isolated-native-disposal-metadata-basis-composition',actualLifecyclePublication:true,staleBasisRefused:true,protectedReleaseAndNonRefund:true,physicalOrHostTruthProved:false}));
}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
