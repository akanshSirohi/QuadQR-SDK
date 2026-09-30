#![cfg(feature = "image")]

use quadqr::{
    encode_text, render_image, scan_rgba, EccLevel, EncodeOptions, RenderOptions, ScanOptions,
};

#[test]
fn rendered_rgbw_scans_back() {
    let options = EncodeOptions {
        ecc: EccLevel::H,
        ..EncodeOptions::default()
    };
    let code = encode_text("QuadQR Rust image scanner compatibility", options).unwrap();
    let image = render_image(
        &code,
        RenderOptions {
            module_size: 10,
            quiet_zone: 4,
            ..RenderOptions::default()
        },
    )
    .unwrap();
    let result = scan_rgba(&image, &ScanOptions::default()).unwrap();
    assert_eq!(
        result.decoded.text.as_deref(),
        Some("QuadQR Rust image scanner compatibility")
    );
}

#[test]
fn rendered_triangle16_scans_back() {
    let options = EncodeOptions {
        ecc: EccLevel::Q,
        high_density: true,
        ..EncodeOptions::default()
    };
    let code = encode_text("QuadQR Triangle16 native Rust", options).unwrap();
    let image = render_image(
        &code,
        RenderOptions {
            module_size: 14,
            quiet_zone: 4,
            ..RenderOptions::default()
        },
    )
    .unwrap();
    let result = scan_rgba(&image, &ScanOptions::default()).unwrap();
    assert_eq!(
        result.decoded.text.as_deref(),
        Some("QuadQR Triangle16 native Rust")
    );
    assert!(result.decoded.high_density);
}
