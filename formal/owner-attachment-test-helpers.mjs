// Trusted test coupling to the actual extra reservation, not a sizing oracle.
// Frame sufficiency is independently challenged in owner-attachment-checks.
import assert from 'node:assert/strict';
export function parentFundedPoolAttachmentBytes(api,domain,handle){
 const inventory=api.invocationOwnerAttachmentInventory(domain,api.invocationAuthorityContext(domain),{ownerControlAllowed:true}),edge=inventory?.entries.find(e=>e.owner===handle);
 assert(edge,'actual parent-funded pool must have its original attachment');
 assert.equal(edge.additionalChildReservationBytes,0,'native pool already funds its own child bytes');
 assert(edge.declaredFrameBytes>0);return edge.declaredFrameBytes;
}
export function parentFundedPoolAttachmentForKind(api,domain,kind){
 const inventory=api.invocationOwnerAttachmentInventory(domain,api.invocationAuthorityContext(domain),{ownerControlAllowed:true}),rows=inventory?.entries.filter(e=>e.kind===kind);
 assert.equal(rows?.length,1,'selected native singleton must have exactly one actual attachment');
 const edge=rows[0];assert.equal(edge.additionalChildReservationBytes,0);assert(edge.declaredFrameBytes>0);return edge;
}
