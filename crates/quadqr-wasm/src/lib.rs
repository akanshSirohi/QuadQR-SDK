#![cfg(target_arch = "wasm32")]

use quadqr::{
    ApplicationEncodeOptions, CellEncoding, CompressionAlgorithm, CompressionMode, DecodeOptions,
    DecodeResult, EccLevel, EncodeOptions, EncodedApplication, PerspectiveOptions,
    ReliabilitySuite, ScanOptions, SecurityMode, SoftDecodeOptions, StressDistortion,
};
use serde::Serialize;
use serde_json::{json, Map, Value};
use wasm_bindgen::prelude::*;

#[cfg(feature = "threads")]
pub use wasm_bindgen_rayon::init_thread_pool;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}

fn js_value_message(error: &JsValue) -> String {
    js_sys::Reflect::get(error, &JsValue::from_str("message"))
        .ok()
        .and_then(|value| value.as_string())
        .or_else(|| error.as_string())
        .unwrap_or_else(|| "QuadQR WASM operation failed".to_owned())
}

fn from_js(value: JsValue) -> Result<Value, JsValue> {
    if value.is_null() || value.is_undefined() {
        return Ok(json!({}));
    }
    serde_wasm_bindgen::from_value(value).map_err(js_error)
}

fn to_js(value: &Value) -> Result<JsValue, JsValue> {
    // serde-wasm-bindgen intentionally serializes map-like values as ES2015
    // Map objects by default. QuadQR's existing JavaScript API returns normal
    // property-accessible objects (code.matrix, result.text, diagnostics.path,
    // etc.), so use the JSON-compatible serializer at the public WASM boundary.
    // It also keeps nested byte arrays JSON-like; the JS wrapper normalizes the
    // public byte fields back to Uint8Array/Uint8ClampedArray where expected.
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(js_error)
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

fn bytes_from_value(value: &Value) -> Result<Vec<u8>, JsValue> {
    if let Some(text) = value.as_str() {
        return quadqr::hex_to_bytes(text).map_err(js_error);
    }
    let Some(items) = value.as_array() else {
        return Err(js_error("expected a byte array or hexadecimal string"));
    };
    items
        .iter()
        .map(|item| {
            item.as_u64()
                .filter(|value| *value <= 255)
                .map(|value| value as u8)
                .ok_or_else(|| js_error("byte array contains a value outside 0..255"))
        })
        .collect()
}

fn optional_bytes(value: &Value, key: &str) -> Result<Option<Vec<u8>>, JsValue> {
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

fn ecc_from_options(options: &Value) -> Result<EccLevel, JsValue> {
    match str_field(options, "ecc")
        .unwrap_or("M")
        .to_ascii_uppercase()
        .as_str()
    {
        "L" => Ok(EccLevel::L),
        "M" => Ok(EccLevel::M),
        "Q" => Ok(EccLevel::Q),
        "H" => Ok(EccLevel::H),
        _ => Err(js_error("ECC level must be one of L, M, Q, H")),
    }
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
    match str_field(options, "cellEncoding").unwrap_or("rgbw") {
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

fn compression_from_options(options: &Value) -> Result<CompressionMode, JsValue> {
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
                return Err(js_error("LZ compressionLevel must be 1..9"));
            }
            Ok(CompressionMode::Lz(level as u8))
        }
        "deflate" => {
            let level = generic_level
                .or_else(|| u64_field(options, "deflateLevel").map(|value| value as u32))
                .unwrap_or(6);
            if !(1..=9).contains(&level) {
                return Err(js_error("DEFLATE compressionLevel must be 1..9"));
            }
            Ok(CompressionMode::Deflate(level))
        }
        "brotli" => {
            let level = generic_level
                .or_else(|| u64_field(options, "brotliQuality").map(|value| value as u32))
                .unwrap_or(11);
            if level > 11 {
                return Err(js_error("Brotli compressionLevel must be 0..11"));
            }
            Ok(CompressionMode::Brotli(level))
        }
        _ => Err(js_error(
            "compression must be none, auto, smart, lz, deflate, or brotli",
        )),
    }
}

fn encode_options(options: &Value, text: bool) -> Result<EncodeOptions, JsValue> {
    let mut format = EncodeOptions::default();
    format.ecc = ecc_from_options(options)?;
    format.high_density = encoding_from_options(options) == CellEncoding::Triangle16;
    format.text = text;
    format.min_version = u64_field(options, "minVersion").unwrap_or(1).clamp(1, 40) as u8;
    format.max_version = u64_field(options, "maxVersion").unwrap_or(40).clamp(1, 40) as u8;
    if format.min_version > format.max_version {
        return Err(js_error("minVersion cannot be greater than maxVersion"));
    }
    if let Some(version) = field(options, "version") {
        if let Some(number) = version.as_u64() {
            if !(1..=40).contains(&number) {
                return Err(js_error("Version must be 1..40 or auto"));
            }
            format.version = Some(number as u8);
        } else if let Some(text) = version.as_str() {
            if text != "auto" {
                return Err(js_error("Version must be 1..40 or auto"));
            }
        }
    }
    Ok(format)
}

fn security_from_options(options: &Value) -> Result<Option<SecurityMode>, JsValue> {
    let Some(security) = field(options, "security") else {
        return Ok(None);
    };
    if security.is_null() {
        return Ok(None);
    }
    let mode = str_field(security, "mode").unwrap_or("none");
    let key_id = optional_bytes(security, "keyId")?;
    match mode {
        "none" => Ok(None),
        "password" => {
            let password = str_field(security, "password")
                .ok_or_else(|| js_error("security.password is required"))?;
            let iterations = u64_field(security, "iterations")
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
                field(security, "key").ok_or_else(|| js_error("security.key is required"))?;
            let key = if let Some(hex) = key_value.as_str() {
                quadqr::normalize_raw_256_key_hex(hex).map_err(js_error)?
            } else {
                quadqr::normalize_raw_256_key_bytes(&bytes_from_value(key_value)?)
                    .map_err(js_error)?
            };
            Ok(Some(SecurityMode::RawKey {
                key,
                key_id,
                disable_auto_key_id: bool_field(security, "disableAutoKeyId").unwrap_or(false),
            }))
        }
        _ => Err(js_error("security.mode must be none, password, or raw-key")),
    }
}

fn signing_from_options(options: &Value) -> Result<Option<quadqr::SigningOptions>, JsValue> {
    let Some(signing) = field(options, "signing") else {
        return Ok(None);
    };
    if signing.is_null() {
        return Ok(None);
    }
    let seed_value = field(signing, "privateKeySeed")
        .or_else(|| field(signing, "privateKey"))
        .ok_or_else(|| js_error("signing.privateKeySeed is required by the WASM bridge"))?;
    let seed_bytes = bytes_from_value(seed_value)?;
    let private_key_seed: [u8; 32] = seed_bytes
        .try_into()
        .map_err(|_| js_error("Ed25519 private key seed must contain exactly 32 bytes"))?;
    Ok(Some(quadqr::SigningOptions {
        private_key_seed,
        key_id: optional_bytes(signing, "keyId")?,
        embed_public_key: bool_field(signing, "embedPublicKey").unwrap_or(false),
    }))
}

fn application_options(options: &Value, text: bool) -> Result<ApplicationEncodeOptions, JsValue> {
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
        "alignmentCenters": info.alignment_centers,
        "bitsPerDataCell": info.bits_per_data_cell,
        "highDensity": info.high_density,
        "cellEncoding": encoding_label(info.cell_encoding),
        "colors": info.colors,
        "statesPerDataCell": info.states_per_data_cell,
        "spectralInterleaving": info.spectral_interleaving,
        "confidenceAwareEcc": info.confidence_aware_ecc,
    })
}

