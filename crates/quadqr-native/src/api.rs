use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use quadqr::{
    ApplicationEncodeOptions, CellEncoding, CompressionAlgorithm, CompressionMode, DecodeOptions,
    DecodeResult, EccLevel, EncodeOptions, EncodedApplication, LogoOptions, Palette,
    PerspectiveOptions, ReliabilityReport, ReliabilitySuite, RenderMode, RenderOptions,
    RenderStyle, ScanOptions, SecurityMode, SoftDecodeOptions, StressDistortion,
};
use serde_json::{json, Map, Value};
use std::io::Cursor;

fn parse_json(input: &str, label: &str) -> Result<Value, String> {
    if input.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(input).map_err(|e| format!("invalid {label} JSON: {e}"))
}

fn parse_options(input: &str) -> Result<Value, String> {
    let value = parse_json(input, "options")?;
    if !value.is_object() {
        return Err("options JSON must be an object".to_owned());
    }
    Ok(value)
}

fn object(value: &Value) -> Option<&Map<String, Value>> {
    value.as_object()
}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    object(value)?.get(key)
}

fn bool_field(value: &Value, key: &str) -> Option<bool> {
    field(value, key)?.as_bool()
}

fn u64_field(value: &Value, key: &str) -> Option<u64> {
    field(value, key)?.as_u64()
}

fn f64_field(value: &Value, key: &str) -> Option<f64> {
    field(value, key)?.as_f64()
}

fn str_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    field(value, key)?.as_str()
}

fn unwrap_result(value: &Value) -> &Value {
    if bool_field(value, "ok") == Some(true) {
        if let Some(result) = field(value, "result") {
            return result;
        }
    }
    value
}

fn bytes_from_value(value: &Value) -> Result<Vec<u8>, String> {
    if let Some(text) = value.as_str() {
        return quadqr::hex_to_bytes(text).map_err(|e| e.to_string());
    }
    let Some(items) = value.as_array() else {
        return Err("expected a byte array or hexadecimal string".to_owned());
    };
    items
        .iter()
        .map(|item| {
            item.as_u64()
                .filter(|value| *value <= 255)
                .map(|value| value as u8)
                .ok_or_else(|| "byte array contains a value outside 0..255".to_owned())
        })
        .collect()
}

fn bytes_from_base64_field(value: &Value, key: &str) -> Result<Option<Vec<u8>>, String> {
    let Some(encoded) = str_field(value, key) else {
        return Ok(None);
    };
    BASE64
        .decode(encoded)
        .map(Some)
        .map_err(|e| format!("{key} is not valid base64: {e}"))
}

fn optional_bytes(value: &Value, key: &str) -> Result<Option<Vec<u8>>, String> {
    let Some(item) = field(value, key) else {
        return Ok(None);
    };
    if item.is_null() {
        return Ok(None);
    }
    if let Some(text) = item.as_str() {
        return Ok(Some(text.as_bytes().to_vec()));
    }
    Ok(Some(bytes_from_value(item)?))
}

fn byte_array_32(value: &Value, label: &str) -> Result<[u8; 32], String> {
    let bytes = bytes_from_value(value)?;
    bytes
        .try_into()
        .map_err(|_| format!("{label} must contain exactly 32 bytes"))
}

fn ecc_from_value(value: &Value, key: &str, default: EccLevel) -> Result<EccLevel, String> {
    let default_label = ecc_label(default);
    match str_field(value, key)
        .unwrap_or(default_label)
        .to_ascii_uppercase()
        .as_str()
    {
        "L" => Ok(EccLevel::L),
        "M" => Ok(EccLevel::M),
        "Q" => Ok(EccLevel::Q),
        "H" => Ok(EccLevel::H),
        _ => Err("ECC level must be one of L, M, Q, H".to_owned()),
    }
}

fn ecc_from_options(options: &Value) -> Result<EccLevel, String> {
    ecc_from_value(options, "ecc", EccLevel::M)
}

fn ecc_label(ecc: EccLevel) -> &'static str {
    match ecc {
        EccLevel::L => "L",
        EccLevel::M => "M",
        EccLevel::Q => "Q",
        EccLevel::H => "H",
    }
}

fn encoding_from_options(options: &Value) -> CellEncoding {
    if let Some(high_density) = bool_field(options, "highDensity") {
        return if high_density {
            CellEncoding::Triangle16
        } else {
            CellEncoding::Rgbw
        };
    }
    match str_field(options, "cellEncoding")
        .unwrap_or("rgbw")
        .to_ascii_lowercase()
        .as_str()
    {
        "triangle16" | "high-density" | "high_density" => CellEncoding::Triangle16,
        _ => CellEncoding::Rgbw,
    }
}

fn encoding_from_result(value: &Value) -> CellEncoding {
    if bool_field(value, "highDensity") == Some(true) {
        return CellEncoding::Triangle16;
    }
    match str_field(value, "cellEncoding").unwrap_or("rgbw") {
        "triangle16" => CellEncoding::Triangle16,
        _ => CellEncoding::Rgbw,
    }
}

fn encoding_label(encoding: CellEncoding) -> &'static str {
    match encoding {
        CellEncoding::Rgbw => "rgbw",
        CellEncoding::Triangle16 => "triangle16",
    }
}

fn algorithm_label(algorithm: CompressionAlgorithm) -> &'static str {
    match algorithm {
        CompressionAlgorithm::None => "none",
        CompressionAlgorithm::Lz => "lz",
        CompressionAlgorithm::Deflate => "deflate",
        CompressionAlgorithm::Brotli => "brotli",
    }
}

fn compression_from_options(options: &Value) -> Result<CompressionMode, String> {
    let mode = str_field(options, "compression")
        .unwrap_or("none")
        .to_ascii_lowercase();
    let generic_level = u64_field(options, "compressionLevel").map(|value| value as u32);
    match mode.as_str() {
        "none" => Ok(CompressionMode::None),
        "auto" => Ok(CompressionMode::Auto),
        "smart" => Ok(CompressionMode::Smart),
        "lz" => {
            let level = generic_level
                .or_else(|| u64_field(options, "lzLevel").map(|value| value as u32))
                .unwrap_or(6);
            if !(1..=9).contains(&level) {
                return Err("LZ compressionLevel must be 1..9".to_owned());
            }
            Ok(CompressionMode::Lz(level as u8))
        }
        "deflate" => {
            let level = generic_level
                .or_else(|| u64_field(options, "deflateLevel").map(|value| value as u32))
                .unwrap_or(6);
            if !(1..=9).contains(&level) {
                return Err("DEFLATE compressionLevel must be 1..9".to_owned());
            }
            Ok(CompressionMode::Deflate(level))
        }
        "brotli" => {
            let level = generic_level
                .or_else(|| u64_field(options, "brotliQuality").map(|value| value as u32))
                .unwrap_or(11);
            if level > 11 {
                return Err("Brotli compressionLevel must be 0..11".to_owned());
            }
            Ok(CompressionMode::Brotli(level))
        }
        _ => Err("compression must be none, auto, smart, lz, deflate, or brotli".to_owned()),
    }
}

fn encode_options(options: &Value, text: bool) -> Result<EncodeOptions, String> {
    let mut format = EncodeOptions {
        ecc: ecc_from_options(options)?,
        high_density: encoding_from_options(options) == CellEncoding::Triangle16,
        text,
        min_version: u64_field(options, "minVersion").unwrap_or(1).clamp(1, 40) as u8,
        max_version: u64_field(options, "maxVersion").unwrap_or(40).clamp(1, 40) as u8,
        ..Default::default()
    };
    if format.min_version > format.max_version {
        return Err("minVersion cannot be greater than maxVersion".to_owned());
    }
    if let Some(version) = field(options, "version") {
        if let Some(number) = version.as_u64() {
            if !(1..=40).contains(&number) {
                return Err("Version must be 1..40 or auto".to_owned());
            }
            format.version = Some(number as u8);
        } else if let Some(text) = version.as_str() {
            if text != "auto" {
                return Err("Version must be 1..40 or auto".to_owned());
            }
        }
    }
    Ok(format)
}

fn security_mode_from_value(value: &Value) -> Result<Option<SecurityMode>, String> {
    if value.is_null() {
        return Ok(None);
    }
    let mode = str_field(value, "mode").unwrap_or("none");
    let key_id = optional_bytes(value, "keyId")?;
    match mode {
        "none" => Ok(None),
        "password" => {
            let password = str_field(value, "password")
                .ok_or_else(|| "security.password is required".to_owned())?;
            let iterations = u64_field(value, "iterations")
                .unwrap_or(quadqr::DEFAULT_PBKDF2_ITERATIONS as u64)
                as u32;
            Ok(Some(SecurityMode::Password {
                password: password.to_owned(),
                iterations,
                key_id,
            }))
        }
        "raw-key" => {
            let key_value =
                field(value, "key").ok_or_else(|| "security.key is required".to_owned())?;
            let key = if let Some(hex) = key_value.as_str() {
                quadqr::normalize_raw_256_key_hex(hex).map_err(|e| e.to_string())?
            } else {
                quadqr::normalize_raw_256_key_bytes(&bytes_from_value(key_value)?)
                    .map_err(|e| e.to_string())?
            };
            Ok(Some(SecurityMode::RawKey {
                key,
                key_id,
                disable_auto_key_id: bool_field(value, "disableAutoKeyId").unwrap_or(false),
            }))
        }
        _ => Err("security.mode must be none, password, or raw-key".to_owned()),
    }
}

