// Fixed native guard declarations, not registration, authority or a wire codec.
import fs from 'node:fs';
const read=name=>JSON.parse(fs.readFileSync(new URL(name,import.meta.url),'utf8'));
const wrapper=read('./lifecycle-invocations.json'),shared=read('./invocation-publication.json'),relations=read('./relation-invocations.json'),exportsCatalog=read('./source-export.json');
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const wrap=(id,variant,profile,nested)=>({id,family:wrapper.family,variant,contract:wrapper.contract,operation:wrapper.operation,profile,fields:[...wrapper.fields],matches:{},nested});
export const sourceLoadInvocationForm=freeze(wrap('source-load','activate-loaded-candidate','native-source-load/1',{fields:[],matches:{}}));
export const annotationRestoreInvocationForm=freeze(wrap('annotation-restore','activate-loaded-candidate','native-annotation-load/1',{fields:['phase','context','binding'],matches:{phase:'restore'}}));
export const annotationBaselineInvocationForm=freeze(wrap('annotation-baseline','activate-loaded-candidate','native-annotation-load/1',{fields:['phase'],matches:{phase:'adopt-baseline'}}));
export const sourceSaveAsInvocationForm=freeze(wrap('source-saveas','rebind','native-source-saveas/1',{fields:['probe'],matches:{}}));
export const sourceRebindInvocationForm=freeze(wrap('source-rebind','rebind',wrapper.profile,{fields:[...exportsCatalog.rebindFields],matches:{contract:exportsCatalog.rebindContract,operation:'activate-source-binding'}}));
const relationForms=Object.keys(relations.operations).flatMap(operation=>Object.keys(relations.actions).map(action=>({id:operation+'-'+action,family:relations.family,variant:operation,contract:relations.contract,operation,profile:relations.profile,fields:[...relations.common,...relations.actions[action]],matches:{action},nested:null})));
const forms=freeze([sourceLoadInvocationForm,annotationRestoreInvocationForm,annotationBaselineInvocationForm,sourceSaveAsInvocationForm,sourceRebindInvocationForm,...relationForms]);
export function lifecycleActivationInputForms(){return forms;}
export const lifecycleActivationContextFields=freeze([...shared.contextFields]);
