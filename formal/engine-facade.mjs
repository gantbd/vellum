// Ownership projection of the editor-wide reference catalog, not a native dispatcher.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {isDeepStrictEqual as equal} from 'node:util';
import {decodeTyped,decodeDynamic,planRoute,validateOperationCatalog} from './operation-catalog.mjs';
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const rows=freeze(JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8')));
const ownership=freeze(JSON.parse(fs.readFileSync(new URL('./engine-ui-ownership.json',import.meta.url),'utf8')));
export function validateOwnership(catalog,partition){
  assert.equal(partition.length,catalog.length);
  for(let i=0;i<catalog.length;i++){
    const p=partition[i],f=catalog[i];
    assert(equal(Object.keys(p).sort(),['engine','family','integration','presentation']));
    assert.equal(p.family,f.id);assert(Array.isArray(p.engine)&&Array.isArray(p.presentation)&&Array.isArray(p.integration));
    const all=[...p.engine,...p.presentation,...p.integration];assert.equal(new Set(all).size,all.length);
    assert(equal(all.toSorted(),f.variants.toSorted()));
  }
}
validateOperationCatalog(rows);validateOwnership(rows,ownership);
function engine(decoded){
  return decoded&&ownership.find(p=>p.family===decoded.descriptor.id).engine.includes(decoded.variant)?decoded:null;
}
export const decodeEngineTyped=request=>engine(decodeTyped(rows,request));
export const decodeEngineDynamic=request=>engine(decodeDynamic(rows,request));
export const routeEngineTyped=(request,facts)=>planRoute(decodeEngineTyped(request),facts);
export const routeEngineDynamic=(request,facts)=>planRoute(decodeEngineDynamic(request),facts);
// Keep original family indices. Filtering then renumbering would change typed identity.
export function engineInventory(){return rows.flatMap((f,index)=>{
  const variants=ownership[index].engine;
  return variants.length?[{family:index,id:f.id,name:f.name,variants:[...variants]}]:[];
});}
