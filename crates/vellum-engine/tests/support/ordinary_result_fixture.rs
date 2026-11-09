//! Test-only public producer/F06 composition. No new engine result or authority.
use std::mem::size_of;
use vellum_engine::capture::InertValueV1 as D;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;

fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "engine".into(),
        engine: "local".into(),
        recipient: "agent".into(),
        purpose: "command-result".into(),
        destination: "native".into(),
        profile: "local-result/1".into(),
        continuity: "session".into(),
    }
}
fn policy(c: &SettingsExecutionContextV1) -> InvocationObservationPolicyV1 {
    InvocationObservationPolicyV1 {
        recipient: c.recipient.clone(),
        epoch: 1,
        grant: true,
        publication: true,
        control: true,
        effect: true,
        reasons: true,
        details: false,
        recovery: true,
    }
}
pub fn fund(o: &mut SourceCoordinator, r: &mut Recorder) {
    if o.retention_usage().maximum.is_none() {
        o.configure_retention_budget(33554432, r).unwrap();
    }
}
pub fn bind(o: &mut SourceCoordinator, id: &InvocationId, r: &mut Recorder) {
    bind_in_context(o, id, &context(), r);
}
pub fn bind_in_context(
    o: &mut SourceCoordinator,
    id: &InvocationId,
    c: &SettingsExecutionContextV1,
    r: &mut Recorder,
) {
    o.bind_invocation_observation_policy_v1(
        id,
        c,
        &policy(c),
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &TrustedInvocationPolicyFactsV1 {
            context: c,
            authorized: Some(true),
            scope: Some(true),
            continuous: Some(true),
            live: Some(true),
            resources_ready: Some(true),
            retained: Some(true),
            transfer_ready: Some(true),
            preparation_reserved: Some(true),
            registry_ready: Some(true),
            retention_ready: Some(true),
        },
        r,
    )
    .unwrap();
}
fn limits() -> InvocationObservationLimitsV1 {
    let mut l = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 16,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: size_of::<InvocationControlReferenceV1>()
            .max(size_of::<InvocationObservationReferenceV1>()),
        max_reply_bytes: 512 + size_of::<InvocationObservationReplyV1>(),
        max_control_reply_bytes: 512,
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    l
}
#[allow(clippy::too_many_arguments)]
fn query(
    o: &SourceCoordinator,
    id: &InvocationId,
    observer: &mut InvocationObservationContextV1,
    c: &SettingsExecutionContextV1,
    typed: bool,
    publication: bool,
    ready: bool,
    r: &mut Recorder,
) -> Result<InvocationObservationReplyV1, InvocationError> {
    use InvocationObservationEnvelopeValueV1 as V;
    let original = observer.context_id_v1();
    let args = [
        ("contract", V::Data(D::Text("execution-observation/1"))),
        ("operation", V::Data(D::Text("retained-outcome"))),
        ("invocation", V::Invocation(id)),
        ("context", V::Context(&original)),
        ("profile", V::Data(D::Text(&c.profile))),
    ];
    let request = [
        (
            if typed { "family" } else { "name" },
            V::Data(if typed {
                D::Number(5.0)
            } else {
                D::Text("inspect-execution")
            }),
        ),
        ("variant", V::Data(D::Text("retained-outcome"))),
        ("version", V::Data(D::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    let outer = [
        (
            "route",
            V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&request)),
    ];
    let mut p = policy(c);
    p.publication = publication;
    let f = TrustedInvocationObservationReleaseV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        control_ready: Some(true),
    };
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(ready),
        resources_ready: Some(true),
    };
    let InvocationObservationReadReplyV1::Ordinary(reply) = o
        .query_decoded_invocation_observation_v1(
            observer,
            &V::Object(&outer),
            InvocationObservationReadPortV1::Ordinary {
                current: &p,
                facts: &ordinary,
            },
            r,
        )?
    else {
        panic!("ordinary result")
    };
    Ok(reply)
}
pub fn check(
    o: &mut SourceCoordinator,
    id: &InvocationId,
    expected: InvocationPublicationV1,
    r: &mut Recorder,
) {
    check_in_context(o, id, &context(), expected, r);
}
pub fn check_in_context(
    o: &mut SourceCoordinator,
    id: &InvocationId,
    c: &SettingsExecutionContextV1,
    expected: InvocationPublicationV1,
    r: &mut Recorder,
) {
    let p = policy(c);
    let before = o.retention_usage();
    let generation = o.generation();
    let mut observer = o
        .open_invocation_observation_v1(
            id,
            c,
            &p,
            limits(),
            &TrustedInvocationPolicyFactsV1 {
                context: c,
                authorized: Some(true),
                scope: Some(true),
                continuous: Some(true),
                live: Some(true),
                resources_ready: Some(true),
                retained: Some(true),
                transfer_ready: Some(true),
                preparation_reserved: Some(true),
                registry_ready: Some(true),
                retention_ready: Some(true),
            },
            r,
        )
        .unwrap();
    let a = query(o, id, &mut observer, c, true, true, true, r).unwrap();
    assert_eq!(a.outcome.unwrap().publication, Some(expected));
    let b = query(o, id, &mut observer, c, false, true, true, r).unwrap();
    assert_eq!(a, b); // Same original result/reference, not another execution.
    assert_eq!(
        query(o, id, &mut observer, c, true, true, false, r),
        Err(InvocationError::Unavailable)
    );
    let withheld = query(o, id, &mut observer, c, false, false, true, r).unwrap();
    assert_eq!(withheld.outcome.unwrap().publication, None);
    assert_eq!(
        query(o, id, &mut observer, c, true, true, true, r)
            .unwrap()
            .outcome,
        a.outcome
    );
    assert_eq!(o.generation(), generation);
    let control = InvocationObservationPolicyControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    o.close_invocation_observation_v1(&mut observer, &control, r)
        .unwrap();
    o.retire_invocation_observation_v1(&mut observer, &control, r)
        .unwrap();
    assert_eq!(o.retention_usage(), before);
    o.retire_invocation_observation_policy_v1(id, &control, r)
        .unwrap();
}
