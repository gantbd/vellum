use super::*;
use crate::coordinator::root_accounting::{Bytes, Result};
impl Frontiers {
    pub(in crate::coordinator) fn root_payload_bytes(&self) -> Result<usize> {
        let mut bytes = Bytes::default();
        bytes.rows::<HistoryGroupId>(self.undo.len())?;
        bytes.rows::<HistoryGroupId>(self.redo.len())?;
        Ok(bytes.finish())
    }
}
impl HistoryState {
    pub(in crate::coordinator) fn root_bytes(&self) -> Result<usize> {
        let Self {
            limits: _,
            groups: _,
            pins: _,
            bytes: _,
        } = self;
        let mut bytes = Bytes::default();
        bytes.rows::<(Option<HistoryLimits>, Vec<Group>)>(1)?;
        for group in &self.groups {
            bytes.add(group.root_bytes()?)?;
        }
        // Pins are separately bounded control holds, not authored history.
        Ok(bytes.finish())
    }
}
impl Group {
    fn root_bytes(&self) -> Result<usize> {
        let mut bytes = Bytes::default();
        bytes.rows::<Self>(1)?;
        bytes.rows::<Member>(self.members.len())?;
        for member in &self.members {
            for edits in [&member.forward, &member.inverse] {
                bytes.rows::<Replacement>(edits.len())?;
                for edit in edits {
                    bytes.add(edit.expected.len())?;
                    bytes.add(edit.inserted.len())?;
                }
            }
        }
        for journal in &self.annotations {
            bytes.add(journal.root_bytes()?)?;
        }
        Ok(bytes.finish())
    }
}

impl HistoryDecision {
    pub(in crate::coordinator) fn root_bytes_after(&self, current: &HistoryState) -> Result<usize> {
        let mut bytes = Bytes::default();
        bytes.add(current.root_bytes()?)?;
        if let Some(group) = &self.group {
            bytes.add(group.root_bytes()?)?;
        }
        Ok(bytes.finish())
    }
    pub(in crate::coordinator) fn source_frontier_bytes(
        &self,
        slot: usize,
        before: &Frontiers,
    ) -> Result<usize> {
        self.updates
            .iter()
            .find_map(|u| match &u.scope {
                Scope::Source(id) if id.slot == slot => Some(&u.frontier),
                _ => None,
            })
            .unwrap_or(before)
            .root_payload_bytes()
    }
    pub(in crate::coordinator) fn annotation_frontier_bytes(
        &self,
        slot: usize,
        before: Option<&Frontiers>,
    ) -> Result<usize> {
        self.updates
            .iter()
            .find_map(|u| match &u.scope {
                Scope::Annotation(id) if id.slot == slot => Some(&u.frontier),
                _ => None,
            })
            .or(before)
            .map_or(Ok(0), Frontiers::root_payload_bytes)
    }
}
