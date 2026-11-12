//! Portable retained coordination for one Save As lifecycle.
//!
//! This module owns no locator, adapter, policy, protocol, or UI behavior. A
//! command-scoped port supplies opaque native tokens. Every fallible effect
//! transition retains the token needed for observation or cleanup, and no
//! reconciliation path can repeat admission, write, or activation.

use vellum_host::save_as::{SaveAsBindingV1, SaveAsPersistenceV1};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreparationDischargeV1 {
    Discarded,
    AlreadyConsumed,
    Unknown,
}

/// Content-free observation of the original native attempt. The command port
/// derives this from engine/adapter evidence; it is not requester input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSaveAsObservationV1 {
    persistence: SaveAsPersistenceV1,
    binding_applied: Option<bool>,
}

impl SourceSaveAsObservationV1 {
    pub const fn new(persistence: SaveAsPersistenceV1, binding_applied: Option<bool>) -> Self {
        Self {
            persistence,
            binding_applied,
        }
    }

    pub const fn persistence(self) -> SaveAsPersistenceV1 {
        self.persistence
    }

    pub const fn binding_applied(self) -> Option<bool> {
        self.binding_applied
    }
}

pub trait SourceSaveAsSessionPortV1 {
    type Preparation;
    type Attempt;
    type Activation;
    type Error;

    fn prepare(&mut self) -> Result<Self::Preparation, Self::Error>;
    fn admit(&mut self, preparation: &Self::Preparation) -> Result<Self::Attempt, Self::Error>;
    fn discard_preparation(
        &mut self,
        preparation: &Self::Preparation,
    ) -> Result<PreparationDischargeV1, Self::Error>;
    fn service(
        &mut self,
        attempt: &Self::Attempt,
    ) -> Result<SourceSaveAsObservationV1, Self::Error>;
    fn observe(
        &mut self,
        attempt: &Self::Attempt,
    ) -> Result<SourceSaveAsObservationV1, Self::Error>;
    fn activate(&mut self, attempt: &Self::Attempt) -> Result<Self::Activation, Self::Error>;
    fn recover_activation(
        &mut self,
        attempt: &Self::Attempt,
    ) -> Result<Self::Activation, Self::Error>;
    fn release(&mut self, attempt: &Self::Attempt) -> Result<bool, Self::Error>;
    fn retire(&mut self, attempt: &Self::Attempt) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetainedSourceSaveAsStageV1 {
    Ready,
    Prepared,
    AdmissionFenced,
    Queued,
    WriteFenced,
    Written,
    ActivationFenced,
    Activated,
    CompletionPending,
    ReleaseFenced,
    Released,
    RetirementFenced,
    Complete,
}

#[derive(Debug)]
pub enum RetainedSourceSaveAsErrorV1<E> {
    InvalidTransition,
    Port(E),
}

enum State<P: SourceSaveAsSessionPortV1> {
    Transitioning,
    Ready,
    Prepared(P::Preparation),
    AdmissionFenced(P::Preparation),
    Queued(P::Attempt),
    WriteFenced(P::Attempt),
    Written(P::Attempt),
    ActivationFenced(P::Attempt),
    Activated {
        attempt: P::Attempt,
        activation: P::Activation,
    },
    CompletionPending {
        attempt: P::Attempt,
        activation: Option<P::Activation>,
    },
    ReleaseFenced {
        attempt: P::Attempt,
        activation: Option<P::Activation>,
    },
    Released {
        attempt: P::Attempt,
        activation: Option<P::Activation>,
    },
    RetirementFenced {
        attempt: P::Attempt,
        activation: Option<P::Activation>,
    },
    Complete(Option<P::Activation>),
}

/// Retains every affine token until the corresponding native obligation is
/// discharged. Dropping a live value performs no hidden cleanup.
#[must_use = "a retained Save As coordinator may own native cleanup or reconciliation obligations"]
pub struct RetainedSourceSaveAsV1<P: SourceSaveAsSessionPortV1> {
    state: State<P>,
    persistence: SaveAsPersistenceV1,
    binding: SaveAsBindingV1,
}

impl<P: SourceSaveAsSessionPortV1> Default for RetainedSourceSaveAsV1<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: SourceSaveAsSessionPortV1> RetainedSourceSaveAsV1<P> {
    pub const fn new() -> Self {
        Self {
            state: State::Ready,
            persistence: SaveAsPersistenceV1::NotAttempted,
            binding: SaveAsBindingV1::NotActivated,
        }
    }

