use super::*;
use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1};

pub(super) struct Roles {
    pub(super) context: DiscoveryContextIdV1,
    pub(super) entry: DiscoveryEntryIdV1,
    pub(super) variant: DiscoveryVariantIdV1,
    pub(super) other_variant: DiscoveryVariantIdV1,
    pub(super) target: DiscoveryTargetIdV1,
    pub(super) cursor: DiscoveryCursorIdV1,
}
pub(super) fn role_cut(f: &mut support::Fixture) -> DiscoveryCutIdV1 {
    metadata(true, true, |base| {
        let b = &base.entries[0];
        let entries: [_; 2] = std::array::from_fn(|i| DiscoveryEntryInputV1 {
            command: DiscoveryCommandV1 {
                authority: b.command.authority,
                id: ["first", "second"][i],
                incarnation: 1,
            },
            contract: b.contract,
            version: b.version,
            kind: b.kind,
            variants: b.variants,
            presentation: b.presentation,
            order: Some(i),
            schema_visible: true,
            presentation_visible: true,
            advice_visible: true,
        });
        f.owner.prepare_discovery_cut_v1(
            &scope(),
            DiscoveryCutInputV1 {
                entries: &entries,
                target: base.target,
                advice_present: true,
            },
            &facts(),
            &mut f.recorder,
        )
    })
    .unwrap()
}
fn role_query(
    f: &mut support::Fixture,
    c: &DiscoveryContextIdV1,
    q: DiscoveryQueryV1<'_>,
) -> DiscoveryReplyV1 {
    let p = role_cut(f);
    f.owner
        .query_discovery_v1(c, &scope(), &p, q, &facts(), &mut f.recorder)
        .unwrap()
}
pub(super) fn roles(f: &mut support::Fixture) -> Roles {
    let p = role_cut(f);
    let a = f
        .owner
        .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
        .unwrap();
    let DiscoveryReplyV1::Page {
        entries,
        continuation: Some(cursor),
        ..
    } = role_query(
        f,
        &a.context,
        DiscoveryQueryV1::List {
            limit: 1,
            continuation: None,
        },
    )
    else {
        panic!("page")
    };
    let entry = entries[0].reference.clone();
    let DiscoveryReplyV1::Descriptor { descriptor, .. } =
        role_query(f, &a.context, DiscoveryQueryV1::Inspect { entry: &entry })
    else {
        panic!("descriptor")
    };
    let variant = descriptor.variants[0].reference.clone();
    let DiscoveryReplyV1::Page { entries, .. } = role_query(
        f,
        &a.context,
        DiscoveryQueryV1::List {
            limit: 2,
            continuation: None,
        },
    ) else {
        panic!("page")
    };
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = role_query(
        f,
        &a.context,
        DiscoveryQueryV1::Inspect {
            entry: &entries[1].reference,
        },
    ) else {
        panic!("descriptor")
    };
    Roles {
        context: a.context,
        entry,
        variant,
        other_variant: descriptor.variants[0].reference.clone(),
        target: a.target.unwrap(),
        cursor,
    }
}
pub(super) fn invalid_queries<'a>(a: &'a Roles, b: &'a Roles) -> [DiscoveryQueryV1<'a>; 10] {
    [
        DiscoveryQueryV1::Inspect { entry: &b.entry },
        DiscoveryQueryV1::AdvisoryEnablement {
            entry: &b.entry,
            variant: &a.variant,
            target: &a.target,
        },
        DiscoveryQueryV1::AdvisoryEnablement {
            entry: &a.entry,
            variant: &b.variant,
            target: &a.target,
        },
        DiscoveryQueryV1::AdvisoryEnablement {
            entry: &a.entry,
            variant: &a.variant,
            target: &b.target,
        },
        DiscoveryQueryV1::AdvisoryEnablement {
            entry: &a.entry,
            variant: &a.other_variant,
            target: &a.target,
        },
        DiscoveryQueryV1::List {
            limit: 1,
            continuation: Some(&b.cursor),
        },
        DiscoveryQueryV1::List {
            limit: 2,
            continuation: Some(&a.cursor),
        },
        DiscoveryQueryV1::List {
            limit: 0,
            continuation: None,
        },
        DiscoveryQueryV1::List {
            limit: usize::MAX,
            continuation: None,
        },
        DiscoveryQueryV1::AdvisoryEnablement {
            entry: &b.entry,
            variant: &b.variant,
            target: &b.target,
        },
    ]
}
pub(super) fn loss_facts(field: usize) -> TrustedDiscoveryFactsV1 {
    let mut g = facts();
    match field {
        0 => g.authorized = Some(false),
        1 => g.scope = Some(false),
        2 => g.continuous = Some(false),
        3 => g.live = Some(false),
        4 => g.not_stopped = Some(false),
        5 => g.discovery_allowed = Some(false),
        6 => g.interpretation_current = Some(false),
        _ => g.retained = Some(false),
    }
    g
}

#[test]
fn foreign_discovery_entry_cannot_apply_loss_to_a_different_original_context() {
    let mut f = setup();
    let a = acquire(&mut f, true, true);
    let own = entry(&mut f, &a.context, true);
    let b = acquire(&mut f, true, true);
    let foreign = entry(&mut f, &b.context, true);
    let p = cut(&mut f, true, true);
    let before = f.owner.discovery_usage_v1().unwrap();
    let mut denied = facts();
    denied.authorized = Some(false);
    assert!(
        f.owner
            .query_discovery_v1(
                &a.context,
                &scope(),
                &p,
                DiscoveryQueryV1::Inspect { entry: &foreign },
                &denied,
                &mut f.recorder
            )
            .is_err()
    );
    assert!(
        !f.owner
            .discovery_context_usage_v1(&a.context)
            .unwrap()
            .stopped
    );
    assert_eq!(
        f.owner.discovery_usage_v1().unwrap().preparations,
        before.preparations
    );
    assert_eq!(
        f.owner.discovery_usage_v1().unwrap().issued_lifetime,
        before.issued_lifetime
    );
    assert!(
        f.owner
            .query_discovery_v1(
                &a.context,
                &scope(),
                &p,
                DiscoveryQueryV1::Inspect { entry: &own },
                &facts(),
                &mut f.recorder
            )
            .is_ok()
    );
}

