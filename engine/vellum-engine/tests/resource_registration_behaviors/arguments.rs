use super::*;
use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
pub(super) fn with_install<'h, R>(
    id: &'h InvocationId,
    change: &ResourceCatalogChangeV1,
    f: impl FnOnce(&ResourceRegistrationEnvelopeValueV1<'_, 'h>) -> R,
) -> R {
    let ResourceCatalogActionV1::Install(cfg) = &change.action else {
        unreachable!()
    };
    envelope::with_install(id, cfg, 0.0, |scoped| {
        use ResourceRegistrationEnvelopeValueV1 as V;
        let V::Object(outer) = scoped else {
            unreachable!()
        };
        let V::Data(D::Object(request)) = outer.iter().find(|(k, _)| *k == "request").unwrap().1
        else {
            unreachable!()
        };
        let D::Array(changes) = request[0].1 else {
            unreachable!()
        };
        let D::Object(row) = changes[0] else {
            unreachable!()
        };
        let config = row.iter().find(|(k, _)| *k == "configuration").unwrap().1;
        let a = &change.address;
        let provider = [
            ("authority", D::Text(&a.provider.authority)),
            ("id", D::Text(&a.provider.id)),
            ("incarnation", D::Number(a.provider.incarnation as f64)),
        ];
        let resource = [
            ("authority", D::Text(&a.resource.authority)),
            ("id", D::Text(&a.resource.id)),
            ("incarnation", D::Number(a.resource.incarnation as f64)),
        ];
        let expected = [("state", D::Text("absent"))];
        let row = [
            ("configuration", config),
            ("expected", D::Object(&expected)),
            ("resource", D::Object(&resource)),
            ("provider", D::Object(&provider)),
            ("action", D::Text("install")),
            (
                "table",
                D::Text(match a.table {
                    ResourceCatalogTableV1::Read => "read",
                    ResourceCatalogTableV1::Write => "write",
                }),
            ),
        ];
        let changes = [D::Object(&row)];
        let request = [("changes", D::Array(&changes))];
        let outer = [
            ("request", V::Data(D::Object(&request))),
            (
                "profile",
                V::Data(D::Text(NATIVE_RESOURCE_REGISTRATION_PROFILE_V1)),
            ),
            ("invocation", V::Invocation(id)),
            ("operation", V::Data(D::Text("activate-context"))),
            ("contract", V::Data(D::Text("registration-invocation/1"))),
        ];
        f(&V::Object(&outer))
    })
}
#[test]
fn passive_native_registration_normalizes_to_the_same_owned_empty_batch_without_conferring_authority()
 {
    let (mut owner, mut r) = setup();
    let id = owner.issue_invocation(&mut r).unwrap();
    let fields = [("changes", D::Array(&[]))];
    use ResourceRegistrationEnvelopeValueV1 as V;
    let outer = [
        ("contract", V::Data(D::Text("registration-invocation/1"))),
        ("operation", V::Data(D::Text("activate-context"))),
        ("invocation", V::Invocation(&id)),
        (
            "profile",
            V::Data(D::Text(NATIVE_RESOURCE_REGISTRATION_PROFILE_V1)),
        ),
        ("request", V::Data(D::Object(&fields))),
    ];
    let q = normalize_resource_registration_envelope_v1(
        &V::Object(&outer),
        support::limits(),
        CaptureLimitsV1 {
            max_bytes: 65536,
            max_nodes: 256,
            max_depth: 16,
        },
        131072,
        &mut r,
    )
    .unwrap();
    assert!(q.changes().is_empty());
    let c = context();
    let n = support::facts();
    let mut f = host(&c, &n);
    f.interpretation_current = None;
    assert_eq!(
        owner.submit_normalized_resource_registration_v1(&q, &c, &f, &mut r),
        Err(InvocationError::Unavailable)
    );
    assert!(matches!(
        owner.invocation_state(&id, &mut r),
        Ok(InvocationState::Issued)
    ));
    let f = host(&c, &n);
    owner
        .submit_normalized_resource_registration_v1(&q, &c, &f, &mut r)
        .unwrap();
    owner
        .run_resource_registration_v1(&id, &c, &f, &mut r)
        .unwrap();
    assert_eq!(
        owner
            .retained_resource_registration_result_v1(&id, &release(&c), &mut r)
            .unwrap()
            .result
            .status,
        ResourceCatalogStatusV1::Unchanged
    );
}

