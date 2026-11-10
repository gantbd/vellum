//! Actual queued consumers, not a caller-supplied root or discharge assertion.
use super::*;
impl SourceCoordinator {
    pub(super) fn queued_source_producer_retained(
        &self,
        id: &SaveId,
        producer: &Task,
    ) -> Result<bool, ResourceError> {
        // A typed current read/ack anchor is also a consumer of the later
        // write prefix. "Newer qualified" alone is not a published newer root
        // anchor and cannot erase records before a future own-chain admission.
        if producer.knowledge.data.application == Application::Written {
            for b in self.resources.bindings.iter().filter(|b| {
                b.identity == producer.identity
                    && b.baseline.as_ref().is_some_and(|v| v.epoch.is_some())
            }) {
                let baseline = b.baseline.as_ref().ok_or(ResourceError::Faulted)?;
                let Some(anchor) = &baseline.anchor else {
                    return Ok(true);
                };
                let t = self
                    .resource_task(anchor)
                    .map_err(|_| ResourceError::Faulted)?;
                if !t.source_baseline_candidate()
                    || t.identity != b.identity
                    || t.captured.document != b.document
                    || t.binding_generation != b.generation
                    || !Arc::ptr_eq(&t.registration, &b.registration)
                    || t.payload != baseline.bytes
                {
                    return Err(ResourceError::Faulted);
                }
                let root_serial = t
                    .dispatch_serial
                    .filter(|s| *s > 0)
                    .ok_or(ResourceError::Faulted)?;
                let serial = producer
                    .dispatch_serial
                    .filter(|s| *s > 0)
                    .ok_or(ResourceError::Faulted)?;
                if !t.qualified() || root_serial <= serial {
                    return Ok(true);
                }
            }
        }
        for peer in self
            .resources
            .tasks
            .iter()
            .flatten()
            .filter(|p| p.phase == SavePhase::Queued && !p.stop_requested)
        {
            let anchor_serial = match &peer.condition {
                WriteCondition::Absent => 0,
                WriteCondition::Present(baseline) => match &baseline.anchor {
                    None => 0,
                    Some(anchor) => {
                        let task = self
                            .resource_task(anchor)
                            .map_err(|_| ResourceError::Faulted)?;
                        if !task.source_baseline_candidate()
                            || task.phase != SavePhase::Dispatched
                            || task.identity != peer.identity
                            || task.captured.document != peer.captured.document
                            || task.binding_generation != peer.binding_generation
                            || !Arc::ptr_eq(&task.registration, &peer.registration)
                            || task.payload != baseline.bytes
                            || task.knowledge.data.version != baseline.validator
                            || task.knowledge.data.epoch != baseline.epoch
                        {
                            return Err(ResourceError::Faulted);
                        }
                        self.validate_source_capture(task)?;
                        let serial = task
                            .dispatch_serial
                            .filter(|s| *s > 0)
                            .ok_or(ResourceError::Faulted)?;
                        if anchor == id {
                            return Ok(true);
                        }
                        serial
                    }
                },
            };
            // A newer current baseline cannot discard intermediate written
            // records an older queued capture still needs to explain its chain.
            // Loaded/explicit destination captures have no own-write anchor (0).
            if peer.identity == producer.identity
                && producer.knowledge.data.application == Application::Written
                && (producer.kind == ArtifactKind::Source || peer.continuity.is_some())
            {
                let serial = producer
                    .dispatch_serial
                    .filter(|s| *s > 0)
                    .ok_or(ResourceError::Faulted)?;
                if anchor_serial <= serial {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
