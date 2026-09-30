//! Native Rust implementation of the QuadQR v6 RGBW/Triangle16 symbology.
//!
//! The wire format follows the JavaScript reference implementation in
//! https://github.com/akanshSirohi/QuadQR.

pub mod benchmark;
pub mod compression;
pub mod error;
pub mod format;
pub mod geometry;
#[cfg(feature = "image")]
pub mod reliability;
pub mod render;
pub mod rs;
#[cfg(feature = "security")]
pub mod security;
#[cfg(feature = "image")]
pub mod vision;

pub use error::{QuadQrError, Result};
pub use format::{
    crc32, decode_matrix, decode_matrix_with_confidence, decode_matrix_with_options,
    decode_matrix_with_soft, decode_uint8_array, encode_bytes, encode_prepared_bytes, encode_text,
    encode_uint8_array, get_version_info, layout_for_version, rotate_matrix, CellEncoding,
    DecodeOptions, DecodeResult, EccLevel, EncodeOptions, QuadQrCode, SoftDecodeOptions,
    VersionInfo, BLACK, BLUE, DEFAULT_ECC_LEVEL, FORMAT_VERSION, GREEN, LEGACY_FORMAT_VERSION, RED,
    WHITE,
};
pub use geometry::{MAX_VERSION, MIN_VERSION};
#[cfg(feature = "image")]
pub use reliability::{
    apply_stress_distortion, assess_scanability, run_image_stress_test, run_perspective_sweep,
    run_reliability_lab, PerspectiveOptions, PerspectiveSweepResult, ReliabilityReport,
    ReliabilitySuite, ScanabilityAssessment, StressDistortion, StressScenarioResult,
};
pub use render::{
    estimate_safe_logo_size, get_print_guidance, render_svg, render_svg_with_logo_href, save_svg,
    save_svg_with_logo_href, LogoOptions, LogoSearchResult, Palette, PrintGuidance, RenderMode,
    RenderOptions, RenderStyle,
};
#[cfg(feature = "image")]
pub use render::{
    find_max_safe_logo_size, render_image, render_image_with_logo, save_png, to_png,
    to_png_with_logo,
};
#[cfg(feature = "image")]
pub use vision::{
    debug_scan_rgba, decode_image_buffer, rectify_detected_code, scan_buffer, scan_dynamic_image,
    scan_file, scan_rgba, FinderPoint, RectifiedCode, ScanDiagnostics, ScanOptions, ScanResult,
};

pub use benchmark::{
    benchmark_codec, benchmark_report, build_capacity_comparison, calculate_capacity_plan,
    compare_capacity, get_standard_qr_byte_capacity, BenchmarkReport, CapacityComparison,
    CapacityPlan, CodecBenchmarkRow, TimingSummary,
};
#[cfg(feature = "compression")]
pub use compression::{compress_brotli, compress_deflate, decompress_brotli, decompress_deflate};
pub use compression::{
    compress_lz, decompress_lz, parse_payload_envelope, prepare_payload, CompressionAlgorithm,
    CompressionMode, PayloadEnvelope, PreparedPayload, PAYLOAD_ENVELOPE_HEADER_BYTES,
    PAYLOAD_ENVELOPE_MAGIC, PAYLOAD_ENVELOPE_VERSION,
};
#[cfg(feature = "security")]
pub use security::{
    bytes_to_hex, decrypt_secure_payload, derive_signing_key_id, encrypt_secure_payload,
    estimate_secure_envelope_overhead, generate_raw_256_key, generate_signing_key_pair,
    generate_signing_keypair, hex_to_bytes, inspect_secure_envelope, normalize_raw_256_key_bytes,
    normalize_raw_256_key_hex, sign_payload, signing_seed_from_pkcs8, verify_signed_payload,
    SecureEnvelopeInfo, SecurityMode, SigningKeyPair, DEFAULT_PBKDF2_ITERATIONS,
    MAX_PBKDF2_ITERATIONS, MIN_PBKDF2_ITERATIONS, SECURE_PAYLOAD_VERSION,
};

#[derive(Debug, Clone)]
pub struct ApplicationDecode {
    pub matrix: DecodeResult,
    pub payload: Vec<u8>,
    pub text: Option<String>,
    pub compression: compression::CompressionAlgorithm,
    pub signed: bool,
    pub signing_key_id: Option<Vec<u8>>,
}