#[test]
fn complete_configuration_products_round_trip_to_one_native_intent_and_original_receipt() {
    for shape in 0..72 {
        let (mut o, mut r) = setup();
        let id = o.issue_invocation(&mut r).unwrap();
        let c = context();
        let n = support::facts();
        let table = if shape < 64 {
            ResourceCatalogTableV1::Write
        } else {
            ResourceCatalogTableV1::Read
        };
        let mut cfg = support::configuration(table);
        cfg.identity_known = shape % 2 == 0;
        cfg.capabilities = if shape < 64 {
            ResourceCatalogCapabilitiesV1::Write {
                procedure: "PRIVATE_PROCEDURE".into(),
                logical_continuity: shape & 1 != 0,
                synchronization_supported: shape & 2 != 0,
                conditional_write: shape & 4 != 0,
                causal_receipts: shape & 8 != 0,
                non_reuse_tokens: shape & 16 != 0,
                exclusive_create: shape & 32 != 0,
            }
        } else if shape < 70 {
            ResourceCatalogCapabilitiesV1::StrictUtf8OneBom {
                atomic_read: shape % 2 == 0,
                insertion_line_break: [
                    CatalogInsertionLineBreakV1::Lf,
                    CatalogInsertionLineBreakV1::CrLf,
                    CatalogInsertionLineBreakV1::Cr,
                ][(shape - 64) / 2],
            }
        } else {
            ResourceCatalogCapabilitiesV1::AnnotationsV1 {
                atomic_read: shape % 2 == 0,
            }
        };
        let change = ResourceCatalogChangeV1 {
            address: support::address(table, "PRIVATE_TARGET"),
            expected: CatalogExpectationV1::Absent,
            action: ResourceCatalogActionV1::Install(cfg),
        };
        let q = with_install(&id, &change, |value| {
            normalize_resource_registration_envelope_v1(
                value,
                support::limits(),
                CaptureLimitsV1 {
                    max_bytes: 65536,
                    max_nodes: 256,
                    max_depth: 16,
                },
                131072,
                &mut r,
            )
            .unwrap()
        });
        assert_eq!(q.changes(), std::slice::from_ref(&change));
        o.submit_normalized_resource_registration_v1(&q, &c, &host(&c, &n), &mut r)
            .unwrap();
        o.submit_resource_registration_v1(
            &id,
            std::slice::from_ref(&change),
            &c,
            &host(&c, &n),
            &mut r,
        )
        .unwrap();
        o.run_resource_registration_v1(&id, &c, &host(&c, &n), &mut r)
            .unwrap();
        let generation = o.generation();
        o.submit_normalized_resource_registration_v1(&q, &c, &host(&c, &n), &mut r)
            .unwrap();
        assert_eq!(o.generation(), generation);
        assert_eq!(o.invocation_usage().issued_lifetime, 1);
        assert_eq!(
            o.retained_resource_registration_result_v1(&id, &release(&c), &mut r)
                .unwrap()
                .result
                .status,
            ResourceCatalogStatusV1::Applied
        );
    }
}

#[test]
fn native_decoded_admission_and_table_preflight_preserve_behavior_when_diagnostics_are_off_or_saturated()
 {
    for capacity in [0, 1, 128] {
        let (mut o, mut r) = setup();
        let id = o.issue_invocation(&mut r).unwrap();
        let c = context();
        let n = support::facts();
        r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let mut change = support::install(ResourceCatalogTableV1::Read, "target");
        let q = with_install(&id, &change, |value| {
            normalize_resource_registration_envelope_v1(
                value,
                support::limits(),
                CaptureLimitsV1 {
                    max_bytes: 65536,
                    max_nodes: 256,
                    max_depth: 16,
                },
                131072,
                &mut r,
            )
            .unwrap()
        });
        change.address.table = ResourceCatalogTableV1::Write;
        assert!(
            with_install(&id, &change, |value| {
                normalize_resource_registration_envelope_v1(
                    value,
                    support::limits(),
                    CaptureLimitsV1 {
                        max_bytes: 65536,
                        max_nodes: 256,
                        max_depth: 16,
                    },
                    131072,
                    &mut r,
                )
            })
            .is_err()
        );
        let mut f = host(&c, &n);
        f.profile.max_relations += 1;
        assert_eq!(
            o.submit_normalized_resource_registration_v1(&q, &c, &f, &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        let f = host(&c, &n);
        o.submit_normalized_resource_registration_v1(&q, &c, &f, &mut r)
            .unwrap();
        o.run_resource_registration_v1(&id, &c, &f, &mut r).unwrap();
        let generation = o.generation();
        o.submit_normalized_resource_registration_v1(&q, &c, &f, &mut r)
            .unwrap();
        assert_eq!(o.generation(), generation);
        assert_eq!(o.invocation_usage().issued_lifetime, 1);
        let mut normalized = 0;
        let mut submitted = 0;
        while let Some(record) = r.pop_front() {
            match record.boundary {
                Boundary::RegistrationArgumentNormalization => {
                    normalized += 1;
                    assert!(record.service_time.is_some());
                }
                Boundary::RegistrationDecodedAdmission => {
                    submitted += 1;
                    assert!(record.service_time.is_some());
                }
                _ => {}
            }
        }
        if capacity == 128 {
            assert_eq!(normalized, 2);
            assert_eq!(submitted, 3);
        }
        if capacity == 1 {
            assert!(r.dropped() > 0);
        }
    }
}
