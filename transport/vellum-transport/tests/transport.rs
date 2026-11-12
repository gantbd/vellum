use std::io::{self, Cursor, Read, Write};
use vellum_client::Client;
use vellum_protocol::{
    ClientId, DurableEffectIntentV1, DurableExecuteOutcomeV1, DurableExecuteRequestV1,
    DurableOperationRefV1, DurableReceiptLookupOutcomeV1, DurableReceiptLookupV1,
    DurableReceiptStatusRequestV1, DurableReceiptStatusV1, RefusalV1, ReplyV1, RequestId,
    RequestV1, ServiceAccessErrorV1, ServiceControlErrorV1, ServicePort, WorkspaceDocumentIdV1,
    WorkspaceIdV1,
};
use vellum_transport::{
    FrameExchange, RemoteTransport, TransportError, TransportLimits, dispatch_v1, read_frame,
    serve_one, write_frame,
};
use vellum_wire::WireLimits;

fn limits() -> TransportLimits {
    TransportLimits {
        max_frame_bytes: 256,
        wire: WireLimits {
            max_frame_bytes: 256,
            max_text_bytes: 64,
            max_fields: 16,
        },
    }
}

struct StubService {
    client: ClientId,
    disconnected: bool,
    ordinary_executions: u64,
    durable_executions: u64,
    durable_lookups: u64,
    durable_statuses: u64,
}

impl StubService {
    fn new() -> Self {
        Self {
            client: ClientId::new(7).unwrap(),
            disconnected: false,
            ordinary_executions: 0,
            durable_executions: 0,
            durable_lookups: 0,
            durable_statuses: 0,
        }
    }
}

impl ServicePort for StubService {
    fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
        Ok(self.client)
    }

    fn execute(&mut self, _request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.ordinary_executions += 1;
        Ok(ReplyV1::Refused(RefusalV1::Stale))
    }

    fn execute_durable(
        &mut self,
        _request: DurableExecuteRequestV1,
    ) -> Result<DurableExecuteOutcomeV1, ServiceAccessErrorV1> {
        self.durable_executions += 1;
        Ok(DurableExecuteOutcomeV1::Refused(RefusalV1::Stale))
    }

    fn lookup_durable_receipt(
        &mut self,
        _lookup: DurableReceiptLookupV1,
    ) -> Result<DurableReceiptLookupOutcomeV1, ServiceAccessErrorV1> {
        self.durable_lookups += 1;
        Ok(DurableReceiptLookupOutcomeV1::OutcomeUnknown)
    }

    fn durable_receipt_status(
        &mut self,
        _request: DurableReceiptStatusRequestV1,
    ) -> Result<DurableReceiptStatusV1, ServiceAccessErrorV1> {
        self.durable_statuses += 1;
        Ok(DurableReceiptStatusV1::Pending)
    }

    fn disconnect(&mut self, client: ClientId) -> Result<(), ServiceControlErrorV1> {
        if client != self.client {
            return Err(ServiceControlErrorV1::Refused(RefusalV1::UnknownClient));
        }
        self.disconnected = true;
        Ok(())
    }
}

struct Loopback {
    service: StubService,
    limits: TransportLimits,
}

impl FrameExchange for Loopback {
    fn exchange(
        &mut self,
        request: &[u8],
        max_reply_bytes: usize,
    ) -> Result<Vec<u8>, TransportError> {
        assert_eq!(max_reply_bytes, self.limits.max_frame_bytes);
        dispatch_v1(&mut self.service, request, self.limits.wire)
    }
}

#[test]
fn remote_client_preserves_semantic_refusals_and_disconnects_explicitly() {
    let transport = RemoteTransport::new(
        Loopback {
            service: StubService::new(),
            limits: limits(),
        },
        limits(),
    )
    .unwrap();
    let mut client = Client::connect(transport).unwrap();
    assert_eq!(client.snapshot(), Ok(ReplyV1::Refused(RefusalV1::Stale)));
    let transport = match client.disconnect() {
        Ok(transport) => transport,
        Err((_, error)) => panic!("disconnect failed: {error:?}"),
    };
    assert!(transport.into_exchange().service.disconnected);
}

