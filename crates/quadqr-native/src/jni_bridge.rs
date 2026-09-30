use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::jint;
use jni::EnvUnowned;

use crate::api::*;

macro_rules! jni_noarg {
    ($name:ident, $handler:ident) => {
        #[no_mangle]
        pub extern "system" fn $name<'caller>(
            mut unowned_env: EnvUnowned<'caller>,
            _class: JClass<'caller>,
        ) -> JString<'caller> {
            let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
                JString::from_str(env, $handler())
            });
            outcome.resolve::<ThrowRuntimeExAndDefault>()
        }
    };
}

macro_rules! jni_int_options {
    ($name:ident, $handler:ident) => {
        #[no_mangle]
        pub extern "system" fn $name<'caller>(
            mut unowned_env: EnvUnowned<'caller>,
            _class: JClass<'caller>,
            value: jint,
            options_json: JString<'caller>,
        ) -> JString<'caller> {
            let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
                let options = options_json.try_to_string(env)?;
                JString::from_str(env, $handler(value.clamp(0, 255) as u8, &options))
            });
            outcome.resolve::<ThrowRuntimeExAndDefault>()
        }
    };
}

macro_rules! jni_string_options {
    ($name:ident, $handler:ident) => {
        #[no_mangle]
        pub extern "system" fn $name<'caller>(
            mut unowned_env: EnvUnowned<'caller>,
            _class: JClass<'caller>,
            input_json: JString<'caller>,
            options_json: JString<'caller>,
        ) -> JString<'caller> {
            let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
                let input = input_json.try_to_string(env)?;
                let options = options_json.try_to_string(env)?;
                JString::from_str(env, $handler(&input, &options))
            });
            outcome.resolve::<ThrowRuntimeExAndDefault>()
        }
    };
}

macro_rules! jni_bytes_options {
    ($name:ident, $handler:ident) => {
        #[no_mangle]
        pub extern "system" fn $name<'caller>(
            mut unowned_env: EnvUnowned<'caller>,
            _class: JClass<'caller>,
            data: JByteArray<'caller>,
            options_json: JString<'caller>,
        ) -> JString<'caller> {
            let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
                let data = env.convert_byte_array(&data)?;
                let options = options_json.try_to_string(env)?;
                JString::from_str(env, $handler(&data, &options))
            });
            outcome.resolve::<ThrowRuntimeExAndDefault>()
        }
    };
}

macro_rules! jni_bytes_only {
    ($name:ident, $handler:ident) => {
        #[no_mangle]
        pub extern "system" fn $name<'caller>(
            mut unowned_env: EnvUnowned<'caller>,
            _class: JClass<'caller>,
            data: JByteArray<'caller>,
        ) -> JString<'caller> {
            let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
                let data = env.convert_byte_array(&data)?;
                JString::from_str(env, $handler(&data))
            });
            outcome.resolve::<ThrowRuntimeExAndDefault>()
        }
    };
}

jni_noarg!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeVersionJson,
    version_response
);
jni_noarg!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeGenerateRaw256KeyJson,
    generate_raw_256_key_response
);
jni_noarg!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeGenerateSigningKeyPairJson,
    generate_signing_key_pair_response
);

jni_int_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeGetVersionInfoJson,
    get_version_info_response
);
jni_int_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeCapacityJson,
    capacity_response
);

#[no_mangle]
pub extern "system" fn Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeEncodeTextJson<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    text: JString<'caller>,
    options_json: JString<'caller>,
) -> JString<'caller> {
    let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
        let text = text.try_to_string(env)?;
        let options = options_json.try_to_string(env)?;
        JString::from_str(env, encode_text_response(&text, &options))
    });
    outcome.resolve::<ThrowRuntimeExAndDefault>()
}

jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeEncodeBytesJson,
    encode_bytes_response
);
jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeCalculateCapacityPlanJson,
    calculate_capacity_plan_response
);

jni_string_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeDecodeMatrixJson,
    decode_matrix_response
);
jni_string_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeDecodeUint8ArrayJson,
    decode_uint8_array_response
);

#[no_mangle]
pub extern "system" fn Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeRotateMatrixJson<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    matrix_json: JString<'caller>,
    quarter_turns: jint,
) -> JString<'caller> {
    let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
        let matrix = matrix_json.try_to_string(env)?;
        JString::from_str(env, rotate_matrix_response(&matrix, quarter_turns))
    });
    outcome.resolve::<ThrowRuntimeExAndDefault>()
}

jni_bytes_only!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeCrc32Json,
    crc32_response
);
jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeCompressLzJson,
    compress_lz_response
);
jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeDecompressLzJson,
    decompress_lz_response
);
jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeCompressDeflateJson,
    compress_deflate_response
);
jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeDecompressDeflateJson,
    decompress_deflate_response
);
jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeCompressBrotliJson,
    compress_brotli_response
);
jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeDecompressBrotliJson,
    decompress_brotli_response
);

jni_bytes_only!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeInspectSecurePayloadJson,
    inspect_secure_payload_response
);

