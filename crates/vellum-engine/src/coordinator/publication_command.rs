//! Shared finite original-command lifecycle; payload/storage remain owner-local.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationCommandPhaseV1 {
    Issued,
    Executing,
    Settled,
    Faulted,
}
pub(super) struct Lifecycle<R> {
    pub phase: PublicationCommandPhaseV1,
    pub started: bool,
    pub result: Option<R>,
}
impl<R> Lifecycle<R> {
    pub fn issued() -> Self {
        Self {
            phase: PublicationCommandPhaseV1::Issued,
            started: false,
            result: None,
        }
    }
    pub fn valid(&self) -> bool {
        match self.phase {
            PublicationCommandPhaseV1::Issued => !self.started && self.result.is_none(),
            PublicationCommandPhaseV1::Executing => self.started && self.result.is_none(),
            PublicationCommandPhaseV1::Settled => self.started && self.result.is_some(),
            PublicationCommandPhaseV1::Faulted => self.started && self.result.is_none(),
        }
    }
    pub fn begin(&mut self) {
        self.phase = PublicationCommandPhaseV1::Executing;
        self.started = true;
    }
    pub fn settle(&mut self, result: R) {
        self.result = Some(result);
        self.phase = PublicationCommandPhaseV1::Settled;
    }
    pub fn fault(&mut self) {
        self.phase = PublicationCommandPhaseV1::Faulted;
    }
}
