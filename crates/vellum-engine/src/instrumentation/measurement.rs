//! Native engine-measurement/1 values. No exporter, authority or clock probe.
//! This is a value kernel, not a funded general series/provenance registry or a
//! recipient disclosure surface. Recorder's four diagnostic stages are installed
//! here; the coordinator separately instruments its source/annotation publisher.
//! Other descriptor populations are not observed zeros.
use std::sync::Arc;
impl AdmissionV1 {
    pub const ALL: [Self; 7] = [
        Self::FirstAdmitted,
        Self::ReplayObserved,
        Self::Refused,
        Self::Stopped,
        Self::Unavailable,
        Self::Unknown,
        Self::Fault,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::FirstAdmitted => "first-admitted",
            Self::ReplayObserved => "replay-observed",
            Self::Refused => "refused",
            Self::Stopped => "stopped",
            Self::Unavailable => "unavailable",
            Self::Unknown => "unknown",
            Self::Fault => "fault",
        }
    }
}
impl DispositionV1 {
    pub const ALL: [Self; 6] = [
        Self::Success,
        Self::Unchanged,
        Self::Refused,
        Self::Stopped,
        Self::Unknown,
        Self::Fault,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Unchanged => "unchanged",
            Self::Refused => "refused",
            Self::Stopped => "stopped",
            Self::Unknown => "unknown",
            Self::Fault => "fault",
        }
    }
}
impl ArtifactV1 {
    pub const ALL: [Self; 2] = [Self::Source, Self::Annotation];
    pub fn label(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Annotation => "annotation",
        }
    }
}
impl EvidenceV1 {
    pub const ALL: [Self; 7] = [
        Self::Progress,
        Self::Acknowledgment,
        Self::Failure,
        Self::Quiescence,
        Self::Unknown,
        Self::Duplicate,
        Self::Contradiction,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Progress => "progress",
            Self::Acknowledgment => "acknowledgment",
            Self::Failure => "failure",
            Self::Quiescence => "quiescence",
            Self::Unknown => "unknown",
            Self::Duplicate => "duplicate",
            Self::Contradiction => "contradiction",
        }
    }
}
impl DerivedV1 {
    pub const ALL: [Self; 10] = [
        Self::Accepted,
        Self::Partial,
        Self::Stale,
        Self::Superseded,
        Self::Unsupported,
        Self::Refused,
        Self::Stopped,
        Self::Unavailable,
        Self::Unknown,
        Self::Fault,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Partial => "partial",
            Self::Stale => "stale",
            Self::Superseded => "superseded",
            Self::Unsupported => "unsupported",
            Self::Refused => "refused",
            Self::Stopped => "stopped",
            Self::Unavailable => "unavailable",
            Self::Unknown => "unknown",
            Self::Fault => "fault",
        }
    }
}
impl DiagnosticV1 {
    pub const ALL: [Self; 7] = [
        Self::Offered,
        Self::Captured,
        Self::Drained,
        Self::Dropped,
        Self::Invalid,
        Self::Discarded,
        Self::PreparationRefused,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Offered => "offered",
            Self::Captured => "captured",
            Self::Drained => "drained",
            Self::Dropped => "dropped",
            Self::Invalid => "invalid",
            Self::Discarded => "discarded",
            Self::PreparationRefused => "preparation-refused",
        }
    }
}
impl OwnerV1 {
    pub const ALL: [Self; 11] = [
        Self::Invocation,
        Self::Publication,
        Self::Release,
        Self::Effect,
        Self::Analysis,
        Self::History,
        Self::Artifact,
        Self::Lifecycle,
        Self::Configuration,
        Self::Diagnostics,
        Self::Control,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Invocation => "invocation",
            Self::Publication => "publication",
            Self::Release => "release",
            Self::Effect => "effect",
            Self::Analysis => "analysis",
            Self::History => "history",
            Self::Artifact => "artifact",
            Self::Lifecycle => "lifecycle",
            Self::Configuration => "configuration",
            Self::Diagnostics => "diagnostics",
            Self::Control => "control",
        }
    }
}
impl CoordinateV1 {
    pub const ALL: [Self; 6] = [
        Self::NormalizedBytes,
        Self::SourceOctets,
        Self::Slots,
        Self::CompletionPromises,
        Self::WorkSteps,
        Self::ChildGrants,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::NormalizedBytes => "normalized-bytes",
            Self::SourceOctets => "source-octets",
            Self::Slots => "slots",
            Self::CompletionPromises => "completion-promises",
            Self::WorkSteps => "work-steps",
            Self::ChildGrants => "child-grants",
        }
    }
}

