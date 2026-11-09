// Actual-root source-load facet. No callback, provider procedure or public token codec.
import {prepareNativeSourceReadActivation} from './resource-read.mjs';
import {prepareRetainedLifecycleWriterGuard} from './source-save.mjs';
import {nativeSourceReadWriterBasis,attachNativeSourceReadWriter,discardNativeSourceRead} from './source-read-state.mjs';
import {discardRetainedLifecycleWriterGuard} from './retained-lifecycle-state.mjs';
import {installInvocationSourceRead} from './invocation-publication.mjs';
export function activateInvocationSourceRead(owner,task,writer,g){const p=prepareNativeSourceReadActivation(owner,task,g);if(!p)return {status:'unavailable'};let guard=null;try{const b=nativeSourceReadWriterBasis(p);guard=b&&prepareRetainedLifecycleWriterGuard(writer,b.document,b.binding,g);if(!guard||!attachNativeSourceReadWriter(p,guard))return {status:'unavailable'};return installInvocationSourceRead(b.domain,p,g);}finally{discardNativeSourceRead(p);if(guard)discardRetainedLifecycleWriterGuard(guard);}}
