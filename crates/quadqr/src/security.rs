use crate::compression::{
    make_envelope_header, parse_payload_envelope, CompressionAlgorithm, CompressionMode,
    PAYLOAD_ENVELOPE_COMPRESSED_FLAG, PAYLOAD_ENVELOPE_EMBEDDED_KEY_FLAG,
    PAYLOAD_ENVELOPE_SIGNED_FLAG,
};
use crate::error::{invalid, QuadQrError, Result};

pub const SECURE_PAYLOAD_VERSION: u8 = 1;
pub const DEFAULT_PBKDF2_ITERATIONS: u32 = 600_000;
pub const MIN_PBKDF2_ITERATIONS: u32 = 100_000;
pub const MAX_PBKDF2_ITERATIONS: u32 = 2_000_000;

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn hex_to_bytes(value: &str) -> Result<Vec<u8>> {
    let clean: String = value
        .trim()
        .trim_start_matches("0x")
        .chars()
        .filter(|ch| !matches!(ch, ' ' | '\t' | '\r' | '\n' | ':' | '-'))
        .collect();
    if !clean.len().is_multiple_of(2) || !clean.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(invalid("expected an even-length hexadecimal byte string"));
    }
    let mut out = Vec::with_capacity(clean.len() / 2);
    for index in (0..clean.len()).step_by(2) {
        let byte = u8::from_str_radix(&clean[index..index + 2], 16)
            .map_err(|_| invalid("invalid hexadecimal byte string"))?;
        out.push(byte);
    }
    Ok(out)
}

pub fn normalize_raw_256_key_bytes(input: &[u8]) -> Result<[u8; 32]> {
    if input.len() != 32 {
        return Err(invalid(
            "raw QuadQR encryption key must be exactly 32 bytes",
        ));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(input);
    Ok(out)
}

pub fn normalize_raw_256_key_hex(input: &str) -> Result<[u8; 32]> {
    normalize_raw_256_key_bytes(&hex_to_bytes(input)?)
}

#[derive(Debug, Clone)]
pub struct SigningKeyPair {
    pub private_key_seed: [u8; 32],
    pub private_key_pkcs8: Vec<u8>,
    pub public_key: [u8; 32],
    pub key_id: Vec<u8>,
}

const SECURE_MAGIC: [u8; 4] = *b"QSEC";
const MODE_PASSWORD: u8 = 1;
const MODE_RAW_KEY: u8 = 2;
const ALGORITHM_AES_256_GCM: u8 = 1;
const KDF_NONE: u8 = 0;
const KDF_PBKDF2_SHA256: u8 = 1;
const FLAG_KEY_ID_UTF8: u8 = 1;
const FLAG_KEY_ID_AUTO: u8 = 1 << 1;
const FIXED_HEADER_BYTES: usize = 24;
const SALT_BYTES: usize = 16;
const NONCE_BYTES: usize = 12;
const TAG_BYTES: usize = 16;
const AUTO_KEY_ID_BYTES: usize = 8;
const MAX_KEY_ID_BYTES: usize = 32;

#[derive(Debug, Clone)]
pub enum SecurityMode {
    Password {
        password: String,
        iterations: u32,
        key_id: Option<Vec<u8>>,
    },
    RawKey {
        key: [u8; 32],
        key_id: Option<Vec<u8>>,
        disable_auto_key_id: bool,
    },
}

impl SecurityMode {
    pub fn password(password: impl Into<String>) -> Self {
        Self::Password {
            password: password.into(),
            iterations: DEFAULT_PBKDF2_ITERATIONS,
            key_id: None,
        }
    }
    pub fn raw_key(key: [u8; 32]) -> Self {
        Self::RawKey {
            key,
            key_id: None,
            disable_auto_key_id: false,
        }
    }
}

pub fn estimate_secure_envelope_overhead(mode: &SecurityMode) -> Result<usize> {
    let key_id_length = match mode {
        SecurityMode::Password { key_id, .. } => {
            let length = key_id.as_ref().map_or(0, Vec::len);
            if length > MAX_KEY_ID_BYTES {
                return Err(invalid(format!(
                    "key ID must be at most {MAX_KEY_ID_BYTES} bytes"
                )));
            }
            length
        }
        SecurityMode::RawKey {
            key_id,
            disable_auto_key_id,
            ..
        } => {
            if let Some(key_id) = key_id {
                if key_id.len() > MAX_KEY_ID_BYTES {
                    return Err(invalid(format!(
                        "key ID must be at most {MAX_KEY_ID_BYTES} bytes"
                    )));
                }
                key_id.len()
            } else if *disable_auto_key_id {
                0
            } else {
                AUTO_KEY_ID_BYTES
            }
        }
    };
    let salt_length = match mode {
        SecurityMode::Password { .. } => SALT_BYTES,
        SecurityMode::RawKey { .. } => 0,
    };
    Ok(FIXED_HEADER_BYTES + key_id_length + salt_length + NONCE_BYTES + TAG_BYTES)
}

#[cfg(feature = "security")]
pub fn generate_raw_256_key() -> [u8; 32] {
    use rand::RngCore;
    let mut key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key);
    key
}

