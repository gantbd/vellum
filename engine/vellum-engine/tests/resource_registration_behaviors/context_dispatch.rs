use super::*;
#[test]
fn installed_native_registration_dispatch_preserves_actual_installation_result_and_replay() {
    let (mut o, mut r) = setup();
    let c = context();
    let native = support::facts();
    o.configure_retention_budget(
        discovered_context::limits().declared_bytes().unwrap(),
        &mut r,
    )
    .unwrap();
    let route = discovered_context::Route::install(&mut o, &c, &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    let change = support::install(ResourceCatalogTableV1::Read, "installed");
    let q = arguments::with_install(&id, &change, |v| {
        normalize_resource_registration_envelope_v1(
            v,
            support::limits(),
            vellum_engine::capture::CaptureLimitsV1 {
                max_bytes: 65536,
                max_nodes: 256,
                max_depth: 16,
            },
            131072,
            &mut r,
        )
        .unwrap()
    });
    let mut bad = host(&c, &native);
    bad.interpretation_current = None;
    assert_eq!(
        route.submit(
            &mut o,
            &c,
            NativeDiscoveredContextCommandV1::NativeRegistration {
                arguments: &q,
                facts: &bad
            },
            &mut r
        ),
        Err(DiscoveredNativeCommandErrorV1::Invocation(
            InvocationError::Unavailable
        ))
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    let f = host(&c, &native);
    assert_eq!(
        route.submit(
            &mut o,
            &c,
            NativeDiscoveredContextCommandV1::NativeRegistration {
                arguments: &q,
                facts: &f
            },
            &mut r
        ),
        Ok(InvocationState::Pending)
    );
    o.run_resource_registration_v1(&id, &c, &f, &mut r).unwrap();
    let generation = o.generation();
    assert_eq!(
        o.retained_resource_registration_result_v1(&id, &release(&c), &mut r)
            .unwrap()
            .result
            .status,
        ResourceCatalogStatusV1::Applied
    );
    assert_eq!(
        route.submit(
            &mut o,
            &c,
            NativeDiscoveredContextCommandV1::NativeRegistration {
                arguments: &q,
                facts: &f
            },
            &mut r
        ),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation);
}