fn security_from_options(options: &Value) -> Result<Option<SecurityMode>, String> {
    let Some(security) = field(options, "security") else {
        return Ok(None);
    };
    security_mode_from_value(security)
}

fn signing_from_options(options: &Value) -> Result<Option<quadqr::SigningOptions>, String> {
    let Some(signing) = field(options, "signing") else {
        return Ok(None);
    };
    if signing.is_null() {
        return Ok(None);
    }
    let seed_value = field(signing, "privateKeySeed")
        .or_else(|| field(signing, "privateKey"))
        .ok_or_else(|| "signing.privateKeySeed is required".to_owned())?;
    let private_key_seed = byte_array_32(seed_value, "Ed25519 private key seed")?;
    Ok(Some(quadqr::SigningOptions {
        private_key_seed,
        key_id: optional_bytes(signing, "keyId")?,
        embed_public_key: bool_field(signing, "embedPublicKey").unwrap_or(false),
    }))
}

fn application_options(options: &Value, text: bool) -> Result<ApplicationEncodeOptions, String> {
    Ok(ApplicationEncodeOptions {
        format: encode_options(options, text)?,
        compression: compression_from_options(options)?,
        security: security_from_options(options)?,
        signing: signing_from_options(options)?,
    })
}

fn signing_key_id_string(bytes: Option<&[u8]>) -> Option<String> {
    bytes.map(|value| {
        String::from_utf8(value.to_vec()).unwrap_or_else(|_| quadqr::bytes_to_hex(value))
    })
}

fn encoded_value(
    encoded: EncodedApplication,
    security: Option<&SecurityMode>,
) -> Result<Value, String> {
    let code = encoded.code;
    let info = quadqr::get_version_info(code.version, code.ecc_level, code.cell_encoding)
        .map_err(|e| e.to_string())?;
    let security_value = match security {
        Some(SecurityMode::Password { .. }) => json!({ "mode": "password" }),
        Some(SecurityMode::RawKey { .. }) => json!({ "mode": "raw-key" }),
        None => Value::Null,
    };
    Ok(json!({
        "formatVersion": code.format_version,
        "version": code.version,
        "size": code.size,
        "matrix": code.matrix,
        "maskId": code.mask_id,
        "payloadBytes": code.payload_bytes,
        "sourcePayloadBytes": encoded.source_payload_bytes,
        "meaningfulCells": code.meaningful_cells,
        "dataCells": code.data_cells,
        "capacityBytes": code.capacity_bytes,
        "utilization": code.utilization,
        "highDensity": code.high_density,
        "cellEncoding": encoding_label(code.cell_encoding),
        "bitsPerDataCell": if code.high_density { 4 } else { 2 },
        "eccLevel": ecc_label(code.ecc_level),
        "eccParitySymbols": code.ecc_parity_symbols,
        "eccBlocks": code.ecc_blocks,
        "correctableSymbolsPerBlock": code.correctable_symbols_per_block,
        "alignmentPatterns": info.alignment_patterns,
        "spectralInterleaving": true,
        "crc32": code.crc32,
        "compressed": encoded.compression != CompressionAlgorithm::None,
        "compression": algorithm_label(encoded.compression),
        "compressionLevel": encoded.compression_level,
        "secure": encoded.secure,
        "security": security_value,
        "signed": encoded.signed,
        "signingKeyId": signing_key_id_string(encoded.signing_key_id.as_deref()),
    }))
}

fn secure_info_value(payload: &[u8]) -> Value {
    match quadqr::inspect_secure_envelope(payload) {
        Ok(info) => json!({
            "mode": info.mode,
            "iterations": info.iterations,
            "keyId": if info.key_id_utf8 { Some(String::from_utf8_lossy(&info.key_id).into_owned()) } else { None },
            "keyIdHex": quadqr::bytes_to_hex(&info.key_id),
            "keyIdAuto": info.key_id_auto,
            "plaintextBytes": info.plaintext_bytes,
            "envelopeBytes": info.envelope_bytes,
            "securePayloadVersion": quadqr::SECURE_PAYLOAD_VERSION,
            "algorithm": "AES-256-GCM",
        }),
        Err(_) => Value::Null,
    }
}

fn decode_base_value(decoded: &DecodeResult) -> Result<Value, String> {
    let info = quadqr::get_version_info(decoded.version, decoded.ecc_level, decoded.cell_encoding)
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "formatVersion": decoded.format_version,
        "version": decoded.version,
        "size": decoded.size,
        "maskId": decoded.mask_id,
        "rotation": decoded.rotation,
        "flags": decoded.flags,
        "highDensity": decoded.high_density,
        "cellEncoding": encoding_label(decoded.cell_encoding),
        "bitsPerDataCell": if decoded.high_density { 4 } else { 2 },
        "eccLevel": ecc_label(decoded.ecc_level),
        "eccParitySymbols": decoded.ecc_parity_symbols,
        "eccBlocks": decoded.ecc_blocks,
        "correctedHeaderSymbols": decoded.corrected_header_symbols,
        "correctedBodySymbols": decoded.corrected_body_symbols,
        "correctedSymbols": decoded.corrected_header_symbols + decoded.corrected_body_symbols,
        "erasureSymbols": decoded.erasure_symbols,
        "spectralInterleaving": decoded.spectral_interleaving,
        "spectrumEccVersion": decoded.spectrum_ecc_version,
        "softDecoded": decoded.soft_decoded,
        "softSubstitutions": decoded.soft_substitutions,
        "softDecodeAttempts": decoded.soft_decode_attempts,
        "softDecodeCellsConsidered": decoded.soft_decode_cells_considered,
        "secure": decoded.secure,
        "extended": decoded.extended,
        "signed": decoded.signed,
        "crc32": decoded.crc32,
        "alignmentPatterns": info.alignment_patterns,
    }))
}

fn binary_value(data: &[u8], media_type: Option<&str>) -> Value {
    json!({
        "dataBase64": BASE64.encode(data),
        "byteLength": data.len(),
        "mediaType": media_type,
    })
}

fn decoded_value(decoded: DecodeResult) -> Result<Value, String> {
    let mut base = decode_base_value(&decoded)?;
    let target = base.as_object_mut().expect("decode object");
    if decoded.secure {
        let payload_base64 = BASE64.encode(&decoded.payload);
        let security_value = secure_info_value(&decoded.payload);
        target.insert("payloadBase64".into(), json!(payload_base64));
        target.insert("payload".into(), json!(decoded.payload));
        target.insert("text".into(), Value::Null);
        target.insert("requiresDecryption".into(), json!(true));
        target.insert("decrypted".into(), json!(false));
        target.insert("compressed".into(), json!(false));
        target.insert("compression".into(), json!("none"));
        target.insert("protectedPayload".into(), Value::Null);
        target.insert("security".into(), security_value);
        return Ok(base);
    }

    let original_protected = decoded.payload.clone();
    let original_protected_base64 = BASE64.encode(&original_protected);
    let app = quadqr::unwrap_application(decoded).map_err(|e| e.to_string())?;
    let payload_base64 = BASE64.encode(&app.payload);
    target.insert("payloadBase64".into(), json!(payload_base64));
    target.insert("payload".into(), json!(app.payload));
    target.insert("text".into(), app.text.map_or(Value::Null, Value::String));
    target.insert("requiresDecryption".into(), json!(false));
    target.insert("decrypted".into(), json!(false));
    target.insert(
        "compressed".into(),
        json!(app.compression != CompressionAlgorithm::None),
    );
    target.insert(
        "compression".into(),
        json!(algorithm_label(app.compression)),
    );
    target.insert("signed".into(), json!(app.signed));
    target.insert(
        "signingKeyId".into(),
        signing_key_id_string(app.signing_key_id.as_deref()).map_or(Value::Null, Value::String),
    );
    target.insert(
        "protectedPayload".into(),
        if app.signed {
            json!(original_protected)
        } else {
            Value::Null
        },
    );
    target.insert(
        "protectedPayloadBase64".into(),
        if app.signed {
            json!(original_protected_base64)
        } else {
            Value::Null
        },
    );
    target.insert("security".into(), Value::Null);
    Ok(base)
}

fn finder_value(point: &quadqr::FinderPoint) -> Value {
    json!({
        "x": point.x,
        "y": point.y,
        "moduleSize": point.module_size,
        "score": point.score,
    })
}