#[derive(Debug, Clone)]
pub struct SecureEnvelopeInfo {
    pub mode: &'static str,
    pub iterations: Option<u32>,
    pub key_id: Vec<u8>,
    pub key_id_utf8: bool,
    pub key_id_auto: bool,
    pub plaintext_bytes: usize,
    pub envelope_bytes: usize,
}

#[derive(Debug, Clone)]
struct ParsedSecureEnvelope {
    mode_id: u8,
    flags: u8,
    salt: Vec<u8>,
    nonce: [u8; NONCE_BYTES],
    iterations: u32,
    plaintext_length: usize,
    key_id: Vec<u8>,
    aad: Vec<u8>,
    ciphertext: Vec<u8>,
    tag: [u8; TAG_BYTES],
}

fn validate_key_id(id: &[u8]) -> Result<()> {
    if id.len() > MAX_KEY_ID_BYTES {
        return Err(invalid(format!(
            "key ID must be at most {MAX_KEY_ID_BYTES} bytes"
        )));
    }
    Ok(())
}

fn parse_secure_envelope(input: &[u8]) -> Result<ParsedSecureEnvelope> {
    if input.len() < FIXED_HEADER_BYTES + NONCE_BYTES + TAG_BYTES {
        return Err(QuadQrError::Security(
            "secure payload envelope is too short".into(),
        ));
    }
    if input[..4] != SECURE_MAGIC {
        return Err(QuadQrError::Security(
            "secure payload magic mismatch".into(),
        ));
    }
    if input[4] != SECURE_PAYLOAD_VERSION {
        return Err(QuadQrError::Security(format!(
            "unsupported secure payload version {}",
            input[4]
        )));
    }
    if input[6] != ALGORITHM_AES_256_GCM {
        return Err(QuadQrError::Security(format!(
            "unsupported secure algorithm id {}",
            input[6]
        )));
    }
    let mode_id = input[5];
    let kdf_id = input[7];
    let flags = input[8];
    let salt_length = input[9] as usize;
    let nonce_length = input[10] as usize;
    let tag_length = input[11] as usize;
    let key_id_length = input[12] as usize;
    let iterations = u32::from_be_bytes([input[16], input[17], input[18], input[19]]);
    let plaintext_length =
        u32::from_be_bytes([input[20], input[21], input[22], input[23]]) as usize;
    if nonce_length != NONCE_BYTES || tag_length != TAG_BYTES || key_id_length > MAX_KEY_ID_BYTES {
        return Err(QuadQrError::Security(
            "unsupported secure payload metadata sizes".into(),
        ));
    }
    match mode_id {
        MODE_PASSWORD => {
            if kdf_id != KDF_PBKDF2_SHA256
                || salt_length != SALT_BYTES
                || !(MIN_PBKDF2_ITERATIONS..=MAX_PBKDF2_ITERATIONS).contains(&iterations)
            {
                return Err(QuadQrError::Security(
                    "invalid password-mode secure metadata".into(),
                ));
            }
        }
        MODE_RAW_KEY => {
            if kdf_id != KDF_NONE || salt_length != 0 || iterations != 0 {
                return Err(QuadQrError::Security(
                    "invalid raw-key secure metadata".into(),
                ));
            }
        }
        _ => {
            return Err(QuadQrError::Security(format!(
                "unsupported secure mode id {mode_id}"
            )))
        }
    }
    let metadata_length = FIXED_HEADER_BYTES + key_id_length + salt_length + nonce_length;
    let expected_length = metadata_length + plaintext_length + tag_length;
    if input.len() != expected_length {
        return Err(QuadQrError::Security(format!(
            "secure payload length mismatch: expected {expected_length}, got {}",
            input.len()
        )));
    }
    let mut cursor = FIXED_HEADER_BYTES;
    let key_id = input[cursor..cursor + key_id_length].to_vec();
    cursor += key_id_length;
    let salt = input[cursor..cursor + salt_length].to_vec();
    cursor += salt_length;
    let mut nonce = [0u8; NONCE_BYTES];
    nonce.copy_from_slice(&input[cursor..cursor + NONCE_BYTES]);
    cursor += NONCE_BYTES;
    let aad = input[..cursor].to_vec();
    let ciphertext = input[cursor..cursor + plaintext_length].to_vec();
    cursor += plaintext_length;
    let mut tag = [0u8; TAG_BYTES];
    tag.copy_from_slice(&input[cursor..cursor + TAG_BYTES]);
    Ok(ParsedSecureEnvelope {
        mode_id,
        flags,
        salt,
        nonce,
        iterations,
        plaintext_length,
        key_id,
        aad,
        ciphertext,
        tag,
    })
}

