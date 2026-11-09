// F10 query routing only. No invocation issuance, publication or effect launch.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {decodeTyped,decodeDynamic,planRoute} from './operation-catalog.mjs';
import {validateInvocationProposal} from './invocation-publication.mjs';
const C=JSON.parse(fs.readFileSync(new URL('./proposal-validation.json',import.meta.url)));
const rows=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url)));
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
export function queryProposalValidation(domain,envelope,context,routing,gates,host){
 const no=()=>({status:'unavailable'});
 if(!exact(envelope,C.envelopeFields)||!['typed','dynamic'].includes(envelope.route))return no();
 const decoded=envelope.route==='typed'?decodeTyped(rows,envelope.request):decodeDynamic(rows,envelope.request);
 if(!decoded||decoded.descriptor.id!==C.family||decoded.variant!==C.operation||planRoute(decoded,routing).status!=='routed')return no();
 return validateInvocationProposal(domain,envelope.proposal,context,gates,host);
}
