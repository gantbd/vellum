use std::io::{self, Cursor, Read, Write};
use vellum_client::Client;
use vellum_protocol::{
    ClientId, RefusalV1, ReplyV1, RequestV1, ServiceAccessErrorV1, ServiceControlErrorV1,
    ServicePort,
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
}

impl StubService {
    fn new() -> Self {
        Self {
            client: ClientId::new(7).unwrap(),
            disconnected: false,
        }
    }
}

impl ServicePort for StubService {
    fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
        Ok(self.client)
    }

    fn execute(&mut self, _request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
        Ok(ReplyV1::Refused(RefusalV1::Stale))
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

struct FailingExchange(TransportError);

impl FrameExchange for FailingExchange {
    fn exchange(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, TransportError> {
        Err(self.0)
    }
}

#[test]
fn access_failures_never_masquerade_as_domain_refusals() {
    let unavailable = RemoteTransport::new(FailingExchange(TransportError::Io), limits()).unwrap();
    assert_eq!(
        Client::connect(unavailable).err(),
        Some(ServiceControlErrorV1::Access(
            ServiceAccessErrorV1::Unavailable
        ))
    );
    let malformed =
        RemoteTransport::new(FailingExchange(TransportError::Malformed), limits()).unwrap();
    assert_eq!(
        Client::connect(malformed).err(),
        Some(ServiceControlErrorV1::Access(
            ServiceAccessErrorV1::Malformed
        ))
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