#[test]
fn every_discovery_query_recognizes_original_roles_and_binding_before_any_loss_effect() {
    let mut f = setup();
    let a = roles(&mut f);
    let b = roles(&mut f);
    for field in 0..8 {
        for q in invalid_queries(&a, &b) {
            let p = role_cut(&mut f);
            let before = f.owner.discovery_usage_v1().unwrap();
            let result = f.owner.query_discovery_v1(
                &a.context,
                &scope(),
                &p,
                q,
                &loss_facts(field),
                &mut f.recorder,
            );
            assert!(matches!(
                result,
                Err(DiscoveryErrorV1::Unavailable | DiscoveryErrorV1::InvalidRequest)
            ));
            assert!(
                !f.owner
                    .discovery_context_usage_v1(&a.context)
                    .unwrap()
                    .stopped
            );
            assert!(
                !f.owner
                    .discovery_context_usage_v1(&b.context)
                    .unwrap()
                    .stopped
            );
            let after = f.owner.discovery_usage_v1().unwrap();
            assert_eq!(after, before);
            f.owner
                .retire_discovery_cut_v1(&p, &facts(), &mut f.recorder)
                .unwrap();
        }
    }
    role_query(
        &mut f,
        &a.context,
        DiscoveryQueryV1::Inspect { entry: &a.entry },
    );
    role_query(
        &mut f,
        &b.context,
        DiscoveryQueryV1::Inspect { entry: &b.entry },
    );
}

#[test]
fn recognized_discovery_loss_still_absorbs_before_transient_delivery_refusal_for_every_form() {
    for field in 0..8 {
        for form in 0..5 {
            let mut f = setup();
            let a = roles(&mut f);
            let query = match form {
                0 => DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None,
                },
                1 => DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: Some(&a.cursor),
                },
                2 => DiscoveryQueryV1::Inspect { entry: &a.entry },
                3 => DiscoveryQueryV1::List {
                    limit: limits().max_page_rows + 1,
                    continuation: None,
                },
                _ => DiscoveryQueryV1::AdvisoryEnablement {
                    entry: &a.entry,
                    variant: &a.variant,
                    target: &a.target,
                },
            };
            let p = role_cut(&mut f);
            let mut g = loss_facts(field);
            g.resources_ready = None;
            g.reply_ready = Some(false);
            assert!(
                f.owner
                    .query_discovery_v1(&a.context, &scope(), &p, query, &g, &mut f.recorder)
                    .is_err()
            );
            assert!(
                f.owner
                    .discovery_context_usage_v1(&a.context)
                    .unwrap()
                    .stopped
            );
            assert!(
                f.owner
                    .query_discovery_v1(&a.context, &scope(), &p, query, &facts(), &mut f.recorder)
                    .is_err()
            );
        }
    }
}

#[test]
fn discovery_recognition_is_observable_without_sensitive_labels_or_diagnostic_dependencies() {
    for capacity in [0, 1, 128] {
        let mut f = setup();
        let a = roles(&mut f);
        let b = roles(&mut f);
        f.recorder = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let p = role_cut(&mut f);
        assert!(
            f.owner
                .query_discovery_v1(
                    &a.context,
                    &scope(),
                    &p,
                    DiscoveryQueryV1::Inspect { entry: &b.entry },
                    &loss_facts(0),
                    &mut f.recorder
                )
                .is_err()
        );
        role_query(
            &mut f,
            &a.context,
            DiscoveryQueryV1::Inspect { entry: &a.entry },
        );
        assert!(f.recorder.queued() <= capacity);
        if capacity == 128 {
            let mut refused = false;
            let mut completed = false;
            let mut timed = false;
            while let Some(o) = f.recorder.pop_front() {
                if o.boundary == Boundary::DiscoveryRequestRecognition {
                    refused |= o.disposition == Disposition::Refused;
                    completed |= o.disposition == Disposition::Completed;
                }
                if o.boundary == Boundary::DiscoveryQuery {
                    timed |= o.service_time.is_some();
                }
            }
            assert!(refused && completed && timed);
        }
    }
}

#[test]
fn resource_catalog_bootstrap_stales_an_old_root_cut_but_idempotent_configuration_does_not() {
    let mut f = setup();
    let p = cut(&mut f, true, true);
    let generation = f.owner.generation();
    f.owner
        .configure_resource_catalog_v1(
            &[],
            catalog_support::limits(),
            &catalog_support::facts(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(f.owner.generation(), generation);
    assert!(
        f.owner
            .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
            .is_err()
    );
    f.owner
        .retire_discovery_cut_v1(&p, &facts(), &mut f.recorder)
        .unwrap();
    let p = cut(&mut f, true, true);
    assert_eq!(
        f.owner.configure_resource_catalog_v1(
            &[],
            catalog_support::limits(),
            &catalog_support::facts(),
            &mut f.recorder
        ),
        Ok(false)
    );
    assert!(
        f.owner
            .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
            .is_ok()
    );
}
