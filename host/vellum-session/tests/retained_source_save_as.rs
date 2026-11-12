use std::collections::VecDeque;
use vellum_host::save_as::{SaveAsBindingV1, SaveAsPersistenceV1};
use vellum_session::host::{
    PreparationDischargeV1, RetainedSourceSaveAsErrorV1, RetainedSourceSaveAsStageV1,
    RetainedSourceSaveAsV1, SourceSaveAsObservationV1, SourceSaveAsSessionPortV1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fault {
    Injected,
}

#[derive(Default)]
struct Calls {
    prepare: usize,
    admit: usize,
    discard: usize,
    service: usize,
    observe: usize,
    activate: usize,
    recover: usize,
    release: usize,
    retire: usize,
}

struct FakePort {
    calls: Calls,
    prepare_fails: bool,
    admit_fails: bool,
    discard: Result<PreparationDischargeV1, Fault>,
    service: Result<SourceSaveAsObservationV1, Fault>,
    observations: VecDeque<Result<SourceSaveAsObservationV1, Fault>>,
    activate_fails: bool,
    recover_fails: bool,
    release_failures: usize,
    retire_failures: usize,
}

impl Default for FakePort {
    fn default() -> Self {
        Self {
            calls: Calls::default(),
            prepare_fails: false,
            admit_fails: false,
            discard: Ok(PreparationDischargeV1::Discarded),
            service: Ok(observation(SaveAsPersistenceV1::Durable, Some(false))),
            observations: VecDeque::new(),
            activate_fails: false,
            recover_fails: false,
            release_failures: 0,
            retire_failures: 0,
        }
    }
}

impl SourceSaveAsSessionPortV1 for FakePort {
    type Preparation = u8;
    type Attempt = u8;
    type Activation = u8;
    type Error = Fault;

    fn prepare(&mut self) -> Result<Self::Preparation, Self::Error> {
        self.calls.prepare += 1;
        if self.prepare_fails {
            Err(Fault::Injected)
        } else {
            Ok(1)
        }
    }

    fn admit(&mut self, preparation: &Self::Preparation) -> Result<Self::Attempt, Self::Error> {
        self.calls.admit += 1;
        assert_eq!(*preparation, 1);
        if self.admit_fails {
            Err(Fault::Injected)
        } else {
            Ok(2)
        }
    }

    fn discard_preparation(
        &mut self,
        preparation: &Self::Preparation,
    ) -> Result<PreparationDischargeV1, Self::Error> {
        self.calls.discard += 1;
        assert_eq!(*preparation, 1);
        self.discard
    }

    fn service(
        &mut self,
        attempt: &Self::Attempt,
    ) -> Result<SourceSaveAsObservationV1, Self::Error> {
        self.calls.service += 1;
        assert_eq!(*attempt, 2);
        self.service
    }

    fn observe(
        &mut self,
        attempt: &Self::Attempt,
    ) -> Result<SourceSaveAsObservationV1, Self::Error> {
        self.calls.observe += 1;
        assert_eq!(*attempt, 2);
        self.observations
            .pop_front()
            .unwrap_or(Ok(observation(SaveAsPersistenceV1::Unknown, None)))
    }

    fn activate(&mut self, attempt: &Self::Attempt) -> Result<Self::Activation, Self::Error> {
        self.calls.activate += 1;
        assert_eq!(*attempt, 2);
        if self.activate_fails {
            Err(Fault::Injected)
        } else {
            Ok(3)
        }
    }

    fn recover_activation(
        &mut self,
        attempt: &Self::Attempt,
    ) -> Result<Self::Activation, Self::Error> {
        self.calls.recover += 1;
        assert_eq!(*attempt, 2);
        if self.recover_fails {
            Err(Fault::Injected)
        } else {
            Ok(3)
        }
    }

    fn release(&mut self, attempt: &Self::Attempt) -> Result<bool, Self::Error> {
        self.calls.release += 1;
        assert_eq!(*attempt, 2);
        if self.release_failures > 0 {
            self.release_failures -= 1;
            Err(Fault::Injected)
        } else {
            Ok(true)
        }
    }

    fn retire(&mut self, attempt: &Self::Attempt) -> Result<(), Self::Error> {
        self.calls.retire += 1;
        assert_eq!(*attempt, 2);
        if self.retire_failures > 0 {
            self.retire_failures -= 1;
            Err(Fault::Injected)
        } else {
            Ok(())
        }
    }
}

const fn observation(
    persistence: SaveAsPersistenceV1,
    binding: Option<bool>,
) -> SourceSaveAsObservationV1 {
    SourceSaveAsObservationV1::new(persistence, binding)
}

fn queued(port: &mut FakePort) -> RetainedSourceSaveAsV1<FakePort> {
    let mut operation = RetainedSourceSaveAsV1::new();
    operation.prepare(port).unwrap();
    operation.admit(port).unwrap();
    operation
}

fn written(port: &mut FakePort) -> RetainedSourceSaveAsV1<FakePort> {
    let mut operation = queued(port);
    operation.service(port).unwrap();
    assert_eq!(operation.stage(), RetainedSourceSaveAsStageV1::Written);
    operation
}

#[test]
fn happy_path_performs_each_effect_once_and_returns_activation() {
    let mut port = FakePort::default();
    let mut operation = written(&mut port);
    operation.activate(&mut port).unwrap();
    operation.release(&mut port).unwrap();
    operation.retire(&mut port).unwrap();

    assert_eq!(operation.persistence(), SaveAsPersistenceV1::Durable);
    assert_eq!(operation.binding(), SaveAsBindingV1::Activated);
    assert_eq!(operation.stage(), RetainedSourceSaveAsStageV1::Complete);
    let Ok(activation) = operation.into_activation() else {
        panic!("complete operation must yield its activation")
    };
    assert_eq!(activation, Some(3));
    assert_eq!(port.calls.prepare, 1);
    assert_eq!(port.calls.admit, 1);
    assert_eq!(port.calls.service, 1);
    assert_eq!(port.calls.activate, 1);
    assert_eq!(port.calls.recover, 0);
    assert_eq!(port.calls.release, 1);
    assert_eq!(port.calls.retire, 1);
}

#[test]
fn admission_error_fences_preparation_and_never_retries_admission() {
    let mut port = FakePort {
        admit_fails: true,
        ..FakePort::default()
    };
    let mut operation = RetainedSourceSaveAsV1::new();
    operation.prepare(&mut port).unwrap();
    assert!(matches!(
        operation.admit(&mut port),
        Err(RetainedSourceSaveAsErrorV1::Port(Fault::Injected))
    ));
    assert_eq!(
        operation.stage(),
        RetainedSourceSaveAsStageV1::AdmissionFenced
    );
    assert!(matches!(
        operation.admit(&mut port),
        Err(RetainedSourceSaveAsErrorV1::InvalidTransition)
    ));
    assert_eq!(port.calls.admit, 1);
    assert!(operation.reconcile_admission(&mut port).unwrap());
    assert_eq!(operation.stage(), RetainedSourceSaveAsStageV1::Complete);
    assert_eq!(port.calls.discard, 1);
}

#[test]
fn consumed_or_unknown_preparation_remains_fenced() {
    for discharge in [
        PreparationDischargeV1::AlreadyConsumed,
        PreparationDischargeV1::Unknown,
    ] {
        let mut port = FakePort {
            admit_fails: true,
            discard: Ok(discharge),
            ..FakePort::default()
        };
        let mut operation = RetainedSourceSaveAsV1::new();
        operation.prepare(&mut port).unwrap();
        let _ = operation.admit(&mut port);
        assert!(!operation.reconcile_admission(&mut port).unwrap());
        assert_eq!(
            operation.stage(),
            RetainedSourceSaveAsStageV1::AdmissionFenced
        );
        assert_eq!(port.calls.admit, 1);
    }
}

#[test]
fn service_error_can_close_not_durable_without_activation() {
    let mut port = FakePort {
        service: Err(Fault::Injected),
        observations: VecDeque::from([Ok(observation(
            SaveAsPersistenceV1::NotDurable,
            Some(false),
        ))]),
        ..FakePort::default()
    };
    let mut operation = queued(&mut port);
    assert!(operation.service(&mut port).is_err());
    assert_eq!(operation.stage(), RetainedSourceSaveAsStageV1::WriteFenced);
    assert!(operation.reconcile_write(&mut port).unwrap());
    assert_eq!(operation.persistence(), SaveAsPersistenceV1::NotDurable);
    assert_eq!(operation.binding(), SaveAsBindingV1::NotActivated);
    assert_eq!(
        operation.stage(),
        RetainedSourceSaveAsStageV1::CompletionPending
    );
    assert!(matches!(
        operation.activate(&mut port),
        Err(RetainedSourceSaveAsErrorV1::InvalidTransition)
    ));
    assert_eq!(port.calls.activate, 0);
}

#[test]
fn unknown_write_remains_fenced_and_is_never_serviced_twice() {
    let mut port = FakePort {
        service: Err(Fault::Injected),
        observations: VecDeque::from([Ok(observation(SaveAsPersistenceV1::Unknown, None))]),
        ..FakePort::default()
    };
    let mut operation = queued(&mut port);
    let _ = operation.service(&mut port);
    assert!(!operation.reconcile_write(&mut port).unwrap());
    assert_eq!(operation.stage(), RetainedSourceSaveAsStageV1::WriteFenced);
    assert_eq!(operation.persistence(), SaveAsPersistenceV1::Unknown);
    assert!(operation.service(&mut port).is_err());
    assert_eq!(port.calls.service, 1);
}

#[test]
fn activation_error_recovers_published_binding_without_second_activation() {
    let mut port = FakePort {
        activate_fails: true,
        observations: VecDeque::from([Ok(observation(SaveAsPersistenceV1::Durable, Some(true)))]),
        ..FakePort::default()
    };
    let mut operation = written(&mut port);
    assert!(operation.activate(&mut port).is_err());
    assert_eq!(
        operation.stage(),
        RetainedSourceSaveAsStageV1::ActivationFenced
    );
    assert!(operation.reconcile_activation(&mut port).unwrap());
    assert_eq!(operation.stage(), RetainedSourceSaveAsStageV1::Activated);
    assert_eq!(operation.binding(), SaveAsBindingV1::Activated);
    assert_eq!(port.calls.activate, 1);
    assert_eq!(port.calls.recover, 1);
}

#[test]
fn activation_refusal_closes_not_activated_without_retry() {
    let mut port = FakePort {
        activate_fails: true,
        observations: VecDeque::from([Ok(observation(SaveAsPersistenceV1::Durable, Some(false)))]),
        ..FakePort::default()
    };
    let mut operation = written(&mut port);
    let _ = operation.activate(&mut port);
    assert!(operation.reconcile_activation(&mut port).unwrap());
    assert_eq!(operation.persistence(), SaveAsPersistenceV1::Durable);
    assert_eq!(operation.binding(), SaveAsBindingV1::NotActivated);
    assert_eq!(
        operation.stage(),
        RetainedSourceSaveAsStageV1::CompletionPending
    );
    assert_eq!(port.calls.activate, 1);
    assert_eq!(port.calls.recover, 0);
}

#[test]
fn activation_uncertainty_remains_fenced() {
    let mut port = FakePort {
        activate_fails: true,
        observations: VecDeque::from([Ok(observation(SaveAsPersistenceV1::Durable, None))]),
        ..FakePort::default()
    };
    let mut operation = written(&mut port);
    let _ = operation.activate(&mut port);
    assert!(!operation.reconcile_activation(&mut port).unwrap());
    assert_eq!(operation.binding(), SaveAsBindingV1::Unknown);
    assert_eq!(
        operation.stage(),
        RetainedSourceSaveAsStageV1::ActivationFenced
    );
}

#[test]
fn release_and_retirement_errors_retain_tokens_for_reconciliation() {
    let mut port = FakePort {
        release_failures: 1,
        retire_failures: 1,
        ..FakePort::default()
    };
    let mut operation = written(&mut port);
    operation.activate(&mut port).unwrap();
    assert!(operation.release(&mut port).is_err());
    assert_eq!(
        operation.stage(),
        RetainedSourceSaveAsStageV1::ReleaseFenced
    );
    assert!(operation.reconcile_release(&mut port).unwrap());
    assert_eq!(operation.stage(), RetainedSourceSaveAsStageV1::Released);
    assert!(operation.retire(&mut port).is_err());
    assert_eq!(
        operation.stage(),
        RetainedSourceSaveAsStageV1::RetirementFenced
    );
    assert!(operation.reconcile_retirement(&mut port).unwrap());
    assert_eq!(operation.stage(), RetainedSourceSaveAsStageV1::Complete);
    assert_eq!(port.calls.release, 2);
    assert_eq!(port.calls.retire, 2);
}

#[test]
fn illegal_transitions_preserve_state_and_make_no_port_calls() {
    let mut port = FakePort::default();
    let mut operation = RetainedSourceSaveAsV1::new();
    assert!(matches!(
        operation.service(&mut port),
        Err(RetainedSourceSaveAsErrorV1::InvalidTransition)
    ));
    assert_eq!(operation.stage(), RetainedSourceSaveAsStageV1::Ready);
    assert_eq!(port.calls.service, 0);
    operation.prepare(&mut port).unwrap();
    assert!(operation.activate(&mut port).is_err());
    assert_eq!(operation.stage(), RetainedSourceSaveAsStageV1::Prepared);
    assert_eq!(port.calls.activate, 0);
}
