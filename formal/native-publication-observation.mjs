// Actual-domain composition of the existing observer. Internal native products,
// not a public codec, dispatcher, transport or whole-observer retirement contract.
import {invocationAuthorityRoot,invocationAuthorityContext,invocationExecutionState,beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment} from './invocation-publication.mjs';
import {openPublicationObserver,observePublicationRoot,capturePublicationRead,releaseBoundPublicationEvent,resyncPublicationObserver,publicationObserverClaimMatches,captureNativePublicationFacts,releaseBoundPublicationOutcome,retirePublicationOutcome,stopPublicationObserver,publicationObserverUsage} from './publication-observation.mjs';
const owners=new WeakMap(),marker=Object.freeze({profile:'recipient-publication-observation-reference/2'}),unavailable=()=>({status:'unavailable'}),size=v=>Buffer.byteLength(JSON.stringify(v)),natural=n=>Number.isSafeInteger(n)&&n>=0;
const recordBytes=size({domain:{kind:'invocation-domain/1'},rawOwner:marker,attachment:{kind:'native-owner-attachment/1'},maxOutcomeBytes:Number.MAX_SAFE_INTEGER});
export function openInvocationPublicationObserver(domain,context,limits,g,packet=null){
 if(!limits||typeof limits!=='object'||Array.isArray(limits))return null;const captured={...limits},units=captured.maxHeldBytes+recordBytes;if(!natural(units))return null;
 const attachment=beginInvocationOwnerAttachment(domain,'publication-observer',units,marker);if(!attachment)return null;
 try{
  if(g?.ownerControlAllowed!==true||g?.resourcesReady!==true)return null;
  const opened=openPublicationObserver(invocationAuthorityRoot(domain),context,captured,g,packet);if(!opened)return null;
  const owner=Object.freeze({...marker});owners.set(owner,{domain,rawOwner:opened.owner,attachment,maxOutcomeBytes:captured.maxOutcomeBytes});
  if(!commitInvocationOwnerAttachment(domain,attachment,owner))throw Error('native publication observer attachment invariant');
  return {owner,baseline:opened.baseline};
 }finally{abortInvocationOwnerAttachment(domain,attachment);}
}
// Engine-driven current-root service. It takes no caller-supplied root or loss
// claim. Current context/visibility/policy facts remain trusted supplier premises.
export function observeInvocationPublicationRoot(owner,currentContext,g,packet=null){const o=owners.get(owner);return o?observePublicationRoot(o.rawOwner,invocationAuthorityRoot(o.domain),currentContext,g,packet):unavailable();}
// Caller binding precedes reading attributable current facts or consuming packets.
export function captureInvocationPublicationRead(owner,claimed,document,data,currentContext,g,packet=null){const o=owners.get(owner);return o&&publicationObserverClaimMatches(o.rawOwner,claimed)?capturePublicationRead(o.rawOwner,invocationAuthorityRoot(o.domain),document,data,currentContext,g,packet):null;}
export function releaseInvocationPublicationEvent(owner,claimed,currentContext,g){const o=owners.get(owner);return o?releaseBoundPublicationEvent(o.rawOwner,claimed,currentContext,g):unavailable();}
export function resyncInvocationPublicationObserver(owner,claimed,currentContext,g,packet=null){const o=owners.get(owner);return o&&publicationObserverClaimMatches(o.rawOwner,claimed)?resyncPublicationObserver(o.rawOwner,invocationAuthorityRoot(o.domain),currentContext,g,packet):unavailable();}
// Original actual ticket -> native retained facts -> observer's captured ceiling.
// No before/result payload, root equality claim, new attempt or new correlation
// string is accepted from a caller. Current outcome authorization is still needed.
export function captureInvocationPublicationOutcome(owner,ticket,currentContext,g){
 const o=owners.get(owner);if(!o)return null;const state=invocationExecutionState(o.domain,ticket,invocationAuthorityContext(o.domain),true,o.maxOutcomeBytes);
 if(state?.phase!=='terminal'||!state.prepared)return null;
 return captureNativePublicationFacts(o.rawOwner,state.prepared.facts,state.reference,currentContext,g);
}
export function releaseInvocationPublicationOutcome(owner,handle,claimed,currentContext,g){const o=owners.get(owner);return o?releaseBoundPublicationOutcome(o.rawOwner,handle,claimed,currentContext,g):unavailable();}
export function retireInvocationPublicationOutcome(owner,handle,g){const o=owners.get(owner);if(!o)return unavailable();const facts={retentionReleased:g?.retentionReleased===true,deliveryDrained:g?.deliveryDrained===true};return facts.retentionReleased&&facts.deliveryDrained?retirePublicationOutcome(o.rawOwner,handle,facts):unavailable();}
export function stopInvocationPublicationObserver(owner,g){const o=owners.get(owner);return o&&g?.ownerControlAllowed===true?stopPublicationObserver(o.rawOwner,{ownerControlAllowed:true}):unavailable();}
export function stopBoundInvocationPublicationObserver(owner,claimed,g){const o=owners.get(owner);return o&&publicationObserverClaimMatches(o.rawOwner,claimed)?stopInvocationPublicationObserver(owner,g):unavailable();}
// Trusted supervision, not recipient reflection. Stop retains native owner,
// outcomes, reference bindings, spent origins and original parent reservation.
export function invocationPublicationObserverUsage(owner){const o=owners.get(owner);return o?{...publicationObserverUsage(o.rawOwner),wholeOwnerRetirementSupported:false}:null;}
