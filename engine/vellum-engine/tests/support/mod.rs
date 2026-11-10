//! Test-only fault injection seam. NOT the production resource/effect API.
//! No identity, authorization, atomicity or persistence guarantee is modeled.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteObservation {
    Completed,
    Failed,
    Unknown,
}

pub trait SubstrateFixture {
    fn read(&self) -> Result<Vec<u8>, ()>;
    fn write(&mut self, bytes: &[u8]) -> WriteObservation;
}

pub struct ScriptedSubstrate {
    pub bytes: Vec<u8>,
    pub fail_read: bool,
    pub next_write: WriteObservation,
    pub write_attempts: usize,
}

impl SubstrateFixture for ScriptedSubstrate {
    fn read(&self) -> Result<Vec<u8>, ()> {
        if self.fail_read {
            Err(())
        } else {
            Ok(self.bytes.clone())
        }
    }

    fn write(&mut self, bytes: &[u8]) -> WriteObservation {
        self.write_attempts += 1;
        if self.next_write == WriteObservation::Completed {
            self.bytes = bytes.to_vec();
        }
        // Unknown deliberately makes no assertion about external state.
        self.next_write
    }
}
