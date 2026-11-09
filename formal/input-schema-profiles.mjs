// Closed declarative input metadata. No general schema evaluator or resolver.
import {isDeepStrictEqual as equal} from 'node:util';
export const inputSchemaDialect='https://json-schema.org/draft/2020-12/schema';
export const inputSchemaCoverage='outer-structure-only';
const object=v=>v&&typeof v==='object'&&!Array.isArray(v);
const exact=(v,keys)=>object(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const fieldTypes=Object.freeze({contract:'string',operation:'string',invocation:'string',profile:'string',expected:'object',definition:'object',request:'object',create:'array',remove:'array',targets:'array',changes:'array',dependencies:'array',document:'object',expectedSerialization:'integer',serialization:'object'});
export function outerInputSchema(forms){
 if(!Array.isArray(forms)||!forms.length||forms.some(f=>!object(f)||typeof f.contract!=='string'||!f.contract||typeof f.operation!=='string'||!f.operation||!Array.isArray(f.fields)||!f.fields.includes('contract')||!f.fields.includes('operation')||new Set(f.fields).size!==f.fields.length||f.fields.some(k=>!Object.hasOwn(fieldTypes,k))||Object.hasOwn(f,'profileConstant')&&(typeof f.profileConstant!=='string'||!f.profileConstant))||new Set(forms.map(f=>f.contract)).size!==forms.length)return null;
 const oneOf=forms.map(f=>({type:'object',properties:Object.fromEntries(f.fields.map(k=>[k,k==='contract'||k==='operation'?{const:f[k]}:k==='profile'&&Object.hasOwn(f,'profileConstant')?{const:f.profileConstant}:fieldTypes[k]==='string'?{type:'string',minLength:1}:fieldTypes[k]==='integer'?{type:'integer',minimum:1,maximum:Number.MAX_SAFE_INTEGER}:{type:fieldTypes[k]}])),required:[...f.fields],additionalProperties:false}));
 return freeze({$schema:inputSchemaDialect,oneOf});
}
// Recognition of this finite metadata profile, not JSON Schema execution. It
// deliberately admits no refs, custom keywords, callbacks or remote loading.
export function outerInputSchemaValid(schema){
 if(!exact(schema,['$schema','oneOf'])||schema.$schema!==inputSchemaDialect||!Array.isArray(schema.oneOf)||!schema.oneOf.length)return false;
 const contracts=[];
 for(const b of schema.oneOf){
  if(!exact(b,['type','properties','required','additionalProperties'])||b.type!=='object'||b.additionalProperties!==false||!object(b.properties)||!Array.isArray(b.required)||!b.required.includes('contract')||!b.required.includes('operation')||new Set(b.required).size!==b.required.length||!equal([...b.required].sort(),Object.keys(b.properties).sort()))return false;
  for(const k of b.required){const p=b.properties[k];if(!Object.hasOwn(fieldTypes,k))return false;
   if(k==='contract'||k==='operation'||k==='profile'&&Object.hasOwn(p??{},'const')){if(!exact(p,['const'])||typeof p.const!=='string'||!p.const)return false;}
   else if(fieldTypes[k]==='string'){if(!exact(p,['type','minLength'])||p.type!=='string'||p.minLength!==1)return false;}
   else if(fieldTypes[k]==='integer'){if(!exact(p,['type','minimum','maximum'])||p.type!=='integer'||p.minimum!==1||p.maximum!==Number.MAX_SAFE_INTEGER)return false;}
   else if(!exact(p,['type'])||p.type!==fieldTypes[k])return false;
  }
  contracts.push(b.properties.contract.const);
 }
 return new Set(contracts).size===contracts.length;
}