    pub const fn persistence(&self) -> SaveAsPersistenceV1 {
        self.persistence
    }

    pub const fn binding(&self) -> SaveAsBindingV1 {
        self.binding
    }

    pub const fn stage(&self) -> RetainedSourceSaveAsStageV1 {
        match self.state {
            State::Transitioning => RetainedSourceSaveAsStageV1::Ready,
            State::Ready => RetainedSourceSaveAsStageV1::Ready,
            State::Prepared(_) => RetainedSourceSaveAsStageV1::Prepared,
            State::AdmissionFenced(_) => RetainedSourceSaveAsStageV1::AdmissionFenced,
            State::Queued(_) => RetainedSourceSaveAsStageV1::Queued,
            State::WriteFenced(_) => RetainedSourceSaveAsStageV1::WriteFenced,
            State::Written(_) => RetainedSourceSaveAsStageV1::Written,
            State::ActivationFenced(_) => RetainedSourceSaveAsStageV1::ActivationFenced,
            State::Activated { .. } => RetainedSourceSaveAsStageV1::Activated,
            State::CompletionPending { .. } => RetainedSourceSaveAsStageV1::CompletionPending,
            State::ReleaseFenced { .. } => RetainedSourceSaveAsStageV1::ReleaseFenced,
            State::Released { .. } => RetainedSourceSaveAsStageV1::Released,
            State::RetirementFenced { .. } => RetainedSourceSaveAsStageV1::RetirementFenced,
            State::Complete(_) => RetainedSourceSaveAsStageV1::Complete,
        }
    }

    pub fn prepare(&mut self, port: &mut P) -> Result<(), RetainedSourceSaveAsErrorV1<P::Error>> {
        let state = self.take_state();
        if !matches!(state, State::Ready) {
            self.state = state;
            return Err(RetainedSourceSaveAsErrorV1::InvalidTransition);
        };
        match port.prepare() {
            Ok(preparation) => {
                self.state = State::Prepared(preparation);
                Ok(())
            }
            Err(error) => {
                self.state = State::Ready;
                Err(RetainedSourceSaveAsErrorV1::Port(error))
            }
        }
    }

    pub fn admit(&mut self, port: &mut P) -> Result<(), RetainedSourceSaveAsErrorV1<P::Error>> {
        let state = self.take_state();
        let State::Prepared(preparation) = state else {
            self.state = state;
            return Err(RetainedSourceSaveAsErrorV1::InvalidTransition);
        };
        match port.admit(&preparation) {
            Ok(attempt) => {
                self.state = State::Queued(attempt);
                Ok(())
            }
            Err(error) => {
                self.state = State::AdmissionFenced(preparation);
                Err(RetainedSourceSaveAsErrorV1::Port(error))
            }
        }
    }

    pub fn reconcile_admission(
        &mut self,
        port: &mut P,
    ) -> Result<bool, RetainedSourceSaveAsErrorV1<P::Error>> {
        let state = self.take_state();
        let State::AdmissionFenced(preparation) = state else {
            self.state = state;
            return Err(RetainedSourceSaveAsErrorV1::InvalidTransition);
        };
        match port.discard_preparation(&preparation) {
            Ok(PreparationDischargeV1::Discarded) => {
                self.state = State::Complete(None);
                Ok(true)
            }
            Ok(PreparationDischargeV1::AlreadyConsumed | PreparationDischargeV1::Unknown) => {
                self.state = State::AdmissionFenced(preparation);
                Ok(false)
            }
            Err(error) => {
                self.state = State::AdmissionFenced(preparation);
                Err(RetainedSourceSaveAsErrorV1::Port(error))
            }
        }
    }