fn encoded_value(
    encoded: EncodedApplication,
    security: Option<&SecurityMode>,
) -> Result<Value, JsValue> {
    let code = encoded.code;
    let info = quadqr::get_version_info(code.version, code.ecc_level, code.cell_encoding)
        .map_err(js_error)?;
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

fn decode_base_value(decoded: &DecodeResult) -> Result<Value, JsValue> {
    let info = quadqr::get_version_info(decoded.version, decoded.ecc_level, decoded.cell_encoding)
        .map_err(js_error)?;
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

fn decoded_value(decoded: DecodeResult) -> Result<Value, JsValue> {
    let mut base = decode_base_value(&decoded)?;
    let target = base.as_object_mut().expect("decode object");
    if decoded.secure {
        target.insert("payload".into(), json!(decoded.payload));
        target.insert("text".into(), Value::Null);
        target.insert("requiresDecryption".into(), json!(true));
        target.insert("decrypted".into(), json!(false));
        target.insert("compressed".into(), json!(false));
        target.insert("compression".into(), json!("none"));
        target.insert("protectedPayload".into(), Value::Null);
        target.insert("security".into(), secure_info_value(&decoded.payload));
        return Ok(base);
    }

    let original_protected = decoded.payload.clone();
    let app = quadqr::unwrap_application(decoded).map_err(js_error)?;
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

fn scan_value(scan: quadqr::ScanResult) -> Result<Value, JsValue> {
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
    target.insert("diagnostics".into(), json!({
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
                "geometries": if geometry.is_null() { Vec::<Value>::new() } else { vec![geometry.clone()] },
                "finderMethod": "rust-parallel-locator"
            }]
        }
    }));
    Ok(value)
}