fn scan_value(scan: quadqr::ScanResult) -> Result<Value, String> {
    let mut value = decoded_value(scan.decoded)?;
    let target = value.as_object_mut().expect("scan object");
    let finders: Vec<Value> = scan.finders.iter().map(finder_value).collect();
    let geometry = if scan.finders.len() >= 3 {
        json!({
            "version": scan.diagnostics.selected_version,
            "finders": {
                "topLeft": finder_value(&scan.finders[0]),
                "topRight": finder_value(&scan.finders[1]),
                "bottomLeft": finder_value(&scan.finders[2]),
            },
            "score": 1.0,
            "alignmentGridScore": 1.0,
            "alignmentScore": 1.0,
        })
    } else {
        Value::Null
    };
    let perspective = scan.diagnostics.path.starts_with("perspective");
    target.insert("perspectiveCorrected".into(), json!(perspective));
    target.insert("colorCalibrated".into(), json!(true));
    target.insert("confidence".into(), json!(1.0));
    target.insert("geometry".into(), geometry.clone());
    target.insert("finders".into(), json!(finders));
    target.insert("scanPath".into(), json!(scan.diagnostics.path));
    target.insert(
        "diagnostics".into(),
        json!({
            "elapsedMs": scan.diagnostics.elapsed_ms,
            "finderCandidates": scan.diagnostics.finder_candidates,
            "geometryAttempts": scan.diagnostics.geometry_attempts,
            "selectedVersion": scan.diagnostics.selected_version,
            "path": scan.diagnostics.path,
            "confidence": 1.0,
            "geometry": geometry,
            "vision": {
                "passes": [{
                    "finderCount": scan.finders.len(),
                    "finders": finders,
                    "finderMethod": "rust-native-locator"
                }]
            }
        }),
    );
    Ok(value)
}

fn scan_options(options: &Value) -> Result<ScanOptions, String> {
    let mut scan = ScanOptions::default();
    scan.min_version = u64_field(options, "minVersion")
        .unwrap_or(scan.min_version as u64)
        .clamp(1, 40) as u8;
    scan.max_version = u64_field(options, "maxVersion")
        .unwrap_or(scan.max_version as u64)
        .clamp(1, 40) as u8;
    if scan.min_version > scan.max_version {
        return Err("minVersion cannot be greater than maxVersion".to_owned());
    }
    scan.structural_tolerance = f64_field(options, "structuralTolerance")
        .or_else(|| f64_field(options, "structureTolerance"))
        .unwrap_or(scan.structural_tolerance);
    scan.axis_aligned_fallback =
        bool_field(options, "axisAlignedFallback").unwrap_or(scan.axis_aligned_fallback);
    scan.max_finder_candidates = u64_field(options, "maxFinderCandidates")
        .unwrap_or(scan.max_finder_candidates as u64) as usize;
    scan.locator_max_dimension = u64_field(options, "locatorMaxDimension")
        .unwrap_or(scan.locator_max_dimension as u64) as u32;
    scan.working_max_dimension = u64_field(options, "workingMaxDimension")
        .unwrap_or(scan.working_max_dimension as u64) as u32;
    scan.finder_recovery = bool_field(options, "finderRecovery").unwrap_or(scan.finder_recovery);
    scan.rotation_recovery =
        bool_field(options, "rotationRecovery").unwrap_or(scan.rotation_recovery);
    scan.sample_radius = f64_field(options, "sampleRadius").unwrap_or(scan.sample_radius);
    scan.high_density_sample_radius =
        f64_field(options, "highDensitySampleRadius").unwrap_or(scan.high_density_sample_radius);
    scan.max_erasure_confidence = f64_field(options, "maxErasureConfidence")
        .unwrap_or(scan.max_erasure_confidence as f64) as f32;
    scan.soft_decoding = bool_field(options, "softDecoding").unwrap_or(scan.soft_decoding);
    scan.soft_decode = SoftDecodeOptions {
        confidence_threshold: f64_field(options, "softConfidenceThreshold")
            .unwrap_or(scan.soft_decode.confidence_threshold as f64)
            as f32,
        max_cells: u64_field(options, "softMaxCells").unwrap_or(scan.soft_decode.max_cells as u64)
            as usize,
        pair_cells: u64_field(options, "softPairCells")
            .unwrap_or(scan.soft_decode.pair_cells as u64) as usize,
    };
    Ok(scan)
}

fn decode_options(options: &Value) -> DecodeOptions {
    let defaults = DecodeOptions::default();
    DecodeOptions {
        structure_tolerance: f64_field(options, "structuralTolerance")
            .or_else(|| f64_field(options, "structureTolerance"))
            .unwrap_or(defaults.structure_tolerance),
        max_erasure_confidence: f64_field(options, "maxErasureConfidence")
            .unwrap_or(defaults.max_erasure_confidence as f64)
            as f32,
        soft_decoding: bool_field(options, "softDecoding").unwrap_or(defaults.soft_decoding),
        soft: SoftDecodeOptions {
            confidence_threshold: f64_field(options, "softConfidenceThreshold")
                .unwrap_or(defaults.soft.confidence_threshold as f64)
                as f32,
            max_cells: u64_field(options, "softMaxCells").unwrap_or(defaults.soft.max_cells as u64)
                as usize,
            pair_cells: u64_field(options, "softPairCells")
                .unwrap_or(defaults.soft.pair_cells as u64) as usize,
        },
    }
}

