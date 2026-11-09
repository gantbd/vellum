// Working L02 counterexample, outside the frozen qualification inventory.
// Run with the pinned VELLUM_ICU4J_JAR used by the native invocation fixtures.
import assert from 'node:assert/strict';
import * as kernel from '../formal/invocation-publication.mjs';
import {
  editingInvocationFixture,
  editingInvocationContext as context,
  editingInvocationPolicy as policy,
} from '../formal/editing-invocation-checks.mjs';

const fixture = editingInvocationFixture('selection');
const state = structuredClone(fixture.root());
const view = state.views[0];
view.selections = [
  {anchor: 0, head: 1, affinity: 'after'},
  {anchor: 2, head: 3, affinity: 'after'},
];
view.primary = 1;
const host = {...fixture.h, outsidePublicationGuard: true, preparationReserved: true};
const domain = kernel.openInvocationDomain(state, context, fixture.ls, host);
assert(domain);
const issued = kernel.issueInvocation(domain, context, host);
assert(issued);
const request = {...fixture.request, invocation: issued.reference};
const envelope = {route: 'typed', request: {
  family: 10, variant: 'ordinary-edit', version: 1, arguments: request,
}};
const limits = {maxRequestBytes: 65536, maxInputNodes: 16384, maxInputDepth: 64};
assert.equal(kernel.submitInvocationOrdinaryEdit(
  domain, issued.ticket, envelope, limits, context, policy, host,
).status, 'accepted');
assert.equal(kernel.runInvocation(domain, issued.ticket, context, host, host).status, 'settled');
const after = kernel.invocationAuthorityRoot(domain);
const actual = Buffer.from(after.documents[0].source).toString('utf8');
assert.equal(actual, 'XbX'); // Correct ordinary multi-selection insertion.
assert.notEqual(actual, 'abX'); // Required primary-only composition replacement.
console.log(JSON.stringify({
  actualOrdinaryText: actual,
  requiredPrimaryOnlyText: 'abX',
  primaryBefore: 1,
  selectionsBefore: 2,
  selectionsAfter: after.views[0].selections,
  ordinaryBehaviorCorrect: true,
  ordinaryAliasWouldDuplicateCommit: true,
  compositionBindingProved: false,
}));