pub fn inspect_secure_envelope(input: &[u8]) -> Result<SecureEnvelopeInfo> {
    let parsed = parse_secure_envelope(input)?;
    Ok(SecureEnvelopeInfo {
        mode: if parsed.mode_id == MODE_PASSWORD {
            "password"
        } else {
            "raw-key"
        },
        iterations: if parsed.mode_id == MODE_PASSWORD {
            Some(parsed.iterations)
        } else {
            None
        },
        key_id: parsed.key_id,
        key_id_utf8: parsed.flags & FLAG_KEY_ID_UTF8 != 0,
        key_id_auto: parsed.flags & FLAG_KEY_ID_AUTO != 0,
        plaintext_bytes: parsed.plaintext_length,
        envelope_bytes: input.len(),
    })
}

#[cfg(feature = "security")]
fn derive_password_key(password: &str, salt: &[u8], iterations: u32) -> Result<[u8; 32]> {
    if password.is_empty() {
        return Err(invalid("a non-empty password is required"));
    }
    if !(MIN_PBKDF2_ITERATIONS..=MAX_PBKDF2_ITERATIONS).contains(&iterations) {
        return Err(invalid("PBKDF2 iteration count is outside supported range"));
    }
    use pbkdf2::pbkdf2_hmac;
    use sha2::Sha256;
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut key);
    Ok(key)
}

#[cfg(feature = "security")]
pub fn encrypt_secure_payload(input: &[u8], mode: &SecurityMode) -> Result<Vec<u8>> {
    use aes_gcm::aead::{AeadInPlace, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce};
    use rand::RngCore;
    use sha2::{Digest, Sha256};

    if input.len() > u32::MAX as usize {
        return Err(invalid("secure payload too large"));
    }
    let mut rng = rand::rngs::OsRng;
    let mut nonce = [0u8; NONCE_BYTES];
    rng.fill_bytes(&mut nonce);

    let (mode_id, kdf_id, iterations, salt, key, key_id, flags) = match mode {
        SecurityMode::Password {
            password,
            iterations,
            key_id,
        } => {
            let iterations = *iterations;
            let mut salt = vec![0u8; SALT_BYTES];
            rng.fill_bytes(&mut salt);
            let key = derive_password_key(password, &salt, iterations)?;
            let id = key_id.clone().unwrap_or_default();
            validate_key_id(&id)?;
            let flags = if key_id.is_some() && std::str::from_utf8(&id).is_ok() {
                FLAG_KEY_ID_UTF8
            } else {
                0
            };
            (
                MODE_PASSWORD,
                KDF_PBKDF2_SHA256,
                iterations,
                salt,
                key,
                id,
                flags,
            )
        }
        SecurityMode::RawKey {
            key,
            key_id,
            disable_auto_key_id,
        } => {
            let (id, flags) = if let Some(id) = key_id {
                validate_key_id(id)?;
                (
                    id.clone(),
                    if std::str::from_utf8(id).is_ok() {
                        FLAG_KEY_ID_UTF8
                    } else {
                        0
                    },
                )
            } else if *disable_auto_key_id {
                (Vec::new(), 0)
            } else {
                let digest = Sha256::digest(key);
                (digest[..AUTO_KEY_ID_BYTES].to_vec(), FLAG_KEY_ID_AUTO)
            };
            (MODE_RAW_KEY, KDF_NONE, 0, Vec::new(), *key, id, flags)
        }
    };

    let mut fixed = [0u8; FIXED_HEADER_BYTES];
    fixed[..4].copy_from_slice(&SECURE_MAGIC);
    fixed[4] = SECURE_PAYLOAD_VERSION;
    fixed[5] = mode_id;
    fixed[6] = ALGORITHM_AES_256_GCM;
    fixed[7] = kdf_id;
    fixed[8] = flags;
    fixed[9] = salt.len() as u8;
    fixed[10] = NONCE_BYTES as u8;
    fixed[11] = TAG_BYTES as u8;
    fixed[12] = key_id.len() as u8;
    fixed[16..20].copy_from_slice(&iterations.to_be_bytes());
    fixed[20..24].copy_from_slice(&(input.len() as u32).to_be_bytes());
    let mut aad = fixed.to_vec();
    aad.extend_from_slice(&key_id);
    aad.extend_from_slice(&salt);
    aad.extend_from_slice(&nonce);

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| QuadQrError::Security("invalid AES key".into()))?;
    let mut ciphertext = input.to_vec();
    let tag = cipher
        .encrypt_in_place_detached(Nonce::from_slice(&nonce), &aad, &mut ciphertext)
        .map_err(|_| QuadQrError::Security("AES-GCM encryption failed".into()))?;
    let mut envelope = aad;
    envelope.extend_from_slice(&ciphertext);
    envelope.extend_from_slice(tag.as_slice());
    Ok(envelope)
}

