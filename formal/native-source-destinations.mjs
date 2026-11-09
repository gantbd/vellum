// Trusted local source destination facets, not public verbs or a filesystem API.
import {prepareNativeSourceDestination,ordinarySourceSaveOperationPrior} from './source-save.mjs';
import {installInvocationSourceDestination,invocationAuthorityRoot} from './invocation-publication.mjs';
import {discardSourceDestinationTransition,readSourceDestinationTransition} from './source-destination-state.mjs';
import {readSourceSaveAsActivationAuthority} from './source-saveas-authority.mjs';
// A fixed SaveAs task can prepare only its existing local activation transition.
// Preparation does not commit producer flags/root, acknowledge I/O or issue work.
export function prepareInvocationSourceSaveAsActivation(authority,domain,profile,probe,g){const p=readSourceSaveAsActivationAuthority(authority,domain);if(!p)return null;const token=prepareNativeSourceDestination(p.owner,domain,profile,'activate',{task:p.task,probe},g);if(!token)return null;if(!readSourceDestinationTransition(token,domain,invocationAuthorityRoot(domain),g)){discardSourceDestinationTransition(token);return null;}return token;}
export function publishInvocationSourceDestination(domain,writer,profile,kind,input,g,operationContext=null){const token=prepareNativeSourceDestination(writer,domain,profile,kind,input,g,operationContext);if(!token)return {status:'unavailable'};try{return installInvocationSourceDestination(domain,token,g);}finally{discardSourceDestinationTransition(token);}}
export function requestInvocationOrdinarySourceSave(domain,writer,profile,request,c,g){
 const prior=ordinarySourceSaveOperationPrior(writer,domain,request,c,g);
 return prior.status==='unconsumed'?publishInvocationSourceDestination(domain,writer,profile,'ordinary-admit',prior.request,g,prior.context):prior;
}