fn parse_matrix(value: &Value) -> Result<Vec<Vec<i16>>, String> {
    let value = unwrap_result(value);
    let matrix = field(value, "matrix").unwrap_or(value);
    let rows = matrix
        .as_array()
        .ok_or_else(|| "matrix must be an array of rows".to_owned())?;
    if rows.is_empty() {
        return Err("matrix cannot be empty".to_owned());
    }
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let cells = row
            .as_array()
            .ok_or_else(|| "matrix rows must be arrays".to_owned())?;
        let parsed = cells
            .iter()
            .map(|cell| {
                cell.as_i64()
                    .and_then(|n| i16::try_from(n).ok())
                    .ok_or_else(|| "matrix contains a non-integer or out-of-range cell".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        out.push(parsed);
    }
    if out.iter().any(|row| row.len() != out.len()) {
        return Err("matrix must be square".to_owned());
    }
    Ok(out)
}

fn parse_f32_matrix(value: &Value, key: &str) -> Result<Option<Vec<Vec<f32>>>, String> {
    let Some(matrix) = field(value, key) else {
        return Ok(None);
    };
    if matrix.is_null() {
        return Ok(None);
    }
    let rows = matrix
        .as_array()
        .ok_or_else(|| format!("{key} must be an array of rows"))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let cells = row
            .as_array()
            .ok_or_else(|| format!("{key} rows must be arrays"))?;
        out.push(
            cells
                .iter()
                .map(|cell| {
                    cell.as_f64()
                        .map(|v| v as f32)
                        .ok_or_else(|| format!("{key} contains a non-number"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    Ok(Some(out))
}

fn parse_i16_matrix_field(value: &Value, key: &str) -> Result<Option<Vec<Vec<i16>>>, String> {
    let Some(matrix) = field(value, key) else {
        return Ok(None);
    };
    if matrix.is_null() {
        return Ok(None);
    }
    parse_matrix(matrix).map(Some)
}

fn quad_code_from_json(input: &str) -> Result<quadqr::QuadQrCode, String> {
    let value = parse_json(input, "code")?;
    let root = unwrap_result(&value);
    let matrix = parse_matrix(root)?;
    let decoded = quadqr::decode_matrix(&matrix).map_err(|e| e.to_string())?;
    let info = quadqr::get_version_info(decoded.version, decoded.ecc_level, decoded.cell_encoding)
        .map_err(|e| e.to_string())?;
    let payload_bytes =
        u64_field(root, "payloadBytes").unwrap_or(decoded.payload.len() as u64) as usize;
    let capacity_bytes =
        u64_field(root, "capacityBytes").unwrap_or(info.capacity_bytes as u64) as usize;
    let utilization = f64_field(root, "utilization").unwrap_or_else(|| {
        if capacity_bytes == 0 {
            0.0
        } else {
            payload_bytes as f64 / capacity_bytes as f64
        }
    });
    Ok(quadqr::QuadQrCode {
        format_version: decoded.format_version,
        version: decoded.version,
        size: matrix.len(),
        matrix,
        mask_id: decoded.mask_id,
        payload_bytes,
        meaningful_cells: u64_field(root, "meaningfulCells").unwrap_or(info.data_cells as u64)
            as usize,
        data_cells: u64_field(root, "dataCells").unwrap_or(info.data_cells as u64) as usize,
        capacity_bytes,
        utilization,
        high_density: decoded.high_density,
        cell_encoding: decoded.cell_encoding,
        ecc_level: decoded.ecc_level,
        ecc_parity_symbols: decoded.ecc_parity_symbols,
        ecc_blocks: decoded.ecc_blocks,
        correctable_symbols_per_block: info.correctable_symbols_per_block,
        crc32: decoded.crc32,
    })
}

fn decode_result_from_json(input: &str) -> Result<DecodeResult, String> {
    let value = parse_json(input, "decoded result")?;
    let root = unwrap_result(&value);
    let secure = bool_field(root, "secure").unwrap_or(false);
    let signed = bool_field(root, "signed").unwrap_or(false);
    let payload = if signed && !secure {
        if let Some(protected) = field(root, "protectedPayload").filter(|v| !v.is_null()) {
            bytes_from_value(protected)?
        } else if let Some(protected) = bytes_from_base64_field(root, "protectedPayloadBase64")? {
            protected
        } else if let Some(payload) = field(root, "payload") {
            bytes_from_value(payload)?
        } else if let Some(payload) = bytes_from_base64_field(root, "payloadBase64")? {
            payload
        } else {
            return Err("decoded result must contain payload/protectedPayload data".to_owned());
        }
    } else if let Some(payload) = field(root, "payload") {
        bytes_from_value(payload)?
    } else if let Some(payload) = bytes_from_base64_field(root, "payloadBase64")? {
        payload
    } else {
        return Err("decoded result must contain payload or payloadBase64".to_owned());
    };
    let version = u64_field(root, "version").unwrap_or(1).clamp(1, 40) as u8;
    let ecc = ecc_from_value(root, "eccLevel", EccLevel::M)?;
    let encoding = encoding_from_result(root);
    let info = quadqr::get_version_info(version, ecc, encoding).map_err(|e| e.to_string())?;
    Ok(DecodeResult {
        format_version: u64_field(root, "formatVersion").unwrap_or(quadqr::FORMAT_VERSION as u64)
            as u8,
        version,
        size: u64_field(root, "size").unwrap_or(info.size as u64) as usize,
        mask_id: u64_field(root, "maskId").unwrap_or(0) as u8,
        rotation: u64_field(root, "rotation").unwrap_or(0) as u8,
        flags: u64_field(root, "flags").unwrap_or(0) as u8,
        high_density: encoding == CellEncoding::Triangle16,
        cell_encoding: encoding,
        ecc_level: ecc,
        ecc_parity_symbols: u64_field(root, "eccParitySymbols")
            .unwrap_or(info.body_parity_symbols as u64) as usize,
        ecc_blocks: u64_field(root, "eccBlocks").unwrap_or(1) as usize,
        corrected_header_symbols: u64_field(root, "correctedHeaderSymbols").unwrap_or(0) as usize,
        corrected_body_symbols: u64_field(root, "correctedBodySymbols").unwrap_or(0) as usize,
        erasure_symbols: u64_field(root, "erasureSymbols").unwrap_or(0) as usize,
        spectral_interleaving: bool_field(root, "spectralInterleaving").unwrap_or(true),
        spectrum_ecc_version: u64_field(root, "spectrumEccVersion").unwrap_or(2) as u8,
        soft_decoded: bool_field(root, "softDecoded").unwrap_or(false),
        soft_substitutions: u64_field(root, "softSubstitutions").unwrap_or(0) as usize,
        soft_decode_attempts: u64_field(root, "softDecodeAttempts").unwrap_or(0) as usize,
        soft_decode_cells_considered: u64_field(root, "softDecodeCellsConsidered").unwrap_or(0)
            as usize,
        payload,
        text: str_field(root, "text").map(str::to_owned),
        secure,
        extended: bool_field(root, "extended").unwrap_or(false),
        signed,
        crc32: u64_field(root, "crc32").unwrap_or(0) as u32,
    })
}

fn rgb_from_value(value: &Value) -> Result<[u8; 3], String> {
    if let Some(text) = value.as_str() {
        let clean = text.trim().trim_start_matches('#');
        if clean.len() != 6 {
            return Err("palette color must be #RRGGBB or [r,g,b]".to_owned());
        }
        let bytes = (0..3)
            .map(|i| u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "invalid hexadecimal palette color".to_owned())?;
        return Ok([bytes[0], bytes[1], bytes[2]]);
    }
    let bytes = bytes_from_value(value)?;
    if bytes.len() != 3 {
        return Err("palette color array must contain exactly 3 bytes".to_owned());
    }
    Ok([bytes[0], bytes[1], bytes[2]])
}

fn render_mode(options: &Value) -> RenderMode {
    match str_field(options, "mode").unwrap_or("screen") {
        "print" => RenderMode::Print,
        _ => RenderMode::Screen,
    }
}

fn render_style(options: &Value) -> Result<RenderStyle, String> {
    match str_field(options, "style").unwrap_or("classic") {
        "classic" => Ok(RenderStyle::Classic),
        "depth" => Ok(RenderStyle::Depth),
        "soft" => Ok(RenderStyle::Soft),
        "inset" => Ok(RenderStyle::Inset),
        _ => Err("render style must be classic, depth, soft, or inset".to_owned()),
    }
}

fn palette_from_options(options: &Value, mode: RenderMode) -> Result<Option<Palette>, String> {
    let Some(palette_value) = field(options, "palette") else {
        return Ok(None);
    };
    if palette_value.is_null() {
        return Ok(None);
    }
    let mut palette = if mode == RenderMode::Print {
        Palette::print()
    } else {
        Palette::screen()
    };
    for (key, target) in [
        ("black", &mut palette.black),
        ("white", &mut palette.white),
        ("red", &mut palette.red),
        ("green", &mut palette.green),
        ("blue", &mut palette.blue),
    ] {
        if let Some(value) = field(palette_value, key) {
            *target = rgb_from_value(value)?;
        }
    }
    Ok(Some(palette))
}

fn render_options(options: &Value, code_size: usize) -> Result<RenderOptions, String> {
    let mode = render_mode(options);
    let quiet_zone = u64_field(options, "quietZone").unwrap_or(4) as u32;
    let mut module_size = u64_field(options, "moduleSize").unwrap_or(8) as u32;
    if let Some(image_size) = u64_field(options, "imageSize") {
        let modules = code_size as u64 + quiet_zone as u64 * 2;
        if let Some(size) = image_size.checked_div(modules) {
            module_size = size.max(1) as u32;
        }
    }
    if module_size == 0 {
        return Err("moduleSize must be positive".to_owned());
    }
    Ok(RenderOptions {
        module_size,
        quiet_zone,
        palette: palette_from_options(options, mode)?,
        mode,
        style: render_style(options)?,
        allow_styled_print: bool_field(options, "allowStyledPrint").unwrap_or(false),
    })
}

fn logo_options(options: &Value) -> Result<LogoOptions, String> {
    let logo = field(options, "logo").unwrap_or(options);
    let size_value = field(logo, "size").or_else(|| field(options, "logoSize"));
    let auto_size = size_value
        .and_then(Value::as_str)
        .map(|value| value == "auto")
        .unwrap_or(false);
    let size_ratio = size_value
        .and_then(Value::as_f64)
        .or_else(|| f64_field(logo, "sizeRatio"))
        .unwrap_or(0.18) as f32;
    let background_color = field(logo, "backgroundColor")
        .or_else(|| field(options, "logoBackgroundColor"))
        .map(rgb_from_value)
        .transpose()?;
    Ok(LogoOptions {
        size_ratio,
        auto_size,
        padding_modules: f64_field(logo, "padding")
            .or_else(|| f64_field(options, "logoPadding"))
            .unwrap_or(0.65) as f32,
        radius_modules: f64_field(logo, "radius")
            .or_else(|| f64_field(options, "logoRadius"))
            .unwrap_or(0.8) as f32,
        clear_background: bool_field(logo, "clearBackground")
            .or_else(|| bool_field(options, "logoClearBackground"))
            .unwrap_or(false),
        background_color,
    })
}

fn rgba_image(data: &[u8], width: u32, height: u32) -> Result<image::RgbaImage, String> {
    if width == 0 || height == 0 {
        return Err("image width and height must be positive".to_owned());
    }
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|value| value.checked_mul(4))
        .ok_or_else(|| "RGBA dimensions overflow native address space".to_owned())?;
    if data.len() != expected {
        return Err(format!(
            "RGBA byte length mismatch: expected {expected}, received {}",
            data.len()
        ));
    }
    image::RgbaImage::from_raw(width, height, data.to_vec())
        .ok_or_else(|| "failed to construct RGBA image".to_owned())
}

fn working_dimensions(width: u32, height: u32, max_dimension: u32) -> (u32, u32, f64) {
    let max_input = width.max(height);
    if max_dimension == 0 || max_input <= max_dimension {
        return (width, height, 1.0);
    }
    let scale = max_dimension as f64 / max_input as f64;
    (
        (width as f64 * scale).round().max(1.0) as u32,
        (height as f64 * scale).round().max(1.0) as u32,
        scale,
    )
}

fn rgba_scan_image(
    data: &[u8],
    width: u32,
    height: u32,
    working_max_dimension: u32,
) -> Result<(image::RgbaImage, f64), String> {
    if width == 0 || height == 0 {
        return Err("image width and height must be positive".to_owned());
    }
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|value| value.checked_mul(4))
        .ok_or_else(|| "RGBA dimensions overflow native address space".to_owned())?;
    if data.len() != expected {
        return Err(format!(
            "RGBA byte length mismatch: expected {expected}, received {}",
            data.len()
        ));
    }

    let (out_width, out_height, scale) = working_dimensions(width, height, working_max_dimension);
    if scale >= 0.999 {
        return Ok((rgba_image(data, width, height)?, 1.0));
    }

    // Build only the bounded working image. This avoids first cloning the full
    // 1080p/4K RGBA camera frame into a Rust Vec and then allocating a second
    // image for the resize.
    let mut out = image::RgbaImage::new(out_width, out_height);
    let inv_scale = 1.0 / scale;
    let max_x = width.saturating_sub(1) as f64;
    let max_y = height.saturating_sub(1) as f64;
    for y in 0..out_height {
        let sy = ((y as f64 + 0.5) * inv_scale - 0.5).clamp(0.0, max_y);
        let y0 = sy.floor() as u32;
        let fy = sy - y0 as f64;
        for x in 0..out_width {
            let sx = ((x as f64 + 0.5) * inv_scale - 0.5).clamp(0.0, max_x);
            let x0 = sx.floor() as u32;
            let x1 = (x0 + 1).min(width.saturating_sub(1));
            let fx = sx - x0 as f64;
            let y1 = (y0 + 1).min(height.saturating_sub(1));
            let idx = |px: u32, py: u32| (py as usize * width as usize + px as usize) * 4;
            let i00 = idx(x0, y0);
            let i10 = idx(x1, y0);
            let i01 = idx(x0, y1);
            let i11 = idx(x1, y1);
            let mut rgba = [0u8; 4];
            for c in 0..4 {
                let a = data[i00 + c] as f64 * (1.0 - fx) + data[i10 + c] as f64 * fx;
                let b = data[i01 + c] as f64 * (1.0 - fx) + data[i11 + c] as f64 * fx;
                rgba[c] = (a * (1.0 - fy) + b * fy).round().clamp(0.0, 255.0) as u8;
            }
            out.put_pixel(x, y, image::Rgba(rgba));
        }
    }
    Ok((out, scale))
}

fn required_plane_len(
    width: u32,
    height: u32,
    row_stride: usize,
    pixel_stride: usize,
) -> Result<usize, String> {
    if width == 0 || height == 0 {
        return Ok(0);
    }
    if row_stride == 0 || pixel_stride == 0 {
        return Err("YUV row/pixel strides must be positive".to_owned());
    }
    let row_offset = (height as usize - 1)
        .checked_mul(row_stride)
        .ok_or_else(|| "YUV plane dimensions overflow native address space".to_owned())?;
    let column_offset = (width as usize - 1)
        .checked_mul(pixel_stride)
        .ok_or_else(|| "YUV plane dimensions overflow native address space".to_owned())?;
    row_offset
        .checked_add(column_offset)
        .and_then(|v| v.checked_add(1))
        .ok_or_else(|| "YUV plane dimensions overflow native address space".to_owned())
}

#[allow(clippy::too_many_arguments)]
fn yuv420_scan_image(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: u32,
    height: u32,
    y_row_stride: usize,
    y_pixel_stride: usize,
    u_row_stride: usize,
    u_pixel_stride: usize,
    v_row_stride: usize,
    v_pixel_stride: usize,
    working_max_dimension: u32,
) -> Result<(image::RgbaImage, f64), String> {
    if width == 0 || height == 0 {
        return Err("image width and height must be positive".to_owned());
    }
    let chroma_width = width.div_ceil(2);
    let chroma_height = height.div_ceil(2);
    let y_required = required_plane_len(width, height, y_row_stride, y_pixel_stride)?;
    let u_required = required_plane_len(chroma_width, chroma_height, u_row_stride, u_pixel_stride)?;
    let v_required = required_plane_len(chroma_width, chroma_height, v_row_stride, v_pixel_stride)?;
    if y_plane.len() < y_required {
        return Err(format!(
            "Y plane is too small: need {y_required}, received {}",
            y_plane.len()
        ));
    }
    if u_plane.len() < u_required {
        return Err(format!(
            "U plane is too small: need {u_required}, received {}",
            u_plane.len()
        ));
    }
    if v_plane.len() < v_required {
        return Err(format!(
            "V plane is too small: need {v_required}, received {}",
            v_plane.len()
        ));
    }

    let (out_width, out_height, scale) = working_dimensions(width, height, working_max_dimension);
    let inv_scale = 1.0 / scale;
    let mut out = image::RgbaImage::new(out_width, out_height);

    for y in 0..out_height {
        let src_y = (((y as f64 + 0.5) * inv_scale - 0.5).round() as i64)
            .clamp(0, height.saturating_sub(1) as i64) as usize;
        let uv_y = src_y >> 1;
        for x in 0..out_width {
            let src_x = (((x as f64 + 0.5) * inv_scale - 0.5).round() as i64)
                .clamp(0, width.saturating_sub(1) as i64) as usize;
            let uv_x = src_x >> 1;
            let yv = y_plane[src_y * y_row_stride + src_x * y_pixel_stride] as i32;
            let u = u_plane[uv_y * u_row_stride + uv_x * u_pixel_stride] as i32 - 128;
            let v = v_plane[uv_y * v_row_stride + uv_x * v_pixel_stride] as i32 - 128;
            let c = (yv - 16).max(0);
            let r = ((298 * c + 409 * v + 128) >> 8).clamp(0, 255) as u8;
            let g = ((298 * c - 100 * u - 208 * v + 128) >> 8).clamp(0, 255) as u8;
            let b = ((298 * c + 516 * u + 128) >> 8).clamp(0, 255) as u8;
            out.put_pixel(x, y, image::Rgba([r, g, b, 255]));
        }
    }
    Ok((out, scale))
}

fn restore_finder_scale(scan: &mut quadqr::ScanResult, scale: f64) {
    if scale >= 0.999 {
        return;
    }
    let inverse = 1.0 / scale;
    for finder in &mut scan.finders {
        finder.x *= inverse;
        finder.y *= inverse;
        finder.module_size *= inverse;
    }
}

fn image_to_png(image: image::RgbaImage) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(cursor.into_inner())
}

fn version_info_value(info: quadqr::VersionInfo) -> Value {
    json!({
        "version": info.version,
        "formatVersion": info.format_version,
        "eccLevel": ecc_label(info.ecc_level),
        "size": info.size,
        "dataCells": info.data_cells,
        "theoreticalBits": info.theoretical_bits,
        "capacityBytes": info.capacity_bytes,
        "headerCells": info.header_cells,
        "headerBytes": info.header_bytes,
        "headerParitySymbols": info.header_parity_symbols,
        "bodyParitySymbols": info.body_parity_symbols,
        "correctableHeaderSymbols": info.correctable_header_symbols,
        "correctableSymbolsPerBlock": info.correctable_symbols_per_block,
        "compactSmallSymbol": info.compact_small_symbol,
        "calibrationCells": info.calibration_cells,
        "hasAlignmentMarker": info.has_alignment_marker,
        "alignmentPatterns": info.alignment_patterns,
        "alignmentCenters": info.alignment_centers.iter().map(|(row, col)| json!([row, col])).collect::<Vec<_>>(),
        "bitsPerDataCell": info.bits_per_data_cell,
        "highDensity": info.high_density,
        "cellEncoding": encoding_label(info.cell_encoding),
        "colors": info.colors,
        "statesPerDataCell": info.states_per_data_cell,
        "spectralInterleaving": info.spectral_interleaving,
        "confidenceAwareEcc": info.confidence_aware_ecc,
    })
}

fn reliability_suite(options: &Value) -> Result<ReliabilitySuite, String> {
    match str_field(options, "suite").unwrap_or("quick") {
        "quick" => Ok(ReliabilitySuite::Quick),
        "full" => Ok(ReliabilitySuite::Full),
        "extreme" => Ok(ReliabilitySuite::Extreme),
        _ => Err("suite must be quick, full, or extreme".to_owned()),
    }
}

fn reliability_report_value(report: ReliabilityReport) -> Value {
    json!({
        "score": report.score,
        "rating": report.rating,
        "passed": report.passed,
        "total": report.total,
        "scenarios": report.scenarios.into_iter().map(|scenario| json!({
            "name": scenario.name,
            "category": scenario.category,
            "passed": scenario.passed,
            "elapsedMs": scenario.elapsed_ms,
            "correctedSymbols": scenario.corrected_symbols,
            "erasureSymbols": scenario.erasure_symbols,
            "scanPath": scenario.scan_path,
        })).collect::<Vec<_>>(),
        "categoryScores": report.category_scores,
    })
}

fn stress_distortion(value: &str) -> Result<StressDistortion, String> {
    match value {
        "blur" => Ok(StressDistortion::Blur),
        "motionBlur" | "motion-blur" => Ok(StressDistortion::MotionBlur),
        "lowExposure" | "low-exposure" => Ok(StressDistortion::LowExposure),
        "highExposure" | "high-exposure" => Ok(StressDistortion::HighExposure),
        "shadow" => Ok(StressDistortion::Shadow),
        "glare" => Ok(StressDistortion::Glare),
        "warmCast" | "warm-cast" => Ok(StressDistortion::WarmCast),
        "coolCast" | "cool-cast" => Ok(StressDistortion::CoolCast),
        "contrastLoss" | "contrast-loss" => Ok(StressDistortion::ContrastLoss),
        "sensorNoise" | "sensor-noise" => Ok(StressDistortion::SensorNoise),
        "jpegLike" | "jpeg-like" => Ok(StressDistortion::JpegLike),
        "downscale" => Ok(StressDistortion::Downscale),
        "perspective" => Ok(StressDistortion::Perspective),
        "perspective3d" | "perspective-3d" => Ok(StressDistortion::Perspective3d),
        "rotation" => Ok(StressDistortion::Rotation),
        _ => Err("unsupported stress distortion".to_owned()),
    }
}

fn perspective_options(options: &Value) -> PerspectiveOptions {
    let nested = field(options, "perspective").unwrap_or(options);
    PerspectiveOptions {
        pitch_degrees: f64_field(nested, "pitchDegrees")
            .or_else(|| f64_field(nested, "pitch"))
            .unwrap_or(0.0),
        yaw_degrees: f64_field(nested, "yawDegrees")
            .or_else(|| f64_field(nested, "yaw"))
            .unwrap_or(0.0),
        roll_degrees: f64_field(nested, "rollDegrees")
            .or_else(|| f64_field(nested, "roll"))
            .unwrap_or(0.0),
    }
}

fn trusted_public_key(options: &Value) -> Result<Option<[u8; 32]>, String> {
    let Some(value) = field(options, "publicKey").or_else(|| field(options, "trustedPublicKey"))
    else {
        return Ok(None);
    };
    Ok(Some(byte_array_32(value, "Ed25519 public key")?))
}

fn response(result: Result<Value, String>) -> String {
    match result {
        Ok(value) => json!({ "ok": true, "result": value }).to_string(),
        Err(error) => json!({ "ok": false, "error": error }).to_string(),
    }
}

pub fn version_response() -> String {
    response(Ok(json!({
        "nativeBridgeVersion": env!("CARGO_PKG_VERSION"),
        "formatVersion": quadqr::FORMAT_VERSION,
        "legacyFormatVersion": quadqr::LEGACY_FORMAT_VERSION,
        "minVersion": quadqr::MIN_VERSION,
        "maxVersion": quadqr::MAX_VERSION,
        "parallel": true,
        "interfaces": ["ffi", "jni"],
        "features": ["encode", "decode", "render", "scan", "compression", "security", "signing", "reliability"]
    })))
}

pub fn get_version_info_response(version: u8, options_json: &str) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let info = quadqr::get_version_info(
            version,
            ecc_from_options(&options)?,
            encoding_from_options(&options),
        )
        .map_err(|e| e.to_string())?;
        Ok(version_info_value(info))
    })())
}

