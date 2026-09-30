use quadqr::{
    compression::{parse_payload_envelope, prepare_payload, CompressionMode},
    decode_matrix, encode_text_with_compression, unwrap_application, EncodeOptions,
};

#[test]
fn auto_compression_matrix_round_trip() {
    let text = "QuadQR compression ".repeat(80);
    let code = encode_text_with_compression(&text, EncodeOptions::default(), CompressionMode::Auto)
        .unwrap();
    let decoded = decode_matrix(&code.matrix).unwrap();
    let app = unwrap_application(decoded).unwrap();
    assert_eq!(app.text.as_deref(), Some(text.as_str()));
}

#[test]
fn qpx1_round_trip() {
    let source = b"abcabcabcabcabcabcabcabcabcabcabcabc";
    let prepared = prepare_payload(source, CompressionMode::Lz(6)).unwrap();
    let envelope = parse_payload_envelope(&prepared.payload).unwrap();
    assert_eq!(envelope.payload, source);
}

#[cfg(feature = "security")]
#[test]
fn raw_key_secure_round_trip() {
    use quadqr::security::{decrypt_secure_payload, encrypt_secure_payload, SecurityMode};
    let mode = SecurityMode::RawKey {
        key: [0x42; 32],
        key_id: None,
        disable_auto_key_id: false,
    };
    let envelope = encrypt_secure_payload(b"secret QuadQR", &mode).unwrap();
    assert_eq!(
        decrypt_secure_payload(&envelope, &mode).unwrap(),
        b"secret QuadQR"
    );
}

#[cfg(feature = "security")]
#[test]
fn signed_envelope_verifies() {
    use ed25519_dalek::SigningKey;
    use quadqr::security::{sign_payload, verify_signed_payload};
    let private = [7u8; 32];
    let signed = sign_payload(
        b"signed QuadQR",
        &private,
        CompressionMode::Auto,
        Some(b"test-key"),
        false,
    )
    .unwrap();
    let public = SigningKey::from_bytes(&private).verifying_key().to_bytes();
    assert!(verify_signed_payload(&signed, Some(&public), false).unwrap());
}