/// Fixed four-series actual diagnostic pipeline, not request/invocation traffic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiagnosticMeasurementLimitsV1 {
    pub values: ValueLimitsV1,
    pub max_state_bytes: usize,
}
impl DiagnosticMeasurementLimitsV1 {
    pub fn required_state_bytes() -> usize {
        std::mem::size_of::<DiagnosticMeasurementsV1>()
            + std::mem::size_of::<[MeasurementPointV1; 4]>()
    }
    fn validate(self) -> Result<(), MeasurementErrorV1> {
        self.values.validate()?;
        if Self::required_state_bytes() > self.max_state_bytes {
            Err(MeasurementErrorV1::Limit)
        } else {
            Ok(())
        }
    }
}
pub(super) struct DiagnosticMeasurementsV1 {
    series: [MeasurementSeriesV1; 4],
    counts: [CounterV1; 4],
    limits: DiagnosticMeasurementLimitsV1,
}
impl std::fmt::Debug for DiagnosticMeasurementsV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DiagnosticMeasurementsV1(<owned>)")
    }
}
impl DiagnosticMeasurementsV1 {
    pub(super) fn new(limits: DiagnosticMeasurementLimitsV1) -> Result<Self, MeasurementErrorV1> {
        limits.validate()?;
        let series = |d| {
            MeasurementSeriesV1::new(
                MeasureV1::DiagnosticRecords,
                DimensionsV1::Diagnostic(d),
                limits.values,
            )
        };
        Ok(Self {
            series: [
                series(DiagnosticV1::Offered)?,
                series(DiagnosticV1::Captured)?,
                series(DiagnosticV1::Drained)?,
                series(DiagnosticV1::Dropped)?,
            ],
            counts: [CounterV1 {
                total: 0,
                exact: true,
            }; 4],
            limits,
        })
    }
    pub(super) fn increment(&mut self, stage: DiagnosticV1) {
        let index = match stage {
            DiagnosticV1::Offered => 0,
            DiagnosticV1::Captured => 1,
            DiagnosticV1::Drained => 2,
            DiagnosticV1::Dropped => 3,
            _ => return,
        };
        // State is private and immutable limits were checked before activation. No
        // telemetry refusal, allocation, callback or clock enters engine execution.
        let v = self.counts[index];
        let max = self.limits.values.max_integer;
        self.counts[index] = if v.total == max {
            CounterV1 {
                total: max,
                exact: false,
            }
        } else {
            CounterV1 {
                total: v.total + 1,
                exact: v.exact,
            }
        };
    }
    pub(super) fn snapshot(&mut self) -> Result<[MeasurementPointV1; 4], MeasurementErrorV1> {
        let states = self.counts.map(|value| PointStateV1::Observed {
            freshness: FreshnessV1::Current,
            coverage: if value.exact {
                CoverageV1::Complete
            } else {
                CoverageV1::Inexact
            },
            value: ValueV1::Counter(value),
        });
        if self
            .series
            .iter()
            .zip(states)
            .any(|(s, state)| !state.valid(s.measure, s.limits))
        {
            return Err(MeasurementErrorV1::InvalidValue);
        }
        if self.series.iter().any(|s| !s.can_observe()) {
            return Err(MeasurementErrorV1::Exhausted);
        }
        Ok([
            self.series[0].observe(states[0])?,
            self.series[1].observe(states[1])?,
            self.series[2].observe(states[2])?,
            self.series[3].observe(states[3])?,
        ])
    }
}
#[cfg(test)]
mod tests;
pub const PROFILE_V1: &str = "engine-measurement/1";
pub const MAX_INTEGER_V1: u64 = 9_007_199_254_740_991;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionV1 {
    FirstAdmitted,
    ReplayObserved,
    Refused,
    Stopped,
    Unavailable,
    Unknown,
    Fault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispositionV1 {
    Success,
    Unchanged,
    Refused,
    Stopped,
    Unknown,
    Fault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactV1 {
    Source,
    Annotation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceV1 {
    Progress,
    Acknowledgment,
    Failure,
    Quiescence,
    Unknown,
    Duplicate,
    Contradiction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DerivedV1 {
    Accepted,
    Partial,
    Stale,
    Superseded,
    Unsupported,
    Refused,
    Stopped,
    Unavailable,
    Unknown,
    Fault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticV1 {
    Offered,
    Captured,
    Drained,
    Dropped,
    Invalid,
    Discarded,
    PreparationRefused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerV1 {
    Invocation,
    Publication,
    Release,
    Effect,
    Analysis,
    History,
    Artifact,
    Lifecycle,
    Configuration,
    Diagnostics,
    Control,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinateV1 {
    NormalizedBytes,
    SourceOctets,
    Slots,
    CompletionPromises,
    WorkSteps,
    ChildGrants,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DimensionsV1 {
    None,
    Admission(AdmissionV1),
    Disposition(DispositionV1),
    Artifact(ArtifactV1),
    Evidence(EvidenceV1),
    Derived(DerivedV1),
    Diagnostic(DiagnosticV1),
    Owner(OwnerV1),
    ArtifactDisposition(ArtifactV1, DispositionV1),
    OwnerCoordinate(OwnerV1, CoordinateV1),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKindV1 {
    Counter,
    Gauge,
    Duration,
}
#[derive(Debug)]
pub struct DescriptorV1 {
    pub id: &'static str,
    pub site: &'static str,
    pub kind: ValueKindV1,
    pub unit: &'static str,
    pub population: &'static str,
    pub boundary: &'static str,
    pub dimensions: &'static [&'static str],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasureV1 {
    RequestOffers,
    RequestDispositions,
    InvocationAdmissions,
    InvocationDispositions,
    AdmissionDuration,
    PreparationDuration,
    PublicationDispositions,
    PublicationDuration,
    ReleaseDispositions,
    ReleaseDuration,
    EffectDispatches,
    EffectEvidence,
    SaveBaselineAdoptions,
    EffectQueueDuration,
    EffectFirstResultDuration,
    DerivedDispositions,
    DerivedDuration,
    HistoryDispositions,
    ArtifactDispositions,
    ArtifactDuration,
    ArtifactSize,
    LifecycleDispositions,
    ConfigurationDispositions,
    ConfigurationDuration,
    DiagnosticRecords,
    ControlDispositions,
    ControlDuration,
    RetentionDuration,
    ResourceOccupancy,
    ResourceCommitments,
    ResourceConsumed,
    ResourceCapacity,
}
const DESCRIPTORS: [DescriptorV1; 32] = [
    DescriptorV1 {
        id: "request.offers",
        site: "INS-01",
        kind: ValueKindV1::Counter,
        unit: "{offer}",
        population: "each engine ingress offer, including replay/retry offers",
        boundary: "before admission disposition",
        dimensions: &[],
    },
    DescriptorV1 {
        id: "request.dispositions",
        site: "INS-01",
        kind: ValueKindV1::Counter,
        unit: "{offer}",
        population: "one resolution per ingress offer, including pre-admission refusal and replay observation; pending offers remain pending",
        boundary: "actual first-admission/replay/refusal disposition of that offer",
        dimensions: &["admission"],
    },
    DescriptorV1 {
        id: "invocation.admissions",
        site: "INS-01",
        kind: ValueKindV1::Counter,
        unit: "{invocation}",
        population: "first admitted logical invocations, excluding dedup/replay observations",
        boundary: "actual invocation admission",
        dimensions: &[],
    },
    DescriptorV1 {
        id: "invocation.dispositions",
        site: "INS-01",
        kind: ValueKindV1::Counter,
        unit: "{invocation}",
        population: "one local terminal disposition per admitted logical invocation; not provider quiescence",
        boundary: "actual local terminal disposition",
        dimensions: &["disposition"],
    },
    DescriptorV1 {
        id: "admission.duration",
        site: "INS-01",
        kind: ValueKindV1::Duration,
        unit: "s",
        population: "individual ingress offers",
        boundary: "ingress offer to admission/refusal or observation cutoff",
        dimensions: &[],
    },
    DescriptorV1 {
        id: "preparation.duration",
        site: "INS-02",
        kind: ValueKindV1::Duration,
        unit: "s",
        population: "individual admitted preparation episodes",
        boundary: "preparation start to prepared/refused or observation cutoff",
        dimensions: &[],
    },
    DescriptorV1 {
        id: "publication.dispositions",
        site: "INS-02",
        kind: ValueKindV1::Counter,
        unit: "{publication}",
        population: "whole local publication attempts, not changed participants or partial commits",
        boundary: "actual final publication/no-op/refusal disposition",
        dimensions: &["disposition"],
    },
    DescriptorV1 {
        id: "publication.duration",
        site: "INS-02",
        kind: ValueKindV1::Duration,
        unit: "s",
        population: "individual final validation/publication critical sections",
        boundary: "entry to final validation through its exit or observation cutoff",
        dimensions: &[],
    },
    DescriptorV1 {
        id: "release.dispositions",
        site: "INS-03",
        kind: ValueKindV1::Counter,
        unit: "{release}",
        population: "whole release attempts at the authorized handoff, not rows or hidden members",
        boundary: "current-authorized handoff or refusal",
        dimensions: &["disposition"],
    },
    DescriptorV1 {
        id: "release.duration",
        site: "INS-03",
        kind: ValueKindV1::Duration,
        unit: "s",
        population: "individual release attempts",
        boundary: "release request to handoff/refusal or observation cutoff",
        dimensions: &[],
    },
    DescriptorV1 {
        id: "effect.dispatches",
        site: "INS-04",
        kind: ValueKindV1::Counter,
        unit: "{attempt}",
        population: "actual external effect attempt dispatches; not logical invocations",
        boundary: "effect port accepts actual dispatch",
        dimensions: &[],
    },
    DescriptorV1 {
        id: "effect.evidence",
        site: "INS-04",
        kind: ValueKindV1::Counter,
        unit: "{observation}",
        population: "attributed evidence observations, including duplicates, not unique effect completions",
        boundary: "evidence classified after attribution",
        dimensions: &["evidence"],
    },
    DescriptorV1 {
        id: "save.baseline-adoptions",
        site: "INS-04",
        kind: ValueKindV1::Counter,
        unit: "{baseline}",
        population: "new accepted exact captured-output save baselines, excluding mere acknowledgment and idempotent re-observation",
        boundary: "actual authorized source/annotation baseline adoption",
        dimensions: &["artifact"],
    },
    DescriptorV1 {
        id: "effect.queue.duration",
        site: "INS-04",
        kind: ValueKindV1::Duration,
        unit: "s",
        population: "individual admitted external attempts",
        boundary: "queue admission to dispatch/not-issued disposition or observation cutoff",
        dimensions: &[],
    },
    DescriptorV1 {
        id: "effect.first-result.duration",
        site: "INS-04",
        kind: ValueKindV1::Duration,
        unit: "s",
        population: "individual dispatched external attempts, at most one completed first-result observation per attempt",
        boundary: "dispatch to first authentic attributed acknowledgment/failure/unknown/contradiction result observation or cutoff; progress/quiescence/duplicates do not end this phase; not physical service or persistence completion",
        dimensions: &[],
    },
    DescriptorV1 {
        id: "derived.dispositions",
        site: "INS-05",
        kind: ValueKindV1::Counter,
        unit: "{result}",
        population: "derived result acceptance attempts under captured context; syntax data is not supplier fault",
        boundary: "current acceptance/refusal disposition",
        dimensions: &["derived"],
    },
    DescriptorV1 {
        id: "derived.duration",
        site: "INS-05",
        kind: ValueKindV1::Duration,
        unit: "s",
        population: "individual admitted derived-work episodes",
        boundary: "work admission to local acceptance/refusal or observation cutoff; not inferred worker CPU time",
        dimensions: &[],
    },
    DescriptorV1 {
        id: "history.dispositions",
        site: "INS-06",
        kind: ValueKindV1::Counter,
        unit: "{reversal}",
        population: "whole undo/redo attempts, not group participants",
        boundary: "actual reversal publication or refusal",
        dimensions: &["disposition"],
    },
    DescriptorV1 {
        id: "artifact.dispositions",
        site: "INS-07",
        kind: ValueKindV1::Counter,
        unit: "{artifact}",
        population: "individual source or annotation serialization preparations; not persistence acknowledgments",
        boundary: "payload preparation success/refusal",
        dimensions: &["artifact", "disposition"],
    },
    DescriptorV1 {
        id: "artifact.duration",
        site: "INS-07",
        kind: ValueKindV1::Duration,
        unit: "s",
        population: "individual source or annotation serialization preparations",
        boundary: "capture/encoding preparation start to payload/refusal or observation cutoff",
        dimensions: &["artifact"],
    },
    DescriptorV1 {
        id: "artifact.size",
        site: "INS-07",
        kind: ValueKindV1::Gauge,
        unit: "By",
        population: "one prepared source or annotation payload in encoded output octets",
        boundary: "payload size after successful preparation",
        dimensions: &["artifact"],
    },
    DescriptorV1 {
        id: "lifecycle.dispositions",
        site: "INS-08",
        kind: ValueKindV1::Counter,
        unit: "{transition}",
        population: "logical binding/lifecycle transition attempts; alias reuse is not a new document",
        boundary: "actual lifecycle activation/reuse/refusal disposition",
        dimensions: &["disposition"],
    },
    DescriptorV1 {
        id: "configuration.dispositions",
        site: "INS-09",
        kind: ValueKindV1::Counter,
        unit: "{candidate}",
        population: "whole configuration candidate activation attempts",
        boundary: "coherent activation/refusal, not each changed setting",
        dimensions: &["disposition"],
    },
    DescriptorV1 {
        id: "configuration.duration",
        site: "INS-09",
        kind: ValueKindV1::Duration,
        unit: "s",
        population: "individual configuration candidate preparation episodes",
        boundary: "staging start to activation/refusal or observation cutoff",
        dimensions: &[],
    },
    DescriptorV1 {
        id: "diagnostic.records",
        site: "INS-12",
        kind: ValueKindV1::Counter,
        unit: "{record}",
        population: "diagnostic pipeline records at the declared stage; never engine traffic",
        boundary: "actual pipeline-stage transition",
        dimensions: &["diagnostic"],
    },
    DescriptorV1 {
        id: "control.dispositions",
        site: "INS-13",
        kind: ValueKindV1::Counter,
        unit: "{control}",
        population: "protected control attempts, not original producer invocations",
        boundary: "actual protected control disposition",
        dimensions: &["disposition"],
    },
    DescriptorV1 {
        id: "control.duration",
        site: "INS-13",
        kind: ValueKindV1::Duration,
        unit: "s",
        population: "individual protected control or shutdown-wait episodes",
        boundary: "accepted control/wait to local disposition or observation cutoff; not native lifetime",
        dimensions: &[],
    },
    DescriptorV1 {
        id: "retention.duration",
        site: "INS-13",
        kind: ValueKindV1::Duration,
        unit: "s",
        population: "individual retained obligations",
        boundary: "obligation installation to justified discharge or observation cutoff",
        dimensions: &["owner"],
    },
    DescriptorV1 {
        id: "resource.occupancy",
        site: "INS-08",
        kind: ValueKindV1::Gauge,
        unit: "coordinate",
        population: "actual reusable live occupancy in one declared owner/accounting coordinate",
        boundary: "safe owner-ledger observation",
        dimensions: &["owner", "coordinate"],
    },
    DescriptorV1 {
        id: "resource.commitments",
        site: "INS-08",
        kind: ValueKindV1::Gauge,
        unit: "coordinate",
        population: "retained admitted commitments in one declared owner/accounting coordinate",
        boundary: "safe owner-ledger observation; not a second additive copy of occupancy",
        dimensions: &["owner", "coordinate"],
    },
    DescriptorV1 {
        id: "resource.consumed",
        site: "INS-08",
        kind: ValueKindV1::Counter,
        unit: "coordinate",
        population: "consumed nonrefundable work-steps or child-grants in one owner scope",
        boundary: "actual consumption, never cancellation refund",
        dimensions: &["owner", "coordinate"],
    },
    DescriptorV1 {
        id: "resource.capacity",
        site: "INS-08",
        kind: ValueKindV1::Gauge,
        unit: "coordinate",
        population: "effective hard capacity in the same coordinate/scope as the corresponding use",
        boundary: "safe effective-profile observation; soft target excluded",
        dimensions: &["owner", "coordinate"],
    },
];
impl MeasureV1 {
    pub const ALL: [Self; 32] = [
        Self::RequestOffers,
        Self::RequestDispositions,
        Self::InvocationAdmissions,
        Self::InvocationDispositions,
        Self::AdmissionDuration,
        Self::PreparationDuration,
        Self::PublicationDispositions,
        Self::PublicationDuration,
        Self::ReleaseDispositions,
        Self::ReleaseDuration,
        Self::EffectDispatches,
        Self::EffectEvidence,
        Self::SaveBaselineAdoptions,
        Self::EffectQueueDuration,
        Self::EffectFirstResultDuration,
        Self::DerivedDispositions,
        Self::DerivedDuration,
        Self::HistoryDispositions,
        Self::ArtifactDispositions,
        Self::ArtifactDuration,
        Self::ArtifactSize,
        Self::LifecycleDispositions,
        Self::ConfigurationDispositions,
        Self::ConfigurationDuration,
        Self::DiagnosticRecords,
        Self::ControlDispositions,
        Self::ControlDuration,
        Self::RetentionDuration,
        Self::ResourceOccupancy,
        Self::ResourceCommitments,
        Self::ResourceConsumed,
        Self::ResourceCapacity,
    ];
    pub fn descriptor(self) -> &'static DescriptorV1 {
        match self {
            Self::RequestOffers => &DESCRIPTORS[0],
            Self::RequestDispositions => &DESCRIPTORS[1],
            Self::InvocationAdmissions => &DESCRIPTORS[2],
            Self::InvocationDispositions => &DESCRIPTORS[3],
            Self::AdmissionDuration => &DESCRIPTORS[4],
            Self::PreparationDuration => &DESCRIPTORS[5],
            Self::PublicationDispositions => &DESCRIPTORS[6],
            Self::PublicationDuration => &DESCRIPTORS[7],
            Self::ReleaseDispositions => &DESCRIPTORS[8],
            Self::ReleaseDuration => &DESCRIPTORS[9],
            Self::EffectDispatches => &DESCRIPTORS[10],
            Self::EffectEvidence => &DESCRIPTORS[11],
            Self::SaveBaselineAdoptions => &DESCRIPTORS[12],
            Self::EffectQueueDuration => &DESCRIPTORS[13],
            Self::EffectFirstResultDuration => &DESCRIPTORS[14],
            Self::DerivedDispositions => &DESCRIPTORS[15],
            Self::DerivedDuration => &DESCRIPTORS[16],
            Self::HistoryDispositions => &DESCRIPTORS[17],
            Self::ArtifactDispositions => &DESCRIPTORS[18],
            Self::ArtifactDuration => &DESCRIPTORS[19],
            Self::ArtifactSize => &DESCRIPTORS[20],
            Self::LifecycleDispositions => &DESCRIPTORS[21],
            Self::ConfigurationDispositions => &DESCRIPTORS[22],
            Self::ConfigurationDuration => &DESCRIPTORS[23],
            Self::DiagnosticRecords => &DESCRIPTORS[24],
            Self::ControlDispositions => &DESCRIPTORS[25],
            Self::ControlDuration => &DESCRIPTORS[26],
            Self::RetentionDuration => &DESCRIPTORS[27],
            Self::ResourceOccupancy => &DESCRIPTORS[28],
            Self::ResourceCommitments => &DESCRIPTORS[29],
            Self::ResourceConsumed => &DESCRIPTORS[30],
            Self::ResourceCapacity => &DESCRIPTORS[31],
        }
    }
    pub fn accepts(self, d: DimensionsV1) -> bool {
        let fields = self.descriptor().dimensions;
        let shape = match d {
            DimensionsV1::None => fields.is_empty(),
            DimensionsV1::Admission(_) => fields == ["admission"],
            DimensionsV1::Disposition(_) => fields == ["disposition"],
            DimensionsV1::Artifact(_) => fields == ["artifact"],
            DimensionsV1::Evidence(_) => fields == ["evidence"],
            DimensionsV1::Derived(_) => fields == ["derived"],
            DimensionsV1::Diagnostic(_) => fields == ["diagnostic"],
            DimensionsV1::Owner(_) => fields == ["owner"],
            DimensionsV1::ArtifactDisposition(_, _) => fields == ["artifact", "disposition"],
            DimensionsV1::OwnerCoordinate(_, _) => fields == ["owner", "coordinate"],
        };
        shape
            && match d {
                DimensionsV1::OwnerCoordinate(_, c) => match self {
                    Self::ResourceConsumed => {
                        matches!(c, CoordinateV1::WorkSteps | CoordinateV1::ChildGrants)
                    }
                    Self::ResourceOccupancy | Self::ResourceCommitments => {
                        !matches!(c, CoordinateV1::WorkSteps | CoordinateV1::ChildGrants)
                    }
                    _ => true,
                },
                _ => true,
            }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementErrorV1 {
    Unavailable,
    Faulted,
    InvalidProfile,
    InvalidValue,
    InvalidDimensions,
    Exhausted,
    Limit,
    Capacity,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValueLimitsV1 {
    pub max_integer: u64,
    pub max_seconds: f64,
}
impl ValueLimitsV1 {
    pub fn validate(self) -> Result<(), MeasurementErrorV1> {
        if self.max_integer > MAX_INTEGER_V1
            || !self.max_seconds.is_finite()
            || self.max_seconds < 0.0
        {
            Err(MeasurementErrorV1::InvalidProfile)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreshnessV1 {
    Current,
    Stale,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverageV1 {
    Complete,
    Sampled,
    Inexact,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndpointV1 {
    Completed,
    Pending,
    Censored,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CounterV1 {
    pub total: u64,
    pub exact: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueV1 {
    Counter(CounterV1),
    Gauge { amount: u64 },
    Duration { seconds: f64, endpoint: EndpointV1 },
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointStateV1 {
    Observed {
        freshness: FreshnessV1,
        coverage: CoverageV1,
        value: ValueV1,
    },
    Off,
    Unavailable,
    Unknown,
}
impl PointStateV1 {
    fn valid(self, measure: MeasureV1, limits: ValueLimitsV1) -> bool {
        match self {
            Self::Off | Self::Unavailable | Self::Unknown => true,
            Self::Observed {
                coverage, value, ..
            } => match (measure.descriptor().kind, value) {
                (ValueKindV1::Counter, ValueV1::Counter(v)) => {
                    v.total <= limits.max_integer && (coverage == CoverageV1::Complete || !v.exact)
                }
                (ValueKindV1::Gauge, ValueV1::Gauge { amount }) => amount <= limits.max_integer,
                (ValueKindV1::Duration, ValueV1::Duration { seconds, .. }) => {
                    seconds.is_finite() && seconds >= 0.0 && seconds <= limits.max_seconds
                }
                _ => false,
            },
        }
    }
}
#[derive(Clone)]
pub struct MeasurementGenerationV1(Arc<()>);
impl std::fmt::Debug for MeasurementGenerationV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MeasurementGenerationV1(<opaque>)")
    }
}
impl PartialEq for MeasurementGenerationV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for MeasurementGenerationV1 {}
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementPointV1 {
    measure: MeasureV1,
    dimensions: DimensionsV1,
    generation: MeasurementGenerationV1,
    sequence: u64,
    state: PointStateV1,
}
impl MeasurementPointV1 {
    pub fn profile(&self) -> &'static str {
        PROFILE_V1
    }
    pub fn measure(&self) -> MeasureV1 {
        self.measure
    }
    pub fn dimensions(&self) -> DimensionsV1 {
        self.dimensions
    }
    pub fn generation(&self) -> &MeasurementGenerationV1 {
        &self.generation
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn state(&self) -> PointStateV1 {
        self.state
    }
}
/// Trusted producer value utility, not a collection/disclosure grant or complete
/// retained provenance registry. One immutable descriptor/dimension binding.
pub struct MeasurementSeriesV1 {
    measure: MeasureV1,
    dimensions: DimensionsV1,
    generation: MeasurementGenerationV1,
    limits: ValueLimitsV1,
    next: u64,
}
impl std::fmt::Debug for MeasurementSeriesV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MeasurementSeriesV1(<opaque>)")
    }
}
impl MeasurementSeriesV1 {
    pub fn new(
        measure: MeasureV1,
        dimensions: DimensionsV1,
        limits: ValueLimitsV1,
    ) -> Result<Self, MeasurementErrorV1> {
        limits.validate()?;
        if !measure.accepts(dimensions) {
            return Err(MeasurementErrorV1::InvalidDimensions);
        }
        Ok(Self {
            measure,
            dimensions,
            generation: MeasurementGenerationV1(Arc::new(())),
            limits,
            next: 0,
        })
    }
    pub fn can_observe(&self) -> bool {
        self.next <= self.limits.max_integer
    }
    pub fn observe(
        &mut self,
        state: PointStateV1,
    ) -> Result<MeasurementPointV1, MeasurementErrorV1> {
        if !state.valid(self.measure, self.limits) {
            return Err(MeasurementErrorV1::InvalidValue);
        }
        if !self.can_observe() {
            return Err(MeasurementErrorV1::Exhausted);
        }
        let point = MeasurementPointV1 {
            measure: self.measure,
            dimensions: self.dimensions,
            generation: self.generation.clone(),
            sequence: self.next,
            state,
        };
        self.next += 1;
        Ok(point) // profile max integer leaves representable exhausted sentinel
    }
}
pub fn counter_step_v1(
    value: CounterV1,
    amount: u64,
    maximum: u64,
) -> Result<CounterV1, MeasurementErrorV1> {
    if maximum > MAX_INTEGER_V1 || amount > MAX_INTEGER_V1 || value.total > maximum {
        return Err(MeasurementErrorV1::InvalidValue);
    }
    Ok(if amount > maximum - value.total {
        CounterV1 {
            total: maximum,
            exact: false,
        }
    } else {
        CounterV1 {
            total: value.total + amount,
            exact: value.exact,
        }
    })
}
/// Whole-interval continuity is a separately established trusted fact, not
/// inferred from endpoint equality, a matching opaque generation or ordinal.
pub fn counter_delta_v1(
    a: &MeasurementPointV1,
    b: &MeasurementPointV1,
    continuous: Option<bool>,
) -> Option<u64> {
    if continuous != Some(true)
        || a.measure != b.measure
        || a.dimensions != b.dimensions
        || a.generation != b.generation
        || b.sequence <= a.sequence
    {
        return None;
    }
    let exact = |p: &MeasurementPointV1| match p.state {
        PointStateV1::Observed {
            freshness: FreshnessV1::Current,
            coverage: CoverageV1::Complete,
            value: ValueV1::Counter(CounterV1 { total, exact: true }),
        } => Some(total),
        _ => None,
    };
    exact(b)?.checked_sub(exact(a)?)
}
/// Pure first-authentic-result classification. Caller owns actual attempt state;
/// quiescence/progress/duplicates never finish this duration or prove persistence.
pub fn first_result_v1(kind: EvidenceV1, attributed: Option<bool>, already_seen: bool) -> bool {
    attributed == Some(true)
        && !already_seen
        && matches!(
            kind,
            EvidenceV1::Acknowledgment
                | EvidenceV1::Failure
                | EvidenceV1::Unknown
                | EvidenceV1::Contradiction
        )
}
#[derive(Clone)]
pub struct ClockStampV1 {
    domain: Arc<()>,
    epoch: Arc<()>,
    tick: u64,
}
pub struct MeasurementClockV1 {
    domain: Arc<()>,
    epoch: Arc<()>,
    seconds_per_tick: f64,
    max_tick: u64,
}
impl MeasurementClockV1 {
    pub fn new(seconds_per_tick: f64, max_tick: u64) -> Result<Self, MeasurementErrorV1> {
        if !seconds_per_tick.is_finite() || seconds_per_tick <= 0.0 || max_tick > MAX_INTEGER_V1 {
            return Err(MeasurementErrorV1::InvalidProfile);
        }
        Ok(Self {
            domain: Arc::new(()),
            epoch: Arc::new(()),
            seconds_per_tick,
            max_tick,
        })
    }
    /// Trusted host-supplied tick, not a local clock sample or validity attestation.
    pub fn stamp(&self, tick: u64) -> Option<ClockStampV1> {
        if tick > self.max_tick {
            None
        } else {
            Some(ClockStampV1 {
                domain: self.domain.clone(),
                epoch: self.epoch.clone(),
                tick,
            })
        }
    }
    pub fn new_epoch(&mut self) {
        self.epoch = Arc::new(());
    }
    pub fn seconds(
        &self,
        start: &ClockStampV1,
        end: &ClockStampV1,
        valid: Option<bool>,
        maximum: f64,
    ) -> Option<f64> {
        if valid != Some(true)
            || !maximum.is_finite()
            || maximum < 0.0
            || !Arc::ptr_eq(&start.domain, &self.domain)
            || !Arc::ptr_eq(&end.domain, &self.domain)
            || !Arc::ptr_eq(&start.epoch, &self.epoch)
            || !Arc::ptr_eq(&end.epoch, &self.epoch)
        {
            return None;
        }
        let seconds = end.tick.checked_sub(start.tick)? as f64 * self.seconds_per_tick;
        (seconds.is_finite() && seconds <= maximum).then_some(seconds)
    }
}