pub fn capacity_response(version: u8, options_json: &str) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let ecc = ecc_from_options(&options)?;
        let encoding = encoding_from_options(&options);
        let info = quadqr::get_version_info(version, ecc, encoding).map_err(|e| e.to_string())?;
        let standard =
            quadqr::get_standard_qr_byte_capacity(version, ecc).map_err(|e| e.to_string())?;
        let difference = info.capacity_bytes as isize - standard as isize;
        Ok(json!({
            "version": version,
            "eccLevel": ecc_label(ecc),
            "cellEncoding": encoding_label(encoding),
            "highDensity": encoding == CellEncoding::Triangle16,
            "quadqrBytes": info.capacity_bytes,
            "standardQrBytes": standard,
            "differenceBytes": difference,
            "ratio": if standard == 0 { 0.0 } else { info.capacity_bytes as f64 / standard as f64 },
            "gainPercent": if standard == 0 { 0.0 } else { difference as f64 / standard as f64 * 100.0 },
            "size": info.size,
            "bitsPerDataCell": info.bits_per_data_cell,
        }))
    })())
}

pub fn calculate_capacity_plan_response(input: &[u8], options_json: &str) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let plan = quadqr::calculate_capacity_plan(
            input,
            ecc_from_options(&options)?,
            encoding_from_options(&options),
            compression_from_options(&options)?,
        )
        .map_err(|e| e.to_string())?;
        Ok(json!({
            "eccLevel": ecc_label(plan.ecc),
            "highDensity": plan.high_density,
            "cellEncoding": encoding_label(plan.cell_encoding),
            "sourceBytes": plan.source_bytes,
            "encodedBytes": plan.encoded_bytes,
            "storedBytes": plan.stored_bytes,
            "extensionOverheadBytes": plan.extension_overhead_bytes,
            "compression": algorithm_label(plan.compression),
            "compressionLevel": plan.compression_level,
            "compressed": plan.compressed,
            "quadqrVersion": plan.quadqr_version,
            "quadqrSize": plan.quadqr_size,
            "quadqrCapacityBytes": plan.quadqr_capacity_bytes,
            "remainingBytes": plan.remaining_bytes,
            "utilizationPercent": plan.utilization_percent,
            "standardQrVersion": plan.standard_qr_version,
            "standardQrSize": plan.standard_qr_size,
            "matrixWidthSavingsPercent": plan.matrix_width_savings_percent,
        }))
    })())
}

