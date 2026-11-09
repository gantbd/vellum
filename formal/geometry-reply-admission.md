# Geometry source reply admission before target installation

A successful geometry source reply must fit its current reply budget before its
new target is installed in the layout's persistent target cache. Refusing the
reply must not consume a target slot that the caller cannot use. This is distinct
from rolling back a delivered reference or refunding a spent lifetime identity.

F09 geometry is presentation-owned under engine-ui-ownership.json. This helper
supports frontend/reference integration; it is not part of the symbolic engine
facade or its selected static dependency graph. No engine behavior or ownership
boundary is broadened by correcting its local resource admission.

The previous native helper installed a source target and only then checked final
reply size. With a one-target quota, a refused navigation request exhausted the
quota for another valid request; retrying the original request with more reply
capacity exposed the cached target. A fresh layout admitted the other request.
The source/publication root did not change, but persistent query state did.

The corrected helper stages the prospective source reply and checks its exact
UTF-8 JSON byte size before the target WeakMap/cache installation. The existing
final reply check remains. Already delivered cached references retain their exact
identity when a later reply is withheld; current authority, quota and retirement
checks still apply. Hit and navigation paths share the same installation helper.
This adds no public operation, UI behavior, reference format or cleanup verb.

Six independent byte-boundary cases cover one byte below, exactly at and one byte
above the expected hit/navigation source reply. Four controls cover repeated
withholding without consuming the slot and withholding an already delivered
reference without revoking it. Ten withheld replies are checked. Three native
mutations remove admission, move it after installation or substitute request
capacity for reply capacity; all load successfully and fail semantic assertions.
The existing geometry corpus remains 2,550 oracle cases, 128 query and 65,536
selection gate combinations, 87 controls and nineteen detected mutations.

This is a local native preinstallation invariant, not an owned public geometry
gateway, raw-ingress proof, complete target ownership ledger, global resource
bound or physical allocation guarantee. A prospective JavaScript object may be
allocated before refusal but is not installed as native target authority. Later
delivery failures and owner faults do not authorize revoking already installed
references or treating observer failure as producer rollback.

IF-06, PATH-09 and INS-03/10/11/12/13 distinguish reply-envelope refusal before
target installation, successful cache reuse, actual quota pressure, currentness
loss and retirement. Keep MELT/golden/RED/USE and STRIDE/LINDDUN/privacy/hazard
signals bounded by stage/result class; do not log source positions, coordinates,
layout/reference identities or request-derived labels. No telemetry path is added.
Full frozen qualification and endpoint adoption remain required. Provider/native
measurement, production refinement and independent assurance remain gates.