    pub fn service(&mut self, port: &mut P) -> Result<(), RetainedSourceSaveAsErrorV1<P::Error>> {
        let state = self.take_state();
        let State::Queued(attempt) = state else {
            self.state = state;
            return Err(RetainedSourceSaveAsErrorV1::InvalidTransition);
        };
        match port.service(&attempt) {
            Ok(observation) => {
                self.persistence = observation.persistence();
                self.state = match self.persistence {
                    SaveAsPersistenceV1::Durable => State::Written(attempt),
                    SaveAsPersistenceV1::NotDurable | SaveAsPersistenceV1::NotAttempted => {
                        State::CompletionPending {
                            attempt,
                            activation: None,
                        }
                    }
                    SaveAsPersistenceV1::Unknown => State::WriteFenced(attempt),
                };
                Ok(())
            }
            Err(error) => {
                self.persistence = SaveAsPersistenceV1::Unknown;
                self.state = State::WriteFenced(attempt);
                Err(RetainedSourceSaveAsErrorV1::Port(error))
            }
        }
    }

    pub fn reconcile_write(
        &mut self,
        port: &mut P,
    ) -> Result<bool, RetainedSourceSaveAsErrorV1<P::Error>> {
        let state = self.take_state();
        let State::WriteFenced(attempt) = state else {
            self.state = state;
            return Err(RetainedSourceSaveAsErrorV1::InvalidTransition);
        };
        match port.observe(&attempt) {
            Ok(observation) => {
                self.persistence = observation.persistence();
                match self.persistence {
                    SaveAsPersistenceV1::Durable => {
                        self.state = State::Written(attempt);
                        Ok(true)
                    }
                    SaveAsPersistenceV1::NotDurable | SaveAsPersistenceV1::NotAttempted => {
                        self.state = State::CompletionPending {
                            attempt,
                            activation: None,
                        };
                        Ok(true)
                    }
                    SaveAsPersistenceV1::Unknown => {
                        self.state = State::WriteFenced(attempt);
                        Ok(false)
                    }
                }
            }
            Err(error) => {
                self.state = State::WriteFenced(attempt);
                Err(RetainedSourceSaveAsErrorV1::Port(error))
            }
        }
    }

    pub fn activate(&mut self, port: &mut P) -> Result<(), RetainedSourceSaveAsErrorV1<P::Error>> {
        let state = self.take_state();
        let State::Written(attempt) = state else {
            self.state = state;
            return Err(RetainedSourceSaveAsErrorV1::InvalidTransition);
        };
        match port.activate(&attempt) {
            Ok(activation) => {
                self.binding = SaveAsBindingV1::Activated;
                self.state = State::Activated {
                    attempt,
                    activation,
                };
                Ok(())
            }
            Err(error) => {
                self.binding = SaveAsBindingV1::Unknown;
                self.state = State::ActivationFenced(attempt);
                Err(RetainedSourceSaveAsErrorV1::Port(error))
            }
        }
    }

    pub fn reconcile_activation(
        &mut self,
        port: &mut P,
    ) -> Result<bool, RetainedSourceSaveAsErrorV1<P::Error>> {
        let state = self.take_state();
        let State::ActivationFenced(attempt) = state else {
            self.state = state;
            return Err(RetainedSourceSaveAsErrorV1::InvalidTransition);
        };
        match port.observe(&attempt) {
            Ok(observation) if observation.binding_applied() == Some(true) => {
                match port.recover_activation(&attempt) {
                    Ok(activation) => {
                        self.binding = SaveAsBindingV1::Activated;
                        self.state = State::Activated {
                            attempt,
                            activation,
                        };
                        Ok(true)
                    }
                    Err(error) => {
                        self.state = State::ActivationFenced(attempt);
                        Err(RetainedSourceSaveAsErrorV1::Port(error))
                    }
                }
            }
            Ok(observation) if observation.binding_applied() == Some(false) => {
                self.binding = SaveAsBindingV1::NotActivated;
                self.state = State::CompletionPending {
                    attempt,
                    activation: None,
                };
                Ok(true)
            }
            Ok(_) => {
                self.state = State::ActivationFenced(attempt);
                Ok(false)
            }
            Err(error) => {
                self.state = State::ActivationFenced(attempt);
                Err(RetainedSourceSaveAsErrorV1::Port(error))
            }
        }
    }