pub fn encode_text_response(text: &str, options_json: &str) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let app_options = application_options(&options, true)?;
        let security = app_options.security.clone();
        let encoded =
            quadqr::encode_application_text(text, app_options).map_err(|e| e.to_string())?;
        encoded_value(encoded, security.as_ref())
    })())
}

pub fn encode_bytes_response(input: &[u8], options_json: &str) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let text = bool_field(&options, "text").unwrap_or(false);
        let app_options = application_options(&options, text)?;
        let security = app_options.security.clone();
        let encoded =
            quadqr::encode_application_bytes(input, app_options).map_err(|e| e.to_string())?;
        encoded_value(encoded, security.as_ref())
    })())
}

pub fn decode_matrix_response(matrix_json: &str, options_json: &str) -> String {
    response((|| {
        let matrix_value = parse_json(matrix_json, "matrix")?;
        let matrix = parse_matrix(&matrix_value)?;
        let options = parse_options(options_json)?;
        let confidence = parse_f32_matrix(&options, "confidence")?;
        let alternatives = parse_i16_matrix_field(&options, "alternatives")?;
        let decoded = quadqr::decode_matrix_with_options(
            &matrix,
            confidence.as_deref(),
            alternatives.as_deref(),
            decode_options(&options),
        )
        .map_err(|e| e.to_string())?;
        decoded_value(decoded)
    })())
}

pub fn decode_uint8_array_response(matrix_json: &str, options_json: &str) -> String {
    response((|| {
        let matrix_value = parse_json(matrix_json, "matrix")?;
        let matrix = parse_matrix(&matrix_value)?;
        let options = parse_options(options_json)?;
        let confidence = parse_f32_matrix(&options, "confidence")?;
        let alternatives = parse_i16_matrix_field(&options, "alternatives")?;
        let decoded = quadqr::decode_matrix_with_options(
            &matrix,
            confidence.as_deref(),
            alternatives.as_deref(),
            decode_options(&options),
        )
        .map_err(|e| e.to_string())?;
        let app = if decoded.secure {
            return Err(
                "secure matrix must be decrypted before extracting application payload".to_owned(),
            );
        } else {
            quadqr::unwrap_application(decoded).map_err(|e| e.to_string())?
        };
        Ok(binary_value(&app.payload, Some("application/octet-stream")))
    })())
}

pub fn rotate_matrix_response(matrix_json: &str, quarter_turns: i32) -> String {
    response((|| {
        let matrix_value = parse_json(matrix_json, "matrix")?;
        let matrix = parse_matrix(&matrix_value)?;
        let rotated = quadqr::rotate_matrix(&matrix, quarter_turns).map_err(|e| e.to_string())?;
        Ok(json!({ "matrix": rotated, "quarterTurns": quarter_turns }))
    })())
}

pub fn crc32_response(input: &[u8]) -> String {
    response(Ok(json!({ "crc32": quadqr::crc32(input) })))
}

pub fn compress_lz_response(input: &[u8], options_json: &str) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let level = u64_field(&options, "level").unwrap_or(6) as u8;
        let data = quadqr::compress_lz(input, level).map_err(|e| e.to_string())?;
        Ok(binary_value(&data, Some("application/octet-stream")))
    })())
}

pub fn decompress_lz_response(input: &[u8], options_json: &str) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let expected = u64_field(&options, "expectedLength").map(|v| v as usize);
        let data = quadqr::decompress_lz(input, expected).map_err(|e| e.to_string())?;
        Ok(binary_value(&data, Some("application/octet-stream")))
    })())
}

pub fn compress_deflate_response(input: &[u8], options_json: &str) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let level = u64_field(&options, "level").unwrap_or(6) as u32;
        let data = quadqr::compress_deflate(input, level).map_err(|e| e.to_string())?;
        Ok(binary_value(&data, Some("application/octet-stream")))
    })())
}

pub fn decompress_deflate_response(input: &[u8], options_json: &str) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let expected = u64_field(&options, "expectedLength").map(|v| v as usize);
        let data = quadqr::decompress_deflate(input, expected).map_err(|e| e.to_string())?;
        Ok(binary_value(&data, Some("application/octet-stream")))
    })())
}

