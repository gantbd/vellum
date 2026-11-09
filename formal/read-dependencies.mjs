// Fixed read predicates only: no arbitrary property paths, validators or callbacks.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {isSourceScopePlan,sourceScopePlanFields,normalizeSourceScopes} from './source-scopes.mjs';
import {validatedEngineSettingsState} from './engine-settings-state.mjs';
const C=JSON.parse(fs.readFileSync(new URL('./read-dependencies.json',import.meta.url))),nat=n=>Number.isSafeInteger(n)&&n>=0;
const exact=(v,keys)=>v!==null&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const dense=a=>Array.isArray(a)&&Object.keys(a).length===a.length&&Object.keys(a).every((k,i)=>k===String(i));
const text=(s,n)=>typeof s==='string'&&s.length>0&&s.length<=n&&Buffer.byteLength(s)<=n&&Buffer.from(s).toString('utf8')===s;
const identity=(d,n)=>exact(d,['authority','id','incarnation'])&&text(d.authority,n)&&text(d.id,n)&&nat(d.incarnation);
const limitsValid=l=>exact(l,C.limitFields)&&Object.values(l).every(nat);
const configuration=r=>Object.hasOwn(C.configurationKinds,r.kind),version=n=>nat(n)&&n>0;
const accepted=profile=>profile===C.contract||isSourceScopePlan({contract:profile});
const key=r=>r.kind==='workspace-membership'?JSON.stringify([r.kind,r.workspace.authority,r.workspace.id,r.workspace.incarnation]):r.kind==='configuration-state'?JSON.stringify([r.kind,r.authority]):r.kind==='configuration-domain'?JSON.stringify([r.kind,r.target.authority,r.target.id,r.target.incarnation,r.domain]):JSON.stringify([r.kind,r.document.authority,r.document.id,r.document.incarnation,...(Object.hasOwn(r,'id')?[r.id,r.incarnation]:[])]);
function readValid(r,l,profile=C.contract){
 if(r===null||typeof r!=='object')return false;
 if(r.kind==='workspace-membership')return profile===C.workspaceContract&&exact(r,C.workspaceFields)&&identity(r.workspace,l.maxIdentifierBytes)&&nat(r.expectedWorkspace);
 if(configuration(r))return C.configurationContracts.includes(profile)&&exact(r,C.configurationKinds[r.kind])&&['expectedSchema','expectedRestrictions','expectedAssociation'].every(k=>version(r[k]))&&(r.kind==='configuration-state'?text(r.authority,l.maxIdentifierBytes)&&version(r.expectedConfiguration):identity(r.target,l.maxIdentifierBytes)&&C.configurationDomains.includes(r.domain)&&version(r.expectedGeneration));
 return Object.hasOwn(C.kinds,r.kind)&&exact(r,C.kinds[r.kind])&&identity(r.document,l.maxIdentifierBytes)&&nat(r[C.versions[r.kind]])&&(!Object.hasOwn(r,'id')||text(r.id,l.maxIdentifierBytes)&&nat(r.incarnation));
}
export function validReadDependencies(reads,l,profile=C.contract){return accepted(profile)&&limitsValid(l)&&dense(reads)&&reads.length<=l.maxDependencies&&reads.every(r=>readValid(r,l,profile))&&new Set(reads.map(key)).size===reads.length;}
export function isReadDependencyPlan(plan){return plan?.contract===C.contract||isSourceScopePlan(plan);}
export function readDependencyPlanFields(plan){return isSourceScopePlan(plan)?sourceScopePlanFields():[...C.planFields];}
export function normalizeReadDependencyPlan(plan,maximum){
 if(!nat(maximum)||!exact(plan,readDependencyPlanFields(plan))||!isReadDependencyPlan(plan)||!Array.isArray(plan.sources)||!Array.isArray(plan.annotations)||!validReadDependencies(plan.reads,{maxDependencies:maximum,maxIdentifierBytes:maximum},plan.contract))return null;
 const scopes=isSourceScopePlan(plan)?normalizeSourceScopes(plan.scopes,{maxScopes:maximum,maxReplacementRanges:maximum,maxInsertionIntervals:maximum,maxIdentifierBytes:maximum}):null;if(isSourceScopePlan(plan)&&!scopes)return null;
 // Canonicalize only the conjunctive read set; do not reorder/coalesce edits.
 return {...structuredClone(plan),reads:structuredClone(plan.reads).sort((a,b)=>Buffer.compare(Buffer.from(key(a)),Buffer.from(key(b)))),...(scopes?{scopes}:{})};
}
export function declaredReadsCurrent(state,reads,authority,context,l,profile=C.contract,configurationProfile=null){
 if(!accepted(profile))return false;
 if(dense(reads)&&reads.length===0)return true;
 if(!validReadDependencies(reads,l,profile)||!exact(context,C.contextFields)||context.authority!==state.authority||!exact(authority,C.authorityFields)||authority.contract!==(C.configurationContracts.includes(profile)?C.configurationAuthorityContract:C.authorityContract)||!equal(authority.context,context)||!dense(authority.grants)||authority.grants.length!==reads.length)return false;
 const grants=new Map();for(const g of authority.grants){if(!exact(g,C.grantFields)||!readValid(g.dependency,l,profile)||!C.permissionFields.every(k=>g[k]===true)||grants.has(key(g.dependency)))return false;grants.set(key(g.dependency),g.dependency);}
 const cfg=reads.some(configuration)?validatedEngineSettingsState(state,configurationProfile):null;if(reads.some(configuration)&&!cfg)return false;
 for(const r of reads){if(!equal(grants.get(key(r)),r))return false;
  if(r.kind==='workspace-membership'){const rows=Array.isArray(state.workspaces)?state.workspaces.filter(w=>equal(w?.workspace,r.workspace)):[];if(r.workspace.authority!==state.authority||rows.length!==1||rows[0].live!==true||!nat(rows[0].generation)||rows[0].generation!==r.expectedWorkspace)return false;continue;}
  if(configuration(r)){
   if(cfg.schemaGeneration!==r.expectedSchema||cfg.restrictionGeneration!==r.expectedRestrictions||cfg.associationGeneration!==r.expectedAssociation)return false;
   if(r.kind==='configuration-state'){if(r.authority!==state.authority||cfg.generation!==r.expectedConfiguration)return false;}
   else{if(r.target.authority!==state.authority)return false;const target=cfg.targets.find(t=>equal(t.target,r.target)),k=target&&JSON.stringify([target.target.authority,target.target.id,target.target.incarnation]);if(!target||cfg.domains[k][r.domain]!==r.expectedGeneration)return false;}
   continue;
  }
  if(r.document.authority!==state.authority)return false;const d=state.documents.find(d=>equal(d.document,r.document));if(!d||d.live===false)return false;
  if(r.kind==='source'){if(d.revision!==r.expectedSource)return false;}
  else if(r.kind==='annotation-set'){if(d.annotationSet!==r.expectedAnnotationSet)return false;}
  else{const row=d.records.find(row=>row.id===r.id&&row.incarnation===r.incarnation);if(!row||row.live===false)return false;if(r.kind==='annotation-body'?row.bodyVersion!==r.expectedBody:row.targetGeneration!==r.expectedTarget)return false;}
 }
 return true;
}
