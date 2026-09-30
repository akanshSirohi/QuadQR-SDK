use quadqr::{
    calculate_capacity_plan, crc32, decode_matrix, encode_application_text, encode_text,
    get_version_info, rotate_matrix, ApplicationEncodeOptions, CellEncoding, CompressionMode,
    EccLevel, EncodeOptions,
};

#[test]
fn core_wrapper_api_round_trip() {
    let mut options = ApplicationEncodeOptions::default();
    options.compression = CompressionMode::Auto;
    options.format.ecc = EccLevel::M;
    let encoded = encode_application_text("Rust core wrapper API", options).unwrap();
    let decoded = decode_matrix(&encoded.code.matrix).unwrap();
    assert!(decoded.extended || decoded.text.as_deref() == Some("Rust core wrapper API"));
    assert_eq!(encoded.source_payload_bytes, "Rust core wrapper API".len());
}

#[test]
fn version_info_and_rotation_are_public() {
    let info = get_version_info(2, EccLevel::M, CellEncoding::Rgbw).unwrap();
    assert_eq!(info.size, 25);
    let code = encode_text("rotate", EncodeOptions::default()).unwrap();
    let rotated = rotate_matrix(&code.matrix, 1).unwrap();
    let decoded = decode_matrix(&rotated).unwrap();
    assert_eq!(decoded.text.as_deref(), Some("rotate"));
}

#[test]
fn capacity_plan_is_available_to_bindings() {
    let plan = calculate_capacity_plan(
        b"capacity plan payload",
        EccLevel::M,
        CellEncoding::Rgbw,
        CompressionMode::Auto,
    )
    .unwrap();
    assert!(plan.quadqr_version.is_some());
    assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
}

#[cfg(feature = "image")]
#[test]
fn png_bytes_can_be_scanned_without_file_io() {
    use quadqr::{scan_buffer, to_png, RenderOptions, ScanOptions};
    let code = encode_text("PNG byte API", EncodeOptions::default()).unwrap();
    let png = to_png(&code, RenderOptions::default()).unwrap();
    let scan = scan_buffer(&png, &ScanOptions::default()).unwrap();
    assert_eq!(scan.decoded.text.as_deref(), Some("PNG byte API"));
}

#[cfg(feature = "security")]
#[test]
fn high_level_secure_and_signed_api_is_available() {
    use quadqr::{generate_signing_key_pair, SecurityMode, SigningOptions};
    let pair = generate_signing_key_pair().unwrap();
    let mut options = ApplicationEncodeOptions::default();
    options.compression = CompressionMode::Auto;
    options.security = Some(SecurityMode::raw_key([0x44; 32]));
    options.signing = Some(SigningOptions {
        private_key_seed: pair.private_key_seed,
        key_id: Some(pair.key_id),
        embed_public_key: false,
    });
    let encoded = encode_application_text("secure signed core", options).unwrap();
    assert!(encoded.secure);
    assert!(encoded.signed);
}