pub fn compress_brotli_response(input: &[u8], options_json: &str) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let quality = u64_field(&options, "quality")
            .or_else(|| u64_field(&options, "level"))
            .unwrap_or(11) as u32;
        let data = quadqr::compress_brotli(input, quality).map_err(|e| e.to_string())?;
        Ok(binary_value(&data, Some("application/octet-stream")))
    })())
}

pub fn decompress_brotli_response(input: &[u8], options_json: &str) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let expected = u64_field(&options, "expectedLength").map(|v| v as usize);
        let data = quadqr::decompress_brotli(input, expected).map_err(|e| e.to_string())?;
        Ok(binary_value(&data, Some("application/octet-stream")))
    })())
}

pub fn generate_raw_256_key_response() -> String {
    let key = quadqr::generate_raw_256_key();
    response(Ok(json!({
        "key": key,
        "hex": quadqr::bytes_to_hex(&key),
        "base64": BASE64.encode(key),
        "bits": 256,
    })))
}

pub fn inspect_secure_payload_response(input: &[u8]) -> String {
    response((|| {
        let info = quadqr::inspect_secure_envelope(input).map_err(|e| e.to_string())?;
        Ok(json!({
            "mode": info.mode,
            "iterations": info.iterations,
            "keyId": if info.key_id_utf8 { Some(String::from_utf8_lossy(&info.key_id).into_owned()) } else { None },
            "keyIdHex": quadqr::bytes_to_hex(&info.key_id),
            "keyIdAuto": info.key_id_auto,
            "plaintextBytes": info.plaintext_bytes,
            "envelopeBytes": info.envelope_bytes,
            "securePayloadVersion": quadqr::SECURE_PAYLOAD_VERSION,
            "algorithm": "AES-256-GCM",
        }))
    })())
}

pub fn decrypt_decoded_response(decoded_json: &str, security_json: &str) -> String {
    response((|| {
        let decoded = decode_result_from_json(decoded_json)?;
        let security_value = parse_json(security_json, "security")?;
        let security = security_mode_from_value(&security_value)?
            .ok_or_else(|| "a password or raw-key security mode is required".to_owned())?;
        let app = quadqr::decrypt_application(decoded, &security).map_err(|e| e.to_string())?;
        let payload_base64 = BASE64.encode(&app.payload);
        Ok(json!({
            "payload": app.payload,
            "payloadBase64": payload_base64,
            "text": app.text,
            "compression": algorithm_label(app.compression),
            "compressed": app.compression != CompressionAlgorithm::None,
            "signed": app.signed,
            "signingKeyId": signing_key_id_string(app.signing_key_id.as_deref()),
            "decrypted": true,
        }))
    })())
}

pub fn generate_signing_key_pair_response() -> String {
    response((|| {
        let pair = quadqr::generate_signing_key_pair().map_err(|e| e.to_string())?;
        let private_key_pkcs8_base64 = BASE64.encode(&pair.private_key_pkcs8);
        let key_id_hex = quadqr::bytes_to_hex(&pair.key_id);
        Ok(json!({
            "privateKeySeed": pair.private_key_seed,
            "privateKeySeedHex": quadqr::bytes_to_hex(&pair.private_key_seed),
            "privateKeyPkcs8": pair.private_key_pkcs8,
            "privateKeyPkcs8Base64": private_key_pkcs8_base64,
            "publicKey": pair.public_key,
            "publicKeyHex": quadqr::bytes_to_hex(&pair.public_key),
            "keyId": pair.key_id,
            "keyIdHex": key_id_hex,
            "algorithm": "Ed25519",
        }))
    })())
}

pub fn derive_signing_key_id_response(public_key: &[u8], options_json: &str) -> String {
    response((|| {
        let key: [u8; 32] = public_key
            .try_into()
            .map_err(|_| "Ed25519 public key must contain exactly 32 bytes".to_owned())?;
        let options = parse_options(options_json)?;
        let bytes = u64_field(&options, "bytes").unwrap_or(8) as usize;
        let id = quadqr::derive_signing_key_id(&key, bytes);
        let id_hex = quadqr::bytes_to_hex(&id);
        let id_len = id.len();
        Ok(json!({
            "keyId": id,
            "keyIdHex": id_hex,
            "bytes": id_len,
        }))
    })())
}

pub fn signing_seed_from_pkcs8_response(input: &[u8]) -> String {
    response((|| {
        let seed = quadqr::signing_seed_from_pkcs8(input).map_err(|e| e.to_string())?;
        Ok(json!({
            "privateKeySeed": seed,
            "privateKeySeedHex": quadqr::bytes_to_hex(&seed),
        }))
    })())
}

pub fn verify_decoded_signature_response(decoded_json: &str, options_json: &str) -> String {
    response((|| {
        let decoded = decode_result_from_json(decoded_json)?;
        let options = parse_options(options_json)?;
        let trusted = trusted_public_key(&options)?;
        let verified = quadqr::verify_decoded_signature(
            &decoded,
            trusted.as_ref(),
            bool_field(&options, "allowEmbeddedKey").unwrap_or(false),
        )
        .map_err(|e| e.to_string())?;
        Ok(json!({ "verified": verified, "algorithm": "Ed25519" }))
    })())
}

pub fn decrypt_and_verify_decoded_response(decoded_json: &str, options_json: &str) -> String {
    response((|| {
        let decoded = decode_result_from_json(decoded_json)?;
        let options = parse_options(options_json)?;
        let security_value = field(&options, "security").unwrap_or(&options);
        let security = security_mode_from_value(security_value)?
            .ok_or_else(|| "a password or raw-key security mode is required".to_owned())?;
        let trusted = trusted_public_key(&options)?;
        let (app, verified) = quadqr::decrypt_and_verify_application(
            decoded,
            &security,
            trusted.as_ref(),
            bool_field(&options, "allowEmbeddedKey").unwrap_or(false),
        )
        .map_err(|e| e.to_string())?;
        let payload_base64 = BASE64.encode(&app.payload);
        Ok(json!({
            "payload": app.payload,
            "payloadBase64": payload_base64,
            "text": app.text,
            "compression": algorithm_label(app.compression),
            "compressed": app.compression != CompressionAlgorithm::None,
            "signed": app.signed,
            "signingKeyId": signing_key_id_string(app.signing_key_id.as_deref()),
            "verified": verified,
            "decrypted": true,
        }))
    })())
}

pub fn render_png_response(code_json: &str, options_json: &str, logo_data: &[u8]) -> String {
    response((|| {
        let code = quad_code_from_json(code_json)?;
        let options = parse_options(options_json)?;
        let render = render_options(&options, code.size)?;
        let png = if logo_data.is_empty() {
            quadqr::to_png(&code, render).map_err(|e| e.to_string())?
        } else {
            let logo = image::load_from_memory(logo_data)
                .map_err(|e| format!("invalid logo image: {e}"))?;
            quadqr::to_png_with_logo(&code, render, &logo, logo_options(&options)?)
                .map_err(|e| e.to_string())?
        };
        let mut value = binary_value(&png, Some("image/png"));
        if let Some(map) = value.as_object_mut() {
            map.insert(
                "width".into(),
                json!((code.size as u32 + render.quiet_zone * 2) * render.module_size),
            );
            map.insert(
                "height".into(),
                json!((code.size as u32 + render.quiet_zone * 2) * render.module_size),
            );
        }
        Ok(value)
    })())
}

pub fn render_svg_response(code_json: &str, options_json: &str) -> String {
    response((|| {
        let code = quad_code_from_json(code_json)?;
        let options = parse_options(options_json)?;
        let render = render_options(&options, code.size)?;
        let svg = if let Some(href) = str_field(&options, "logoHref") {
            quadqr::render_svg_with_logo_href(&code, render, href, logo_options(&options)?)
                .map_err(|e| e.to_string())?
        } else {
            quadqr::render_svg(&code, render).map_err(|e| e.to_string())?
        };
        let byte_length = svg.len();
        Ok(json!({
            "svg": svg,
            "byteLength": byte_length,
            "mediaType": "image/svg+xml",
        }))
    })())
}

pub fn estimate_safe_logo_size_response(code_json: &str, options_json: &str) -> String {
    response((|| {
        let code = quad_code_from_json(code_json)?;
        let options = parse_options(options_json)?;
        let mode = render_mode(&options);
        let logo = logo_options(&options)?;
        let safe = quadqr::estimate_safe_logo_size(&code, mode, logo.clear_background);
        Ok(json!({ "safeSize": safe, "empirical": false }))
    })())
}

pub fn find_max_safe_logo_size_response(
    code_json: &str,
    logo_data: &[u8],
    options_json: &str,
) -> String {
    response((|| {
        if logo_data.is_empty() {
            return Err("logo image bytes are required".to_owned());
        }
        let code = quad_code_from_json(code_json)?;
        let options = parse_options(options_json)?;
        let render = render_options(&options, code.size)?;
        let logo =
            image::load_from_memory(logo_data).map_err(|e| format!("invalid logo image: {e}"))?;
        let result = quadqr::find_max_safe_logo_size(
            &code,
            render,
            &logo,
            logo_options(&options)?,
            f64_field(&options, "minSize").unwrap_or(0.05) as f32,
            f64_field(&options, "maxSize").unwrap_or(0.28) as f32,
            u64_field(&options, "iterations").unwrap_or(7) as usize,
        )
        .map_err(|e| e.to_string())?;
        Ok(json!({
            "safeSize": result.safe_size,
            "testedMax": result.tested_max,
            "iterations": result.iterations,
            "empirical": result.empirical,
        }))
    })())
}

