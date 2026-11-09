// Working L02 probe, outside frozen qualification. Private core evidence is not a public facade.
import assert from 'node:assert/strict';
import * as kernel from '../formal/invocation-publication.mjs';
import * as annotation from '../formal/annotation-editing.mjs';
import {annotationEditingFixture} from '../formal/annotation-editing-checks.mjs';
import {invocationPublicationFixture as fixture} from '../formal/invocation-publication-checks.mjs';

const id=n=>({authority:'a',id:n,incarnation:1});
const old=fixture(), state=structuredClone(old.root());
const front=annotationEditingFixture({...kernel,...annotation},'target-1',{
  state,request:{target:{document:id('A'),sourceRevision:1,
    target:{kind:'range',start:3,end:4}}}
});
assert.equal(front.plan,null);
assert.deepEqual(front.root(),state);

const z=fixture(), before=structuredClone(z.root());
const original=before.documents[0].records[0];
const job={kind:'compound',plan:{
  contract:'owned-annotation-plan/1',invocation:z.reference,descriptor:z.reference,
  sources:[{document:id('A'),expectedSource:1,expectedAnnotationSet:1,
    edits:[{lo:3,hi:3,expected:[],ins:[88]}]}],
  annotations:[{kind:'reattach',document:id('A'),id:'note',incarnation:1,
    expectedTarget:1,target:{basis:'candidate',descriptor:z.reference,
      target:{kind:'range',start:3,end:4}}}]
}};
assert.equal(z.submit(job).status,'accepted');
assert.equal(z.run().status,'settled');
assert.equal(z.read().outcome.publication,'applied');
const root=z.root(),record=root.documents[0].records[0];
assert.equal(root.publication,1);
assert.equal(root.groups.length,1);
assert.equal(root.groups[0].changes.length,2);
assert.equal(Buffer.from(root.documents[0].source).toString(),'abcX');
assert.deepEqual(record.target.target,{kind:'range',start:3,end:4});
assert.equal(record.body,original.body);
assert.equal(record.bodyVersion,original.bodyVersion);
console.log(JSON.stringify({
  standaloneLiveTargetPreparation:'refused',standaloneRootUnchanged:true,
  ownedCandidatePublication:'applied',sourceText:'abcX',
  authoredCandidateTarget:record.target.target,publicationCount:root.publication,
  linkedHistoryGroups:root.groups.length,
  authoredParticipants:root.groups[0].changes.length,
  bodyPreserved:true,bodyVersionPreserved:true,publicMixedFacadeProved:false
}));
