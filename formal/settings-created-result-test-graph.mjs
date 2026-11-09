// Verification-only graph builder. No runtime owner accessor is selected.
import assert from 'node:assert/strict';
import fs from 'node:fs';
const base=new URL('./',import.meta.url),read=name=>fs.readFileSync(new URL(name,base),'utf8');
const url=(raw,overrides={})=>'data:text/javascript;base64,'+Buffer.from(raw.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,base).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,base).href)+',import.meta.url)')).toString('base64');
export async function createdSettingsTestGraph(m=null){
 const raw={state:read('settings-created-results.mjs'),kernel:read('invocation-publication.mjs')},accessor='export function createdSettingsTestOwner(domain){return domains.get(domain);}';
 assert(!raw.kernel.includes('createdSettingsTestOwner'));
 if(m){assert.equal(raw[m[1]].split(m[2]).length-1,1,m[0]);raw[m[1]]=raw[m[1]].replace(m[2],m[3]);}
 const state=url(raw.state);return import(url(raw.kernel+'\n'+accessor+'\n',{'./settings-created-results.mjs':state}));
}