pub fn print_guidance_response(code_json: &str, options_json: &str) -> String {
    response((|| {
        let code = quad_code_from_json(code_json)?;
        let options = parse_options(options_json)?;
        let guidance = quadqr::get_print_guidance(
            &code,
            u64_field(&options, "quietZone").unwrap_or(4) as u32,
            f64_field(&options, "physicalSizeMm").unwrap_or(35.0),
            f64_field(&options, "dpi").unwrap_or(300.0),
            f64_field(&options, "minimumModuleMm").unwrap_or(0.40),
        );
        Ok(json!({
            "mode": "print",
            "physicalSizeMm": guidance.physical_size_mm,
            "quietZone": guidance.quiet_zone,
            "moduleSizeMm": guidance.module_size_mm,
            "modulePixelsAtDpi": guidance.module_pixels_at_dpi,
            "dpi": guidance.dpi,
            "recommendedMinimumModuleMm": guidance.recommended_minimum_module_mm,
            "meetsRecommendedModuleSize": guidance.meets_recommended_module_size,
            "recommendedMinimumPhysicalSizeMm": guidance.recommended_minimum_physical_size_mm,
        }))
    })())
}

pub fn scan_rgba_response(data: &[u8], width: u32, height: u32, options_json: &str) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let mut scan_options = scan_options(&options)?;
        let (image, scale) =
            rgba_scan_image(data, width, height, scan_options.working_max_dimension)?;
        // The bridge already applied the working cap directly from the borrowed
        // input bytes, so prevent the core from allocating another resized copy.
        scan_options.working_max_dimension = 0;
        let mut scan = quadqr::scan_rgba(&image, &scan_options).map_err(|e| e.to_string())?;
        restore_finder_scale(&mut scan, scale);
        scan_value(scan)
    })())
}

#[allow(clippy::too_many_arguments)]
pub fn scan_yuv420_response(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: u32,
    height: u32,
    y_row_stride: usize,
    y_pixel_stride: usize,
    u_row_stride: usize,
    u_pixel_stride: usize,
    v_row_stride: usize,
    v_pixel_stride: usize,
    options_json: &str,
) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let mut scan_options = scan_options(&options)?;
        let (image, scale) = yuv420_scan_image(
            y_plane,
            u_plane,
            v_plane,
            width,
            height,
            y_row_stride,
            y_pixel_stride,
            u_row_stride,
            u_pixel_stride,
            v_row_stride,
            v_pixel_stride,
            scan_options.working_max_dimension,
        )?;
        scan_options.working_max_dimension = 0;
        let mut scan = quadqr::scan_rgba(&image, &scan_options).map_err(|e| e.to_string())?;
        restore_finder_scale(&mut scan, scale);
        scan_value(scan)
    })())
}

pub fn debug_scan_rgba_response(
    data: &[u8],
    width: u32,
    height: u32,
    options_json: &str,
) -> String {
    response((|| {
        let image = rgba_image(data, width, height)?;
        let options = parse_options(options_json)?;
        let scan =
            quadqr::debug_scan_rgba(&image, &scan_options(&options)?).map_err(|e| e.to_string())?;
        scan_value(scan)
    })())
}

pub fn scan_image_response(data: &[u8], options_json: &str) -> String {
    response((|| {
        let options = parse_options(options_json)?;
        let scan =
            quadqr::scan_buffer(data, &scan_options(&options)?).map_err(|e| e.to_string())?;
        scan_value(scan)
    })())
}

pub fn rectify_image_response(data: &[u8], options_json: &str) -> String {
    response((|| {
        let image = quadqr::decode_image_buffer(data).map_err(|e| e.to_string())?;
        let options = parse_options(options_json)?;
        let rectified = quadqr::rectify_detected_code(
            &image,
            &scan_options(&options)?,
            u64_field(&options, "moduleSize").unwrap_or(8) as u32,
        )
        .map_err(|e| e.to_string())?;
        let png = image_to_png(rectified.image)?;
        Ok(json!({
            "version": rectified.version,
            "path": rectified.path,
            "finders": rectified.finders.iter().map(finder_value).collect::<Vec<_>>(),
            "image": binary_value(&png, Some("image/png")),
        }))
    })())
}

pub fn apply_stress_distortion_response(data: &[u8], options_json: &str) -> String {
    response((|| {
        let image = quadqr::decode_image_buffer(data).map_err(|e| e.to_string())?;
        let options = parse_options(options_json)?;
        let distortion = stress_distortion(str_field(&options, "type").unwrap_or("blur"))?;
        let severity = f64_field(&options, "severity")
            .or_else(|| f64_field(&options, "amount"))
            .unwrap_or(0.5);
        let transformed = quadqr::apply_stress_distortion(
            &image,
            distortion,
            severity,
            perspective_options(&options),
        )
        .map_err(|e| e.to_string())?;
        let png = image_to_png(transformed)?;
        Ok(binary_value(&png, Some("image/png")))
    })())
}

pub fn run_image_stress_test_response(data: &[u8], options_json: &str) -> String {
    response((|| {
        let image = quadqr::decode_image_buffer(data).map_err(|e| e.to_string())?;
        let options = parse_options(options_json)?;
        let expected = u64_field(&options, "expectedCrc32")
            .ok_or_else(|| "expectedCrc32 is required".to_owned())? as u32;
        let report = quadqr::run_image_stress_test(&image, expected, &scan_options(&options)?)
            .map_err(|e| e.to_string())?;
        Ok(reliability_report_value(report))
    })())
}

pub fn run_reliability_lab_response(data: &[u8], options_json: &str) -> String {
    response((|| {
        let image = quadqr::decode_image_buffer(data).map_err(|e| e.to_string())?;
        let options = parse_options(options_json)?;
        let expected = u64_field(&options, "expectedCrc32")
            .ok_or_else(|| "expectedCrc32 is required".to_owned())? as u32;
        let report = quadqr::run_reliability_lab(
            &image,
            expected,
            reliability_suite(&options)?,
            &scan_options(&options)?,
        )
        .map_err(|e| e.to_string())?;
        Ok(reliability_report_value(report))
    })())
}

pub fn run_perspective_sweep_response(data: &[u8], options_json: &str) -> String {
    response((|| {
        let image = quadqr::decode_image_buffer(data).map_err(|e| e.to_string())?;
        let options = parse_options(options_json)?;
        let expected = u64_field(&options, "expectedCrc32")
            .ok_or_else(|| "expectedCrc32 is required".to_owned())? as u32;
        let axis = match str_field(&options, "axis").unwrap_or("yaw") {
            "pitch" => "pitch",
            "yaw" => "yaw",
            "roll" => "roll",
            _ => return Err("axis must be pitch, yaw, or roll".to_owned()),
        };
        let angles = field(&options, "angles")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .map(|value| {
                        value
                            .as_f64()
                            .ok_or_else(|| "angles must contain numbers".to_owned())
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_else(|| vec![5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0]);
        let result = quadqr::run_perspective_sweep(
            &image,
            expected,
            axis,
            &angles,
            &scan_options(&options)?,
        )
        .map_err(|e| e.to_string())?;
        Ok(json!({
            "axis": result.axis,
            "angles": result.angles,
            "passed": result.passed,
            "largestPassingAngle": result.largest_passing_angle,
        }))
    })())
}

pub fn assess_scanability_response(code_json: &str, options_json: &str) -> String {
    response((|| {
        let code = quad_code_from_json(code_json)?;
        let options = parse_options(options_json)?;
        let render = render_options(&options, code.size)?;
        let assessment = quadqr::assess_scanability(
            &code,
            render,
            reliability_suite(&options)?,
            &scan_options(&options)?,
        )
        .map_err(|e| e.to_string())?;
        Ok(json!({
            "score": assessment.score,
            "rating": assessment.rating,
            "report": reliability_report_value(assessment.report),
            "recommendations": assessment.recommendations,
        }))
    })())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_response_is_valid_json() {
        let value: Value = serde_json::from_str(&version_response()).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["result"]["interfaces"][0], "ffi");
    }

    #[test]
    fn encode_text_and_decode_matrix_work() {
        let encoded: Value = serde_json::from_str(&encode_text_response(
            "QuadQR Android",
            r#"{"ecc":"M","highDensity":false}"#,
        ))
        .unwrap();
        assert_eq!(encoded["ok"], true, "{}", encoded);
        let matrix = serde_json::to_string(&encoded["result"]["matrix"]).unwrap();
        let decoded: Value = serde_json::from_str(&decode_matrix_response(&matrix, "{}")).unwrap();
        assert_eq!(decoded["ok"], true, "{}", decoded);
        assert_eq!(decoded["result"]["text"], "QuadQR Android");
    }
}
