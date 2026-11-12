//! Bounded client-side Unix stream framing for one persistent local session.
//!
//! Socket ownership and mode checks establish a private local endpoint policy,
//! not authentication. Path identity is checked before and after connect but
//! remains observed-only because pathname connect is not descriptor-anchored.

use std::os::unix::net::UnixStream;
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::{
        fs::{FileTypeExt as _, MetadataExt as _},
        io::OwnedFd,
    },
    path::{Path, PathBuf},
    time::Duration,
};

use socket2::{Domain, SockAddr, Socket, Type};
use vellum_semantic_protocol::SemanticExchangeDispositionV1;

use crate::SemanticFrameExchangeV1;

pub const MAX_UNIX_SEMANTIC_SOCKET_PATH_BYTES_V1: usize = 4_096;
pub const MAX_UNIX_SEMANTIC_DEADLINE_MILLIS_V1: u64 = 60_000;
pub const UNIX_SEMANTIC_FRAME_HEADER_BYTES_V1: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnixSemanticFrameDeadlinesV1 {
    pub connect: Duration,
    pub write: Duration,
    pub read: Duration,
}

impl UnixSemanticFrameDeadlinesV1 {
    pub const fn reference() -> Self {
        Self {
            connect: Duration::from_secs(5),
            write: Duration::from_secs(5),
            read: Duration::from_secs(30),
        }
    }

    const fn is_valid(self) -> bool {
        valid_deadline(self.connect) && valid_deadline(self.write) && valid_deadline(self.read)
    }
}