#[cfg(feature = "security")]
pub fn decrypt_secure_payload(input: &[u8], mode: &SecurityMode) -> Result<Vec<u8>> {
    use aes_gcm::aead::{AeadInPlace, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce, Tag};
    use sha2::{Digest, Sha256};

    let parsed = parse_secure_envelope(input)?;
    let key = match (parsed.mode_id, mode) {
        (MODE_PASSWORD, SecurityMode::Password { password, .. }) => {
            derive_password_key(password, &parsed.salt, parsed.iterations)?
        }
        (MODE_RAW_KEY, SecurityMode::RawKey { key, .. }) => {
            if parsed.flags & FLAG_KEY_ID_AUTO != 0 && parsed.key_id.len() == AUTO_KEY_ID_BYTES {
                let digest = Sha256::digest(key);
                if parsed.key_id != digest[..AUTO_KEY_ID_BYTES] {
                    return Err(QuadQrError::Security(
                        "raw-key fingerprint does not match".into(),
                    ));
                }
            }
            *key
        }
        _ => {
            return Err(QuadQrError::Security(
                "security mode does not match envelope".into(),
            ))
        }
    };
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| QuadQrError::Security("invalid AES key".into()))?;
    let mut plaintext = parsed.ciphertext;
    cipher
        .decrypt_in_place_detached(
            Nonce::from_slice(&parsed.nonce),
            &parsed.aad,
            &mut plaintext,
            Tag::from_slice(&parsed.tag),
        )
        .map_err(|_| QuadQrError::Security("AES-GCM authentication failed".into()))?;
    if plaintext.len() != parsed.plaintext_length {
        return Err(QuadQrError::Security(
            "decrypted payload length mismatch".into(),
        ));
    }
    Ok(plaintext)
}

#[cfg(feature = "security")]
pub fn signing_key_id(public_key: &[u8; 32], bytes: usize) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(public_key);
    digest[..bytes.clamp(4, 16)].to_vec()
}

#[cfg(feature = "security")]
pub fn sign_payload(
    input: &[u8],
    private_key: &[u8; 32],
    compression: CompressionMode,
    key_id: Option<&[u8]>,
    embed_public_key: bool,
) -> Result<Vec<u8>> {
    use ed25519_dalek::{Signer, SigningKey};
    let prepared = crate::compression::prepare_payload(input, compression)?;
    let (algorithm, stored) = if prepared.extended {
        let parsed = parse_payload_envelope(&prepared.payload)?;
        (parsed.compression, parsed.stored_payload)
    } else {
        (CompressionAlgorithm::None, input.to_vec())
    };
    let signing = SigningKey::from_bytes(private_key);
    let verifying = signing.verifying_key();
    let public_key = if embed_public_key {
        verifying.to_bytes().to_vec()
    } else {
        Vec::new()
    };
    let key_id = key_id.map(ToOwned::to_owned).unwrap_or_default();
    if key_id.len() > 64 {
        return Err(invalid("signing key ID must be 64 bytes or fewer"));
    }
    let flags = PAYLOAD_ENVELOPE_SIGNED_FLAG
        | if algorithm != CompressionAlgorithm::None {
            PAYLOAD_ENVELOPE_COMPRESSED_FLAG
        } else {
            0
        }
        | if embed_public_key {
            PAYLOAD_ENVELOPE_EMBEDDED_KEY_FLAG
        } else {
            0
        };
    let header = make_envelope_header(
        algorithm,
        flags,
        input.len(),
        key_id.len(),
        public_key.len(),
        64,
        1,
    )?;
    let mut signed_bytes = header.to_vec();
    signed_bytes.extend_from_slice(&key_id);
    signed_bytes.extend_from_slice(&public_key);
    signed_bytes.extend_from_slice(&stored);
    let signature = signing.sign(&signed_bytes).to_bytes();
    let mut envelope = header.to_vec();
    envelope.extend_from_slice(&key_id);
    envelope.extend_from_slice(&public_key);
    envelope.extend_from_slice(&signature);
    envelope.extend_from_slice(&stored);
    Ok(envelope)
}

