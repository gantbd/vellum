use super::*;
#[test]
fn installed_scoped_registration_dispatch_preserves_opaque_basis_order_and_historical_result() {
    let (mut o, mut r, s, c) =
        setup_with_headroom(discovered_context::limits().declared_bytes().unwrap());
    let route = discovered_context::Route::install(&mut o, &c, &mut r);
    let b = basis_facts(&s);
    let native = support::facts();
    let basis = o
        .capture_registration_basis_v1(
            &s,
            &support::address(ResourceCatalogTableV1::Read, "installed"),
            &b,
            &mut r,
        )
        .unwrap();
    let refs = [basis.basis];
    let id = o.issue_invocation(&mut r).unwrap();
    let config = support::configuration(ResourceCatalogTableV1::Read);
    let q = envelope::with_install(&id, &config, 0.0, |v| {
        normalize_scoped_registration_envelope_v1(
            v,
            1,
            support::limits(),
            vellum_engine::capture::CaptureLimitsV1 {
                max_bytes: 32768,
                max_nodes: 128,
                max_depth: 8,
            },
            65536,
            &mut r,
        )
        .unwrap()
    });
    let f = host(&c, &s, &b, &native);
    assert_eq!(
        route.submit(
            &mut o,
            &c,
            NativeDiscoveredContextCommandV1::ScopedRegistration {
                arguments: &q,
                bases: &refs,
                facts: &f
            },
            &mut r
        ),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        route.submit(
            &mut o,
            &c,
            NativeDiscoveredContextCommandV1::ScopedRegistration {
                arguments: &q,
                bases: &refs,
                facts: &f
            },
            &mut r
        ),
        Ok(InvocationState::Pending)
    );
    o.run_scoped_resource_registration_v1(&id, &c, &f, &mut r)
        .unwrap();
    let generation = o.generation();
    let result = o
        .retained_scoped_resource_registration_result_v1(&id, &release(&c, &s, &b), &mut r)
        .unwrap();
    assert_eq!(result.status, ResourceCatalogStatusV1::Applied);
    assert_eq!(result.registrations[0].basis, 0);
    // Scoped resubmission is current service: its original absent-slot basis is
    // stale after installation. Historical result delivery remains independent.
    assert_eq!(
        route.submit(
            &mut o,
            &c,
            NativeDiscoveredContextCommandV1::ScopedRegistration {
                arguments: &q,
                bases: &refs,
                facts: &f
            },
            &mut r
        ),
        Err(DiscoveredNativeCommandErrorV1::Invocation(
            InvocationError::Unavailable
        ))
    );
    assert_eq!(
        o.retained_scoped_resource_registration_result_v1(&id, &release(&c, &s, &b), &mut r)
            .unwrap(),
        result
    );
    assert_eq!(o.generation(), generation);
}
