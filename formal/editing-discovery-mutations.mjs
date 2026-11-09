import assert from 'node:assert/strict';
import fs from 'node:fs';
import {checkEditingDeclarations,editingDeclarationApi} from './editing-declaration-checks.mjs';
import {checkEditingDiscovery} from './editing-discovery-checks.mjs';
const base=new URL('./',import.meta.url),names=['ordinary-edit-descriptor.mjs','native-discovery.mjs'],files=Object.fromEntries(names.map(n=>[n,fs.readFileSync(new URL(n,base),'utf8')]));
const mutants=[
 ['new-profile-unrecognized',1,'[catalog.profile,catalog.schemaProfile,catalog.editingSchemaProfile,catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(c.profile)','[catalog.profile,catalog.schemaProfile,catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(c.profile)'],
 ['old-profile-catalog-expanded',1,'[catalog.editingSchemaProfile,catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(c.profile)?[ordinaryEditEngineDescriptor()]:[]','true?[ordinaryEditEngineDescriptor()]:[]'],
 ['ordinary-edit-builtin-omitted',1,'[catalog.editingSchemaProfile,catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(c.profile)?[ordinaryEditEngineDescriptor()]:[]','[catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(c.profile)?[ordinaryEditEngineDescriptor()]:[]'],
 ['legacy-schema-profile-widened',1,"[catalog.editingSchemaProfile,catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(profile)&&contract==='engine-ordinary-edit-declaration/1'","true&&contract==='engine-ordinary-edit-declaration/1'"],
 ['arbitrary-editing-schema-admitted',1,'ordinaryEditSchemaValid(input.schema)','true'],
 ['captured-schema-ceiling-widened',1,'selected.schema&&f.schema?{input','f.schema?{input'],
 ['context-profile-binding-ignored',1,'s.stopped||!scopeValid(m,c)||!equal(s.scope,c)','s.stopped||!scopeValid(m,c)'],
 ['required-field-omitted',0,'required:[...f.fields]','required:[...f.fields].slice(0,-1)'],
 ['extra-fields-admitted',0,'additionalProperties:false','additionalProperties:true'],
 ['profile-constant-not-enforced',0,"k==='profile'?{const:f.profileConstant}","k==='profile'?{type:'string'}"],
 ['configured-form-omitted',0,',configuredLineInputForm()]',']'],
 ['unknown-authority-closes-context',1,"[...catalog.authorityGates,'discoveryInterpretationCurrent','retained'].some(k=>g?.[k]===false)","[...catalog.authorityGates,'discoveryInterpretationCurrent','retained'].some(k=>g?.[k]!==true)"],
 ['observed-authority-loss-not-absorbing',1,"[...catalog.authorityGates,'discoveryInterpretationCurrent','retained'].some(k=>g?.[k]===false)||invocationUsage(m.domain)?.faulted===true",'false'],
 ['publication-guard-bypassed',1,"g?.outsidePublicationGuard!==true||!request||!Object.hasOwn(queries.variants,request.operation)","!request||!Object.hasOwn(queries.variants,request.operation)"],
 ['protected-retirement-proof-ignored',1,"!s?.stopped||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])",'!s?.stopped'],
 ['generic-origin-substitutes-for-builtin',1,'(p.builtin===true)!==(s.builtin===true)','false'],
 ['current-advisory-target-gates-bypassed',1,'!p.adviceKnown||!gates(g,catalog.adviceGates)','false'],
];
const url=(source,overrides={})=>'data:text/javascript;base64,'+Buffer.from(source.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,base).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,base).href)+',import.meta.url)')).toString('base64');
async function graph(m){const raw={...files};if(m){const file=names[m[1]];assert.equal(raw[file].split(m[2]).length-1,1,m[0]);raw[file]=raw[file].replace(m[2],m[3]);}const descriptor=url(raw[names[0]]),discovery=url(raw[names[1]],{'./ordinary-edit-descriptor.mjs':descriptor});return {...editingDeclarationApi,...await import(descriptor),...await import(discovery)};}
try{const api=await graph(),declarations=checkEditingDeclarations(api),discovery=checkEditingDiscovery(api),killed=[];for(const m of mutants){const candidate=await graph(m);let failed=false;try{checkEditingDeclarations(candidate);checkEditingDiscovery(candidate);}catch(e){assert.equal(e.code,'ERR_ASSERTION',m[0]+': '+String(e.message).slice(0,140));failed=true;}assert(failed,m[0]+'/survived');killed.push(m[0]);}console.log(JSON.stringify({scope:'isolated-ordinary-edit-declaration-discovery-mutations',declarations,discovery,mutations:killed,constructionOutsideDetection:true,coherentDescriptorAndDiscoveryBaseline:true,allPublicEditingProductsOrIndependentProof:false}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