jni_string_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeDecryptDecodedJson,
    decrypt_decoded_response
);
jni_string_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeVerifyDecodedSignatureJson,
    verify_decoded_signature_response
);
jni_string_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeDecryptAndVerifyDecodedJson,
    decrypt_and_verify_decoded_response
);

jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeDeriveSigningKeyIdJson,
    derive_signing_key_id_response
);
jni_bytes_only!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeSigningSeedFromPkcs8Json,
    signing_seed_from_pkcs8_response
);

#[no_mangle]
pub extern "system" fn Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeRenderPngJson<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    code_json: JString<'caller>,
    options_json: JString<'caller>,
    logo_data: JByteArray<'caller>,
) -> JString<'caller> {
    let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
        let code = code_json.try_to_string(env)?;
        let options = options_json.try_to_string(env)?;
        let logo = env.convert_byte_array(&logo_data)?;
        JString::from_str(env, render_png_response(&code, &options, &logo))
    });
    outcome.resolve::<ThrowRuntimeExAndDefault>()
}

jni_string_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeRenderSvgJson,
    render_svg_response
);
jni_string_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeEstimateSafeLogoSizeJson,
    estimate_safe_logo_size_response
);
jni_string_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeGetPrintGuidanceJson,
    print_guidance_response
);
jni_string_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeAssessScanabilityJson,
    assess_scanability_response
);

#[no_mangle]
pub extern "system" fn Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeFindMaxSafeLogoSizeJson<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    code_json: JString<'caller>,
    logo_data: JByteArray<'caller>,
    options_json: JString<'caller>,
) -> JString<'caller> {
    let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
        let code = code_json.try_to_string(env)?;
        let logo = env.convert_byte_array(&logo_data)?;
        let options = options_json.try_to_string(env)?;
        JString::from_str(
            env,
            find_max_safe_logo_size_response(&code, &logo, &options),
        )
    });
    outcome.resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeScanRgbaJson<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    data: JByteArray<'caller>,
    width: jint,
    height: jint,
    options_json: JString<'caller>,
) -> JString<'caller> {
    let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
        let response = if width <= 0 || height <= 0 {
            serde_json::json!({
                "ok": false,
                "error": "width and height must be positive"
            })
            .to_string()
        } else {
            let data = env.convert_byte_array(&data)?;
            let options = options_json.try_to_string(env)?;
            scan_rgba_response(&data, width as u32, height as u32, &options)
        };
        JString::from_str(env, response)
    });
    outcome.resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeScanYuv420Json<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    y_data: JByteArray<'caller>,
    u_data: JByteArray<'caller>,
    v_data: JByteArray<'caller>,
    width: jint,
    height: jint,
    y_row_stride: jint,
    y_pixel_stride: jint,
    u_row_stride: jint,
    u_pixel_stride: jint,
    v_row_stride: jint,
    v_pixel_stride: jint,
    options_json: JString<'caller>,
) -> JString<'caller> {
    let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
        let response = if width <= 0
            || height <= 0
            || y_row_stride <= 0
            || y_pixel_stride <= 0
            || u_row_stride <= 0
            || u_pixel_stride <= 0
            || v_row_stride <= 0
            || v_pixel_stride <= 0
        {
            serde_json::json!({
                "ok": false,
                "error": "width, height and YUV strides must be positive"
            })
            .to_string()
        } else {
            let y = env.convert_byte_array(&y_data)?;
            let u = env.convert_byte_array(&u_data)?;
            let v = env.convert_byte_array(&v_data)?;
            let options = options_json.try_to_string(env)?;
            scan_yuv420_response(
                &y,
                &u,
                &v,
                width as u32,
                height as u32,
                y_row_stride as usize,
                y_pixel_stride as usize,
                u_row_stride as usize,
                u_pixel_stride as usize,
                v_row_stride as usize,
                v_pixel_stride as usize,
                &options,
            )
        };
        JString::from_str(env, response)
    });
    outcome.resolve::<ThrowRuntimeExAndDefault>()
}

#[no_mangle]
pub extern "system" fn Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeDebugScanRgbaJson<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    data: JByteArray<'caller>,
    width: jint,
    height: jint,
    options_json: JString<'caller>,
) -> JString<'caller> {
    let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
        let response = if width <= 0 || height <= 0 {
            serde_json::json!({
                "ok": false,
                "error": "width and height must be positive"
            })
            .to_string()
        } else {
            let data = env.convert_byte_array(&data)?;
            let options = options_json.try_to_string(env)?;
            debug_scan_rgba_response(&data, width as u32, height as u32, &options)
        };
        JString::from_str(env, response)
    });
    outcome.resolve::<ThrowRuntimeExAndDefault>()
}

jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeScanImageJson,
    scan_image_response
);
jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeRectifyImageJson,
    rectify_image_response
);
jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeApplyStressDistortionJson,
    apply_stress_distortion_response
);
jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeRunImageStressTestJson,
    run_image_stress_test_response
);
jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeRunReliabilityLabJson,
    run_reliability_lab_response
);
jni_bytes_options!(
    Java_io_github_akanshsirohi_quadqr_QuadQRNative_nativeRunPerspectiveSweepJson,
    run_perspective_sweep_response
);
