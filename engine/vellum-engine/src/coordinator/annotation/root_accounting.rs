use super::*;
use crate::coordinator::root_accounting::{Bytes, Result, RootAccountingErrorV1};
pub(super) fn restored_root_growth(records: &[Record]) -> Result<usize> {
    let mut bytes = Bytes::default();
    bytes.rows::<Option<Record>>(records.len())?;
    bytes.rows::<DocumentId>(records.len())?;
    bytes.rows::<Option<RetiredAnnotation>>(records.len())?;
    for record in records {
        bytes.add(record.value.root_payload_bytes()?)?;
        bytes.add(record.history.root_payload_bytes()?)?;
    }
    Ok(bytes.finish())
}
impl Value {
    fn root_payload_bytes(&self) -> Result<usize> {
        let mut bytes = Bytes::default();
        bytes.add(self.body.len())?;
        if let Some(identity) = &self.restored_identity {
            bytes.add(identity.key().len())?;
        }
        Ok(bytes.finish())
    }
}
impl AnnotationState {
    pub(in crate::coordinator) fn root_bytes_after(
        &self,
        decision: &AnnotationDecision,
        history: &history::HistoryDecision,
        documents: &[Option<Document>],
        namespace: &Arc<()>,
    ) -> Result<usize> {
        self.root_bytes_with(Some((decision, history)), documents, namespace, false)
    }
    pub(in crate::coordinator) fn root_bytes_after_advice(
        &self,
        decision: &AnnotationDecision,
        history: &history::HistoryDecision,
        documents: &[Option<Document>],
        namespace: &Arc<()>,
    ) -> Result<usize> {
        self.root_bytes_with(Some((decision, history)), documents, namespace, true)
    }
    pub(in crate::coordinator) fn root_bytes(
        &self,
        documents: &[Option<Document>],
        namespace: &Arc<()>,
    ) -> Result<usize> {
        self.root_bytes_with(None, documents, namespace, false)
    }
    fn root_bytes_with(
        &self,
        after: Option<(&AnnotationDecision, &history::HistoryDecision)>,
        documents: &[Option<Document>],
        namespace: &Arc<()>,
        advisory: bool,
    ) -> Result<usize> {
        let Self {
            limits: _,
            records: _,
            issued_documents: _,
            retired: _,
            snapshots: _,
        } = self;
        let mut bytes = Bytes::default();
        bytes.rows::<(
            Option<AnnotationLimits>,
            Vec<Option<Record>>,
            Vec<DocumentId>,
            Vec<Option<RetiredAnnotation>>,
        )>(1)?;
        if self.retired.len() != self.records.len()
            || self.issued_documents.len() != self.records.len()
        {
            return Err(RootAccountingErrorV1::Faulted);
        }
        bytes.rows::<Option<Record>>(self.records.len())?;
        bytes.rows::<DocumentId>(self.issued_documents.len())?;
        bytes.rows::<Option<RetiredAnnotation>>(self.retired.len())?;
        for (slot, record) in self.records.iter().enumerate() {
            if let Some(retired) = &self.retired[slot] {
                let d = &retired.fact.annotation.document;
                if record.is_some()
                    || (retired.fact.present && !retired.document_reclaimed)
                    || (retired.document_reclaimed
                        && (!Arc::ptr_eq(&d.namespace, namespace)
                            || !documents.get(d.slot).is_some_and(Option::is_none)))
                    || retired.fact.annotation.slot != slot
                    || retired.fact.annotation.document != self.issued_documents[slot]
                    || after.is_some_and(|(d, _)| d.updates.iter().any(|v| v.id.slot == slot))
                {
                    return Err(RootAccountingErrorV1::Faulted);
                }
                if let Some(identity) = &retired.restored_identity {
                    bytes.add(identity.key().len())?;
                }
            }
            let value = after
                .and_then(|(decision, _)| decision.updates.iter().find(|v| v.id.slot == slot))
                .or_else(|| record.as_ref().map(|r| &r.value));
            let Some(value) = value else {
                continue;
            };
            bytes.add(value.root_payload_bytes()?)?;
            let before = record.as_ref().map(|r| &r.history);
            let frontier = if let Some((_, history)) = after {
                history.annotation_frontier_bytes(slot, before)?
            } else {
                before.map_or(Ok(0), history::Frontiers::root_payload_bytes)?
            };
            bytes.add(frontier)?;
        }
        if advisory {
            let Some((decision, history)) = after else {
                return Err(RootAccountingErrorV1::Faulted);
            };
            let mut next_slot = self.records.len();
            for value in decision
                .updates
                .iter()
                .filter(|value| value.id.slot >= self.records.len())
            {
                if value.id.slot != next_slot || !Arc::ptr_eq(&value.id.namespace, namespace) {
                    return Err(RootAccountingErrorV1::Faulted);
                }
                next_slot = next_slot
                    .checked_add(1)
                    .ok_or(RootAccountingErrorV1::Overflow)?;
                bytes.rows::<Option<Record>>(1)?;
                bytes.rows::<DocumentId>(1)?;
                bytes.rows::<Option<RetiredAnnotation>>(1)?;
                bytes.add(value.root_payload_bytes()?)?;
                bytes.add(history.annotation_frontier_bytes(value.id.slot, None)?)?;
            }
        }
        // Weak capture registry and retained snapshot payloads are not the root.
        Ok(bytes.finish())
    }
}
impl AnnotationJournal {
    pub(in crate::coordinator) fn root_bytes(&self) -> Result<usize> {
        let mut bytes = Bytes::default();
        bytes.rows::<Self>(1)?;
        match &self.delta {
            Delta::Membership { before, after } => {
                for authored in before.iter().chain(after) {
                    bytes.add(authored.body.len())?;
                }
            }
            Delta::Fields { body, .. } => {
                if let Some((before, after)) = body {
                    bytes.add(before.len())?;
                    bytes.add(after.len())?;
                }
            }
        }
        Ok(bytes.finish())
    }
}
