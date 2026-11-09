use super::observation_support as observation;
use super::*;
#[path = "../support/scratch_invocation_fixture.rs"]
#[allow(dead_code)]
mod scratch;
use std::mem::size_of;
use vellum_engine::capture::CapturedValueV1 as C;
#[path = "lifecycle/ordinary.rs"]
mod ordinary;
#[path = "lifecycle/relations.rs"]
mod relations;
pub(super) fn scope4() -> DiscoveryScopeV1 {
    let mut c = scope();
    c.context = scratch::context();
    c.context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V4.into();
    c
}
fn visibility() -> NativeBuiltinDiscoveryVisibilityV4 {
    let visible = |order| NativeBuiltinEntryVisibilityV1 {
        order: Some(order),
        schema: true,
        presentation: false,
        advice: false,
        variants: [NativeBuiltinVariantVisibilityV1 {
            order: Some(0),
            enabled: None,
        }],
    };
    NativeBuiltinDiscoveryVisibilityV4 {
        previous: NativeBuiltinDiscoveryVisibilityV3 {
            previous: context::visibility(),
            ordinary: visible(3),
        },
        lifecycle: NativeBuiltinEntryVisibilityV1 {
            order: Some(4),
            schema: true,
            presentation: false,
            advice: false,
            variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
                order: Some(i),
                enabled: None,
            }),
        },
    }
}
pub(super) fn owner() -> (SourceCoordinator, Recorder) {
    let (mut o, mut r) = scratch::owner_with_retention_budget(33554432);
    let mut l = context::profile_limits();
    l.max_page_rows = 5;
    o.configure_discovery_v1(&scope4(), l, &facts(), &mut r)
        .unwrap();
    (o, r)
}
pub(super) fn prepare(o: &mut SourceCoordinator, r: &mut Recorder) -> DiscoveryCutIdV1 {
    o.prepare_native_builtin_discovery_cut_v4(&scope4(), &visibility(), None, false, &facts(), r)
        .unwrap()
}
fn selectors(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
) -> (
    DiscoveryContextIdV1,
    DiscoveryEntryIdV1,
    Vec<DiscoveryVariantDescriptorV1>,
) {
    selectors_for(o, r, 4)
}
pub(super) fn selectors_for(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
    row: usize,
) -> (
    DiscoveryContextIdV1,
    DiscoveryEntryIdV1,
    Vec<DiscoveryVariantDescriptorV1>,
) {
    let p = prepare(o, r);
    let c = o
        .acquire_discovery_v1(&scope4(), &p, &facts(), r)
        .unwrap()
        .context;
    let p = prepare(o, r);
    let DiscoveryReplyV1::Page { entries, .. } = o
        .query_discovery_v1(
            &c,
            &scope4(),
            &p,
            DiscoveryQueryV1::List {
                limit: 5,
                continuation: None,
            },
            &facts(),
            r,
        )
        .unwrap()
    else {
        panic!()
    };
    let entry = entries[row].reference.clone();
    let p = prepare(o, r);
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
        .query_discovery_v1(
            &c,
            &scope4(),
            &p,
            DiscoveryQueryV1::Inspect { entry: &entry },
            &facts(),
            r,
        )
        .unwrap()
    else {
        panic!()
    };
    (c, entry, descriptor.variants)
}
fn normalized<'h>(
    id: &'h InvocationId,
    r: &mut Recorder,
) -> NormalizedDocumentLifecycleArgumentsV1<'h> {
    type V<'a, 'h> = NativeDocumentLifecycleEnvelopeValueV1<'a, 'h>;
    let bytes = [D::Number(65.0), D::Number(10.0)];
    let serialization = [
        ("profile", D::Text("strict-utf8-one-bom/1")),
        ("bom", D::Boolean(false)),
    ];
    let body = [
        ("contract", V::Data(D::Text("document-lifecycle/1"))),
        ("operation", V::Data(D::Text("create-scratch"))),
        ("source", V::Data(D::Array(&bytes))),
        ("serialization", V::Data(D::Object(&serialization))),
        ("insertionLineBreak", V::Data(D::Text("LF"))),
    ];
    let outer = [
        ("contract", V::Data(D::Text("lifecycle-invocation/1"))),
        ("operation", V::Data(D::Text("change-document-lifecycle"))),
        ("invocation", V::Invocation(id)),
        (
            "profile",
            V::Data(D::Text(NATIVE_DOCUMENT_LIFECYCLE_PROFILE_V1)),
        ),
        ("request", V::Object(&body)),
    ];
    normalize_native_document_lifecycle_arguments_v1(
        V::Object(&outer),
        NativeDocumentLifecycleArgumentLimitsV1 {
            max_owned_bytes: 8192,
            max_source_bytes: 4096,
            max_selections: 8,
        },
        r,
    )
    .unwrap()
}
#[test]
fn lifecycle_discovery_dispatch_replay_and_f06_release_use_one_original_allocation() {
    let (mut o, mut r) = owner();
    let c = scratch::context();
    let f = scratch::facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    observation::Profile {
        epoch: 7,
        maximum: 512,
        capabilities: InvocationReceiptCapabilitiesV1 {
            lifecycle: true,
            ..Default::default()
        },
    }
    .bind(&mut o, &id, &c, &mut r);
    let (dc, e, v) = selectors(&mut o, &mut r);
    assert_eq!(v.len(), 4);
    let n = normalized(&id, &mut r);
    for service in [true, false] {
        let p = prepare(&mut o, &mut r);
        o.submit_discovered_native_document_lifecycle_v1(
            NativeDiscoveredLifecycleRouteV1 {
                context: &dc,
                scope: &scope4(),
                cut: &p,
                entry: &e,
                variant: &v[0].reference,
                facts: &facts(),
            },
            &n,
            NativeDocumentLifecycleAdmissionV2::Existing(
                NativeDocumentLifecycleAdmissionV1::Scratch(&scratch::admission(&f)),
            ),
            &mut r,
        )
        .unwrap();
        if service {
            o.run_native_document_lifecycle_v2(
                &id,
                NativeDocumentLifecycleServiceV2::Existing(
                    NativeDocumentLifecycleServiceV1::Scratch(&f),
                ),
                &mut r,
            )
            .unwrap();
        }
    }
    let expected = o
        .scratch_result_v1(&id, &scratch::release(&c), &mut r)
        .unwrap();
    assert_eq!(
        o.read(&expected.document, &mut r).unwrap().source.text(),
        "A\n"
    );
    let mut l = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 0,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: size_of::<InvocationControlReferenceV1>()
            .max(size_of::<InvocationObservationReferenceV1>()),
        max_reply_bytes: InvocationReceiptV6::required_reply_bytes(512).unwrap(),
        max_control_reply_bytes: size_of::<InvocationControlObservationV1>(),
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    let policy = observation::policy(&c, 7);
    let mut observer = o
        .open_invocation_observation_with_receipts_v6(
            &id,
            &c,
            &policy,
            l,
            InvocationReceiptAdmissionV1 {
                observation: &observation::facts(&c),
                capabilities: InvocationReceiptCapabilitiesV1 {
                    lifecycle: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    let release = TrustedInvocationObservationReleaseV1 {
        context: &c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        control_ready: Some(true),
    };
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &release,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let generation = o.generation();
    let held = o.retention_usage();
    for allowed in [false, true, true] {
        let result = o.query_invocation_receipt_v6(
            &mut observer,
            InvocationReceiptRequestV6::Scratch(&scratch::release(&c)),
            &policy,
            &TrustedInvocationReceiptReleaseV1 {
                observation: &ordinary,
                operation_receipt_allowed: Some(allowed),
            },
            &mut r,
        );
        if allowed {
            let InvocationReceiptV6::Scratch(actual) = result.unwrap() else {
                panic!()
            };
            assert_eq!(actual, expected);
        } else {
            assert_eq!(result.err(), Some(InvocationError::Unavailable));
        }
    }
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage(), held);
}
#[test]
fn lifecycle_dispatch_rejects_wrong_original_variant_before_loss_and_does_not_infer_command_authority()
 {
    let (mut o, mut r) = owner();
    let c = scratch::context();
    let id = o.issue_invocation(&mut r).unwrap();
    let n = normalized(&id, &mut r);
    let (dc, e, v) = selectors(&mut o, &mut r);
    let p = prepare(&mut o, &mut r);
    let mut loss = facts();
    loss.authorized = Some(false);
    let f = scratch::facts(&c);
    let generation = o.generation();
    let before = o.discovery_usage_v1().unwrap();
    assert_eq!(
        o.submit_discovered_native_document_lifecycle_v1(
            NativeDiscoveredLifecycleRouteV1 {
                context: &dc,
                scope: &scope4(),
                cut: &p,
                entry: &e,
                variant: &v[1].reference,
                facts: &loss
            },
            &n,
            NativeDocumentLifecycleAdmissionV2::Existing(
                NativeDocumentLifecycleAdmissionV1::Scratch(&scratch::admission(&f))
            ),
            &mut r
        ),
        Err(DiscoveredLifecycleErrorV1::Discovery(
            DiscoveryErrorV1::Unavailable
        ))
    );
    assert_eq!(o.discovery_usage_v1().unwrap(), before);
    assert!(!o.discovery_context_usage_v1(&dc).unwrap().stopped);
    let mut denied = f;
    denied.creation_allowed = Some(false);
    assert_eq!(
        o.submit_discovered_native_document_lifecycle_v1(
            NativeDiscoveredLifecycleRouteV1 {
                context: &dc,
                scope: &scope4(),
                cut: &p,
                entry: &e,
                variant: &v[0].reference,
                facts: &facts()
            },
            &n,
            NativeDocumentLifecycleAdmissionV2::Existing(
                NativeDocumentLifecycleAdmissionV1::Scratch(&scratch::admission(&denied))
            ),
            &mut r
        ),
        Err(DiscoveredLifecycleErrorV1::Invocation(
            InvocationError::Unavailable
        ))
    );
    assert_eq!(o.generation(), generation);
    assert!(
        o.scratch_result_v1(&id, &scratch::release(&c), &mut r)
            .is_err()
    );
}
#[test]
fn lifecycle_descriptor_is_handler_derived_native_metadata_and_preserves_the_old_prefix() {
    let (mut o, mut r) = owner();
    let mut old = scope4();
    old.context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V3.into();
    let a = o
        .native_builtin_discovery_rows_v3(&old, &facts(), &mut r)
        .unwrap();
    let b = o
        .native_builtin_discovery_rows_v4(&scope4(), &facts(), &mut r)
        .unwrap();
    for i in 0..4 {
        assert_eq!(a[i].command, b[i].command);
        assert_eq!(a[i].contract, b[i].contract);
    }
    let (_, _, v) = selectors(&mut o, &mut r);
    fn field<'a>(c: &'a vellum_engine::capture::InertCaptureV1, v: &'a C, k: &str) -> &'a C {
        let C::Object(rows) = v else { panic!() };
        c.node(rows.iter().find(|(key, _)| key == k).unwrap().1)
            .unwrap()
    }
    let ranges = [0..1, 1..3, 3..4, 4..5];
    for (variant, range) in v.iter().zip(ranges) {
        let schema = variant.schema.as_ref().unwrap();
        let input = field(schema, schema.root(), "input");
        let fields = field(schema, input, "fields");
        assert!(
            matches!(field(schema,fields,"contract"),C::Text(s) if s==NormalizedDocumentLifecycleArgumentsV1::ARGUMENT_DECLARATION.contract)
        );
        let C::Array(forms) = field(schema, fields, "request") else {
            panic!()
        };
        assert_eq!(forms.len(), range.len());
        for (index, node) in range.zip(forms) {
            let form = schema.node(*node).unwrap();
            let d = &NormalizedDocumentLifecycleArgumentsV1::REQUEST_DECLARATIONS[index];
            assert!(matches!(field(schema,form,"operation"),C::Text(s) if s==d.variant.operation));
            let C::Object(actual) = field(schema, form, "fields") else {
                panic!()
            };
            assert_eq!(actual.len(), d.variant.fields.len());
            for (key, role) in d.variant.fields {
                assert!(
                    matches!(field(schema,field(schema,form,"fields"),key),C::Text(s) if s==role)
                );
            }
        }
    }
}

#[test]
fn lifecycle_dispatch_stale_cut_revocation_and_foreign_selectors_never_admit_a_command() {
    for case in 0..4 {
        let (mut o, mut r) = owner();
        let c = scratch::context();
        let f = scratch::facts(&c);
        let id = o.issue_invocation(&mut r).unwrap();
        let n = normalized(&id, &mut r);
        let (dc, e, v) = selectors(&mut o, &mut r);
        let foreign = if case == 2 {
            Some(selectors(&mut o, &mut r))
        } else {
            None
        };
        let p = prepare(&mut o, &mut r);
        let mut current = facts();
        if case == 0 {
            o.create(b"unrelated change", &mut r).unwrap();
        }
        if case == 1 || case == 2 {
            current.authorized = Some(false);
        }
        if case == 3 {
            current.transfer_ready = None;
        }
        let selected = foreign
            .as_ref()
            .map_or(&v[0].reference, |(_, _, v)| &v[0].reference);
        let generation = o.generation();
        assert!(matches!(
            o.submit_discovered_native_document_lifecycle_v1(
                NativeDiscoveredLifecycleRouteV1 {
                    context: &dc,
                    scope: &scope4(),
                    cut: &p,
                    entry: &e,
                    variant: selected,
                    facts: &current
                },
                &n,
                NativeDocumentLifecycleAdmissionV2::Existing(
                    NativeDocumentLifecycleAdmissionV1::Scratch(&scratch::admission(&f))
                ),
                &mut r
            ),
            Err(DiscoveredLifecycleErrorV1::Discovery(
                DiscoveryErrorV1::Unavailable
            ))
        ));
        assert_eq!(o.generation(), generation);
        assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        assert_eq!(
            o.discovery_context_usage_v1(&dc).unwrap().stopped,
            case == 1
        );
    }
}

#[test]
fn lifecycle_dispatch_uses_existing_actual_observations_without_a_diagnostic_dependency() {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for capacity in [0, 1, 1024] {
        let (mut o, mut r) = owner();
        let c = scratch::context();
        let f = scratch::facts(&c);
        let id = o.issue_invocation(&mut r).unwrap();
        let n = normalized(&id, &mut r);
        let (dc, e, v) = selectors(&mut o, &mut r);
        let p = prepare(&mut o, &mut r);
        r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        o.submit_discovered_native_document_lifecycle_v1(
            NativeDiscoveredLifecycleRouteV1 {
                context: &dc,
                scope: &scope4(),
                cut: &p,
                entry: &e,
                variant: &v[0].reference,
                facts: &facts(),
            },
            &n,
            NativeDocumentLifecycleAdmissionV2::Existing(
                NativeDocumentLifecycleAdmissionV1::Scratch(&scratch::admission(&f)),
            ),
            &mut r,
        )
        .unwrap();
        o.run_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleServiceV2::Existing(NativeDocumentLifecycleServiceV1::Scratch(
                &f,
            )),
            &mut r,
        )
        .unwrap();
        let result = o
            .scratch_result_v1(&id, &scratch::release(&c), &mut r)
            .unwrap();
        assert_eq!(
            o.read(&result.document, &mut r).unwrap().source.text(),
            "A\n"
        );
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == 1024 {
            assert!(
                records
                    .iter()
                    .any(|v| v.boundary == Boundary::DiscoveryQuery && v.service_time.is_some())
            );
            assert!(
                records
                    .iter()
                    .any(|v| v.boundary == Boundary::DocumentLifecycleFacadeAdmission
                        && v.service_time.is_some())
            );
        } else {
            assert!(records.len() <= capacity);
        }
    }
}

#[test]
fn lifecycle_source_refuses_incomplete_capacity_without_spending_or_partial_installation() {
    for case in 0..6 {
        let (mut o, mut r) = scratch::owner_with_retention_budget(33554432);
        let mut l = context::profile_limits();
        l.max_page_rows = 5;
        match case {
            0 => l.max_entries = 4,
            1 => l.max_variants = 3,
            2 => l.metadata.max_nodes = 16,
            3 => l.metadata.max_depth = 3,
            4 => l.max_row_bytes = 32,
            _ => l.max_packet_bytes = 32,
        }
        o.configure_discovery_v1(&scope4(), l, &facts(), &mut r)
            .unwrap();
        let before = o.discovery_usage_v1().unwrap();
        let generation = o.generation();
        assert!(
            o.prepare_native_builtin_discovery_cut_v4(
                &scope4(),
                &visibility(),
                None,
                false,
                &facts(),
                &mut r
            )
            .is_err()
        );
        assert_eq!(o.discovery_usage_v1().unwrap(), before);
        assert_eq!(o.generation(), generation);
    }
}