#[test]
fn durable_execute_and_read_only_lookup_dispatch_to_distinct_service_methods() {
    let mut transport = RemoteTransport::new(
        Loopback {
            service: StubService::new(),
            limits: limits(),
        },
        limits(),
    )
    .unwrap();
    let intent = DurableEffectIntentV1::SaveDocument {
        document: WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(3).unwrap(), 5).unwrap(),
        expected_revision: 8,
    };
    let operation = DurableOperationRefV1::new([9; 16]).unwrap();
    assert_eq!(
        transport
            .execute_durable(DurableExecuteRequestV1 {
                operation,
                client: ClientId::new(7).unwrap(),
                request: RequestId::new(1).unwrap(),
                intent: intent.clone(),
            })
            .unwrap(),
        DurableExecuteOutcomeV1::Refused(RefusalV1::Stale)
    );
    assert_eq!(
        transport
            .lookup_durable_receipt(DurableReceiptLookupV1 {
                operation,
                client: ClientId::new(7).unwrap(),
                request: RequestId::new(2).unwrap(),
                intent,
            })
            .unwrap(),
        DurableReceiptLookupOutcomeV1::OutcomeUnknown
    );
    assert_eq!(
        transport
            .durable_receipt_status(DurableReceiptStatusRequestV1 {
                operation,
                client: ClientId::new(7).unwrap(),
                request: RequestId::new(3).unwrap(),
            })
            .unwrap(),
        DurableReceiptStatusV1::Pending
    );
    let service = transport.into_exchange().service;
    assert_eq!(service.durable_executions, 1);
    assert_eq!(service.durable_lookups, 1);
    assert_eq!(service.durable_statuses, 1);
    assert_eq!(service.ordinary_executions, 0);
}

struct FailingExchange(TransportError);

