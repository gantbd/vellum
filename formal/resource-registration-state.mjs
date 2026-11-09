// Trusted metadata-only resource catalog activation. No provider callbacks or I/O.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {nativeSourceReadProfileValid} from './source-read-state.mjs';
const read=JSON.parse(fs.readFileSync(new URL('./resource-read.json',import.meta.url))),annotation=JSON.parse(fs.readFileSync(new URL('./annotation-resource-read.json',import.meta.url))),write=JSON.parse(fs.readFileSync(new URL('./source-save.json',import.meta.url))),plans=new WeakMap(),copy=structuredClone;
const natural=n=>Number.isSafeInteger(n)&&n>=0,size=v=>Buffer.byteLength(JSON.stringify(v)),exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const metadataVocabulary=freeze({insertionLineBreaks:['LF','CRLF','CR']});
export function nativeResourceMetadataVocabulary(){return metadataVocabulary;}
const label=(s,l)=>typeof s==='string'&&s.length>0&&Buffer.from(s).toString()===s&&Buffer.byteLength(s)<=l;
const identity=(r,l)=>exact(r,['authority','id','incarnation'])&&label(r.authority,l)&&label(r.id,l)&&natural(r.incarnation);
const slot=(table,p,r)=>JSON.stringify([table,p.authority,p.id,p.incarnation,r.authority,r.id,r.incarnation]);
const tableName=table=>table==='read'?'resourceRegistrations':table==='write'?'writeRegistrations':null;
export const nativeResourceRegistrationGates=Object.freeze(['authorized','scope','continuous','live','notStopped','registrationAllowed','identityAllowed','registryCoverageComplete','companionsReady','versionsAvailable','replyReady','resourcesReady','ownerControlAllowed','preparationReserved']);
function rowValid(table,row,l){const fields=table==='write'?write.registrationFields:row?.profile==='strict-utf8-one-bom/1'?read.registrationFields:row?.profile==='vellum.annotations/1'?annotation.registrationFields:null;if(!fields||!exact(row,fields)||!identity(row.provider,l)||!identity(row.resource,l)||!natural(row.generation)||!['identityDomain','identityKey','locator'].every(k=>label(row[k],l))||typeof row.identityKnown!=='boolean')return false;if(table==='write')return label(row.procedure,l)&&['logicalContinuity','syncSupported','conditionalWrite','causalReceipts','nonReuseTokens','exclusiveCreate'].every(k=>typeof row[k]==='boolean');return typeof row.atomicRead==='boolean'&&(row.profile==='vellum.annotations/1'||metadataVocabulary.insertionLineBreaks.includes(row.insertionLineBreak));}
function catalogsValid(root,profile){const l=profile?.lifecycle;if(!nativeSourceReadProfileValid(root,profile))return false;return ['read','write'].every(table=>{const rows=root[tableName(table)];return Array.isArray(rows)&&rows.length<=l.maxRelations&&rows.every(row=>rowValid(table,row,l.maxTagBytes))&&new Set(rows.map(row=>slot(table,row.provider,row.resource))).size===rows.length;});}
// Read-only native schema validation; this grants no registration mutation rights.
export function nativeResourceCatalogValid(root,profile){return catalogsValid(root,profile);}
function floor(owner,root){if(Object.hasOwn(owner,'registrationFloor'))return natural(owner.registrationFloor)?owner.registrationFloor:null;return Math.max(0,...root.resourceRegistrations.map(r=>r.generation),...root.writeRegistrations.map(r=>r.generation));}
function candidate(owner,before,request,profile){
 if(!catalogsValid(before,profile)||!exact(request,['changes'])||!Array.isArray(request.changes)||request.changes.length>profile.lifecycle.maxRelations||size(request)>profile.lifecycle.maxRequestBytes)return null;
 const base=floor(owner,before);if(base===null||[...before.resourceRegistrations,...before.writeRegistrations].some(r=>r.generation>base))return null;
 const next=copy(before),seen=new Set(),results=[];let changed=false,needsGeneration=false;
 for(const c of request.changes){if(!c||!tableName(c.table)||!['install','remove'].includes(c.action)||!exact(c,['table','action','provider','resource','expected',...(c.action==='install'?['configuration']:[])])||!identity(c.provider,profile.lifecycle.maxTagBytes)||!identity(c.resource,profile.lifecycle.maxTagBytes)||!(exact(c.expected,['state'])&&c.expected.state==='absent'||exact(c.expected,['state','generation'])&&c.expected.state==='present'&&natural(c.expected.generation)))return null;
  const key=slot(c.table,c.provider,c.resource);if(seen.has(key))return null;seen.add(key);const rows=next[tableName(c.table)],i=rows.findIndex(r=>equal(r.provider,c.provider)&&equal(r.resource,c.resource)),old=rows[i];if((c.expected.state==='present')!==(i!==-1)||i!==-1&&old.generation!==c.expected.generation)return null;
  const result={table:c.table,provider:copy(c.provider),resource:copy(c.resource),generation:old?.generation??null};results.push(result);
  if(c.action==='remove'){if(i!==-1){rows.splice(i,1);changed=true;}result.generation=null;continue;}
  if(!c.configuration||Object.hasOwn(c.configuration,'provider')||Object.hasOwn(c.configuration,'resource')||Object.hasOwn(c.configuration,'generation'))return null;const row={provider:copy(c.provider),resource:copy(c.resource),generation:old?.generation??0,...copy(c.configuration)};if(!rowValid(c.table,row,profile.lifecycle.maxTagBytes))return null;
  if(old&&equal(old,row))continue;if(base===Number.MAX_SAFE_INTEGER)return null;row.generation=base+1;result.generation=row.generation;needsGeneration=true;changed=true;if(i===-1)rows.push(row);else rows[i]=row;
 }
 if(changed){if(!natural(before.publication)||before.publication===Number.MAX_SAFE_INTEGER)return null;next.publication++;if(!catalogsValid(next,profile))return null;}
 return {next:changed?next:before,nextFloor:needsGeneration?base+1:base,result:{status:changed?'applied':'unchanged',registrations:results},floor:base};
}
// Only the actual native owner supplies this owner record and before root.
export function prepareNativeResourceRegistration(owner,domain,before,request,profile,g){if(!nativeResourceRegistrationGates.every(k=>g?.[k]===true))return null;const p=candidate(owner,before,request,profile);if(!p||size(p.result)>owner.limits.maxResultBytes)return null;const token=Object.freeze({kind:'native-resource-registration/1'});plans.set(token,{...p,owner,domain,before,request:freeze(copy(request)),profile:freeze(copy(profile))});return token;}
export function readNativeResourceRegistration(token,owner,domain,before,g){const p=plans.get(token);if(!p||p.owner!==owner||p.domain!==domain||p.before!==before||!nativeResourceRegistrationGates.every(k=>g?.[k]===true)||floor(owner,before)!==p.floor)return null;const current=candidate(owner,before,p.request,p.profile);return current&&current.nextFloor===p.nextFloor&&equal(current.result,p.result)&&(current.next===before?p.next===before:equal(current.next,p.next))&&size(p.result)<=owner.limits.maxResultBytes?p:null;}
export function commitNativeResourceRegistration(token){const p=plans.get(token);if(!p||floor(p.owner,p.before)!==p.floor)return null;const result=copy(p.result);p.owner.registrationFloor=p.nextFloor;plans.delete(token);return result;}
export function discardNativeResourceRegistration(token){return plans.delete(token);}