const fn valid_deadline(value: Duration) -> bool {
    !value.is_zero() && value.as_millis() <= MAX_UNIX_SEMANTIC_DEADLINE_MILLIS_V1 as u128
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnixSemanticFrameBuildRefusalV1 {
    InvalidPath,
    InvalidDeadlines,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UnixSemanticFrameUsageV1 {
    pub attempts: u64,
    pub path_refusals: u64,
    pub connect_attempts: u64,
    pub connect_failures: u64,
    pub connections: u64,
    pub write_calls: u64,
    pub written_bytes: u64,
    pub read_calls: u64,
    pub received_bytes: u64,
    pub replies: u64,
    pub not_sent: u64,
    pub outcome_unknown: u64,
    pub local_limit_refusals: u64,
    pub invalid_reply_lengths: u64,
    pub oversized_replies: u64,
    pub write_timeouts: u64,
    pub read_timeouts: u64,
    pub fenced_attempts: u64,
    pub saturated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SocketIdentity {
    device: u64,
    inode: u64,
}

/// One persistent local byte-frame connection. It performs no authentication,
/// reconnect, retry, semantic decoding, or membership resumption.
pub struct UnixSemanticFrameExchangeV1 {
    path: PathBuf,
    deadlines: UnixSemanticFrameDeadlinesV1,
    stream: Option<UnixStream>,
    fenced: bool,
    usage: UnixSemanticFrameUsageV1,
}

impl std::fmt::Debug for UnixSemanticFrameExchangeV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UnixSemanticFrameExchangeV1")
            .field("connected", &self.stream.is_some())
            .field("fenced", &self.fenced)
            .field("usage", &self.usage)
            .finish()
    }
}

impl UnixSemanticFrameExchangeV1 {
    pub fn try_new(
        path: PathBuf,
        deadlines: UnixSemanticFrameDeadlinesV1,
    ) -> Result<Self, UnixSemanticFrameBuildRefusalV1> {
        if !path.is_absolute()
            || path.as_os_str().as_encoded_bytes().is_empty()
            || path.as_os_str().as_encoded_bytes().len() > MAX_UNIX_SEMANTIC_SOCKET_PATH_BYTES_V1
            || path.parent().is_none()
            || SockAddr::unix(&path).is_err()
        {
            return Err(UnixSemanticFrameBuildRefusalV1::InvalidPath);
        }
        if !deadlines.is_valid() {
            return Err(UnixSemanticFrameBuildRefusalV1::InvalidDeadlines);
        }
        Ok(Self {
            path,
            deadlines,
            stream: None,
            fenced: false,
            usage: UnixSemanticFrameUsageV1::default(),
        })
    }

    pub const fn usage(&self) -> UnixSemanticFrameUsageV1 {
        self.usage
    }

    pub const fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    pub const fn is_fenced(&self) -> bool {
        self.fenced
    }

    fn connect(&mut self) -> Result<(), ()> {
        bump(&mut self.usage.connect_attempts, &mut self.usage.saturated);
        let before = match private_socket_identity(&self.path) {
            Ok(identity) => identity,
            Err(()) => {
                bump(&mut self.usage.path_refusals, &mut self.usage.saturated);
                return Err(());
            }
        };
        let address = SockAddr::unix(&self.path).map_err(|_| ())?;
        let socket = Socket::new(Domain::UNIX, Type::STREAM, None).map_err(|_| ())?;
        if socket
            .connect_timeout(&address, self.deadlines.connect)
            .is_err()
        {
            bump(&mut self.usage.connect_failures, &mut self.usage.saturated);
            return Err(());
        }
        let after = private_socket_identity(&self.path).map_err(|_| ())?;
        if before != after {
            bump(&mut self.usage.path_refusals, &mut self.usage.saturated);
            return Err(());
        }
        let descriptor: OwnedFd = socket.into();
        let stream = UnixStream::from(descriptor);
        if stream
            .set_write_timeout(Some(self.deadlines.write))
            .is_err()
            || stream.set_read_timeout(Some(self.deadlines.read)).is_err()
        {
            bump(&mut self.usage.connect_failures, &mut self.usage.saturated);
            return Err(());
        }
        self.stream = Some(stream);
        bump(&mut self.usage.connections, &mut self.usage.saturated);
        Ok(())
    }

    fn not_sent(&mut self) -> SemanticExchangeDispositionV1<Vec<u8>> {
        bump(&mut self.usage.not_sent, &mut self.usage.saturated);
        SemanticExchangeDispositionV1::NotSent
    }

    fn unknown(&mut self) -> SemanticExchangeDispositionV1<Vec<u8>> {
        self.stream = None;
        self.fenced = true;
        bump(&mut self.usage.outcome_unknown, &mut self.usage.saturated);
        SemanticExchangeDispositionV1::OutcomeUnknown
    }
}

impl SemanticFrameExchangeV1 for UnixSemanticFrameExchangeV1 {
    fn exchange(
        &mut self,
        request: &[u8],
        max_reply_bytes: usize,
    ) -> SemanticExchangeDispositionV1<Vec<u8>> {
        bump(&mut self.usage.attempts, &mut self.usage.saturated);
        if self.fenced {
            bump(&mut self.usage.fenced_attempts, &mut self.usage.saturated);
            return self.not_sent();
        }
        if request.is_empty() || request.len() > u32::MAX as usize || max_reply_bytes == 0 {
            bump(
                &mut self.usage.local_limit_refusals,
                &mut self.usage.saturated,
            );
            return self.not_sent();
        }
        if self.stream.is_none() && self.connect().is_err() {
            return self.not_sent();
        }
        let mut frame = Vec::with_capacity(UNIX_SEMANTIC_FRAME_HEADER_BYTES_V1 + request.len());
        frame.extend_from_slice(&(request.len() as u32).to_be_bytes());
        frame.extend_from_slice(request);
        let mut written = 0usize;
        while written < frame.len() {
            bump(&mut self.usage.write_calls, &mut self.usage.saturated);
            let result = self
                .stream
                .as_mut()
                .expect("connected stream")
                .write(&frame[written..]);
            match result {
                Ok(0) => {
                    return if written == 0 {
                        self.stream = None;
                        self.not_sent()
                    } else {
                        self.unknown()
                    };
                }
                Ok(bytes) => {
                    written += bytes;
                    add(
                        &mut self.usage.written_bytes,
                        bytes,
                        &mut self.usage.saturated,
                    );
                }
                Err(error) => {
                    if is_timeout(&error) {
                        bump(&mut self.usage.write_timeouts, &mut self.usage.saturated);
                    }
                    return if written == 0 {
                        self.stream = None;
                        self.not_sent()
                    } else {
                        self.unknown()
                    };
                }
            }
        }

        let mut header = [0u8; UNIX_SEMANTIC_FRAME_HEADER_BYTES_V1];
        if let Err(error) = read_exact_counted(
            self.stream.as_mut().expect("connected stream"),
            &mut header,
            &mut self.usage,
        ) {
            if is_timeout(&error) {
                bump(&mut self.usage.read_timeouts, &mut self.usage.saturated);
            }
            return self.unknown();
        }
        let reply_len = u32::from_be_bytes(header) as usize;
        if reply_len == 0 {
            bump(
                &mut self.usage.invalid_reply_lengths,
                &mut self.usage.saturated,
            );
            return self.unknown();
        }
        if reply_len > max_reply_bytes {
            bump(&mut self.usage.oversized_replies, &mut self.usage.saturated);
            return self.unknown();
        }
        let mut reply = vec![0u8; reply_len];
        if let Err(error) = read_exact_counted(
            self.stream.as_mut().expect("connected stream"),
            &mut reply,
            &mut self.usage,
        ) {
            if is_timeout(&error) {
                bump(&mut self.usage.read_timeouts, &mut self.usage.saturated);
            }
            return self.unknown();
        }
        bump(&mut self.usage.replies, &mut self.usage.saturated);
        SemanticExchangeDispositionV1::Reply(reply)
    }
}

fn private_socket_identity(path: &Path) -> Result<SocketIdentity, ()> {
    let parent = path.parent().ok_or(())?;
    if fs::canonicalize(parent).map_err(|_| ())? != parent {
        return Err(());
    }
    let parent_metadata = fs::metadata(parent).map_err(|_| ())?;
    let socket_metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    let uid = rustix::process::geteuid().as_raw();
    if !parent_metadata.is_dir()
        || parent_metadata.uid() != uid
        || parent_metadata.mode() & 0o077 != 0
        || !socket_metadata.file_type().is_socket()
        || socket_metadata.uid() != uid
        || socket_metadata.mode() & 0o077 != 0
    {
        return Err(());
    }
    Ok(SocketIdentity {
        device: socket_metadata.dev(),
        inode: socket_metadata.ino(),
    })
}

fn read_exact_counted(
    stream: &mut UnixStream,
    bytes: &mut [u8],
    usage: &mut UnixSemanticFrameUsageV1,
) -> io::Result<()> {
    let mut offset = 0usize;
    while offset < bytes.len() {
        bump(&mut usage.read_calls, &mut usage.saturated);
        match stream.read(&mut bytes[offset..]) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
            Ok(read) => {
                offset += read;
                add(&mut usage.received_bytes, read, &mut usage.saturated);
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn is_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
    )
}

fn bump(value: &mut u64, saturated: &mut bool) {
    let prior = *value;
    *value = value.saturating_add(1);
    *saturated |= prior == u64::MAX;
}

fn add(value: &mut u64, amount: usize, saturated: &mut bool) {
    let amount = u64::try_from(amount).unwrap_or(u64::MAX);
    match value.checked_add(amount) {
        Some(total) => *value = total,
        None => {
            *value = u64::MAX;
            *saturated = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::{fs::PermissionsExt as _, net::UnixListener},
        sync::atomic::{AtomicU64, Ordering},
        thread,
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct SocketFixture {
        directory: PathBuf,
        path: PathBuf,
    }

    impl SocketFixture {
        fn new() -> (Self, UnixListener) {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let directory = PathBuf::from("/tmp").join(format!(
                "vellum-semantic-transport-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&directory).unwrap();
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
            let directory = fs::canonicalize(directory).unwrap();
            let path = directory.join("session.sock");
            let listener = UnixListener::bind(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            (Self { directory, path }, listener)
        }
    }

    impl Drop for SocketFixture {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
            let _ = fs::remove_dir(&self.directory);
        }
    }

    fn read_frame(stream: &mut UnixStream) -> Vec<u8> {
        let mut header = [0u8; UNIX_SEMANTIC_FRAME_HEADER_BYTES_V1];
        stream.read_exact(&mut header).unwrap();
        let mut bytes = vec![0u8; u32::from_be_bytes(header) as usize];
        stream.read_exact(&mut bytes).unwrap();
        bytes
    }

    fn write_frame(stream: &mut UnixStream, bytes: &[u8]) {
        stream
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .unwrap();
        stream.write_all(bytes).unwrap();
    }

    #[test]
    fn persistent_connection_exchanges_two_bounded_frames() {
        let (fixture, listener) = SocketFixture::new();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            assert_eq!(read_frame(&mut stream), b"first");
            write_frame(&mut stream, b"one");
            assert_eq!(read_frame(&mut stream), b"second");
            write_frame(&mut stream, b"two");
        });
        let mut exchange = UnixSemanticFrameExchangeV1::try_new(
            fixture.path.clone(),
            UnixSemanticFrameDeadlinesV1::reference(),
        )
        .unwrap();
        assert_eq!(
            exchange.exchange(b"first", 16),
            SemanticExchangeDispositionV1::Reply(b"one".to_vec())
        );
        assert_eq!(
            exchange.exchange(b"second", 16),
            SemanticExchangeDispositionV1::Reply(b"two".to_vec())
        );
        assert_eq!(exchange.usage().connections, 1);
        assert_eq!(exchange.usage().replies, 2);
        assert_eq!(exchange.usage().written_bytes, 19);
        assert_eq!(exchange.usage().received_bytes, 14);
        server.join().unwrap();
    }

    #[test]
    fn lost_reply_is_unknown_and_permanently_fences_connection() {
        let (fixture, listener) = SocketFixture::new();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            assert_eq!(read_frame(&mut stream), b"effect");
        });
        let mut exchange = UnixSemanticFrameExchangeV1::try_new(
            fixture.path.clone(),
            UnixSemanticFrameDeadlinesV1::reference(),
        )
        .unwrap();
        assert_eq!(
            exchange.exchange(b"effect", 16),
            SemanticExchangeDispositionV1::OutcomeUnknown
        );
        assert!(exchange.is_fenced());
        assert_eq!(
            exchange.exchange(b"effect", 16),
            SemanticExchangeDispositionV1::NotSent
        );
        assert_eq!(exchange.usage().outcome_unknown, 1);
        assert_eq!(exchange.usage().fenced_attempts, 1);
        server.join().unwrap();
    }

    #[test]
    fn private_path_and_connect_failures_are_not_sent() {
        let (fixture, _listener) = SocketFixture::new();
        fs::set_permissions(&fixture.directory, fs::Permissions::from_mode(0o755)).unwrap();
        let mut exchange = UnixSemanticFrameExchangeV1::try_new(
            fixture.path.clone(),
            UnixSemanticFrameDeadlinesV1::reference(),
        )
        .unwrap();
        assert_eq!(
            exchange.exchange(b"request", 16),
            SemanticExchangeDispositionV1::NotSent
        );
        assert_eq!(exchange.usage().path_refusals, 1);
        assert_eq!(exchange.usage().written_bytes, 0);
    }

    #[test]
    fn read_deadline_after_request_is_unknown() {
        let (fixture, listener) = SocketFixture::new();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            assert_eq!(read_frame(&mut stream), b"request");
            thread::sleep(Duration::from_millis(100));
        });
        let mut exchange = UnixSemanticFrameExchangeV1::try_new(
            fixture.path.clone(),
            UnixSemanticFrameDeadlinesV1 {
                connect: Duration::from_secs(1),
                write: Duration::from_secs(1),
                read: Duration::from_millis(20),
            },
        )
        .unwrap();
        assert_eq!(
            exchange.exchange(b"request", 16),
            SemanticExchangeDispositionV1::OutcomeUnknown
        );
        assert_eq!(exchange.usage().read_timeouts, 1);
        server.join().unwrap();
    }

    #[test]
    fn oversized_reply_header_fences_without_allocating_payload() {
        let (fixture, listener) = SocketFixture::new();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            assert_eq!(read_frame(&mut stream), b"request");
            stream.write_all(&1024u32.to_be_bytes()).unwrap();
        });
        let mut exchange = UnixSemanticFrameExchangeV1::try_new(
            fixture.path.clone(),
            UnixSemanticFrameDeadlinesV1::reference(),
        )
        .unwrap();
        assert_eq!(
            exchange.exchange(b"request", 16),
            SemanticExchangeDispositionV1::OutcomeUnknown
        );
        assert_eq!(exchange.usage().oversized_replies, 1);
        assert_eq!(exchange.usage().received_bytes, 4);
        server.join().unwrap();
    }
}