impl FrameExchange for FailingExchange {
    fn exchange(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, TransportError> {
        Err(self.0)
    }
}

struct ReplyingExchange(Vec<u8>);

impl FrameExchange for ReplyingExchange {
    fn exchange(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, TransportError> {
        Ok(self.0.clone())
    }
}

#[test]
fn access_failures_never_masquerade_as_domain_refusals() {
    let unavailable =
        RemoteTransport::new(FailingExchange(TransportError::NotSent), limits()).unwrap();
    assert_eq!(
        Client::connect(unavailable).err(),
        Some(ServiceControlErrorV1::Access(ServiceAccessErrorV1::NotSent))
    );
    let uncertain =
        RemoteTransport::new(FailingExchange(TransportError::OutcomeUnknown), limits()).unwrap();
    assert_eq!(
        Client::connect(uncertain).err(),
        Some(ServiceControlErrorV1::Access(
            ServiceAccessErrorV1::OutcomeUnknown
        ))
    );

    let malformed_reply = RemoteTransport::new(ReplyingExchange(vec![0xff]), limits()).unwrap();
    assert_eq!(
        Client::connect(malformed_reply).err(),
        Some(ServiceControlErrorV1::Access(
            ServiceAccessErrorV1::OutcomeUnknown
        ))
    );
}

#[test]
fn exchange_phase_failures_have_separate_content_safe_counters() {
    let mut not_sent =
        RemoteTransport::new(FailingExchange(TransportError::NotSent), limits()).unwrap();
    assert_eq!(
        not_sent.connect(),
        Err(ServiceControlErrorV1::Access(ServiceAccessErrorV1::NotSent))
    );
    assert_eq!(not_sent.usage().exchanges, 1);
    assert_eq!(not_sent.usage().not_sent_failures, 1);
    assert_eq!(not_sent.usage().outcome_unknown_failures, 0);

    let mut unknown =
        RemoteTransport::new(FailingExchange(TransportError::OutcomeUnknown), limits()).unwrap();
    assert_eq!(
        unknown.connect(),
        Err(ServiceControlErrorV1::Access(
            ServiceAccessErrorV1::OutcomeUnknown
        ))
    );
    assert_eq!(unknown.usage().exchanges, 1);
    assert_eq!(unknown.usage().not_sent_failures, 0);
    assert_eq!(unknown.usage().outcome_unknown_failures, 1);
}

struct FailBeforeWrite;

impl Write for FailBeforeWrite {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "content-free"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FailAfterOneByte(bool);

impl Write for FailAfterOneByte {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.0 {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "content-free"))
        } else {
            self.0 = true;
            Ok(buffer.len().min(1))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn frame_writes_distinguish_not_sent_from_partially_sent() {
    assert_eq!(
        write_frame(&mut FailBeforeWrite, b"abc", 3),
        Err(TransportError::NotSent)
    );
    assert_eq!(
        write_frame(&mut FailAfterOneByte(false), b"abc", 3),
        Err(TransportError::OutcomeUnknown)
    );
}

#[test]
fn outer_frames_are_exact_and_refuse_oversize_before_payload_allocation() {
    let mut encoded = Vec::new();
    write_frame(&mut encoded, b"abc", 3).unwrap();
    assert_eq!(encoded, vec![0, 0, 0, 3, b'a', b'b', b'c']);
    assert_eq!(read_frame(&mut encoded.as_slice(), 3).unwrap(), b"abc");

    let oversized = [0, 0, 0, 4];
    assert_eq!(
        read_frame(&mut oversized.as_slice(), 3),
        Err(TransportError::FrameLimit)
    );
    assert_eq!(
        write_frame(&mut Vec::new(), b"abcd", 3),
        Err(TransportError::FrameLimit)
    );
}

struct Duplex {
    input: Cursor<Vec<u8>>,
    output: Vec<u8>,
}

impl Read for Duplex {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.input.read(buffer)
    }
}

impl Write for Duplex {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.output.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn server_consumes_one_outer_frame_and_writes_one_reply() {
    let request =
        vellum_wire::encode_request_v1(&vellum_wire::WireRequestV1::Connect, limits().wire)
            .unwrap();
    let mut framed = Vec::new();
    write_frame(&mut framed, &request, limits().max_frame_bytes).unwrap();
    let mut duplex = Duplex {
        input: Cursor::new(framed),
        output: Vec::new(),
    };
    let mut service = StubService::new();
    serve_one(&mut service, &mut duplex, limits()).unwrap();
    let reply = read_frame(&mut duplex.output.as_slice(), limits().max_frame_bytes).unwrap();
    assert_eq!(
        vellum_wire::decode_reply_v1(&reply, limits().wire).unwrap(),
        vellum_wire::WireReplyV1::Connected(ClientId::new(7).unwrap())
    );
}

#[cfg(unix)]
#[test]
fn unix_connector_exchanges_connect_execute_and_disconnect_with_one_owner() {
    use std::{
        fs,
        os::unix::{fs::PermissionsExt, net::UnixListener},
        sync::atomic::{AtomicU64, Ordering},
    };
    use vellum_transport::UnixConnector;

    static NEXT: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "vellum-transport-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let socket = directory.join("service.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = std::thread::spawn(move || {
        let mut service = StubService::new();
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            serve_one(&mut service, &mut stream, limits()).unwrap();
        }
        assert!(service.disconnected);
    });

