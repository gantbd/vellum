//! Private Unix listener and fixed local-authentication prelude for the
//! standalone semantic service. Provider launch and semantic role policy stay
//! in the executable composition root.

use std::fs;
use std::io::{Read, Write};
use std::os::unix::{
    fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _},
    net::{UnixListener, UnixStream},
};
use std::path::{Path, PathBuf};

use vellum_host::{
    authentication::{
        AuthenticatedSessionV1, AuthenticationChallengeRefV1, AuthenticationClaimRefV1,
        AuthenticationGateV1, AuthenticationPortV1, AuthenticationRequestV1,
    },
    authority::AuthoritySessionRefV1,
};

const AUTHENTICATION_HANDSHAKE_VERSION_V1: u8 = 1;
const AUTHENTICATION_HELLO_BYTES_V1: usize = 33;
const AUTHENTICATION_RESPONSE_BYTES_V1: usize = 49;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticLocalListenerRefusalV1 {
    InvalidPath,
    ParentUnavailable,
    ParentNotPrivate,
    EndpointExists,
    BindRefused,
    EndpointNotPrivate,
    EndpointReplaced,
    AcceptRefused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticLocalAuthenticationRefusalV1 {
    Io,
    Mismatch,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SocketIdentityV1 {
    device: u64,
    inode: u64,
}

/// Affine owner for one private Unix listener. Drop removes the pathname only
/// while it still names the exact socket this value bound.
pub struct SemanticLocalListenerV1 {
    listener: UnixListener,
    path: PathBuf,
    identity: SocketIdentityV1,
}

impl std::fmt::Debug for SemanticLocalListenerV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SemanticLocalListenerV1(<private-endpoint>)")
    }
}