#[cfg(feature = "security")]
pub fn verify_signed_payload(
    input: &[u8],
    trusted_public_key: Option<&[u8; 32]>,
    allow_embedded_key: bool,
) -> Result<bool> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let parsed = parse_payload_envelope(input)?;
    if !parsed.signed() || parsed.signature_algorithm != 1 || parsed.signature.len() != 64 {
        return Err(QuadQrError::Security(
            "payload is not a supported Ed25519 signed envelope".into(),
        ));
    }
    let key_bytes = if let Some(key) = trusted_public_key {
        *key
    } else if (allow_embedded_key || parsed.version == 1) && parsed.public_key.len() == 32 {
        let mut key = [0u8; 32];
        key.copy_from_slice(&parsed.public_key);
        key
    } else {
        return Err(QuadQrError::Security(
            "a trusted Ed25519 public key is required".into(),
        ));
    };
    let key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| QuadQrError::Security("invalid Ed25519 public key".into()))?;
    let signature = Signature::from_slice(&parsed.signature)
        .map_err(|_| QuadQrError::Security("invalid Ed25519 signature".into()))?;
    let mut signed_bytes = parsed.header.to_vec();
    signed_bytes.extend_from_slice(&parsed.key_id);
    signed_bytes.extend_from_slice(&parsed.public_key);
    signed_bytes.extend_from_slice(&parsed.stored_payload);
    Ok(key.verify(&signed_bytes, &signature).is_ok())
}

#[cfg(feature = "security")]
pub fn derive_signing_key_id(public_key: &[u8; 32], bytes: usize) -> Vec<u8> {
    signing_key_id(public_key, bytes)
}

#[cfg(feature = "security")]
pub fn generate_signing_key_pair() -> Result<SigningKeyPair> {
    use ed25519_dalek::pkcs8::EncodePrivateKey;
    use ed25519_dalek::SigningKey;
    let signing = SigningKey::generate(&mut rand::rngs::OsRng);
    let private_key_seed = signing.to_bytes();
    let public_key = signing.verifying_key().to_bytes();
    let private_key_pkcs8 = signing
        .to_pkcs8_der()
        .map_err(|error| {
            QuadQrError::Security(format!("could not encode Ed25519 PKCS#8 key: {error}"))
        })?
        .as_bytes()
        .to_vec();
    let key_id = signing_key_id(&public_key, 8);
    Ok(SigningKeyPair {
        private_key_seed,
        private_key_pkcs8,
        public_key,
        key_id,
    })
}

#[cfg(feature = "security")]
pub fn generate_signing_keypair() -> ([u8; 32], [u8; 32], Vec<u8>) {
    use ed25519_dalek::SigningKey;
    let signing = SigningKey::generate(&mut rand::rngs::OsRng);
    let private = signing.to_bytes();
    let public = signing.verifying_key().to_bytes();
    let key_id = signing_key_id(&public, 8);
    (private, public, key_id)
}

#[cfg(feature = "security")]
pub fn signing_seed_from_pkcs8(pkcs8: &[u8]) -> Result<[u8; 32]> {
    use ed25519_dalek::pkcs8::DecodePrivateKey;
    use ed25519_dalek::SigningKey;
    let key = SigningKey::from_pkcs8_der(pkcs8).map_err(|error| {
        QuadQrError::Security(format!("invalid Ed25519 PKCS#8 private key: {error}"))
    })?;
    Ok(key.to_bytes())
}
