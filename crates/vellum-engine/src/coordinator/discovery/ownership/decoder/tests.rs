use super::super::tests::{facts, limits, scope};
use super::*;
#[test]
fn shutdown_inventory_captures_discovery_context_cut_and_separately_funded_decoder() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    let (o, mut r, _, _) = setup();
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::Discovery, &[]);
    assert!(row.retained_records.unwrap() >= 3);
    assert_eq!(row.reserved_bytes, Some(limits().max_held_bytes));
    assert_eq!(
        row.additional_reserved_bytes,
        Some(decoder_limits().reserved_bytes().unwrap())
    );
}
fn decoder_limits() -> DiscoveryDecoderLimitsV1 {
    DiscoveryDecoderLimitsV1 {
        input: CaptureLimitsV1 {
            max_bytes: 8192,
            max_nodes: 256,
            max_depth: 12,
        },
        max_held_bytes: 262144,
    }
}
fn setup() -> (
    SourceCoordinator,
    Recorder,
    DiscoveryContextIdV1,
    DiscoveryCutIdV1,
) {
    let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
    o.configure_retention_budget(2097152, &mut r).unwrap();
    o.configure_discovery_v1(&scope(), limits(), &facts(), &mut r)
        .unwrap();
    let entries = [DiscoveryEntryInputV1 {
        command: DiscoveryCommandV1 {
            authority: "authority",
            id: "example",
            incarnation: 1,
        },
        contract: "example/1",
        version: 1,
        kind: DiscoveryExecutionKindV1::Query,
        variants: &[],
        presentation: InertValueV1::Object(&[]),
        order: Some(0),
        schema_visible: false,
        presentation_visible: false,
        advice_visible: false,
    }];
    let input = || DiscoveryCutInputV1 {
        entries: &entries,
        target: None,
        advice_present: false,
    };
    let p = o
        .prepare_discovery_cut_v1(&scope(), input(), &facts(), &mut r)
        .unwrap();
    let c = o
        .acquire_discovery_v1(&scope(), &p, &facts(), &mut r)
        .unwrap()
        .context;
    let p = o
        .prepare_discovery_cut_v1(&scope(), input(), &facts(), &mut r)
        .unwrap();
    o.enable_discovery_decoder_v1(&c, &scope(), decoder_limits(), &facts(), &mut r)
        .unwrap();
    (o, r, c, p)
}
fn envelope<T>(
    id: &DiscoveryContextIdV1,
    run: impl FnOnce(&DiscoveryEnvelopeValueV1<'_, '_>) -> T,
) -> T {
    use DiscoveryEnvelopeValueV1 as V;
    let args = [
        (
            "contract",
            V::Data(InertValueV1::Text("contract-discovery/1")),
        ),
        ("operation", V::Data(InertValueV1::Text("list"))),
        ("context", V::Context(id)),
        (
            "profile",
            V::Data(InertValueV1::Text(OWNED_CONTRACT_DISCOVERY_PROFILE_V1)),
        ),
        ("limit", V::Data(InertValueV1::Number(1.0))),
        ("continuation", V::Data(InertValueV1::Null)),
    ];
    let header = [
        ("family", V::Data(InertValueV1::Number(0.0))),
        ("variant", V::Data(InertValueV1::Text("list"))),
        ("version", V::Data(InertValueV1::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    run(&V::Object(&[
        ("route", V::Data(InertValueV1::Text("typed"))),
        ("request", V::Object(&header)),
    ]))
}
#[test]
fn capture_and_post_native_faults_keep_original_obligations_and_never_rollback_issuance() {
    for late in [false, true] {
        let (mut o, mut r, c, p) = setup();
        let before = o.discovery_usage_v1().unwrap();
        let funding = o.retention_usage();
        let d = o.discovery_decoder_mut(&c).unwrap();
        d.panic = !late;
        d.panic_after = late;
        assert!(matches!(
            envelope(&c, |e| o.query_decoded_discovery_v1(
                &c,
                &scope(),
                &p,
                e,
                &facts(),
                &mut r
            )),
            Err(Error::Faulted)
        ));
        let usage = o.discovery_usage_v1().unwrap();
        assert_eq!(
            (usage.issued_lifetime, usage.selectors, usage.preparations),
            (
                before.issued_lifetime + usize::from(late),
                usize::from(late),
                usize::from(!late)
            )
        );
        assert!(usage.faulted && !o.is_faulted());
        let d = o.discovery_decoder_usage_v1(&c).unwrap();
        assert!(d.faulted && !d.busy);
        assert_eq!(o.retention_usage(), funding);
        o.create(b"editing still works", &mut r).unwrap();
        o.stop_discovery_context_v1(&c, &scope(), &facts(), &mut r)
            .unwrap();
        o.retire_discovery_context_v1(&c, &facts(), &mut r).unwrap();
        assert_eq!(
            o.retention_usage().reserved,
            funding.reserved - decoder_limits().reserved_bytes().unwrap()
        );
        if !late {
            o.retire_discovery_cut_v1(&p, &facts(), &mut r).unwrap();
        }
        o.stop_discovery_admission_v1(&facts(), &mut r).unwrap();
        o.retire_discovery_registry_v1(&facts(), &mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
#[test]
fn busy_or_uncertain_workspace_never_loses_its_context_or_parent_credit() {
    let (mut o, mut r, c, p) = setup();
    let funding = o.retention_usage();
    o.discovery_decoder_mut(&c).unwrap().busy = true;
    assert!(matches!(
        o.query_discovery_v1(
            &c,
            &scope(),
            &p,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None
            },
            &facts(),
            &mut r
        ),
        Err(Error::Unavailable)
    ));
    assert!(
        o.stop_discovery_context_v1(&c, &scope(), &facts(), &mut r)
            .is_err()
    );
    assert_eq!(o.retention_usage(), funding);
    o.discovery_decoder_mut(&c).unwrap().busy = false;
    o.stop_discovery_context_v1(&c, &scope(), &facts(), &mut r)
        .unwrap();
    o.discovery_decoder_mut(&c).unwrap().busy = true;
    assert!(o.retire_discovery_context_v1(&c, &facts(), &mut r).is_err());
    o.discovery_decoder_mut(&c).unwrap().busy = false;
    o.discovery_decoder_mut(&c).unwrap().charge += 1;
    assert_eq!(
        o.retire_discovery_context_v1(&c, &facts(), &mut r),
        Err(Error::Faulted)
    );
    assert_eq!(o.retention_usage(), funding);
    o.discovery_decoder_mut(&c).unwrap().charge -= 1; // Restore only the injected test drift.
    o.retire_discovery_context_v1(&c, &facts(), &mut r).unwrap();
    assert_eq!(
        o.retention_usage().reserved,
        funding.reserved - decoder_limits().reserved_bytes().unwrap()
    );
}
#[test]
fn impossible_owned_decoder_profiles_fault_instead_of_becoming_caller_limits_or_refunds() {
    for field in 0..3 {
        let (mut o, mut r, c, p) = setup();
        let funding = o.retention_usage();
        let d = o.discovery_decoder_mut(&c).unwrap();
        match field {
            0 => d.limits.input.max_bytes = usize::MAX,
            1 => d.limits.input.max_nodes = 0,
            _ => d.handoff = true,
        }
        assert!(matches!(
            envelope(&c, |e| o.query_decoded_discovery_v1(
                &c,
                &scope(),
                &p,
                e,
                &facts(),
                &mut r
            )),
            Err(Error::Faulted)
        ));
        assert!(o.discovery_store().unwrap().faulted && !o.is_faulted());
        assert_eq!(o.retention_usage(), funding);
    }
}