    pub fn release(&mut self, port: &mut P) -> Result<(), RetainedSourceSaveAsErrorV1<P::Error>> {
        let state = self.take_state();
        let (attempt, activation) = match state {
            State::Activated {
                attempt,
                activation,
            } => (attempt, Some(activation)),
            State::CompletionPending {
                attempt,
                activation,
            } => (attempt, activation),
            other => {
                self.state = other;
                return Err(RetainedSourceSaveAsErrorV1::InvalidTransition);
            }
        };
        match port.release(&attempt) {
            Ok(_) => {
                self.state = State::Released {
                    attempt,
                    activation,
                };
                Ok(())
            }
            Err(error) => {
                self.state = State::ReleaseFenced {
                    attempt,
                    activation,
                };
                Err(RetainedSourceSaveAsErrorV1::Port(error))
            }
        }
    }

    pub fn reconcile_release(
        &mut self,
        port: &mut P,
    ) -> Result<bool, RetainedSourceSaveAsErrorV1<P::Error>> {
        let state = self.take_state();
        let State::ReleaseFenced {
            attempt,
            activation,
        } = state
        else {
            self.state = state;
            return Err(RetainedSourceSaveAsErrorV1::InvalidTransition);
        };
        match port.release(&attempt) {
            Ok(_) => {
                self.state = State::Released {
                    attempt,
                    activation,
                };
                Ok(true)
            }
            Err(error) => {
                self.state = State::ReleaseFenced {
                    attempt,
                    activation,
                };
                Err(RetainedSourceSaveAsErrorV1::Port(error))
            }
        }
    }

    pub fn retire(&mut self, port: &mut P) -> Result<(), RetainedSourceSaveAsErrorV1<P::Error>> {
        let state = self.take_state();
        let State::Released {
            attempt,
            activation,
        } = state
        else {
            self.state = state;
            return Err(RetainedSourceSaveAsErrorV1::InvalidTransition);
        };
        match port.retire(&attempt) {
            Ok(()) => {
                self.state = State::Complete(activation);
                Ok(())
            }
            Err(error) => {
                self.state = State::RetirementFenced {
                    attempt,
                    activation,
                };
                Err(RetainedSourceSaveAsErrorV1::Port(error))
            }
        }
    }

    pub fn reconcile_retirement(
        &mut self,
        port: &mut P,
    ) -> Result<bool, RetainedSourceSaveAsErrorV1<P::Error>> {
        let state = self.take_state();
        let State::RetirementFenced {
            attempt,
            activation,
        } = state
        else {
            self.state = state;
            return Err(RetainedSourceSaveAsErrorV1::InvalidTransition);
        };
        match port.retire(&attempt) {
            Ok(()) => {
                self.state = State::Complete(activation);
                Ok(true)
            }
            Err(error) => {
                self.state = State::RetirementFenced {
                    attempt,
                    activation,
                };
                Err(RetainedSourceSaveAsErrorV1::Port(error))
            }
        }
    }

    pub fn into_activation(self) -> Result<Option<P::Activation>, Self> {
        match self {
            Self {
                state: State::Complete(activation),
                ..
            } => Ok(activation),
            other => Err(other),
        }
    }

    fn take_state(&mut self) -> State<P> {
        std::mem::replace(&mut self.state, State::Transitioning)
    }
}