    let transport = RemoteTransport::new(UnixConnector::new(socket.clone()), limits()).unwrap();
    let mut client = Client::connect(transport).unwrap();
    assert_eq!(client.snapshot(), Ok(ReplyV1::Refused(RefusalV1::Stale)));
    if let Err((_, error)) = client.disconnect() {
        panic!("disconnect failed: {error:?}");
    }
    server.join().unwrap();
    fs::remove_file(socket).unwrap();
    fs::remove_dir(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn unix_connector_reports_connection_refusal_as_not_sent() {
    use vellum_transport::UnixConnector;

    let missing =
        std::env::temp_dir().join(format!("vellum-missing-socket-{}", std::process::id()));
    let transport = RemoteTransport::new(UnixConnector::new(missing), limits()).unwrap();
    assert_eq!(
        Client::connect(transport).err(),
        Some(ServiceControlErrorV1::Access(ServiceAccessErrorV1::NotSent))
    );
}

#[cfg(unix)]
#[test]
fn unix_connector_reports_lost_reply_as_outcome_unknown() {
    use std::{
        fs,
        os::unix::{fs::PermissionsExt, net::UnixListener},
        sync::atomic::{AtomicU64, Ordering},
    };
    use vellum_transport::UnixConnector;

    static NEXT: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "vellum-transport-unknown-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let socket = directory.join("service.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _request = read_frame(&mut stream, limits().max_frame_bytes).unwrap();
        // Closing after consuming the request simulates a lost reply. The
        // caller cannot infer whether the service applied the operation.
    });

    let transport = RemoteTransport::new(UnixConnector::new(socket.clone()), limits()).unwrap();
    assert_eq!(
        Client::connect(transport).err(),
        Some(ServiceControlErrorV1::Access(
            ServiceAccessErrorV1::OutcomeUnknown
        ))
    );
    server.join().unwrap();
    fs::remove_file(socket).unwrap();
    fs::remove_dir(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn authenticated_connector_delegates_exact_challenge_and_session_before_frame() {
    use std::{
        fs,
        os::unix::{fs::PermissionsExt, net::UnixListener},
        sync::{
            Arc, Mutex,
            atomic::{AtomicU64, Ordering},
        },
    };
    use vellum_transport::{
        AuthenticatedUnixConnector, LocalAuthenticationClaimError,
        LocalAuthenticationClaimProvider, LocalAuthenticationClaimV1,
    };

    type ObservedHandshake = Arc<Mutex<Option<([u8; 16], [u8; 16])>>>;

    #[derive(Clone)]
    struct RecordingProvider(ObservedHandshake);

    impl LocalAuthenticationClaimProvider for RecordingProvider {
        fn claim(
            &mut self,
            challenge: [u8; 16],
            session: [u8; 16],
        ) -> Result<LocalAuthenticationClaimV1, LocalAuthenticationClaimError> {
            *self.0.lock().unwrap() = Some((challenge, session));
            let mut proof = [0_u8; 16];
            for index in 0..16 {
                proof[index] = challenge[index] ^ session[index];
            }
            Ok(LocalAuthenticationClaimV1::new(proof))
        }
    }

    static NEXT: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "vellum-transport-auth-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let socket = directory.join("service.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let challenge = [0x31; 16];
    let session = [0x52; 16];
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut hello = [0_u8; 33];
        hello[0] = 1;
        hello[1..17].copy_from_slice(&challenge);
        hello[17..33].copy_from_slice(&session);
        stream.write_all(&hello).unwrap();
        stream.flush().unwrap();
        let mut response = [0_u8; 49];
        stream.read_exact(&mut response).unwrap();
        assert_eq!(&response[..33], &hello);
        assert_eq!(&response[33..], &[0x63; 16]);
        stream.write_all(&[1, 1]).unwrap();
        stream.flush().unwrap();
        assert_eq!(read_frame(&mut stream, 256).unwrap(), b"request");
        write_frame(&mut stream, b"reply", 256).unwrap();
    });

    let observed = Arc::new(Mutex::new(None));
    let mut connector = AuthenticatedUnixConnector::with_provider(
        socket.clone(),
        RecordingProvider(Arc::clone(&observed)),
    );
    assert_eq!(connector.exchange(b"request", 256).unwrap(), b"reply");
    assert_eq!(*observed.lock().unwrap(), Some((challenge, session)));
    server.join().unwrap();
    fs::remove_file(socket).unwrap();
    fs::remove_dir(directory).unwrap();
}
