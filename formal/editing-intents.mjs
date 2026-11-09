import {configuredLineIntent,publishConfiguredLine} from './configured-line-state.mjs';
// Closed trusted producer adapters. No callback/plugin registration or public ABI.
import {selectionInvocationIntent,publishSelectionEdit} from './selection-products.mjs';
import {lineInvocationIntent,publishLineEdit} from './line-edits.mjs';
import {boundaryInvocationIntent,publishBoundaryCommand} from './boundary-commands.mjs';
import {replacementInvocationIntent,publishReplacement} from './literal-search.mjs';
import {annotationInvocationIntent,publishAnnotationEdit,admitAnnotationResult,captureAnnotationEditResult,releaseAnnotationResult} from './annotation-editing.mjs';
import {nativeEngineSettingsIntent,publishNativeEngineSettings} from './engine-settings-state.mjs';
import {sourceSerializationIntent,publishSourceSerialization} from './source-serialization-state.mjs';
export {admitAnnotationResult,captureAnnotationEditResult,releaseAnnotationResult};
export function captureEditingIntent(kind,plan,maximum,owner,domain,ticket){
 switch(kind){
  case 'selection':return selectionInvocationIntent(plan,maximum);
  case 'line':return configuredLineIntent(plan,maximum,owner,domain,ticket)??lineInvocationIntent(plan,maximum);
  case 'boundary':return boundaryInvocationIntent(plan,maximum);
  case 'replacement':return replacementInvocationIntent(plan,maximum);
  case 'annotation':return annotationInvocationIntent(plan,maximum);
  case 'settings':return sourceSerializationIntent(plan,maximum,owner,domain,ticket)??nativeEngineSettingsIntent(plan,maximum,owner,domain,ticket);
  default:return null;
 }
}
export function publishEditingIntent(kind,state,plan,host,owner,domain,ticket){
 switch(kind){
  case 'selection':return publishSelectionEdit(state,plan,host);
  case 'line':return publishConfiguredLine(state,plan,host,owner,domain,ticket)??publishLineEdit(state,plan,host,host.viewLimits,host.lineWorkLimits);
  case 'boundary':return publishBoundaryCommand(state,plan,host,host.viewLimits,host.boundaryWorkLimits);
  case 'replacement':return publishReplacement(state,plan,host.searchCurrent,host);
  case 'annotation':return publishAnnotationEdit(state,plan,host);
  case 'settings':return publishSourceSerialization(state,plan,host,owner,domain,ticket)??publishNativeEngineSettings(state,plan,host,owner,domain,ticket);
  default:return {status:'not-published',state};
 }
}
