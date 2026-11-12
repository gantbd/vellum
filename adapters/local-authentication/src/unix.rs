use crate::{LocalAuthenticationError, LocalAuthenticationUsageV1, LocalCredentialRecordV1, bump};
use rustix::{
    fs::{AtFlags, Mode, OFlags, openat, statat},
    io::Errno,
};
use std::{
    ffi::OsStr,
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use vellum_host::{
    authentication::{
        AuthenticationDecisionV1, AuthenticationEvidenceV1, AuthenticationPortV1,
        AuthenticationRequestV1,
    },
    authority::AuthoritySubjectRefV1,
};
use vellum_transport::{
    LocalAuthenticationClaimError, LocalAuthenticationClaimProvider, LocalAuthenticationClaimV1,
};

#[derive(Clone, Copy, Eq, PartialEq)]
struct Identity {
    device: u64,
    inode: u64,
}

struct CredentialPath {
    parent_path: PathBuf,
    name: std::ffi::OsString,
}

#[derive(Clone, Copy)]
enum FsOperation {
    Lookup,
    ExclusiveCreate,
}

// Keep the externally observable error alphabet fixed while distinguishing
// rejected names/bindings from failures for which retry can be meaningful.
// Unknown errno values are availability failures: treating a new operational
// errno as an unsafe credential would turn host pressure into a false refusal.
fn classify_errno(error: Errno, operation: FsOperation) -> LocalAuthenticationError {
    if error == Errno::NOENT
        || error == Errno::NOTDIR
        || error == Errno::LOOP
        || error == Errno::ACCESS
        || error == Errno::PERM
        || error == Errno::NAMETOOLONG
        || error == Errno::INVAL
        || error == Errno::ISDIR
        // Special filesystem objects can fail before descriptor validation.
        || error == Errno::NXIO
        || error == Errno::NOTSUP
        || error == Errno::OPNOTSUPP
        || (matches!(operation, FsOperation::ExclusiveCreate) && error == Errno::EXIST)
    {
        LocalAuthenticationError::Refused
    } else {
        LocalAuthenticationError::Unavailable
    }
}

fn classify_io_error(error: &std::io::Error, operation: FsOperation) -> LocalAuthenticationError {
    error.raw_os_error().map_or_else(
        || match error.kind() {
            std::io::ErrorKind::NotFound
            | std::io::ErrorKind::PermissionDenied
            | std::io::ErrorKind::InvalidInput
            | std::io::ErrorKind::InvalidData
            | std::io::ErrorKind::UnexpectedEof => LocalAuthenticationError::Refused,
            _ => LocalAuthenticationError::Unavailable,
        },
        |raw| classify_errno(Errno::from_raw_os_error(raw), operation),
    )
}

impl CredentialPath {
    fn parse(path: &Path) -> Result<Self, LocalAuthenticationError> {
        let parent = path.parent().ok_or(LocalAuthenticationError::Refused)?;
        let name = path.file_name().ok_or(LocalAuthenticationError::Refused)?;
        if parent.as_os_str().is_empty() || name == OsStr::new(".") || name == OsStr::new("..") {
            return Err(LocalAuthenticationError::Refused);
        }
        Ok(Self {
            parent_path: parent.to_owned(),
            name: name.to_owned(),
        })
    }
}

struct AnchoredParent {
    descriptor: File,
    path: PathBuf,
    identity: Identity,
}

impl AnchoredParent {
    fn open(path: &Path) -> Result<Self, LocalAuthenticationError> {
        let flags = i32::try_from((OFlags::NOFOLLOW | OFlags::DIRECTORY | OFlags::CLOEXEC).bits())
            .map_err(|_| LocalAuthenticationError::Unavailable)?;
        let descriptor = OpenOptions::new()
            .read(true)
            .custom_flags(flags)
            .open(path)
            .map_err(|error| classify_io_error(&error, FsOperation::Lookup))?;
        let metadata = descriptor
            .metadata()
            .map_err(|_| LocalAuthenticationError::Unavailable)?;
        if !metadata.file_type().is_dir()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o7777 != 0o700
        {
            return Err(LocalAuthenticationError::Refused);
        }
        Ok(Self {
            descriptor,
            path: path.to_owned(),
            identity: Identity {
                device: metadata.dev(),
                inode: metadata.ino(),
            },
        })
    }

    fn revalidate_parent(&self) -> Result<(), LocalAuthenticationError> {
        let reopened = Self::open(&self.path)?;
        if reopened.identity == self.identity {
            Ok(())
        } else {
            Err(LocalAuthenticationError::Refused)
        }
    }

    fn binding_identity(&self, name: &OsStr) -> Result<Identity, LocalAuthenticationError> {
        let metadata = statat(&self.descriptor, name, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|error| classify_errno(error, FsOperation::Lookup))?;
        Ok(Identity {
            device: u64::try_from(metadata.st_dev)
                .map_err(|_| LocalAuthenticationError::Unavailable)?,
            inode: metadata.st_ino,
        })
    }

    fn read_record(
        &self,
        name: &OsStr,
    ) -> Result<LocalCredentialRecordV1, LocalAuthenticationError> {
        self.read_record_before_revalidate(name, || {})
    }

    fn read_record_before_revalidate(
        &self,
        name: &OsStr,
        before_revalidate: impl FnOnce(),
    ) -> Result<LocalCredentialRecordV1, LocalAuthenticationError> {
        let mut file = File::from(
            openat(
                &self.descriptor,
                name,
                OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
                Mode::empty(),
            )
            .map_err(|error| classify_errno(error, FsOperation::Lookup))?,
        );
        let before = validate_file(&file)?;
        let mut bytes = [0_u8; crate::CREDENTIAL_RECORD_BYTES_V1];
        file.read_exact(&mut bytes)
            .map_err(|error| classify_io_error(&error, FsOperation::Lookup))?;
        let mut trailing = [0_u8; 1];
        if file
            .read(&mut trailing)
            .map_err(|error| classify_io_error(&error, FsOperation::Lookup))?
            != 0
        {
            return Err(LocalAuthenticationError::Refused);
        }
        let after = validate_file(&file)?;
        before_revalidate();
        if before != after || self.binding_identity(name)? != after {
            return Err(LocalAuthenticationError::Refused);
        }
        self.revalidate_parent()?;
        let final_identity = validate_file(&file)?;
        if final_identity != after || self.binding_identity(name)? != final_identity {
            return Err(LocalAuthenticationError::Refused);
        }
        self.revalidate_parent()?;
        LocalCredentialRecordV1::decode(&bytes)
    }

    fn create_record(
        &self,
        name: &OsStr,
        record: &LocalCredentialRecordV1,
    ) -> Result<(), LocalAuthenticationError> {
        let mut file = File::from(
            openat(
                &self.descriptor,
                name,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|error| classify_errno(error, FsOperation::ExclusiveCreate))?,
        );
        file.write_all(&record.encode())
            .map_err(|_| LocalAuthenticationError::OutcomeUnknown)?;
        file.sync_all()
            .map_err(|_| LocalAuthenticationError::OutcomeUnknown)?;
        let identity =
            validate_file(&file).map_err(|_| LocalAuthenticationError::OutcomeUnknown)?;
        let final_identity =
            validate_file(&file).map_err(|_| LocalAuthenticationError::OutcomeUnknown)?;
        if final_identity != identity
            || self
                .binding_identity(name)
                .map_err(|_| LocalAuthenticationError::OutcomeUnknown)?
                != final_identity
        {
            return Err(LocalAuthenticationError::OutcomeUnknown);
        }
        self.revalidate_parent()
            .map_err(|_| LocalAuthenticationError::OutcomeUnknown)?;
        self.descriptor
            .sync_all()
            .map_err(|_| LocalAuthenticationError::OutcomeUnknown)?;
        if self
            .binding_identity(name)
            .map_err(|_| LocalAuthenticationError::OutcomeUnknown)?
            != identity
        {
            return Err(LocalAuthenticationError::OutcomeUnknown);
        }
        self.revalidate_parent()
            .map_err(|_| LocalAuthenticationError::OutcomeUnknown)
    }
}

fn validate_file(file: &File) -> Result<Identity, LocalAuthenticationError> {
    validate_file_for_owner(file, rustix::process::geteuid().as_raw())
}

fn validate_file_for_owner(
    file: &File,
    expected_owner: u32,
) -> Result<Identity, LocalAuthenticationError> {
    let metadata = file
        .metadata()
        .map_err(|_| LocalAuthenticationError::Unavailable)?;
    if !metadata.file_type().is_file()
        || metadata.file_type().is_socket()
        || metadata.uid() != expected_owner
        || metadata.mode() & 0o7777 != 0o600
        || metadata.nlink() != 1
        || metadata.len() != 68_u64
    {
        return Err(LocalAuthenticationError::Refused);
    }
    Ok(Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

/// Descriptor-relative credential loading, provisioning and staged rotation.
#[derive(Default)]
pub struct LocalCredentialStoreV1 {
    usage: LocalAuthenticationUsageV1,
}

impl LocalCredentialStoreV1 {
    pub const fn usage(&self) -> LocalAuthenticationUsageV1 {
        self.usage
    }

    pub fn load(
        &mut self,
        path: &Path,
    ) -> Result<LocalCredentialRecordV1, LocalAuthenticationError> {
        let result = load(path);
        if result.is_ok() {
            bump(
                &mut self.usage.credential_loads_completed,
                &mut self.usage.counter_saturated,
            );
        } else {
            bump(
                &mut self.usage.credential_loads_refused,
                &mut self.usage.counter_saturated,
            );
        }
        result
    }

    pub fn provision(
        &mut self,
        path: &Path,
    ) -> Result<LocalCredentialRecordV1, LocalAuthenticationError> {
        let record = match random_record(1) {
            Ok(record) => record,
            Err(reason) => {
                bump(
                    &mut self.usage.randomness_failures,
                    &mut self.usage.counter_saturated,
                );
                bump(
                    &mut self.usage.provisions_refused,
                    &mut self.usage.counter_saturated,
                );
                return Err(reason);
            }
        };
        let result = (|| {
            create(path, &record)?;
            Ok(record)
        })();
        match result {
            Ok(record) => {
                bump(
                    &mut self.usage.provisions_completed,
                    &mut self.usage.counter_saturated,
                );
                Ok(record)
            }
            Err(reason) => {
                bump(
                    &mut self.usage.provisions_refused,
                    &mut self.usage.counter_saturated,
                );
                Err(reason)
            }
        }
    }

    pub fn stage_rotation(
        &mut self,
        current_path: &Path,
        next_path: &Path,
    ) -> Result<LocalCredentialRecordV1, LocalAuthenticationError> {
        let result = self.load(current_path).and_then(|current| {
            let generation = current
                .generation
                .checked_add(1)
                .ok_or(LocalAuthenticationError::Refused)?;
            let mut secret = [0_u8; 32];
            if getrandom::fill(&mut secret).is_err() {
                bump(
                    &mut self.usage.randomness_failures,
                    &mut self.usage.counter_saturated,
                );
                return Err(LocalAuthenticationError::Unavailable);
            }
            if secret == [0; 32] || secret == current.secret {
                bump(
                    &mut self.usage.randomness_failures,
                    &mut self.usage.counter_saturated,
                );
                return Err(LocalAuthenticationError::Unavailable);
            }
            let next = LocalCredentialRecordV1 {
                generation,
                subject: current.subject,
                secret,
            };
            create(next_path, &next)?;
            Ok(next)
        });
        match result {
            Ok(record) => {
                bump(
                    &mut self.usage.rotations_staged,
                    &mut self.usage.counter_saturated,
                );
                Ok(record)
            }
            Err(reason) => {
                bump(
                    &mut self.usage.rotations_refused,
                    &mut self.usage.counter_saturated,
                );
                Err(reason)
            }
        }
    }
}

fn load(path: &Path) -> Result<LocalCredentialRecordV1, LocalAuthenticationError> {
    let path = CredentialPath::parse(path)?;
    AnchoredParent::open(&path.parent_path)?.read_record(&path.name)
}

fn create(path: &Path, record: &LocalCredentialRecordV1) -> Result<(), LocalAuthenticationError> {
    let path = CredentialPath::parse(path)?;
    AnchoredParent::open(&path.parent_path)?.create_record(&path.name, record)
}

fn random_record(generation: u64) -> Result<LocalCredentialRecordV1, LocalAuthenticationError> {
    let mut material = [0_u8; 48];
    getrandom::fill(&mut material).map_err(|_| LocalAuthenticationError::Unavailable)?;
    let subject = material[..16]
        .try_into()
        .map_err(|_| LocalAuthenticationError::Unavailable)?;
    let secret = material[16..]
        .try_into()
        .map_err(|_| LocalAuthenticationError::Unavailable)?;
    if subject == [0; 16] || secret == [0; 32] {
        return Err(LocalAuthenticationError::Unavailable);
    }
    Ok(LocalCredentialRecordV1 {
        generation,
        subject,
        secret,
    })
}

/// Connector-side proof provider for one already-loaded record.
pub struct LocalCredentialClaimProviderV1 {
    record: LocalCredentialRecordV1,
    usage: LocalAuthenticationUsageV1,
}

impl LocalCredentialClaimProviderV1 {
    pub const fn new(record: LocalCredentialRecordV1) -> Self {
        Self {
            record,
            usage: LocalAuthenticationUsageV1 {
                credential_loads_completed: 0,
                credential_loads_refused: 0,
                provisions_completed: 0,
                provisions_refused: 0,
                rotations_staged: 0,
                rotations_refused: 0,
                randomness_failures: 0,
                proofs_completed: 0,
                proofs_refused: 0,
                counter_saturated: false,
            },
        }
    }

    pub const fn usage(&self) -> LocalAuthenticationUsageV1 {
        self.usage
    }
}

impl LocalAuthenticationClaimProvider for LocalCredentialClaimProviderV1 {
    fn claim(
        &mut self,
        challenge: [u8; 16],
        session: [u8; 16],
    ) -> Result<LocalAuthenticationClaimV1, LocalAuthenticationClaimError> {
        bump(
            &mut self.usage.proofs_completed,
            &mut self.usage.counter_saturated,
        );
        Ok(LocalAuthenticationClaimV1::new(
            self.record.claim(challenge, session),
        ))
    }
}

/// Host-side verifier implementing the existing authentication port.
pub struct LocalCredentialVerifierV1 {
    record: LocalCredentialRecordV1,
    usage: LocalAuthenticationUsageV1,
}

impl LocalCredentialVerifierV1 {
    pub const fn new(record: LocalCredentialRecordV1) -> Self {
        Self {
            record,
            usage: LocalAuthenticationUsageV1 {
                credential_loads_completed: 0,
                credential_loads_refused: 0,
                provisions_completed: 0,
                provisions_refused: 0,
                rotations_staged: 0,
                rotations_refused: 0,
                randomness_failures: 0,
                proofs_completed: 0,
                proofs_refused: 0,
                counter_saturated: false,
            },
        }
    }

    pub const fn usage(&self) -> LocalAuthenticationUsageV1 {
        self.usage
    }
}

impl AuthenticationPortV1 for LocalCredentialVerifierV1 {
    type Context = ();

    fn authenticate(
        &mut self,
        request: AuthenticationRequestV1,
        _context: &Self::Context,
    ) -> AuthenticationDecisionV1 {
        if self.record.verify(
            request.challenge.to_bytes(),
            request.session.to_bytes(),
            request.claim.to_bytes(),
        ) {
            bump(
                &mut self.usage.proofs_completed,
                &mut self.usage.counter_saturated,
            );
            AuthenticationDecisionV1::Authenticated(AuthenticationEvidenceV1::from_port(
                request,
                AuthoritySubjectRefV1::new(self.record.subject),
            ))
        } else {
            bump(
                &mut self.usage.proofs_refused,
                &mut self.usage.counter_saturated,
            );
            AuthenticationDecisionV1::Rejected
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::{
        fs::{PermissionsExt, symlink},
        net::UnixListener,
    };
    use vellum_host::{
        authentication::{AuthenticationChallengeRefV1, AuthenticationPortV1},
        authority::AuthoritySessionRefV1,
    };

    fn private_directory() -> PathBuf {
        let directory = PathBuf::from("/private/tmp").join(format!(
            "vellum-local-auth-{}-{}",
            std::process::id(),
            random_suffix()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        directory
    }

    fn random_suffix() -> u64 {
        let mut bytes = [0_u8; 8];
        getrandom::fill(&mut bytes).unwrap();
        u64::from_be_bytes(bytes)
    }

    fn assert_same_record(actual: &LocalCredentialRecordV1, expected: &LocalCredentialRecordV1) {
        assert_eq!(actual.generation, expected.generation);
        assert_eq!(actual.subject, expected.subject);
        assert_eq!(actual.secret, expected.secret);
    }

    #[test]
    fn provision_load_authenticate_and_stage_rotation() {
        let directory = private_directory();
        let current_path = directory.join("current");
        let next_path = directory.join("next");
        let mut store = LocalCredentialStoreV1::default();
        let current = store.provision(&current_path).unwrap();
        assert_same_record(&store.load(&current_path).unwrap(), &current);

        let challenge = [4; 16];
        let session = [5; 16];
        let claim = current.claim(challenge, session);
        let request = AuthenticationRequestV1 {
            challenge: AuthenticationChallengeRefV1::new(challenge),
            claim: vellum_host::authentication::AuthenticationClaimRefV1::new(claim),
            session: AuthoritySessionRefV1::new(session),
        };
        let mut verifier = LocalCredentialVerifierV1::new(store.load(&current_path).unwrap());
        assert!(matches!(
            verifier.authenticate(request, &()),
            AuthenticationDecisionV1::Authenticated(_)
        ));

        let next = store.stage_rotation(&current_path, &next_path).unwrap();
        assert_eq!(next.generation(), current.generation() + 1);
        assert_same_record(&store.load(&current_path).unwrap(), &current);
        assert_same_record(&store.load(&next_path).unwrap(), &next);
        assert!(store.stage_rotation(&current_path, &next_path).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn existing_target_is_never_overwritten() {
        let directory = private_directory();
        let path = directory.join("credential");
        let mut store = LocalCredentialStoreV1::default();
        let first = store.provision(&path).unwrap();
        assert!(store.provision(&path).is_err());
        assert_same_record(&store.load(&path).unwrap(), &first);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn filesystem_adversaries_refuse() {
        let directory = private_directory();
        let mut store = LocalCredentialStoreV1::default();
        let valid = directory.join("valid");
        store.provision(&valid).unwrap();

        let symlink_path = directory.join("symlink");
        symlink(&valid, &symlink_path).unwrap();
        assert!(store.load(&symlink_path).is_err());

        let wrong_mode = directory.join("wrong-mode");
        std::fs::copy(&valid, &wrong_mode).unwrap();
        std::fs::set_permissions(&wrong_mode, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(store.load(&wrong_mode).is_err());

        let hard_link = directory.join("hard-link");
        std::fs::hard_link(&valid, &hard_link).unwrap();
        assert!(store.load(&valid).is_err());
        assert!(store.load(&hard_link).is_err());

        let child = directory.join("directory");
        std::fs::create_dir(&child).unwrap();
        assert!(store.load(&child).is_err());

        let wrong_size = directory.join("wrong-size");
        std::fs::write(&wrong_size, [0_u8; 67]).unwrap();
        std::fs::set_permissions(&wrong_size, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(store.load(&wrong_size).is_err());

        let fifo_path = directory.join("fifo");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo_path)
                .status()
                .unwrap()
                .success()
        );
        assert!(store.load(&fifo_path).is_err());

        let socket_path = directory.join("socket");
        let _socket = UnixListener::bind(&socket_path).unwrap();
        let socket_result = store.load(&socket_path);
        assert!(
            matches!(socket_result, Err(LocalAuthenticationError::Refused)),
            "{socket_result:?}"
        );

        let device = File::open("/dev/null").unwrap();
        assert!(matches!(
            validate_file(&device),
            Err(LocalAuthenticationError::Refused)
        ));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn wrong_owner_is_refused_without_requiring_privileged_chown() {
        let directory = private_directory();
        let path = directory.join("credential");
        let mut store = LocalCredentialStoreV1::default();
        store.provision(&path).unwrap();
        let file = File::open(&path).unwrap();
        let other_owner = rustix::process::geteuid().as_raw().wrapping_add(1);
        assert!(matches!(
            validate_file_for_owner(&file, other_owner),
            Err(LocalAuthenticationError::Refused)
        ));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn filesystem_errno_classes_are_fixed_and_content_free() {
        for error in [
            Errno::NOENT,
            Errno::NOTDIR,
            Errno::LOOP,
            Errno::ACCESS,
            Errno::PERM,
            Errno::NAMETOOLONG,
            Errno::INVAL,
            Errno::ISDIR,
            Errno::NXIO,
            Errno::NOTSUP,
            Errno::OPNOTSUPP,
        ] {
            assert_eq!(
                classify_errno(error, FsOperation::Lookup),
                LocalAuthenticationError::Refused
            );
        }
        assert_eq!(
            classify_errno(Errno::EXIST, FsOperation::ExclusiveCreate),
            LocalAuthenticationError::Refused
        );
        for error in [
            Errno::IO,
            Errno::MFILE,
            Errno::NFILE,
            Errno::NOMEM,
            Errno::NOSPC,
            Errno::INTR,
        ] {
            assert_eq!(
                classify_errno(error, FsOperation::Lookup),
                LocalAuthenticationError::Unavailable
            );
        }
    }

    #[test]
    fn public_diagnostics_do_not_project_path_or_credential_material() {
        let directory = private_directory();
        let path = directory.join("credential-secret-name");
        let mut store = LocalCredentialStoreV1::default();
        let record = store.provision(&path).unwrap();
        let encoded = record.encode();
        let secret_hex = encoded[36..]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        for diagnostic in [
            format!("{:?}", LocalAuthenticationError::Refused),
            format!("{:?}", LocalAuthenticationError::Unavailable),
            format!("{:?}", LocalAuthenticationError::OutcomeUnknown),
            format!("{:?}", store.usage()),
            format!("{record:?}"),
        ] {
            assert!(!diagnostic.contains("credential-secret-name"));
            assert!(!diagnostic.contains(&secret_hex));
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn generation_exhaustion_refuses_without_creating_next() {
        let directory = private_directory();
        let current_path = directory.join("current");
        let next_path = directory.join("next");
        let record = LocalCredentialRecordV1 {
            generation: u64::MAX,
            subject: [1; 16],
            secret: [2; 32],
        };
        create(&current_path, &record).unwrap();
        let mut store = LocalCredentialStoreV1::default();
        assert!(matches!(
            store.stage_rotation(&current_path, &next_path),
            Err(LocalAuthenticationError::Refused)
        ));
        assert!(!next_path.exists());
        assert_same_record(&store.load(&current_path).unwrap(), &record);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn parent_must_be_exact_private_directory() {
        let directory = private_directory();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut store = LocalCredentialStoreV1::default();
        assert!(store.provision(&directory.join("credential")).is_err());
        assert!(
            !format!("{:?}", LocalAuthenticationError::Refused)
                .contains(directory.to_string_lossy().as_ref())
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn file_namespace_swap_after_read_refuses() {
        let directory = private_directory();
        let path = directory.join("credential");
        let displaced = directory.join("displaced");
        let mut store = LocalCredentialStoreV1::default();
        store.provision(&path).unwrap();
        let parent = AnchoredParent::open(&directory).unwrap();
        let result = parent.read_record_before_revalidate(OsStr::new("credential"), || {
            std::fs::rename(&path, &displaced).unwrap();
            std::fs::copy(&displaced, &path).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        });
        assert!(matches!(result, Err(LocalAuthenticationError::Refused)));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn parent_namespace_swap_after_read_refuses() {
        let directory = private_directory();
        let path = directory.join("credential");
        let displaced = directory.with_extension("displaced");
        let mut store = LocalCredentialStoreV1::default();
        store.provision(&path).unwrap();
        let parent = AnchoredParent::open(&directory).unwrap();
        let result = parent.read_record_before_revalidate(OsStr::new("credential"), || {
            std::fs::rename(&directory, &displaced).unwrap();
            std::fs::create_dir(&directory).unwrap();
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        });
        assert!(matches!(result, Err(LocalAuthenticationError::Refused)));
        std::fs::remove_dir_all(directory).unwrap();
        std::fs::remove_dir_all(displaced).unwrap();
    }
}
