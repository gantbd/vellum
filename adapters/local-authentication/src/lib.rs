//! The single-subject local authentication profile.
//!
//! Authentication establishes attribution only. It grants no Vellum action.
//! Credential paths and credential material are deliberately absent from all
//! errors, debug output and observations.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::fmt;

pub const CREDENTIAL_RECORD_BYTES_V1: usize = 68;
const MAGIC: &[u8; 8] = b"VLCRED1\0";
const FORMAT_V1: u16 = 1;
const ALGORITHM_HMAC_SHA256_LEFT_128: u16 = 1;
const HANDSHAKE_VERSION_V1: u8 = 1;
const DOMAIN: &[u8] = b"vellum.local-auth-proof.v1\0";

/// Fixed-cardinality, content-free refusal classes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalAuthenticationError {
    Refused,
    Unavailable,
    OutcomeUnknown,
}

/// Exact credential record v1. Debug output never projects record fields.
pub struct LocalCredentialRecordV1 {
    generation: u64,
    subject: [u8; 16],
    secret: [u8; 32],
}

impl fmt::Debug for LocalCredentialRecordV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LocalCredentialRecordV1(<redacted>)")
    }
}

impl LocalCredentialRecordV1 {
    pub fn decode(bytes: &[u8]) -> Result<Self, LocalAuthenticationError> {
        if bytes.len() != CREDENTIAL_RECORD_BYTES_V1
            || &bytes[0..8] != MAGIC
            || u16::from_be_bytes([bytes[8], bytes[9]]) != FORMAT_V1
            || u16::from_be_bytes([bytes[10], bytes[11]]) != ALGORITHM_HMAC_SHA256_LEFT_128
        {
            return Err(LocalAuthenticationError::Refused);
        }
        let generation = u64::from_be_bytes(
            bytes[12..20]
                .try_into()
                .map_err(|_| LocalAuthenticationError::Refused)?,
        );
        let subject: [u8; 16] = bytes[20..36]
            .try_into()
            .map_err(|_| LocalAuthenticationError::Refused)?;
        let secret: [u8; 32] = bytes[36..68]
            .try_into()
            .map_err(|_| LocalAuthenticationError::Refused)?;
        if generation == 0 || subject == [0; 16] || secret == [0; 32] {
            return Err(LocalAuthenticationError::Refused);
        }
        Ok(Self {
            generation,
            subject,
            secret,
        })
    }

    pub(crate) fn encode(&self) -> [u8; CREDENTIAL_RECORD_BYTES_V1] {
        let mut bytes = [0_u8; CREDENTIAL_RECORD_BYTES_V1];
        bytes[0..8].copy_from_slice(MAGIC);
        bytes[8..10].copy_from_slice(&FORMAT_V1.to_be_bytes());
        bytes[10..12].copy_from_slice(&ALGORITHM_HMAC_SHA256_LEFT_128.to_be_bytes());
        bytes[12..20].copy_from_slice(&self.generation.to_be_bytes());
        bytes[20..36].copy_from_slice(&self.subject);
        bytes[36..68].copy_from_slice(&self.secret);
        bytes
    }

    /// Monotone generation metadata, never authority.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    fn full_claim(&self, challenge: [u8; 16], session: [u8; 16]) -> [u8; 32] {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.secret)
            .expect("HMAC-SHA-256 accepts a 32-byte key");
        mac.update(DOMAIN);
        mac.update(&FORMAT_V1.to_be_bytes());
        mac.update(&ALGORITHM_HMAC_SHA256_LEFT_128.to_be_bytes());
        mac.update(&self.generation.to_be_bytes());
        mac.update(&self.subject);
        mac.update(&[HANDSHAKE_VERSION_V1]);
        mac.update(&challenge);
        mac.update(&session);
        mac.finalize().into_bytes().into()
    }

    fn claim(&self, challenge: [u8; 16], session: [u8; 16]) -> [u8; 16] {
        self.full_claim(challenge, session)[..16]
            .try_into()
            .expect("fixed HMAC prefix")
    }

    fn verify(&self, challenge: [u8; 16], session: [u8; 16], claim: [u8; 16]) -> bool {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.secret)
            .expect("HMAC-SHA-256 accepts a 32-byte key");
        mac.update(DOMAIN);
        mac.update(&FORMAT_V1.to_be_bytes());
        mac.update(&ALGORITHM_HMAC_SHA256_LEFT_128.to_be_bytes());
        mac.update(&self.generation.to_be_bytes());
        mac.update(&self.subject);
        mac.update(&[HANDSHAKE_VERSION_V1]);
        mac.update(&challenge);
        mac.update(&session);
        mac.verify_truncated_left(&claim).is_ok()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LocalAuthenticationUsageV1 {
    pub credential_loads_completed: u64,
    pub credential_loads_refused: u64,
    pub provisions_completed: u64,
    pub provisions_refused: u64,
    pub rotations_staged: u64,
    pub rotations_refused: u64,
    pub randomness_failures: u64,
    pub proofs_completed: u64,
    pub proofs_refused: u64,
    pub counter_saturated: bool,
}

