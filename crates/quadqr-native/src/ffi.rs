use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::slice;

use crate::api::*;

fn owned_c_string(value: String) -> *mut c_char {
    let sanitized = value.replace('\0', "\\u0000");
    CString::new(sanitized)
        .map(CString::into_raw)
        .unwrap_or(ptr::null_mut())
}

fn error_response(error: impl AsRef<str>) -> *mut c_char {
    owned_c_string(serde_json::json!({"ok": false, "error": error.as_ref()}).to_string())
}

fn panic_response() -> *mut c_char {
    error_response("QuadQR native bridge panicked")
}

unsafe fn input_string(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Ok(String::new());
    }
    CStr::from_ptr(ptr)
        .to_str()
        .map(str::to_owned)
        .map_err(|e| format!("input string is not valid UTF-8: {e}"))
}

unsafe fn input_bytes<'a>(data: *const u8, len: usize) -> Result<&'a [u8], String> {
    if data.is_null() && len != 0 {
        return Err("data pointer is null".to_owned());
    }
    Ok(if len == 0 {
        &[]
    } else {
        slice::from_raw_parts(data, len)
    })
}

unsafe fn string_arg(ptr: *const c_char) -> Result<String, *mut c_char> {
    input_string(ptr).map_err(error_response)
}

unsafe fn bytes_arg<'a>(data: *const u8, len: usize) -> Result<&'a [u8], *mut c_char> {
    input_bytes(data, len).map_err(error_response)
}

