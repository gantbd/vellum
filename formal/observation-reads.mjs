// Serialized trusted-host composition; no event-to-historical-selector conversion.
import {isDeepStrictEqual as equal} from 'node:util';
import {capturePublicationRead} from './publication-observation.mjs';
import {prepareReadContext,acquireReadContext} from './read-contexts.mjs';
export function acquireObservationRead(registry,owner,root,request,context,current,limits,g,packet=null){
 const fail=()=>({state:registry,result:{status:'unavailable'}});
 if(request?.selection?.kind!=='capture-current'||!current?.captureComplete||!current.preparationReserved)return fail();
 const captured=capturePublicationRead(owner,root,request.document,request.data,context,g,packet);if(!captured)return fail();
 if(!Object.entries(captured.scope).every(([key,value])=>equal(value,current[key])))return fail();
 const candidate=prepareReadContext(captured.root,request,current,limits);if(!candidate)return fail();
 return acquireReadContext(registry,candidate,current);
}
