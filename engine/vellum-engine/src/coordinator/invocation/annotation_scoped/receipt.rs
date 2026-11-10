//! Private original decision facts. No recipient grant, projection or F06 owner.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::coordinator) enum Operation {
    Create,
    Body,
    Target,
    Delete,
}
impl Operation {
    pub(in crate::coordinator) fn of(mutation: &AnnotationMutation) -> Self {
        match mutation {
            AnnotationMutation::Create { .. } => Self::Create,
            AnnotationMutation::UpdateBody { .. } => Self::Body,
            AnnotationMutation::Reattach { .. } => Self::Target,
            AnnotationMutation::Delete { .. } => Self::Delete,
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(in crate::coordinator) struct Record {
    pub(in crate::coordinator) annotation: AnnotationId,
    pub(in crate::coordinator) present: bool,
    pub(in crate::coordinator) content_version: u64,
    pub(in crate::coordinator) target_generation: u64,
}
#[derive(Clone, PartialEq, Eq)]
pub(in crate::coordinator) enum Decision {
    Applied {
        publication: InvocationId,
        record: Record,
    },
    Unchanged {
        record: Record,
    },
    NotPublished,
}
#[derive(Clone, PartialEq, Eq)]
pub(in crate::coordinator) struct Receipt {
    pub(in crate::coordinator) invocation: InvocationId,
    pub(in crate::coordinator) operation: Operation,
    pub(in crate::coordinator) decision: Decision,
}
redacted!(Record, Decision, Receipt);
impl Receipt {
    pub(in crate::coordinator) fn not_published(id: &InvocationId, m: &AnnotationMutation) -> Self {
        Self {
            invocation: id.clone(),
            operation: Operation::of(m),
            decision: Decision::NotPublished,
        }
    }
}
pub(in crate::coordinator) struct Storage<'a> {
    pub(in crate::coordinator) invocation: &'a InvocationId,
    pub(in crate::coordinator) receipt: &'a mut Option<Receipt>,
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn stage_annotation_invocation_receipt(
        &self,
        id: &InvocationId,
        prepared: &PreparedTransaction,
        staged: &annotation::AnnotationDecision,
        r: &mut Recorder,
    ) -> Result<Receipt, CoordinatorError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let slot = self
                .invocation_slot(id)
                .map_err(|_| CoordinatorError::Faulted)?;
            let Some(BoundIntent::Annotation(original)) = &slot.intent else {
                return Err(CoordinatorError::Faulted);
            };
            let hold = slot
                .annotation_hold
                .as_ref()
                .ok_or(CoordinatorError::Faulted)?;
            if !matches!(slot.phase, Phase::Running)
                || original.invocation != *id
                || !Arc::ptr_eq(&original.owner, &self.namespace)
                || !Arc::ptr_eq(&original.marker, &hold.marker)
                || hold.receipt.is_some()
                || prepared.standalone_annotation_seal() != original.plan
                || !prepared.standalone_annotation_matches_mutation(&original.mutation)
            {
                return Err(CoordinatorError::Faulted);
            }
            let plan = original.plan.as_ref().ok_or(CoordinatorError::Faulted)?;
            let fact = staged.standalone_fact(self, &plan.id)?;
            let operation = Operation::of(&original.mutation);
            if fact.content_version == 0
                || fact.target_generation == 0
                || fact.present == (operation == Operation::Delete)
                || (!staged.changed() && matches!(operation, Operation::Create | Operation::Delete))
            {
                return Err(CoordinatorError::Faulted);
            }
            let record = Record {
                annotation: fact.annotation,
                present: fact.present,
                content_version: fact.content_version,
                target_generation: fact.target_generation,
            };
            let decision = if staged.changed() {
                Decision::Applied {
                    publication: id.clone(),
                    record,
                }
            } else {
                Decision::Unchanged { record }
            };
            Ok(Receipt {
                invocation: id.clone(),
                operation,
                decision,
            })
        })();
        observe_service(
            r,
            Boundary::AnnotationDecisionStaging,
            &result,
            timing.finish(),
        );
        result
    }
}
