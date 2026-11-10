//! Pure source/attachment candidate preparation, not annotation ownership or an
//! authorized command facade. Callers must bind exact document/source context,
//! acquire the complete real companion set and publish under the shared owner.
//! No body, identity, version, permission, history or persistence is fabricated.
use crate::instrumentation::{Boundary, Disposition, Observation, Recorder};
use crate::source::{Limits, Replacement, SourceError, Utf8Source};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Affinity {
    Before,
    After,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointProfile {
    Position,
    Content,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AttachmentTarget {
    Document,
    Point {
        offset: usize,
        affinity: Affinity,
        profile: PointProfile,
    },
    /// Nonempty half-open original UTF-8 bytes. A collapsed target is a Point.
    ContentRange {
        start: usize,
        end: usize,
    },
}
impl std::fmt::Debug for AttachmentTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AttachmentTarget(<redacted>)")
    }
}
/// Location evidence alone is never proof of semantic validity or review status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocationEvidence {
    Located,
    Affected,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    Located {
        target: AttachmentTarget,
        evidence: LocationEvidence,
    },
    Detached,
    Unavailable,
}
#[derive(Clone, Copy, Debug)]
pub struct AttachmentLimits {
    pub max_attachments: usize,
    /// One unit per original edit inspected in coverage/touch/coordinate mapping.
    /// Shared across the whole candidate; not a physical CPU/allocation bound.
    pub max_mapping_steps: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentError {
    Source(SourceError),
    InvalidTarget,
    InvalidBoundary,
    Limit,
    Capacity,
}
#[derive(Debug)]
pub struct PreparedAttachments {
    source: Utf8Source,
    placements: Vec<Placement>,
    source_changed: bool,
    placements_changed: bool,
    usage: MappingUsage,
}
/// Explicit trusted-holder facts, not default diagnostic labels or release grants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MappingUsage {
    pub mapping_steps: usize,
    pub max_mapping_steps: usize,
    pub located: usize,
    pub affected: usize,
    pub detached: usize,
    pub unavailable: usize,
}
impl PreparedAttachments {
    pub fn source(&self) -> &Utf8Source {
        &self.source
    }
    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }
    pub fn source_changed(&self) -> bool {
        self.source_changed
    }
    pub fn placements_changed(&self) -> bool {
        self.placements_changed
    }
    pub fn usage(&self) -> MappingUsage {
        self.usage
    }
    /// Transfers owned candidates, not a publication token or completeness proof.
    pub fn into_parts(self) -> (Utf8Source, Vec<Placement>) {
        (self.source, self.placements)
    }
}
pub(crate) fn validate_target(
    source: &str,
    target: AttachmentTarget,
) -> Result<(), AttachmentError> {
    let (start, end) = match target {
        AttachmentTarget::Document => return Ok(()),
        AttachmentTarget::Point { offset, .. } => (offset, offset),
        AttachmentTarget::ContentRange { start, end } => {
            if start >= end {
                return Err(AttachmentError::InvalidTarget);
            }
            (start, end)
        }
    };
    if end > source.len() {
        return Err(AttachmentError::InvalidTarget);
    }
    if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return Err(AttachmentError::InvalidBoundary);
    }
    Ok(())
}
/// Internal owner path: source/journal consistency and exact participants are
/// validated by the coordinator. Preserve its journal order, including resolved
/// coincident inverse insertions; do not expose an ordinary-edit bypass.
pub(crate) fn map_validated(
    original: &Utf8Source,
    candidate: &Utf8Source,
    edits: &[Replacement],
    placements: &[Placement],
    remaining: &mut usize,
) -> Result<Vec<Placement>, AttachmentError> {
    if !original.matches_journal(edits, candidate) {
        return Err(AttachmentError::InvalidTarget);
    }
    let mut ordered = Vec::new();
    ordered
        .try_reserve_exact(edits.len())
        .map_err(|_| AttachmentError::Capacity)?;
    ordered.extend(edits);
    let mut output = Vec::new();
    output
        .try_reserve_exact(placements.len())
        .map_err(|_| AttachmentError::Capacity)?;
    for &placement in placements {
        if let Placement::Located { target, .. } = placement {
            validate_target(original.text(), target)?;
        }
        let mapped = map_placement(placement, &ordered, remaining)?;
        if let Placement::Located { target, .. } = mapped {
            validate_target(candidate.text(), target)?;
        }
        output.push(mapped);
    }
    Ok(output)
}
pub(crate) fn validate(
    source: &Utf8Source,
    target: AttachmentTarget,
) -> Result<(), AttachmentError> {
    validate_target(source.text(), target)
}
fn step(remaining: &mut usize) -> Result<(), AttachmentError> {
    *remaining = remaining.checked_sub(1).ok_or(AttachmentError::Limit)?;
    Ok(())
}
fn map_point(
    mut point: usize,
    edits: &[&Replacement],
    affinity: Affinity,
    remaining: &mut usize,
) -> Result<usize, AttachmentError> {
    // Reverse original-coordinate mapping preserves adjacent replacement edges;
    // affinity chooses only coincident insertion/interior collapse, not outer edges.
    for edit in edits.iter().rev() {
        step(remaining)?;
        if point < edit.start {
            continue;
        }
        if point > edit.end {
            point = point
                .checked_sub(edit.end - edit.start)
                .and_then(|p| p.checked_add(edit.inserted.len()))
                .ok_or(AttachmentError::Limit)?;
        } else if edit.start != edit.end && point == edit.start {
            point = edit.start;
        } else if (edit.start != edit.end && point == edit.end) || affinity == Affinity::After {
            point = edit
                .start
                .checked_add(edit.inserted.len())
                .ok_or(AttachmentError::Limit)?;
        } else {
            point = edit.start;
        }
    }
    Ok(point)
}
pub(crate) fn map_positions_validated(
    original: &Utf8Source,
    candidate: &Utf8Source,
    edits: &[Replacement],
    points: &[(usize, Affinity)],
    remaining: &mut usize,
) -> Result<Vec<usize>, AttachmentError> {
    if !original.matches_journal(edits, candidate) {
        return Err(AttachmentError::InvalidTarget);
    }
    let mut ordered = Vec::new();
    ordered
        .try_reserve_exact(edits.len())
        .map_err(|_| AttachmentError::Capacity)?;
    ordered.extend(edits);
    let mut output = Vec::new();
    output
        .try_reserve_exact(points.len())
        .map_err(|_| AttachmentError::Capacity)?;
    for &(point, affinity) in points {
        if !original.text().is_char_boundary(point) {
            return Err(AttachmentError::InvalidBoundary);
        }
        let mapped = map_point(point, &ordered, affinity, remaining)?;
        if !candidate.text().is_char_boundary(mapped) {
            return Err(AttachmentError::InvalidBoundary);
        }
        output.push(mapped);
    }
    Ok(output)
}
fn covered(
    start: usize,
    end: usize,
    edits: &[&Replacement],
    remaining: &mut usize,
) -> Result<bool, AttachmentError> {
    let mut cursor = start;
    for edit in edits {
        step(remaining)?;
        if edit.end <= cursor {
            continue;
        }
        if edit.start > cursor {
            break;
        }
        cursor = cursor.max(edit.end);
        if cursor >= end {
            return Ok(true);
        }
    }
    Ok(false)
}
fn map_placement(
    placement: Placement,
    edits: &[&Replacement],
    remaining: &mut usize,
) -> Result<Placement, AttachmentError> {
    let Placement::Located { target, evidence } = placement else {
        return Ok(placement);
    };
    let (start, end, affinity, content) = match target {
        AttachmentTarget::Document => return Ok(placement),
        AttachmentTarget::Point {
            offset,
            affinity,
            profile,
        } => (offset, offset, affinity, profile == PointProfile::Content),
        AttachmentTarget::ContentRange { start, end } => (start, end, Affinity::After, true),
    };
    if content {
        let lost = if start != end {
            covered(start, end, edits, remaining)?
        } else {
            let mut lost = false;
            for edit in edits {
                step(remaining)?;
                if edit.start < start && start < edit.end {
                    lost = true;
                    break;
                }
            }
            lost
        };
        if lost {
            return Ok(Placement::Detached);
        }
    }
    let mut affected = evidence == LocationEvidence::Affected;
    for edit in edits {
        step(remaining)?;
        affected |= if edit.start == edit.end {
            !edit.inserted.is_empty() && start < edit.start && edit.start < end
        } else {
            edit.start < end && edit.end > start
        };
    }
    let mapped_start = map_point(start, edits, affinity, remaining)?;
    let mapped = match target {
        AttachmentTarget::ContentRange { .. } => AttachmentTarget::ContentRange {
            start: mapped_start,
            end: map_point(end, edits, Affinity::Before, remaining)?,
        },
        AttachmentTarget::Point {
            affinity, profile, ..
        } => AttachmentTarget::Point {
            offset: mapped_start,
            affinity,
            profile,
        },
        AttachmentTarget::Document => unreachable!("document target returned before mapping"),
    };
    Ok(Placement::Located {
        target: mapped,
        evidence: if affected {
            LocationEvidence::Affected
        } else {
            LocationEvidence::Located
        },
    })
}
/// Validate and stage the entire source + supplied placements, or return no
/// candidate. The caller-supplied set is NOT evidence of the actual full registry.
/// Point profile and affinity are explicit; no collapsed-range coercion exists.
pub fn prepare_attachments(
    original: &Utf8Source,
    edits: &[Replacement],
    placements: &[Placement],
    source_limits: Limits,
    limits: AttachmentLimits,
    recorder: &mut Recorder,
) -> Result<PreparedAttachments, AttachmentError> {
    let result = (|| {
        if placements.len() > limits.max_attachments {
            return Err(AttachmentError::Limit);
        }
        for placement in placements {
            if let Placement::Located { target, .. } = placement {
                validate_target(original.text(), *target)?;
            }
        }
        let source = original
            .prepare_observed(edits, source_limits, recorder)
            .map_err(AttachmentError::Source)?;
        let mut ordered = Vec::new();
        ordered
            .try_reserve_exact(edits.len())
            .map_err(|_| AttachmentError::Capacity)?;
        ordered.extend(edits);
        ordered.sort_unstable_by_key(|edit| (edit.start, edit.end));
        let mut mapped = Vec::new();
        mapped
            .try_reserve_exact(placements.len())
            .map_err(|_| AttachmentError::Capacity)?;
        let mut remaining = limits.max_mapping_steps;
        for &placement in placements {
            let next = map_placement(placement, &ordered, &mut remaining)?;
            if let Placement::Located { target, .. } = next {
                validate_target(source.text(), target)?;
            }
            mapped.push(next);
        }
        let source_changed = &source != original;
        let placements_changed = mapped.as_slice() != placements;
        let mut usage = MappingUsage {
            mapping_steps: limits.max_mapping_steps - remaining,
            max_mapping_steps: limits.max_mapping_steps,
            located: 0,
            affected: 0,
            detached: 0,
            unavailable: 0,
        };
        for placement in &mapped {
            match placement {
                Placement::Located {
                    evidence: LocationEvidence::Located,
                    ..
                } => usage.located += 1,
                Placement::Located {
                    evidence: LocationEvidence::Affected,
                    ..
                } => usage.affected += 1,
                Placement::Detached => usage.detached += 1,
                Placement::Unavailable => usage.unavailable += 1,
            }
        }
        Ok(PreparedAttachments {
            source,
            placements: mapped,
            source_changed,
            placements_changed,
            usage,
        })
    })();
    recorder.record(Observation {
        boundary: Boundary::AttachmentMapping,
        disposition: if result.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        service_time: None,
    });
    result
}