#[cfg(feature = "security")]
#[derive(Debug, Clone)]
pub struct SigningOptions {
    pub private_key_seed: [u8; 32],
    pub key_id: Option<Vec<u8>>,
    pub embed_public_key: bool,
}

#[derive(Debug, Clone)]
pub struct ApplicationEncodeOptions {
    pub format: EncodeOptions,
    pub compression: CompressionMode,
    #[cfg(feature = "security")]
    pub security: Option<security::SecurityMode>,
    #[cfg(feature = "security")]
    pub signing: Option<SigningOptions>,
}

impl Default for ApplicationEncodeOptions {
    fn default() -> Self {
        Self {
            format: EncodeOptions::default(),
            compression: CompressionMode::None,
            #[cfg(feature = "security")]
            security: None,
            #[cfg(feature = "security")]
            signing: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EncodedApplication {
    pub code: QuadQrCode,
    pub source_payload_bytes: usize,
    pub compression: CompressionAlgorithm,
    pub compression_level: Option<u32>,
    pub secure: bool,
    pub signed: bool,
    pub signing_key_id: Option<Vec<u8>>,
}

fn application_fields_from_envelope(
    envelope: compression::PayloadEnvelope,
) -> (
    Vec<u8>,
    compression::CompressionAlgorithm,
    bool,
    Option<Vec<u8>>,
) {
    // Read all metadata before moving the owned payload/key-id fields out of
    // the envelope. This avoids partial-move borrow errors (E0382).
    let signed = envelope.signed();
    let compression = envelope.compression;
    let key_id = if envelope.key_id.is_empty() {
        None
    } else {
        Some(envelope.key_id)
    };
    let payload = envelope.payload;
    (payload, compression, signed, key_id)
}

pub fn encode_application_bytes(
    input: &[u8],
    mut options: ApplicationEncodeOptions,
) -> Result<EncodedApplication> {
    let source_payload_bytes = input.len();

    #[cfg(feature = "security")]
    if let Some(signing) = options.signing.as_ref() {
        let protected = security::sign_payload(
            input,
            &signing.private_key_seed,
            options.compression,
            signing.key_id.as_deref(),
            signing.embed_public_key,
        )?;
        let parsed = parse_payload_envelope(&protected)?;
        let compression = parsed.compression;
        let compression_level = None;
        let signing_key_id = if parsed.key_id.is_empty() {
            None
        } else {
            Some(parsed.key_id.clone())
        };
        options.format.extended = true;
        options.format.signed = true;
        let secure = options.security.is_some();
        let payload = if let Some(security_mode) = options.security.as_ref() {
            options.format.secure = true;
            security::encrypt_secure_payload(&protected, security_mode)?
        } else {
            protected
        };
        let code = encode_prepared_bytes(&payload, &options.format)?;
        return Ok(EncodedApplication {
            code,
            source_payload_bytes,
            compression,
            compression_level,
            secure,
            signed: true,
            signing_key_id,
        });
    }

    let prepared = prepare_payload(input, options.compression)?;
    let compression = prepared.compression;
    let compression_level = prepared.compression_level;
    options.format.extended = prepared.extended;

    #[cfg(feature = "security")]
    let (payload, secure) = if let Some(security_mode) = options.security.as_ref() {
        options.format.secure = true;
        (
            security::encrypt_secure_payload(&prepared.payload, security_mode)?,
            true,
        )
    } else {
        (prepared.payload, false)
    };

    #[cfg(not(feature = "security"))]
    let (payload, secure) = (prepared.payload, false);

    let code = encode_prepared_bytes(&payload, &options.format)?;
    Ok(EncodedApplication {
        code,
        source_payload_bytes,
        compression,
        compression_level,
        secure,
        signed: false,
        signing_key_id: None,
    })
}

pub fn encode_application_text(
    text: &str,
    mut options: ApplicationEncodeOptions,
) -> Result<EncodedApplication> {
    options.format.text = true;
    encode_application_bytes(text.as_bytes(), options)
}

/// Encode normal bytes with optional QuadQR Compression 3.0 metadata.
pub fn encode_bytes_with_compression(
    input: &[u8],
    mut options: EncodeOptions,
    compression: CompressionMode,
) -> Result<QuadQrCode> {
    let prepared = prepare_payload(input, compression)?;
    options.extended = prepared.extended;
    options.text = false;
    encode_prepared_bytes(&prepared.payload, &options)
}

/// Encode UTF-8 text with optional QuadQR Compression 3.0 metadata.
pub fn encode_text_with_compression(
    text: &str,
    mut options: EncodeOptions,
    compression: CompressionMode,
) -> Result<QuadQrCode> {
    let prepared = prepare_payload(text.as_bytes(), compression)?;
    options.extended = prepared.extended;
    options.text = true;
    encode_prepared_bytes(&prepared.payload, &options)
}

/// Unwrap the internal QPX1 compression/signature envelope after matrix decode.
/// Secure payloads must be decrypted first with `decrypt_application`.
pub fn unwrap_application(decoded: DecodeResult) -> Result<ApplicationDecode> {
    if decoded.secure {
        return Err(QuadQrError::Security(
            "secure payload must be decrypted before application unwrapping".into(),
        ));
    }

    let (payload, compression, signed, key_id) = if decoded.extended {
        application_fields_from_envelope(parse_payload_envelope(&decoded.payload)?)
    } else {
        (
            decoded.payload.clone(),
            compression::CompressionAlgorithm::None,
            decoded.signed,
            None,
        )
    };

    let text = if decoded.flags & format::TEXT_FLAG != 0 {
        Some(String::from_utf8_lossy(&payload).into_owned())
    } else {
        None
    };

    Ok(ApplicationDecode {
        matrix: decoded,
        payload,
        text,
        compression,
        signed,
        signing_key_id: key_id,
    })
}

#[cfg(feature = "security")]
pub fn encode_secure_bytes(
    input: &[u8],
    mut options: EncodeOptions,
    compression: CompressionMode,
    security_mode: &security::SecurityMode,
) -> Result<QuadQrCode> {
    let prepared = prepare_payload(input, compression)?;
    let encrypted = security::encrypt_secure_payload(&prepared.payload, security_mode)?;
    options.text = false;
    options.secure = true;
    options.extended = prepared.extended;
    encode_prepared_bytes(&encrypted, &options)
}

#[cfg(feature = "security")]
pub fn encode_secure_text(
    text: &str,
    mut options: EncodeOptions,
    compression: CompressionMode,
    security_mode: &security::SecurityMode,
) -> Result<QuadQrCode> {
    let prepared = prepare_payload(text.as_bytes(), compression)?;
    let encrypted = security::encrypt_secure_payload(&prepared.payload, security_mode)?;
    options.text = true;
    options.secure = true;
    options.extended = prepared.extended;
    encode_prepared_bytes(&encrypted, &options)
}

#[cfg(feature = "security")]
pub fn decrypt_application(
    decoded: DecodeResult,
    security_mode: &security::SecurityMode,
) -> Result<ApplicationDecode> {
    if !decoded.secure {
        return unwrap_application(decoded);
    }

    let plaintext = security::decrypt_secure_payload(&decoded.payload, security_mode)?;
    let (payload, compression, signed, key_id) = if decoded.extended {
        application_fields_from_envelope(parse_payload_envelope(&plaintext)?)
    } else {
        (
            plaintext,
            compression::CompressionAlgorithm::None,
            decoded.signed,
            None,
        )
    };

    let text = if decoded.flags & format::TEXT_FLAG != 0 {
        Some(String::from_utf8_lossy(&payload).into_owned())
    } else {
        None
    };

    Ok(ApplicationDecode {
        matrix: decoded,
        payload,
        text,
        compression,
        signed,
        signing_key_id: key_id,
    })
}

#[cfg(feature = "security")]
pub fn encode_signed_bytes(
    input: &[u8],
    mut options: EncodeOptions,
    compression: CompressionMode,
    private_key: &[u8; 32],
    key_id: Option<&[u8]>,
    embed_public_key: bool,
) -> Result<QuadQrCode> {
    let envelope =
        security::sign_payload(input, private_key, compression, key_id, embed_public_key)?;
    options.text = false;
    options.extended = true;
    options.signed = true;
    encode_prepared_bytes(&envelope, &options)
}

#[cfg(feature = "security")]
pub fn encode_signed_text(
    text: &str,
    mut options: EncodeOptions,
    compression: CompressionMode,
    private_key: &[u8; 32],
    key_id: Option<&[u8]>,
    embed_public_key: bool,
) -> Result<QuadQrCode> {
    let envelope = security::sign_payload(
        text.as_bytes(),
        private_key,
        compression,
        key_id,
        embed_public_key,
    )?;
    options.text = true;
    options.extended = true;
    options.signed = true;
    encode_prepared_bytes(&envelope, &options)
}

#[cfg(feature = "security")]
pub fn encode_secure_signed_bytes(
    input: &[u8],
    mut options: EncodeOptions,
    compression: CompressionMode,
    security_mode: &security::SecurityMode,
    private_key: &[u8; 32],
    key_id: Option<&[u8]>,
    embed_public_key: bool,
) -> Result<QuadQrCode> {
    let protected =
        security::sign_payload(input, private_key, compression, key_id, embed_public_key)?;
    let encrypted = security::encrypt_secure_payload(&protected, security_mode)?;
    options.text = false;
    options.secure = true;
    options.extended = true;
    options.signed = true;
    encode_prepared_bytes(&encrypted, &options)
}

#[cfg(feature = "security")]
pub fn encode_secure_signed_text(
    text: &str,
    mut options: EncodeOptions,
    compression: CompressionMode,
    security_mode: &security::SecurityMode,
    private_key: &[u8; 32],
    key_id: Option<&[u8]>,
    embed_public_key: bool,
) -> Result<QuadQrCode> {
    let protected = security::sign_payload(
        text.as_bytes(),
        private_key,
        compression,
        key_id,
        embed_public_key,
    )?;
    let encrypted = security::encrypt_secure_payload(&protected, security_mode)?;
    options.text = true;
    options.secure = true;
    options.extended = true;
    options.signed = true;
    encode_prepared_bytes(&encrypted, &options)
}

#[cfg(feature = "security")]
pub fn verify_decoded_signature(
    decoded: &DecodeResult,
    trusted_public_key: Option<&[u8; 32]>,
    allow_embedded_key: bool,
) -> Result<bool> {
    if decoded.secure {
        return Err(QuadQrError::Security(
            "decrypt secure content before signature verification".into(),
        ));
    }
    if !decoded.signed || !decoded.extended {
        return Err(QuadQrError::Security("decode result is not signed".into()));
    }
    security::verify_signed_payload(&decoded.payload, trusted_public_key, allow_embedded_key)
}

#[cfg(feature = "security")]
pub fn decrypt_and_verify_application(
    decoded: DecodeResult,
    security_mode: &security::SecurityMode,
    trusted_public_key: Option<&[u8; 32]>,
    allow_embedded_key: bool,
) -> Result<(ApplicationDecode, Option<bool>)> {
    if !decoded.secure {
        let verified = if decoded.signed {
            Some(verify_decoded_signature(
                &decoded,
                trusted_public_key,
                allow_embedded_key,
            )?)
        } else {
            None
        };
        return Ok((unwrap_application(decoded)?, verified));
    }

    let plaintext = security::decrypt_secure_payload(&decoded.payload, security_mode)?;
    let verified = if decoded.signed && decoded.extended {
        Some(security::verify_signed_payload(
            &plaintext,
            trusted_public_key,
            allow_embedded_key,
        )?)
    } else {
        None
    };

    let (payload, compression, signed, key_id) = if decoded.extended {
        application_fields_from_envelope(parse_payload_envelope(&plaintext)?)
    } else {
        (
            plaintext,
            compression::CompressionAlgorithm::None,
            decoded.signed,
            None,
        )
    };

    let text = if decoded.flags & format::TEXT_FLAG != 0 {
        Some(String::from_utf8_lossy(&payload).into_owned())
    } else {
        None
    };

    Ok((
        ApplicationDecode {
            matrix: decoded,
            payload,
            text,
            compression,
            signed,
            signing_key_id: key_id,
        },
        verified,
    ))
}
