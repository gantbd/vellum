// Headless integration catalog projection, not OS clipboard access or execution.
// Integration depends on engine contracts; the engine does not import this module.
import fs from 'node:fs';
import {decodeTyped,decodeDynamic,planRoute,validateOperationCatalog} from './operation-catalog.mjs';
import {validateOwnership} from './engine-facade.mjs';
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const rows=freeze(JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8')));
const ownership=freeze(JSON.parse(fs.readFileSync(new URL('./engine-ui-ownership.json',import.meta.url),'utf8')));
validateOperationCatalog(rows);validateOwnership(rows,ownership);
function integration(decoded){return decoded&&ownership.find(p=>p.family===decoded.descriptor.id).integration.includes(decoded.variant)?decoded:null;}
export const decodeIntegrationTyped=request=>integration(decodeTyped(rows,request));
export const decodeIntegrationDynamic=request=>integration(decodeDynamic(rows,request));
export const routeIntegrationTyped=(request,facts)=>planRoute(decodeIntegrationTyped(request),facts);
export const routeIntegrationDynamic=(request,facts)=>planRoute(decodeIntegrationDynamic(request),facts);
export function integrationInventory(){return rows.flatMap((f,index)=>{
 const variants=ownership[index].integration;
 return variants.length?[{family:index,id:f.id,name:f.name,variants:[...variants]}]:[];
});}