#[no_mangle]
pub extern "C" fn quadqr_version_json() -> *mut c_char {
    catch_unwind(version_response)
        .map(owned_c_string)
        .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_get_version_info_json(
    version: u8,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(get_version_info_response(version, &options))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_capacity_json(
    version: u8,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(capacity_response(version, &options))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_calculate_capacity_plan_json(
    data: *const u8,
    len: usize,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let bytes = match bytes_arg(data, len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(calculate_capacity_plan_response(bytes, &options))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_encode_text_json(
    text: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let text = match string_arg(text) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(encode_text_response(&text, &options))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_encode_bytes_json(
    data: *const u8,
    len: usize,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let bytes = match bytes_arg(data, len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(encode_bytes_response(bytes, &options))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_decode_matrix_json(
    matrix_json: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let matrix = match string_arg(matrix_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(decode_matrix_response(&matrix, &options))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_decode_uint8_array_json(
    matrix_json: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let matrix = match string_arg(matrix_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(decode_uint8_array_response(&matrix, &options))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_rotate_matrix_json(
    matrix_json: *const c_char,
    quarter_turns: i32,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let matrix = match string_arg(matrix_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(rotate_matrix_response(&matrix, quarter_turns))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_crc32_json(data: *const u8, len: usize) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let bytes = match bytes_arg(data, len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(crc32_response(bytes))
    }))
    .unwrap_or_else(|_| panic_response())
}

macro_rules! byte_options_export {
    ($name:ident, $handler:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name(
            data: *const u8,
            len: usize,
            options_json: *const c_char,
        ) -> *mut c_char {
            catch_unwind(AssertUnwindSafe(|| {
                let bytes = match bytes_arg(data, len) {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                let options = match string_arg(options_json) {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                owned_c_string($handler(bytes, &options))
            }))
            .unwrap_or_else(|_| panic_response())
        }
    };
}

byte_options_export!(quadqr_compress_lz_json, compress_lz_response);
byte_options_export!(quadqr_decompress_lz_json, decompress_lz_response);
byte_options_export!(quadqr_compress_deflate_json, compress_deflate_response);
byte_options_export!(quadqr_decompress_deflate_json, decompress_deflate_response);
byte_options_export!(quadqr_compress_brotli_json, compress_brotli_response);
byte_options_export!(quadqr_decompress_brotli_json, decompress_brotli_response);
byte_options_export!(quadqr_scan_image_json, scan_image_response);
byte_options_export!(quadqr_rectify_image_json, rectify_image_response);
byte_options_export!(
    quadqr_apply_stress_distortion_json,
    apply_stress_distortion_response
);
byte_options_export!(
    quadqr_run_image_stress_test_json,
    run_image_stress_test_response
);
byte_options_export!(
    quadqr_run_reliability_lab_json,
    run_reliability_lab_response
);
byte_options_export!(
    quadqr_run_perspective_sweep_json,
    run_perspective_sweep_response
);

#[no_mangle]
pub extern "C" fn quadqr_generate_raw_256_key_json() -> *mut c_char {
    catch_unwind(generate_raw_256_key_response)
        .map(owned_c_string)
        .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_inspect_secure_payload_json(
    data: *const u8,
    len: usize,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let bytes = match bytes_arg(data, len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(inspect_secure_payload_response(bytes))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_decrypt_decoded_json(
    decoded_json: *const c_char,
    security_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let decoded = match string_arg(decoded_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let security = match string_arg(security_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(decrypt_decoded_response(&decoded, &security))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub extern "C" fn quadqr_generate_signing_key_pair_json() -> *mut c_char {
    catch_unwind(generate_signing_key_pair_response)
        .map(owned_c_string)
        .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_derive_signing_key_id_json(
    public_key: *const u8,
    len: usize,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let key = match bytes_arg(public_key, len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(derive_signing_key_id_response(key, &options))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_signing_seed_from_pkcs8_json(
    data: *const u8,
    len: usize,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let bytes = match bytes_arg(data, len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(signing_seed_from_pkcs8_response(bytes))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_verify_decoded_signature_json(
    decoded_json: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let decoded = match string_arg(decoded_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(verify_decoded_signature_response(&decoded, &options))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_decrypt_and_verify_decoded_json(
    decoded_json: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let decoded = match string_arg(decoded_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(decrypt_and_verify_decoded_response(&decoded, &options))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_render_png_json(
    code_json: *const c_char,
    options_json: *const c_char,
    logo_data: *const u8,
    logo_len: usize,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let code = match string_arg(code_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let logo = match bytes_arg(logo_data, logo_len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(render_png_response(&code, &options, logo))
    }))
    .unwrap_or_else(|_| panic_response())
}

macro_rules! string_options_export {
    ($name:ident, $handler:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name(
            input_json: *const c_char,
            options_json: *const c_char,
        ) -> *mut c_char {
            catch_unwind(AssertUnwindSafe(|| {
                let input = match string_arg(input_json) {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                let options = match string_arg(options_json) {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                owned_c_string($handler(&input, &options))
            }))
            .unwrap_or_else(|_| panic_response())
        }
    };
}

string_options_export!(quadqr_render_svg_json, render_svg_response);
string_options_export!(
    quadqr_estimate_safe_logo_size_json,
    estimate_safe_logo_size_response
);
string_options_export!(quadqr_get_print_guidance_json, print_guidance_response);
string_options_export!(quadqr_assess_scanability_json, assess_scanability_response);

#[no_mangle]
pub unsafe extern "C" fn quadqr_find_max_safe_logo_size_json(
    code_json: *const c_char,
    logo_data: *const u8,
    logo_len: usize,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let code = match string_arg(code_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let logo = match bytes_arg(logo_data, logo_len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(find_max_safe_logo_size_response(&code, logo, &options))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_scan_rgba_json(
    data: *const u8,
    len: usize,
    width: u32,
    height: u32,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let bytes = match bytes_arg(data, len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(scan_rgba_response(bytes, width, height, &options))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_scan_yuv420_json(
    y_data: *const u8,
    y_len: usize,
    u_data: *const u8,
    u_len: usize,
    v_data: *const u8,
    v_len: usize,
    width: u32,
    height: u32,
    y_row_stride: usize,
    y_pixel_stride: usize,
    u_row_stride: usize,
    u_pixel_stride: usize,
    v_row_stride: usize,
    v_pixel_stride: usize,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let y = match bytes_arg(y_data, y_len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let u = match bytes_arg(u_data, u_len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let v = match bytes_arg(v_data, v_len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(scan_yuv420_response(
            y,
            u,
            v,
            width,
            height,
            y_row_stride,
            y_pixel_stride,
            u_row_stride,
            u_pixel_stride,
            v_row_stride,
            v_pixel_stride,
            &options,
        ))
    }))
    .unwrap_or_else(|_| panic_response())
}

#[no_mangle]
pub unsafe extern "C" fn quadqr_debug_scan_rgba_json(
    data: *const u8,
    len: usize,
    width: u32,
    height: u32,
    options_json: *const c_char,
) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        let bytes = match bytes_arg(data, len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let options = match string_arg(options_json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        owned_c_string(debug_scan_rgba_response(bytes, width, height, &options))
    }))
    .unwrap_or_else(|_| panic_response())
}

/// Frees strings returned by any `quadqr_*_json` C ABI function.
#[no_mangle]
pub unsafe extern "C" fn quadqr_string_free(value: *mut c_char) {
    if !value.is_null() {
        drop(CString::from_raw(value));
    }
}
