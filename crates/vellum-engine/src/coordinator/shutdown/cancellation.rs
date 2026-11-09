//! Resumable finite cancellation over the accepted domain's existing local work.
//! Never dispatches, disposes, retires retention, or cancels an external worker.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownCancellationStatusV1 {
    More,
    FinishedScan,
    Blocked(CoordinatorError),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShutdownCancellationProgressV1 {
    pub status: ShutdownCancellationStatusV1,
    pub scanned_slots: usize,
    pub stopped_invocations: usize,
    pub stopped_queued_saves: usize,
}
#[derive(Clone, Copy)]
enum Stage {
    Invocations,
    QueuedSaves,
    Finished,
    Blocked(CoordinatorError),
}
/// Fixed-size host-held metadata, not a new engine task/lease or wait budget.
/// Private positions advance monotonically within the original fenced tables.
pub struct ShutdownCancellationCursorV1 {
    fence: ShutdownFenceIdV1,
    invocation_end: usize,
    save_end: usize,
    invocation_next: usize,
    save_next: usize,
    stopped_invocations: usize,
    stopped_queued_saves: usize,
    stage: Stage,
}
impl std::fmt::Debug for ShutdownCancellationCursorV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShutdownCancellation(<opaque>)")
    }
}
impl ShutdownCancellationCursorV1 {
    pub fn progress(&self) -> ShutdownCancellationProgressV1 {
        ShutdownCancellationProgressV1 {
            status: match self.stage {
                Stage::Invocations | Stage::QueuedSaves => ShutdownCancellationStatusV1::More,
                Stage::Finished => ShutdownCancellationStatusV1::FinishedScan,
                Stage::Blocked(e) => ShutdownCancellationStatusV1::Blocked(e),
            },
            scanned_slots: self.invocation_next + self.save_next,
            stopped_invocations: self.stopped_invocations,
            stopped_queued_saves: self.stopped_queued_saves,
        }
    }
}
impl SourceCoordinator {
    pub fn begin_shutdown_cancellation_v1(
        &self,
        fence: &ShutdownFenceIdV1,
        control: &TrustedShutdownFenceFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownCancellationCursorV1, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.validate_shutdown_owner_control(fence, control)?;
            self.shutdown_cancellation_plan()?;
            let invocation_end = self.shutdown_cancellation_invocation_end();
            let save_end = self.shutdown_cancellation_save_end();
            invocation_end
                .checked_add(save_end)
                .ok_or(CoordinatorError::Limit)?;
            Ok(ShutdownCancellationCursorV1 {
                fence: fence.clone(),
                invocation_end,
                save_end,
                invocation_next: 0,
                save_next: 0,
                stopped_invocations: 0,
                stopped_queued_saves: 0,
                stage: Stage::Invocations,
            })
        })();
        observe_service(
            r,
            Boundary::ShutdownCancellationStart,
            &result,
            timer.finish(),
        );
        result
    }
    /// Visit at most max_work original slots. Required source/comment saves and
    /// dispatched effects are preserved. FinishedScan is not shutdown completion.
    /// A blocked scan preserves prior progress and cannot resume silently.
    pub fn advance_shutdown_cancellation_v1(
        &mut self,
        cursor: &mut ShutdownCancellationCursorV1,
        max_work: usize,
        control: &TrustedShutdownFenceFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownCancellationProgressV1, CoordinatorError> {
        let timer = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.validate_shutdown_owner_control(&cursor.fence, control)?;
            self.shutdown_cancellation_plan()?;
            if max_work == 0 {
                return Err(CoordinatorError::Limit);
            }
            let mut work = 0usize;
            loop {
                match cursor.stage {
                    Stage::Invocations if cursor.invocation_next == cursor.invocation_end => {
                        cursor.stage = Stage::QueuedSaves;
                        continue;
                    }
                    Stage::QueuedSaves if cursor.save_next == cursor.save_end => {
                        cursor.stage = Stage::Finished;
                        continue;
                    }
                    Stage::Finished | Stage::Blocked(_) => break,
                    _ => {}
                }
                if work == max_work {
                    break;
                }
                let action = (|| {
                    match cursor.stage {
                        Stage::Invocations => {
                            if let Some(id) =
                                self.shutdown_cancellation_invocation_at(cursor.invocation_next)?
                            {
                                self.stop_shutdown_invocation_v1(&cursor.fence, &id, control, r)?;
                                cursor.stopped_invocations += 1;
                            }
                            cursor.invocation_next += 1;
                        }
                        Stage::QueuedSaves => {
                            if let Some((id, document)) =
                                self.shutdown_cancellation_save_at(cursor.save_next)
                                && !self.shutdown_cancellation_required_save(&id, &document)?
                            {
                                self.stop_save(&id, r).map_err(CoordinatorError::Resource)?;
                                cursor.stopped_queued_saves += 1;
                            }
                            cursor.save_next += 1;
                        }
                        Stage::Finished | Stage::Blocked(_) => {}
                    }
                    Ok::<_, CoordinatorError>(())
                })();
                if let Err(e) = action {
                    if matches!(
                        e,
                        CoordinatorError::Faulted
                            | CoordinatorError::Resource(crate::resource::ResourceError::Faulted)
                    ) {
                        self.faulted = true;
                    }
                    cursor.stage = Stage::Blocked(e);
                    break;
                }
                work += 1;
            }
            Ok(cursor.progress())
        }));
        let result = match caught {
            Ok(result) => result,
            Err(panic) => {
                self.faulted = true;
                cursor.stage = Stage::Blocked(CoordinatorError::Faulted);
                observe_service(
                    r,
                    Boundary::ShutdownCancellationStep,
                    &Err::<(), _>(CoordinatorError::Faulted),
                    timer.finish(),
                );
                std::panic::resume_unwind(panic)
            }
        };
        let observed = match &result {
            Ok(progress) => match progress.status {
                ShutdownCancellationStatusV1::Blocked(e) => Err(e),
                _ => Ok(()),
            },
            Err(e) => Err(*e),
        };
        observe_service(
            r,
            Boundary::ShutdownCancellationStep,
            &observed,
            timer.finish(),
        );
        result
    }
}
