// Shared closed canonical target validation; no presentation or provider execution.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
const catalog=JSON.parse(fs.readFileSync(new URL('./annotation-compounds.json',import.meta.url)));
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const natural=n=>Number.isSafeInteger(n)&&n>=0;
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const targetForms=freeze({fields:catalog.targets,pointProfiles:catalog.pointProfiles,pointAffinities:['before','after']});
export function compoundTargetForms(){return targetForms;}
export function compoundTargetShape(target){
 if(!target||!Object.hasOwn(catalog.targets,target.kind)||!exact(target,catalog.targets[target.kind]))return false;
 if(target.kind==='document')return true;
 if(target.kind==='point')return natural(target.offset)&&targetForms.pointAffinities.includes(target.affinity)&&catalog.pointProfiles.includes(target.profile);
 return natural(target.start)&&natural(target.end)&&target.start<target.end;
}
export function validCompoundTarget(source,target){
 if(!compoundTargetShape(target))return false;
 const boundary=p=>p<=source.length&&(p===source.length||(source[p]&0xc0)!==0x80);
 if(target.kind==='document')return true;
 return target.kind==='point'?boundary(target.offset):boundary(target.start)&&boundary(target.end);
}