impl SemanticLocalListenerV1 {
    pub fn try_bind(path: PathBuf) -> Result<Self, SemanticLocalListenerRefusalV1> {
        if !path.is_absolute() || path.file_name().is_none() {
            return Err(SemanticLocalListenerRefusalV1::InvalidPath);
        }
        let parent = path
            .parent()
            .ok_or(SemanticLocalListenerRefusalV1::InvalidPath)?;
        let canonical = fs::canonicalize(parent)
            .map_err(|_| SemanticLocalListenerRefusalV1::ParentUnavailable)?;
        if canonical != parent {
            return Err(SemanticLocalListenerRefusalV1::ParentUnavailable);
        }
        let metadata =
            fs::metadata(parent).map_err(|_| SemanticLocalListenerRefusalV1::ParentUnavailable)?;
        if !metadata.is_dir()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o077 != 0
        {
            return Err(SemanticLocalListenerRefusalV1::ParentNotPrivate);
        }
        match fs::symlink_metadata(&path) {
            Ok(_) => return Err(SemanticLocalListenerRefusalV1::EndpointExists),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(SemanticLocalListenerRefusalV1::EndpointExists),
        }
        let listener =
            UnixListener::bind(&path).map_err(|_| SemanticLocalListenerRefusalV1::BindRefused)?;
        if listener.set_nonblocking(true).is_err() {
            let _ = fs::remove_file(&path);
            return Err(SemanticLocalListenerRefusalV1::BindRefused);
        }
        if fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).is_err() {
            let _ = fs::remove_file(&path);
            return Err(SemanticLocalListenerRefusalV1::EndpointNotPrivate);
        }
        let identity = private_socket_identity(&path).inspect_err(|_reason| {
            let _ = fs::remove_file(&path);
        })?;
        Ok(Self {
            listener,
            path,
            identity,
        })
    }

    /// One nonblocking accept poll with path identity revalidation before and
    /// after the kernel accept. This observes replacement without requiring a
    /// peer to wake an orphaned listener.
    pub fn try_accept(&self) -> Result<Option<UnixStream>, SemanticLocalListenerRefusalV1> {
        if private_socket_identity(&self.path) != Ok(self.identity) {
            return Err(SemanticLocalListenerRefusalV1::EndpointReplaced);
        }
        let stream = match self.listener.accept() {
            Ok((stream, _)) => stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(None),
            Err(_) => return Err(SemanticLocalListenerRefusalV1::AcceptRefused),
        };
        // Listener polling is nonblocking, but the authenticated connection
        // protocol is deadline-bounded blocking I/O. Accepted descriptors can
        // inherit `O_NONBLOCK` on supported Unix variants, so make the
        // connection contract explicit before any handshake byte is written.
        stream
            .set_nonblocking(false)
            .map_err(|_| SemanticLocalListenerRefusalV1::AcceptRefused)?;
        if private_socket_identity(&self.path) != Ok(self.identity) {
            return Err(SemanticLocalListenerRefusalV1::EndpointReplaced);
        }
        Ok(Some(stream))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SemanticLocalListenerV1 {
    fn drop(&mut self) {
        if private_socket_identity(&self.path) == Ok(self.identity) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn private_socket_identity(
    path: &Path,
) -> Result<SocketIdentityV1, SemanticLocalListenerRefusalV1> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| SemanticLocalListenerRefusalV1::EndpointNotPrivate)?;
    if !metadata.file_type().is_socket()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o077 != 0
        || metadata.nlink() != 1
    {
        return Err(SemanticLocalListenerRefusalV1::EndpointNotPrivate);
    }
    Ok(SocketIdentityV1 {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

/// Performs the fixed-size authentication prelude on one already-deadlined
/// accepted stream. Authentication establishes attribution only; role policy
/// remains a separate dispatcher input.
pub fn authenticate_semantic_stream_v1<P>(
    gate: &mut AuthenticationGateV1<P>,
    stream: &mut UnixStream,
    challenge: [u8; 16],
    session: [u8; 16],
) -> Result<AuthenticatedSessionV1, SemanticLocalAuthenticationRefusalV1>
where
    P: AuthenticationPortV1<Context = ()>,
{
    if challenge == [0; 16] || session == [0; 16] {
        return Err(SemanticLocalAuthenticationRefusalV1::Mismatch);
    }
    let mut hello = [0_u8; AUTHENTICATION_HELLO_BYTES_V1];
    hello[0] = AUTHENTICATION_HANDSHAKE_VERSION_V1;
    hello[1..17].copy_from_slice(&challenge);
    hello[17..33].copy_from_slice(&session);
    stream
        .write_all(&hello)
        .and_then(|()| stream.flush())
        .map_err(|_| SemanticLocalAuthenticationRefusalV1::Io)?;

    let mut response = [0_u8; AUTHENTICATION_RESPONSE_BYTES_V1];
    stream
        .read_exact(&mut response)
        .map_err(|_| SemanticLocalAuthenticationRefusalV1::Io)?;
    let exact = response[0] == AUTHENTICATION_HANDSHAKE_VERSION_V1
        && response[1..17] == challenge
        && response[17..33] == session;
    let established = if exact {
        let mut claim = [0_u8; 16];
        claim.copy_from_slice(&response[33..49]);
        gate.establish(
            AuthenticationRequestV1 {
                challenge: AuthenticationChallengeRefV1::new(challenge),
                claim: AuthenticationClaimRefV1::new(claim),
                session: AuthoritySessionRefV1::new(session),
            },
            &(),
        )
        .ok()
    } else {
        None
    };
    let result = [
        AUTHENTICATION_HANDSHAKE_VERSION_V1,
        u8::from(established.is_some()),
    ];
    stream
        .write_all(&result)
        .and_then(|()| stream.flush())
        .map_err(|_| SemanticLocalAuthenticationRefusalV1::Io)?;
    established.ok_or(if exact {
        SemanticLocalAuthenticationRefusalV1::Rejected
    } else {
        SemanticLocalAuthenticationRefusalV1::Mismatch
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use vellum_host::{
        authentication::{
            AuthenticationDecisionV1, AuthenticationEvidenceV1, AuthenticationRequestV1,
        },
        authority::AuthoritySubjectRefV1,
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct ExactVerifier;
    impl AuthenticationPortV1 for ExactVerifier {
        type Context = ();
        fn authenticate(
            &mut self,
            request: AuthenticationRequestV1,
            _context: &(),
        ) -> AuthenticationDecisionV1 {
            if request.claim.to_bytes() == [0x33; 16] {
                AuthenticationDecisionV1::Authenticated(AuthenticationEvidenceV1::from_port(
                    request,
                    AuthoritySubjectRefV1::new([0x44; 16]),
                ))
            } else {
                AuthenticationDecisionV1::Rejected
            }
        }
    }

    struct Fixture {
        directory: PathBuf,
        path: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let directory = PathBuf::from("/tmp").join(format!(
                "vellum-semantic-local-listener-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&directory).unwrap();
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
            let directory = fs::canonicalize(directory).unwrap();
            let path = directory.join("semantic.sock");
            Self { directory, path }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
            let _ = fs::remove_dir(&self.directory);
        }
    }

    #[test]
    fn private_listener_authenticates_one_exact_fixed_prelude() {
        let fixture = Fixture::new();
        let listener = SemanticLocalListenerV1::try_bind(fixture.path.clone()).unwrap();
        let server = thread::spawn(move || {
            let mut stream = loop {
                if let Some(stream) = listener.try_accept().unwrap() {
                    break stream;
                }
                thread::yield_now();
            };
            let mut gate = AuthenticationGateV1::new(ExactVerifier, 1);
            authenticate_semantic_stream_v1(&mut gate, &mut stream, [0x11; 16], [0x22; 16])
                .unwrap()
                .authority_context()
        });
        let mut client = UnixStream::connect(&fixture.path).unwrap();
        let mut hello = [0_u8; AUTHENTICATION_HELLO_BYTES_V1];
        client.read_exact(&mut hello).unwrap();
        // The accepted connection must block here while an authenticated peer
        // computes its proof; inheriting the listener's nonblocking mode would
        // make the server refuse before this response is written.
        thread::sleep(std::time::Duration::from_millis(20));
        let mut response = [0_u8; AUTHENTICATION_RESPONSE_BYTES_V1];
        response[0] = AUTHENTICATION_HANDSHAKE_VERSION_V1;
        response[1..33].copy_from_slice(&hello[1..33]);
        response[33..49].copy_from_slice(&[0x33; 16]);
        client.write_all(&response).unwrap();
        let mut result = [0_u8; 2];
        client.read_exact(&mut result).unwrap();
        assert_eq!(result, [AUTHENTICATION_HANDSHAKE_VERSION_V1, 1]);
        let context = server.join().unwrap();
        assert_eq!(context.subject, AuthoritySubjectRefV1::new([0x44; 16]));
        assert_eq!(
            context.session,
            Some(AuthoritySessionRefV1::new([0x22; 16]))
        );
    }

    #[test]
    fn listener_refuses_nonprivate_parent_without_creating_endpoint() {
        let fixture = Fixture::new();
        fs::set_permissions(&fixture.directory, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            SemanticLocalListenerV1::try_bind(fixture.path.clone()).unwrap_err(),
            SemanticLocalListenerRefusalV1::ParentNotPrivate
        );
        assert!(!fixture.path.exists());
    }

    #[test]
    fn listener_fences_after_path_replacement_and_preserves_replacement() {
        let fixture = Fixture::new();
        let listener = SemanticLocalListenerV1::try_bind(fixture.path.clone()).unwrap();
        fs::remove_file(&fixture.path).unwrap();
        let replacement = UnixListener::bind(&fixture.path).unwrap();
        fs::set_permissions(&fixture.path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            listener.try_accept().unwrap_err(),
            SemanticLocalListenerRefusalV1::EndpointReplaced
        );
        drop(listener);
        assert!(fixture.path.exists());
        drop(replacement);
    }
}
