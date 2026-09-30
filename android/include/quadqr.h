#ifndef QUADQR_NATIVE_H
#define QUADQR_NATIVE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * All functions return UTF-8 JSON allocated by Rust.
 * Release every non-NULL result exactly once with quadqr_string_free().
 *
 * Success: {"ok":true,"result":...}
 * Error:   {"ok":false,"error":"..."}
 *
 * Binary outputs are returned as base64 in result.dataBase64.
 */

/* Runtime / information */
char *quadqr_version_json(void);
char *quadqr_get_version_info_json(uint8_t version, const char *options_json);
char *quadqr_capacity_json(uint8_t version, const char *options_json);
char *quadqr_calculate_capacity_plan_json(
    const uint8_t *data,
    size_t len,
    const char *options_json
);

/* Encode / decode / matrix utilities */
char *quadqr_encode_text_json(const char *text, const char *options_json);
char *quadqr_encode_bytes_json(
    const uint8_t *data,
    size_t len,
    const char *options_json
);
char *quadqr_decode_matrix_json(const char *matrix_json, const char *options_json);
char *quadqr_decode_uint8_array_json(const char *matrix_json, const char *options_json);
char *quadqr_rotate_matrix_json(const char *matrix_json, int32_t quarter_turns);
char *quadqr_crc32_json(const uint8_t *data, size_t len);

/* Compression */
char *quadqr_compress_lz_json(const uint8_t *data, size_t len, const char *options_json);
char *quadqr_decompress_lz_json(const uint8_t *data, size_t len, const char *options_json);
char *quadqr_compress_deflate_json(const uint8_t *data, size_t len, const char *options_json);
char *quadqr_decompress_deflate_json(const uint8_t *data, size_t len, const char *options_json);
char *quadqr_compress_brotli_json(const uint8_t *data, size_t len, const char *options_json);
char *quadqr_decompress_brotli_json(const uint8_t *data, size_t len, const char *options_json);

/* Security / signing */
char *quadqr_generate_raw_256_key_json(void);
char *quadqr_inspect_secure_payload_json(const uint8_t *data, size_t len);
char *quadqr_decrypt_decoded_json(const char *decoded_json, const char *security_json);
char *quadqr_generate_signing_key_pair_json(void);
char *quadqr_derive_signing_key_id_json(
    const uint8_t *public_key,
    size_t len,
    const char *options_json
);
char *quadqr_signing_seed_from_pkcs8_json(const uint8_t *data, size_t len);
char *quadqr_verify_decoded_signature_json(const char *decoded_json, const char *options_json);
char *quadqr_decrypt_and_verify_decoded_json(const char *decoded_json, const char *options_json);

/* Rendering / logos / print */
char *quadqr_render_png_json(
    const char *code_json,
    const char *options_json,
    const uint8_t *logo_data,
    size_t logo_len
);
char *quadqr_render_svg_json(const char *code_json, const char *options_json);
char *quadqr_estimate_safe_logo_size_json(const char *code_json, const char *options_json);
char *quadqr_find_max_safe_logo_size_json(
    const char *code_json,
    const uint8_t *logo_data,
    size_t logo_len,
    const char *options_json
);
char *quadqr_get_print_guidance_json(const char *code_json, const char *options_json);

/* Scanner */
char *quadqr_scan_rgba_json(
    const uint8_t *data,
    size_t len,
    uint32_t width,
    uint32_t height,
    const char *options_json
);
char *quadqr_scan_yuv420_json(
    const uint8_t *y_data,
    size_t y_len,
    const uint8_t *u_data,
    size_t u_len,
    const uint8_t *v_data,
    size_t v_len,
    uint32_t width,
    uint32_t height,
    size_t y_row_stride,
    size_t y_pixel_stride,
    size_t u_row_stride,
    size_t u_pixel_stride,
    size_t v_row_stride,
    size_t v_pixel_stride,
    const char *options_json
);
char *quadqr_debug_scan_rgba_json(
    const uint8_t *data,
    size_t len,
    uint32_t width,
    uint32_t height,
    const char *options_json
);
char *quadqr_scan_image_json(const uint8_t *data, size_t len, const char *options_json);
char *quadqr_rectify_image_json(const uint8_t *data, size_t len, const char *options_json);

/* Reliability / stress tools */
char *quadqr_apply_stress_distortion_json(const uint8_t *data, size_t len, const char *options_json);
char *quadqr_run_image_stress_test_json(const uint8_t *data, size_t len, const char *options_json);
char *quadqr_run_reliability_lab_json(const uint8_t *data, size_t len, const char *options_json);
char *quadqr_run_perspective_sweep_json(const uint8_t *data, size_t len, const char *options_json);
char *quadqr_assess_scanability_json(const char *code_json, const char *options_json);

void quadqr_string_free(char *value);

#ifdef __cplusplus
}
#endif

#endif /* QUADQR_NATIVE_H */