fn decode_options(options: &Value) -> DecodeOptions {
    let defaults = DecodeOptions::default();
    DecodeOptions {
        structure_tolerance: f64_field(options, "structuralTolerance")
            .or_else(|| f64_field(options, "structureTolerance"))
            .unwrap_or(defaults.structure_tolerance),
        max_erasure_confidence: f64_field(options, "maxErasureConfidence")
            .unwrap_or(defaults.max_erasure_confidence as f64)
            .clamp(0.0, 1.0) as f32,
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

fn confidence_matrix_from_options(options: &Value) -> Result<Option<Vec<Vec<f32>>>, JsValue> {
    let Some(value) = field(options, "cellConfidence") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let rows = value
        .as_array()
        .ok_or_else(|| js_error("cellConfidence must be a 2D numeric array"))?;
    rows.iter()
        .map(|row| {
            let cells = row
                .as_array()
                .ok_or_else(|| js_error("cellConfidence must be a 2D numeric array"))?;
            cells
                .iter()
                .map(|cell| {
                    cell.as_f64()
                        .map(|value| value.clamp(0.0, 1.0) as f32)
                        .ok_or_else(|| js_error("cellConfidence contains a non-numeric value"))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn alternatives_matrix_from_options(
    options: &Value,
    matrix: &[Vec<i16>],
) -> Result<Option<Vec<Vec<i16>>>, JsValue> {
    let Some(value) = field(options, "cellAlternatives") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let rows = value
        .as_array()
        .ok_or_else(|| js_error("cellAlternatives must be a 2D array"))?;
    if rows.len() != matrix.len() {
        return Err(js_error(
            "cellAlternatives must match the QuadQR matrix size",
        ));
    }
    let mut out = Vec::with_capacity(rows.len());
    for (row_index, row) in rows.iter().enumerate() {
        let cells = row
            .as_array()
            .ok_or_else(|| js_error("cellAlternatives must be a 2D array"))?;
        if cells.len() != matrix[row_index].len() {
            return Err(js_error(
                "cellAlternatives must match the QuadQR matrix size",
            ));
        }
        let mut output_row = Vec::with_capacity(cells.len());
        for (col_index, cell) in cells.iter().enumerate() {
            let value = if cell.is_null() {
                matrix[row_index][col_index]
            } else {
                cell.as_i64()
                    .and_then(|value| i16::try_from(value).ok())
                    .ok_or_else(|| js_error("cellAlternatives contains an invalid cell value"))?
            };
            output_row.push(value);
        }
        out.push(output_row);
    }
    Ok(Some(out))
}

fn scan_options(options: &Value) -> Result<ScanOptions, JsValue> {
    let mut scan = ScanOptions::default();
    scan.min_version = u64_field(options, "minVersion")
        .unwrap_or(scan.min_version as u64)
        .clamp(1, 40) as u8;
    scan.max_version = u64_field(options, "maxVersion")
        .unwrap_or(scan.max_version as u64)
        .clamp(1, 40) as u8;
    if scan.min_version > scan.max_version {
        return Err(js_error("minVersion cannot be greater than maxVersion"));
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

fn image_from_rgba(data: &[u8], width: u32, height: u32) -> Result<image::RgbaImage, JsValue> {
    if width == 0 || height == 0 {
        return Err(js_error("image width and height must be positive"));
    }
    image::RgbaImage::from_raw(width, height, data.to_vec())
        .ok_or_else(|| js_error("RGBA byte length does not match width × height × 4"))
}

#[wasm_bindgen(js_name = buildKind)]
pub fn build_kind() -> String {
    if cfg!(feature = "threads") {
        "threaded".into()
    } else {
        "single".into()
    }
}

#[wasm_bindgen(js_name = formatVersion)]
pub fn format_version() -> u8 {
    quadqr::FORMAT_VERSION
}

#[wasm_bindgen(js_name = getVersionInfo)]
pub fn get_version_info(version: u8, options: JsValue) -> Result<JsValue, JsValue> {
    let options = from_js(options)?;
    let info = quadqr::get_version_info(
        version,
        ecc_from_options(&options)?,
        encoding_from_options(&options),
    )
    .map_err(js_error)?;
    to_js(&version_info_value(info))
}

#[wasm_bindgen(js_name = encodeText)]
pub fn encode_text(text: &str, options: JsValue) -> Result<JsValue, JsValue> {
    let options = from_js(options)?;
    let app_options = application_options(&options, true)?;
    let security = app_options.security.clone();
    let encoded = quadqr::encode_application_text(text, app_options).map_err(js_error)?;
    to_js(&encoded_value(encoded, security.as_ref())?)
}

#[wasm_bindgen(js_name = encodeBytes)]
pub fn encode_bytes(input: &[u8], options: JsValue) -> Result<JsValue, JsValue> {
    let options = from_js(options)?;
    let app_options = application_options(&options, bool_field(&options, "text").unwrap_or(false))?;
    let security = app_options.security.clone();
    let encoded = quadqr::encode_application_bytes(input, app_options).map_err(js_error)?;
    to_js(&encoded_value(encoded, security.as_ref())?)
}

#[wasm_bindgen(js_name = decodeMatrix)]
pub fn decode_matrix(matrix: JsValue, options: JsValue) -> Result<JsValue, JsValue> {
    let matrix: Vec<Vec<i16>> = serde_wasm_bindgen::from_value(matrix).map_err(js_error)?;
    let options = from_js(options)?;
    let confidence = confidence_matrix_from_options(&options)?;
    let alternatives = alternatives_matrix_from_options(&options, &matrix)?;
    let decoded = quadqr::decode_matrix_with_options(
        &matrix,
        confidence.as_deref(),
        alternatives.as_deref(),
        decode_options(&options),
    )
    .map_err(js_error)?;
    to_js(&decoded_value(decoded)?)
}

#[wasm_bindgen(js_name = scanRgba)]
pub fn scan_rgba_js(
    data: &[u8],
    width: u32,
    height: u32,
    options: JsValue,
) -> Result<JsValue, JsValue> {
    let image = image_from_rgba(data, width, height)?;
    let options = from_js(options)?;
    let scan = quadqr::scan_rgba(&image, &scan_options(&options)?).map_err(js_error)?;
    to_js(&scan_value(scan)?)
}

#[wasm_bindgen(js_name = debugScanRgba)]
pub fn debug_scan_rgba_js(data: &[u8], width: u32, height: u32, options: JsValue) -> JsValue {
    match image_from_rgba(data, width, height).and_then(|image| {
        from_js(options).and_then(|options| {
            let scan_options = scan_options(&options)?;
            quadqr::scan_rgba(&image, &scan_options)
                .map_err(js_error)
                .and_then(|scan| scan_value(scan))
        })
    }) {
        Ok(value) => {
            let debug = value.get("diagnostics").cloned().unwrap_or(Value::Null);
            to_js(&json!({ "ok": true, "result": value, "debug": debug })).unwrap_or(JsValue::NULL)
        }
        Err(error) => {
            let message = js_value_message(&error);
            to_js(&json!({ "ok": false, "error": message, "debug": Value::Null }))
                .unwrap_or(JsValue::NULL)
        }
    }
}

#[wasm_bindgen(js_name = rectifyRgba)]
pub fn rectify_rgba_js(
    data: &[u8],
    width: u32,
    height: u32,
    options: JsValue,
    module_size: u32,
) -> Result<JsValue, JsValue> {
    let image = image_from_rgba(data, width, height)?;
    let options = from_js(options)?;
    let rectified =
        quadqr::rectify_detected_code(&image, &scan_options(&options)?, module_size.max(1))
            .map_err(js_error)?;
    to_js(&json!({
        "imageData": { "width": rectified.image.width(), "height": rectified.image.height(), "data": rectified.image.into_raw() },
        "version": rectified.version,
        "finders": rectified.finders.iter().map(finder_value).collect::<Vec<_>>(),
        "path": rectified.path,
    }))
}

#[wasm_bindgen(js_name = rotateMatrix)]
pub fn rotate_matrix_js(matrix: JsValue, quarter_turns: i32) -> Result<JsValue, JsValue> {
    let matrix: Vec<Vec<i16>> = serde_wasm_bindgen::from_value(matrix).map_err(js_error)?;
    let rotated = quadqr::rotate_matrix(&matrix, quarter_turns).map_err(js_error)?;
    serde_wasm_bindgen::to_value(&rotated).map_err(js_error)
}

#[wasm_bindgen(js_name = crc32)]
pub fn crc32_js(input: &[u8]) -> u32 {
    quadqr::crc32(input)
}

#[wasm_bindgen(js_name = compressLz)]
pub fn compress_lz_js(input: &[u8], level: u8) -> Result<Vec<u8>, JsValue> {
    quadqr::compress_lz(input, level).map_err(js_error)
}

#[wasm_bindgen(js_name = decompressLz)]
pub fn decompress_lz_js(input: &[u8], expected_length: Option<usize>) -> Result<Vec<u8>, JsValue> {
    quadqr::decompress_lz(input, expected_length).map_err(js_error)
}

#[wasm_bindgen(js_name = compressDeflate)]
pub fn compress_deflate_js(input: &[u8], level: u32) -> Result<Vec<u8>, JsValue> {
    quadqr::compress_deflate(input, level).map_err(js_error)
}

#[wasm_bindgen(js_name = decompressDeflate)]
pub fn decompress_deflate_js(
    input: &[u8],
    expected_length: Option<usize>,
) -> Result<Vec<u8>, JsValue> {
    quadqr::decompress_deflate(input, expected_length).map_err(js_error)
}

#[wasm_bindgen(js_name = compressBrotli)]
pub fn compress_brotli_js(input: &[u8], quality: u32) -> Result<Vec<u8>, JsValue> {
    quadqr::compress_brotli(input, quality).map_err(js_error)
}

#[wasm_bindgen(js_name = decompressBrotli)]
pub fn decompress_brotli_js(
    input: &[u8],
    expected_length: Option<usize>,
) -> Result<Vec<u8>, JsValue> {
    quadqr::decompress_brotli(input, expected_length).map_err(js_error)
}

#[wasm_bindgen(js_name = generateRaw256Key)]
pub fn generate_raw_256_key_js() -> Vec<u8> {
    quadqr::generate_raw_256_key().to_vec()
}

#[wasm_bindgen(js_name = normalizeRaw256Key)]
pub fn normalize_raw_256_key_js(input: JsValue) -> Result<Vec<u8>, JsValue> {
    let value = from_js(input)?;
    if let Some(text) = value.as_str() {
        return quadqr::normalize_raw_256_key_hex(text)
            .map(|key| key.to_vec())
            .map_err(js_error);
    }
    quadqr::normalize_raw_256_key_bytes(&bytes_from_value(&value)?)
        .map(|key| key.to_vec())
        .map_err(js_error)
}

#[wasm_bindgen(js_name = bytesToHex)]
pub fn bytes_to_hex_js(input: &[u8]) -> String {
    quadqr::bytes_to_hex(input)
}

#[wasm_bindgen(js_name = signingSeedFromPkcs8)]
pub fn signing_seed_from_pkcs8_js(input: &[u8]) -> Result<Vec<u8>, JsValue> {
    quadqr::signing_seed_from_pkcs8(input)
        .map(|seed| seed.to_vec())
        .map_err(js_error)
}

#[wasm_bindgen(js_name = deriveSigningKeyId)]
pub fn derive_signing_key_id_js(public_key: &[u8], bytes: usize) -> Result<Vec<u8>, JsValue> {
    let public_key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| js_error("Ed25519 public key must contain exactly 32 bytes"))?;
    Ok(quadqr::derive_signing_key_id(&public_key, bytes).to_vec())
}

#[wasm_bindgen(js_name = verifySignedPayload)]
pub fn verify_signed_payload_js(
    input: &[u8],
    public_key: &[u8],
    allow_embedded_key: bool,
) -> Result<bool, JsValue> {
    let trusted: Option<[u8; 32]> = if public_key.is_empty() {
        None
    } else {
        Some(
            public_key
                .try_into()
                .map_err(|_| js_error("Ed25519 public key must contain exactly 32 bytes"))?,
        )
    };
    quadqr::verify_signed_payload(input, trusted.as_ref(), allow_embedded_key).map_err(js_error)
}

#[wasm_bindgen(js_name = decryptPayload)]
pub fn decrypt_payload_js(
    input: &[u8],
    flags: u8,
    extended: bool,
    security: JsValue,
) -> Result<JsValue, JsValue> {
    let security_value = from_js(security)?;
    let wrapper = json!({ "security": security_value });
    let mode = security_from_options(&wrapper)?
        .ok_or_else(|| js_error("security credentials are required"))?;
    let plaintext = quadqr::decrypt_secure_payload(input, &mode).map_err(js_error)?;
    let (payload, compression, signed, signing_key_id, protected_payload) = if extended {
        let envelope = quadqr::parse_payload_envelope(&plaintext).map_err(js_error)?;
        let signed = envelope.signed();
        let compression = envelope.compression;
        let key_id = if envelope.key_id.is_empty() {
            None
        } else {
            Some(envelope.key_id.clone())
        };
        let payload = envelope.payload.clone();
        (
            payload,
            compression,
            signed,
            key_id,
            if signed { Some(plaintext) } else { None },
        )
    } else {
        (plaintext, CompressionAlgorithm::None, false, None, None)
    };
    let text = if flags & quadqr::format::TEXT_FLAG != 0 {
        Some(String::from_utf8_lossy(&payload).into_owned())
    } else {
        None
    };
    to_js(&json!({
        "payload": payload,
        "text": text,
        "compression": algorithm_label(compression),
        "compressed": compression != CompressionAlgorithm::None,
        "signed": signed,
        "signingKeyId": signing_key_id_string(signing_key_id.as_deref()),
        "protectedPayload": protected_payload,
    }))
}

fn distortion_from_str(kind: &str) -> Result<Option<StressDistortion>, JsValue> {
    Ok(Some(match kind {
        "clean" => return Ok(None),
        "blur" => StressDistortion::Blur,
        "motion-blur" => StressDistortion::MotionBlur,
        "brightness-low" | "low-exposure" => StressDistortion::LowExposure,
        "brightness-high" | "high-exposure" => StressDistortion::HighExposure,
        "shadow" => StressDistortion::Shadow,
        "glare" => StressDistortion::Glare,
        "warm" | "warm-cast" => StressDistortion::WarmCast,
        "cool" | "cool-cast" => StressDistortion::CoolCast,
        "contrast-loss" => StressDistortion::ContrastLoss,
        "noise" | "sensor-noise" => StressDistortion::SensorNoise,
        "jpeg" | "jpeg-like" => StressDistortion::JpegLike,
        "downscale" => StressDistortion::Downscale,
        "perspective" => StressDistortion::Perspective,
        "perspective-3d" => StressDistortion::Perspective3d,
        "rotation" => StressDistortion::Rotation,
        _ => return Err(js_error(format!("unknown stress distortion: {kind}"))),
    }))
}

fn perspective_options(options: &Value) -> PerspectiveOptions {
    PerspectiveOptions {
        pitch_degrees: f64_field(options, "pitchDegrees").unwrap_or(0.0),
        yaw_degrees: f64_field(options, "yawDegrees").unwrap_or(0.0),
        roll_degrees: f64_field(options, "rollDegrees").unwrap_or(0.0),
    }
}

#[wasm_bindgen(js_name = applyStressDistortion)]
pub fn apply_stress_distortion_js(
    data: &[u8],
    width: u32,
    height: u32,
    kind: &str,
    severity: f64,
    options: JsValue,
) -> Result<JsValue, JsValue> {
    let image = image_from_rgba(data, width, height)?;
    let options = from_js(options)?;
    let transformed = if let Some(distortion) = distortion_from_str(kind)? {
        quadqr::apply_stress_distortion(&image, distortion, severity, perspective_options(&options))
            .map_err(js_error)?
    } else {
        image
    };
    to_js(
        &json!({ "width": transformed.width(), "height": transformed.height(), "data": transformed.into_raw() }),
    )
}

fn suite_from_str(value: &str) -> ReliabilitySuite {
    match value {
        "quick" => ReliabilitySuite::Quick,
        "extreme" => ReliabilitySuite::Extreme,
        _ => ReliabilitySuite::Full,
    }
}

fn reliability_value(report: quadqr::ReliabilityReport, suite: &str) -> Value {
    let average_confidence = if report.passed > 0 { 1.0 } else { 0.0 };
    let results: Vec<Value> = report
        .scenarios
        .iter()
        .map(|item| {
            json!({
                "id": item.name,
                "name": item.name,
                "label": item.name.replace('-', " "),
                "category": item.category,
                "passed": item.passed,
                "confidence": if item.passed { 1.0 } else { 0.0 },
                "correctedSymbols": item.corrected_symbols,
                "erasureSymbols": item.erasure_symbols,
                "elapsedMs": item.elapsed_ms,
                "scanPath": item.scan_path,
            })
        })
        .collect();
    let categories: Vec<Value> = report
        .category_scores
        .iter()
        .map(|(category, score)| {
            json!({
                "category": category,
                "score": score,
            })
        })
        .collect();
    let weakest = categories
        .iter()
        .min_by_key(|item| item.get("score").and_then(Value::as_u64).unwrap_or(101))
        .cloned();
    json!({
        "score": report.score,
        "rating": report.rating,
        "passed": report.passed,
        "total": report.total,
        "passPercent": report.score,
        "averageConfidence": average_confidence,
        "suite": suite,
        "results": results,
        "categories": categories,
        "weakestCategory": weakest,
    })
}

#[wasm_bindgen(js_name = runReliabilityLab)]
pub fn run_reliability_lab_js(
    data: &[u8],
    width: u32,
    height: u32,
    expected_crc32: u32,
    options: JsValue,
) -> Result<JsValue, JsValue> {
    let image = image_from_rgba(data, width, height)?;
    let options = from_js(options)?;
    let suite_name = str_field(&options, "suite").unwrap_or("full");
    let scan = scan_options(&options)?;
    let report =
        quadqr::run_reliability_lab(&image, expected_crc32, suite_from_str(suite_name), &scan)
            .map_err(js_error)?;
    to_js(&reliability_value(report, suite_name))
}

#[wasm_bindgen(js_name = runImageStressTest)]
pub fn run_image_stress_test_js(
    data: &[u8],
    width: u32,
    height: u32,
    expected_crc32: u32,
    options: JsValue,
) -> Result<JsValue, JsValue> {
    let image = image_from_rgba(data, width, height)?;
    let options = from_js(options)?;
    let scan = scan_options(&options)?;
    let report = quadqr::run_image_stress_test(&image, expected_crc32, &scan).map_err(js_error)?;
    to_js(&reliability_value(report, "quick"))
}

#[wasm_bindgen(js_name = runPerspectiveSweep)]
pub fn run_perspective_sweep_js(
    data: &[u8],
    width: u32,
    height: u32,
    expected_crc32: u32,
    axis: &str,
    angles: JsValue,
    options: JsValue,
) -> Result<JsValue, JsValue> {
    let image = image_from_rgba(data, width, height)?;
    let angles: Vec<f64> = serde_wasm_bindgen::from_value(angles).map_err(js_error)?;
    let options = from_js(options)?;
    let scan = scan_options(&options)?;
    let axis_static = match axis {
        "pitch" => "pitch",
        "roll" => "roll",
        _ => "yaw",
    };
    let result = quadqr::run_perspective_sweep(&image, expected_crc32, axis_static, &angles, &scan)
        .map_err(js_error)?;
    let rows: Vec<Value> = result
        .angles
        .iter()
        .zip(result.passed.iter())
        .map(|(angle, passed)| {
            json!({
                "angle": angle,
                "passed": passed,
                "confidence": if *passed { 1.0 } else { 0.0 },
            })
        })
        .collect();
    to_js(&json!({
        "axis": result.axis,
        "angles": result.angles,
        "passed": result.passed.iter().filter(|value| **value).count(),
        "total": result.passed.len(),
        "maxPassedAngle": result.largest_passing_angle,
        "results": rows,
    }))
}

#[wasm_bindgen(js_name = layoutReserved)]
pub fn layout_reserved_js(version: u8) -> Result<JsValue, JsValue> {
    let layout = quadqr::layout_for_version(version).map_err(js_error)?;
    serde_wasm_bindgen::to_value(&layout.reserved).map_err(js_error)
}
