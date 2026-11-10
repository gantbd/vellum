//! Transport-neutral native Vellum client.
//!
//! The first transport is in-process. A future remote transport must implement
//! the same port without weakening request identity or refusal semantics.
#![forbid(unsafe_code)]

use vellum_protocol::{
    ClientId, CommandV1, RefusalV1, ReplyV1, RequestId, RequestV1, ServicePort, VersionWitness,
};

pub struct Client<T> {
    transport: T,
    client: ClientId,
    next_request: u64,
}

impl<T: ServicePort> Client<T> {
    pub fn connect(mut transport: T) -> Result<Self, RefusalV1> {
        let client = transport.connect()?;
        Ok(Self {
            transport,
            client,
            next_request: 1,
        })
    }

    pub fn snapshot(&mut self) -> ReplyV1 {
        self.send(CommandV1::Snapshot)
    }

    pub fn replace(
        &mut self,
        expected: VersionWitness,
        start: u64,
        end: u64,
        inserted: String,
    ) -> ReplyV1 {
        self.send(CommandV1::Replace {
            expected,
            start,
            end,
            inserted,
        })
    }

    pub fn send(&mut self, command: CommandV1) -> ReplyV1 {
        let Some(request) = RequestId::new(self.next_request) else {
            return ReplyV1::Refused(RefusalV1::CounterExhausted);
        };
        self.next_request = self.next_request.checked_add(1).unwrap_or(0);
        self.transport.execute(RequestV1 {
            client: self.client,
            request,
            command,
        })
    }

    pub fn into_transport(self) -> T {
        self.transport
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vellum_protocol::VersionWitness;

    struct RecordingTransport {
        requests: Vec<RequestV1>,
    }

    impl ServicePort for RecordingTransport {
        fn connect(&mut self) -> Result<ClientId, RefusalV1> {
            Ok(ClientId::new(7).unwrap())
        }

        fn execute(&mut self, request: RequestV1) -> ReplyV1 {
            self.requests.push(request);
            ReplyV1::Refused(RefusalV1::EditorUnavailable)
        }
    }

    #[test]
    fn client_issues_monotone_request_ids_without_interpreting_refusals() {
        let transport = RecordingTransport {
            requests: Vec::new(),
        };
        let mut client = Client::connect(transport).unwrap();
        assert_eq!(
            client.snapshot(),
            ReplyV1::Refused(RefusalV1::EditorUnavailable)
        );
        assert_eq!(
            client.replace(VersionWitness::new(3).unwrap(), 1, 2, "x".into()),
            ReplyV1::Refused(RefusalV1::EditorUnavailable)
        );
        let transport = client.into_transport();
        assert_eq!(transport.requests.len(), 2);
        assert_eq!(transport.requests[0].request.get(), 1);
        assert_eq!(transport.requests[1].request.get(), 2);
    }
}