fn bump(value: &mut u64, saturated: &mut bool) {
    let previous = *value;
    *value = value.saturating_add(1);
    *saturated |= previous == u64::MAX;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector_record() -> LocalCredentialRecordV1 {
        let mut secret = [0; 32];
        for (index, byte) in secret.iter_mut().enumerate() {
            *byte = u8::try_from(index).unwrap();
        }
        let mut subject = [0; 16];
        for (index, byte) in subject.iter_mut().enumerate() {
            *byte = 0x10 + u8::try_from(index).unwrap();
        }
        LocalCredentialRecordV1 {
            generation: 1,
            subject,
            secret,
        }
    }

    #[test]
    fn exact_record_codec_and_all_lengths() {
        let record = vector_record();
        let bytes = record.encode();
        let decoded = LocalCredentialRecordV1::decode(&bytes).unwrap();
        assert_eq!(decoded.generation, record.generation);
        assert_eq!(decoded.subject, record.subject);
        assert_eq!(decoded.secret, record.secret);
        for length in 0..CREDENTIAL_RECORD_BYTES_V1 {
            assert!(matches!(
                LocalCredentialRecordV1::decode(&bytes[..length]),
                Err(LocalAuthenticationError::Refused)
            ));
        }
        let mut extended = bytes.to_vec();
        extended.push(0);
        assert!(matches!(
            LocalCredentialRecordV1::decode(&extended),
            Err(LocalAuthenticationError::Refused)
        ));
    }

    #[test]
    fn rejects_header_and_zero_field_mutations() {
        let original = vector_record().encode();
        for index in 0..12 {
            let mut bytes = original;
            bytes[index] ^= 0x80;
            assert!(LocalCredentialRecordV1::decode(&bytes).is_err());
        }
        for range in [12..20, 20..36, 36..68] {
            let mut bytes = original;
            bytes[range].fill(0);
            assert!(LocalCredentialRecordV1::decode(&bytes).is_err());
        }
    }

    #[test]
    fn normative_golden_vector_and_each_claim_byte() {
        let record = vector_record();
        let challenge = core::array::from_fn(|index| 0x20 + u8::try_from(index).unwrap());
        let session = core::array::from_fn(|index| 0x30 + u8::try_from(index).unwrap());
        let expected_full = [
            0xaf, 0x8e, 0x46, 0x15, 0x06, 0xb5, 0xd5, 0x4e, 0xae, 0x3e, 0xdb, 0x7a, 0x32, 0x07,
            0xf4, 0x2b, 0x44, 0x24, 0x9c, 0x65, 0x06, 0xf7, 0xbc, 0xdc, 0x27, 0x8e, 0x8f, 0x40,
            0xd1, 0xab, 0x97, 0xc1,
        ];
        let expected: [u8; 16] = expected_full[..16].try_into().unwrap();
        assert_eq!(record.full_claim(challenge, session), expected_full);
        assert_eq!(record.claim(challenge, session), expected);
        assert!(record.verify(challenge, session, expected));
        for index in 0..16 {
            let mut changed = expected;
            changed[index] ^= 1;
            assert!(!record.verify(challenge, session, changed));
        }
        assert!(!record.verify([0; 16], session, expected));
        assert!(!record.verify(challenge, [0; 16], expected));
    }

    #[test]
    fn every_credential_transcript_field_is_bound() {
        let record = vector_record();
        let challenge = [0x41; 16];
        let session = [0x52; 16];
        let claim = record.claim(challenge, session);

        let mut generation = vector_record();
        generation.generation += 1;
        assert!(!generation.verify(challenge, session, claim));

        let mut subject = vector_record();
        subject.subject[7] ^= 1;
        assert!(!subject.verify(challenge, session, claim));

        let mut secret = vector_record();
        secret.secret[19] ^= 1;
        assert!(!secret.verify(challenge, session, claim));

        for index in 0..16 {
            let mut changed_challenge = challenge;
            changed_challenge[index] ^= 1;
            assert!(!record.verify(changed_challenge, session, claim));
            let mut changed_session = session;
            changed_session[index] ^= 1;
            assert!(!record.verify(challenge, changed_session, claim));
        }
    }

    #[test]
    fn rustcrypto_hmac_matches_published_rfc_4231_case_one() {
        let mut mac = Hmac::<Sha256>::new_from_slice(&[0x0b; 20]).unwrap();
        mac.update(b"Hi There");
        assert_eq!(
            mac.finalize().into_bytes().as_slice(),
            &[
                0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf, 0x0b,
                0xf1, 0x2b, 0x88, 0x1d, 0xc2, 0x00, 0xc9, 0x83, 0x3d, 0xa7, 0x26, 0xe9, 0x37, 0x6c,
                0x2e, 0x32, 0xcf, 0xf7,
            ]
        );
    }

    #[test]
    fn debug_is_redacted() {
        assert_eq!(
            format!("{:?}", vector_record()),
            "LocalCredentialRecordV1(<redacted>)"
        );
    }

    #[test]
    fn counters_saturate_without_wrapping() {
        let mut value = u64::MAX;
        let mut saturated = false;
        bump(&mut value, &mut saturated);
        assert_eq!(value, u64::MAX);
        assert!(saturated);
    }
}
