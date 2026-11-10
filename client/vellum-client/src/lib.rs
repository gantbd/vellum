//! Transport-neutral native Vellum client.
//!
//! The first transport is in-process. A future remote transport must implement
//! the same port without weakening request identity or refusal semantics.
#![forbid(unsafe_code)]

use vellum_protocol::{
    ClientId, CommandV1, RefusalV1, ReplyV1, RequestId, RequestV1, ServiceAccessErrorV1,
    ServiceControlErrorV1, ServicePort, VersionWitness,
};

pub struct Client<T> {
    transport: T,
    client: ClientId,
    next_request: u64,
}

impl<T: ServicePort> Client<T> {
    pub fn connect(mut transport: T) -> Result<Self, ServiceControlErrorV1> {
        let client = transport.connect()?;
        Ok(Self {
            transport,
            client,
            next_request: 1,
        })
    }

    pub fn snapshot(&mut self) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::Snapshot)
    }

    pub fn replace(
        &mut self,
        expected: VersionWitness,
        start: u64,
        end: u64,
        inserted: String,
    ) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::Replace {
            expected,
            start,
            end,
            inserted,
        })
    }

    pub fn send(&mut self, command: CommandV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
        let Some(request) = RequestId::new(self.next_request) else {
            return Ok(ReplyV1::Refused(RefusalV1::CounterExhausted));
        };
        self.next_request = self.next_request.checked_add(1).unwrap_or(0);
        self.transport.execute(RequestV1 {
            client: self.client,
            request,
            command,
        })
    }

    pub fn disconnect(mut self) -> Result<T, (T, ServiceControlErrorV1)> {
        match self.transport.disconnect(self.client) {
            Ok(()) => Ok(self.transport),
            Err(error) => Err((self.transport, error)),
        }
    }

    pub fn into_transport(self) -> T {
        self.transport
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vellum_protocol::VersionWitness;

    #[derive(Debug)]
    struct RecordingTransport {
        requests: Vec<RequestV1>,
        disconnected: Vec<ClientId>,
    }

    impl ServicePort for RecordingTransport {
        fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
            Ok(ClientId::new(7).unwrap())
        }

        fn execute(&mut self, request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
            self.requests.push(request);
            Ok(ReplyV1::Refused(RefusalV1::EditorUnavailable))
        }

        fn disconnect(&mut self, client: ClientId) -> Result<(), ServiceControlErrorV1> {
            self.disconnected.push(client);
            Ok(())
        }
    }

    #[test]
    fn client_issues_monotone_request_ids_without_interpreting_refusals() {
        let transport = RecordingTransport {
            requests: Vec::new(),
            disconnected: Vec::new(),
        };
        let mut client = Client::connect(transport).unwrap();
        assert_eq!(
            client.snapshot(),
            Ok(ReplyV1::Refused(RefusalV1::EditorUnavailable))
        );
        assert_eq!(
            client.replace(VersionWitness::new(3).unwrap(), 1, 2, "x".into()),
            Ok(ReplyV1::Refused(RefusalV1::EditorUnavailable))
        );
        let transport = client.disconnect().unwrap();
        assert_eq!(transport.requests.len(), 2);
        assert_eq!(transport.requests[0].request.get(), 1);
        assert_eq!(transport.requests[1].request.get(), 2);
        assert_eq!(transport.disconnected, vec![ClientId::new(7).unwrap()]);
    }
}
