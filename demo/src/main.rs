use std::io::Cursor;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

use base64::Engine;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use image::{DynamicImage, RgbaImage};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;
use quadqr::security::{
    generate_raw_256_key, generate_signing_keypair, SecurityMode, DEFAULT_PBKDF2_ITERATIONS,
    MAX_PBKDF2_ITERATIONS, MIN_PBKDF2_ITERATIONS,
};
use quadqr::{
    benchmark_codec, build_capacity_comparison, decrypt_and_verify_application,
    decrypt_application, encode_bytes_with_compression, encode_secure_bytes,
    encode_secure_signed_bytes, encode_secure_signed_text, encode_secure_text, encode_signed_bytes,
    encode_signed_text, encode_text_with_compression, estimate_safe_logo_size, render_image,
    render_image_with_logo, render_svg, render_svg_with_logo_href, run_perspective_sweep,
    run_reliability_lab, scan_dynamic_image, scan_rgba, unwrap_application,
    verify_decoded_signature, CapacityComparison, CellEncoding, CodecBenchmarkRow, CompressionMode,
    DecodeResult, EccLevel, EncodeOptions, LogoOptions, QuadQrCode, ReliabilityReport,
    ReliabilitySuite, RenderMode, RenderOptions, RenderStyle, ScanDiagnostics, ScanOptions,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Generator,
    ImageScanner,
    CameraScanner,
    Reliability,
    Benchmark,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SweepAxis {
    Pitch,
    Yaw,
    Roll,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Text,
    File,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CompressionKind {
    None,
    Auto,
    Smart,
    Lz,
    Deflate,
    Brotli,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SecurityKind {
    None,
    Password,
    RawKey,
}

#[derive(Clone)]
struct SharedCameraFrame {
    sequence: u64,
    captured_at: Instant,
    image: Arc<RgbaImage>,
}

type SigningMaterial = Option<([u8; 32], Option<Vec<u8>>)>;

enum CameraEvent {
    CaptureStats {
        fps: f64,
        width: u32,
        height: u32,
    },
    ScannerStats {
        fps: f64,
        last_scan_ms: f64,
    },
    Scan {
        decoded: DecodeResult,
        diagnostics: ScanDiagnostics,
    },
    Error(String),
    Stopped,
}

struct App {
    tab: Tab,

    // Generator input and encoding.
    input_mode: InputMode,
    generator_text: String,
    generator_file_name: String,
    generator_file_bytes: Vec<u8>,
    ecc: EccLevel,
    high_density: bool,
    version_auto: bool,
    fixed_version: u8,
    min_version: u8,
    max_version: u8,
    compression_kind: CompressionKind,
    compression_level: u32,

    // Security and signing.
    security_kind: SecurityKind,
    security_password: String,
    security_iterations: u32,
    security_raw_key_hex: String,
    security_key_id: String,
    security_disable_auto_key_id: bool,
    signing_enabled: bool,
    signing_private_hex: String,
    signing_public_hex: String,
    signing_key_id: String,
    signing_embed_public_key: bool,

    // Rendering.
    render_mode: RenderMode,
    render_style: RenderStyle,
    render_module_size: u32,
    render_quiet_zone: u32,
    allow_styled_print: bool,
    logo: Option<DynamicImage>,
    logo_name: String,
    logo_auto_size: bool,
    logo_size_ratio: f32,
    logo_padding_modules: f32,
    logo_radius_modules: f32,
    logo_clear_background: bool,

    code: Option<QuadQrCode>,
    generated_svg: Option<Arc<[u8]>>,
    generated_revision: u64,
    generated_status: String,
    generated_error: String,

    // Shared scanner tuning.
    scan_min_version: u8,
    scan_max_version: u8,
    scan_structural_tolerance: f64,
    scan_axis_fallback: bool,
    scan_max_finders: usize,
    scan_locator_max_dimension: u32,
    scan_sample_radius: f64,
    scan_high_density_sample_radius: f64,
    scan_max_erasure_confidence: f32,
    scan_soft_decoding: bool,
    scan_soft_confidence: f32,
    scan_soft_max_cells: usize,
    scan_soft_pair_cells: usize,

    // Reliability lab / benchmark.
    reliability_suite: ReliabilitySuite,
    reliability_report: Option<ReliabilityReport>,
    reliability_status: String,
    sweep_axis: SweepAxis,
    sweep_status: String,
    benchmark_iterations: usize,
    benchmark_rows: Vec<CodecBenchmarkRow>,
    benchmark_capacity: Vec<CapacityComparison>,
    benchmark_status: String,

    // Image scanner.
    image_texture: Option<egui::TextureHandle>,
    image_status: String,
    image_result: String,
    image_decoded: Option<DecodeResult>,
    image_diagnostics: Option<ScanDiagnostics>,
    image_verify_status: String,

    // Scanner credentials are intentionally independent from generator credentials.
    scanner_security_kind: SecurityKind,
    scanner_password: String,
    scanner_raw_key_hex: String,
    scanner_public_key_hex: String,
    scanner_allow_embedded_key: bool,

    // Camera scanner.
    cameras: Vec<(String, CameraIndex)>,
    camera_selected: usize,
    camera_running: bool,
    camera_stop: Option<Arc<AtomicBool>>,
    camera_rx: Option<Receiver<CameraEvent>>,
    camera_latest_frame: Option<Arc<Mutex<Option<SharedCameraFrame>>>>,
    camera_preview_sequence: u64,
    camera_preview_frames: u64,
    camera_preview_window_started: Instant,
    camera_capture_fps: f64,
    camera_preview_fps: f64,
    camera_preview_latency_ms: f64,
    camera_scanner_fps: f64,
    camera_last_scan_ms: f64,
    camera_texture: Option<egui::TextureHandle>,
    camera_status: String,
    camera_result: String,
    camera_decoded: Option<DecodeResult>,
    camera_diagnostics: Option<ScanDiagnostics>,
    camera_verify_status: String,
    camera_scan_every: u32,
    camera_stop_on_decode: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        configure_ui(&cc.egui_ctx);

        let mut app = Self {
            tab: Tab::Generator,
            input_mode: InputMode::Text,
            generator_text: "Hello from QuadQR Rust!".to_owned(),
            generator_file_name: String::new(),
            generator_file_bytes: Vec::new(),
            ecc: EccLevel::M,
            high_density: false,
            version_auto: true,
            fixed_version: 4,
            min_version: 1,
            max_version: 40,
            compression_kind: CompressionKind::Auto,
            compression_level: 6,
            security_kind: SecurityKind::None,
            security_password: String::new(),
            security_iterations: DEFAULT_PBKDF2_ITERATIONS,
            security_raw_key_hex: String::new(),
            security_key_id: String::new(),
            security_disable_auto_key_id: false,
            signing_enabled: false,
            signing_private_hex: String::new(),
            signing_public_hex: String::new(),
            signing_key_id: String::new(),
            signing_embed_public_key: false,
            render_mode: RenderMode::Screen,
            render_style: RenderStyle::Classic,
            render_module_size: 8,
            render_quiet_zone: 4,
            allow_styled_print: false,
            logo: None,
            logo_name: String::new(),
            logo_auto_size: true,
            logo_size_ratio: 0.18,
            logo_padding_modules: 0.65,
            logo_radius_modules: 0.8,
            logo_clear_background: false,
            code: None,
            generated_svg: None,
            generated_revision: 0,
            generated_status: String::new(),
            generated_error: String::new(),
            scan_min_version: 1,
            scan_max_version: 40,
            scan_structural_tolerance: 0.18,
            scan_axis_fallback: true,
            scan_max_finders: 24,
            scan_locator_max_dimension: 960,
            scan_sample_radius: 0.16,
            scan_high_density_sample_radius: 0.065,
            scan_max_erasure_confidence: 0.68,
            scan_soft_decoding: true,
            scan_soft_confidence: 0.72,
            scan_soft_max_cells: 10,
            scan_soft_pair_cells: 6,
            reliability_suite: ReliabilitySuite::Quick,
            reliability_report: None,
            reliability_status: "Generate a QuadQR, then run a reliability suite.".into(),
            sweep_axis: SweepAxis::Yaw,
            sweep_status: String::new(),
            benchmark_iterations: 20,
            benchmark_rows: Vec::new(),
            benchmark_capacity: Vec::new(),
            benchmark_status: "Run the native codec benchmark to compare encode/decode timings."
                .into(),
            image_texture: None,
            image_status: "Choose a QuadQR image to scan.".into(),
            image_result: String::new(),
            image_decoded: None,
            image_diagnostics: None,
            image_verify_status: String::new(),
            scanner_security_kind: SecurityKind::Password,
            scanner_password: String::new(),
            scanner_raw_key_hex: String::new(),
            scanner_public_key_hex: String::new(),
            scanner_allow_embedded_key: false,
            cameras: Vec::new(),
            camera_selected: 0,
            camera_running: false,
            camera_stop: None,
            camera_rx: None,
            camera_latest_frame: None,
            camera_preview_sequence: 0,
            camera_preview_frames: 0,
            camera_preview_window_started: Instant::now(),
            camera_capture_fps: 0.0,
            camera_preview_fps: 0.0,
            camera_preview_latency_ms: 0.0,
            camera_scanner_fps: 0.0,
            camera_last_scan_ms: 0.0,
            camera_texture: None,
            camera_status: "Camera stopped".into(),
            camera_result: String::new(),
            camera_decoded: None,
            camera_diagnostics: None,
            camera_verify_status: String::new(),
            camera_scan_every: 1,
            camera_stop_on_decode: true,
        };
        app.refresh_cameras();
        app.generate();
        app
    }

    fn refresh_cameras(&mut self) {
        self.cameras = match nokhwa::query(ApiBackend::Auto) {
            Ok(items) => items
                .into_iter()
                .map(|info| (info.human_name(), info.index().clone()))
                .collect(),
            Err(err) => {
                self.camera_status = format!("Camera enumeration failed: {err}");
                Vec::new()
            }
        };
        if self.camera_selected >= self.cameras.len() {
            self.camera_selected = 0;
        }
    }

    fn encode_options(&self) -> EncodeOptions {
        EncodeOptions {
            ecc: self.ecc,
            version: if self.version_auto {
                None
            } else {
                Some(self.fixed_version)
            },
            min_version: self.min_version,
            max_version: self.max_version,
            high_density: self.high_density,
            text: self.input_mode == InputMode::Text,
            ..EncodeOptions::default()
        }
    }

    fn compression_mode(&self) -> CompressionMode {
        match self.compression_kind {
            CompressionKind::None => CompressionMode::None,
            CompressionKind::Auto => CompressionMode::Auto,
            CompressionKind::Smart => CompressionMode::Smart,
            CompressionKind::Lz => CompressionMode::Lz(self.compression_level.clamp(1, 9) as u8),
            CompressionKind::Deflate => {
                CompressionMode::Deflate(self.compression_level.clamp(1, 9))
            }
            CompressionKind::Brotli => CompressionMode::Brotli(self.compression_level.clamp(0, 11)),
        }
    }

    fn security_mode(&self) -> std::result::Result<Option<SecurityMode>, String> {
        match self.security_kind {
            SecurityKind::None => Ok(None),
            SecurityKind::Password => {
                if self.security_password.is_empty() {
                    return Err("Password security is enabled but the password is empty.".into());
                }
                Ok(Some(SecurityMode::Password {
                    password: self.security_password.clone(),
                    iterations: self.security_iterations,
                    key_id: non_empty_bytes(&self.security_key_id),
                }))
            }
            SecurityKind::RawKey => {
                let key = parse_hex_32(&self.security_raw_key_hex)
                    .map_err(|e| format!("Raw security key: {e}"))?;
                Ok(Some(SecurityMode::RawKey {
                    key,
                    key_id: non_empty_bytes(&self.security_key_id),
                    disable_auto_key_id: self.security_disable_auto_key_id,
                }))
            }
        }
    }

    fn signing_material(&self) -> std::result::Result<SigningMaterial, String> {
        if !self.signing_enabled {
            return Ok(None);
        }
        let private = parse_hex_32(&self.signing_private_hex)
            .map_err(|e| format!("Signing private key: {e}"))?;
        let key_id = non_empty_bytes(&self.signing_key_id);
        Ok(Some((private, key_id)))
    }

    fn render_options(&self) -> RenderOptions {
        RenderOptions {
            module_size: self.render_module_size.max(1),
            quiet_zone: self.render_quiet_zone,
            palette: None,
            mode: self.render_mode,
            style: self.render_style,
            allow_styled_print: self.allow_styled_print,
        }
    }

    fn logo_options(&self) -> LogoOptions {
        LogoOptions {
            size_ratio: self.logo_size_ratio,
            auto_size: self.logo_auto_size,
            padding_modules: self.logo_padding_modules,
            radius_modules: self.logo_radius_modules,
            clear_background: self.logo_clear_background,
            background_color: None,
        }
    }

    fn scanner_options(&self) -> ScanOptions {
        ScanOptions {
            min_version: self.scan_min_version.min(self.scan_max_version),
            max_version: self.scan_max_version.max(self.scan_min_version),
            structural_tolerance: self.scan_structural_tolerance,
            axis_aligned_fallback: self.scan_axis_fallback,
            max_finder_candidates: self.scan_max_finders,
            locator_max_dimension: self.scan_locator_max_dimension,
            working_max_dimension: 0,
            finder_recovery: true,
            rotation_recovery: true,
            sample_radius: self.scan_sample_radius,
            high_density_sample_radius: self.scan_high_density_sample_radius,
            max_erasure_confidence: self.scan_max_erasure_confidence,
            soft_decoding: self.scan_soft_decoding,
            soft_decode: quadqr::SoftDecodeOptions {
                confidence_threshold: self.scan_soft_confidence,
                max_cells: self.scan_soft_max_cells,
                pair_cells: self.scan_soft_pair_cells,
            },
        }
    }

    fn input_bytes(&self) -> std::result::Result<Vec<u8>, String> {
        match self.input_mode {
            InputMode::Text => Ok(self.generator_text.as_bytes().to_vec()),
            InputMode::File => {
                if self.generator_file_bytes.is_empty() {
                    Err("Choose a file before generating a binary QuadQR.".into())
                } else {
                    Ok(self.generator_file_bytes.clone())
                }
            }
        }
    }

    fn generate(&mut self) {
        self.generated_error.clear();
        let started = Instant::now();
        let payload = match self.input_bytes() {
            Ok(payload) => payload,
            Err(err) => {
                self.generated_error = err;
                return;
            }
        };
        let options = self.encode_options();
        let compression = self.compression_mode();
        let security = match self.security_mode() {
            Ok(v) => v,
            Err(err) => {
                self.generated_error = err;
                return;
            }
        };
        let signing = match self.signing_material() {
            Ok(v) => v,
            Err(err) => {
                self.generated_error = err;
                return;
            }
        };

        let encoded = match (self.input_mode, security.as_ref(), signing.as_ref()) {
            (InputMode::Text, Some(sec), Some((private, key_id))) => encode_secure_signed_text(
                &self.generator_text,
                options,
                compression,
                sec,
                private,
                key_id.as_deref(),
                self.signing_embed_public_key,
            ),
            (InputMode::Text, Some(sec), None) => {
                encode_secure_text(&self.generator_text, options, compression, sec)
            }
            (InputMode::Text, None, Some((private, key_id))) => encode_signed_text(
                &self.generator_text,
                options,
                compression,
                private,
                key_id.as_deref(),
                self.signing_embed_public_key,
            ),
            (InputMode::Text, None, None) => {
                encode_text_with_compression(&self.generator_text, options, compression)
            }
            (InputMode::File, Some(sec), Some((private, key_id))) => encode_secure_signed_bytes(
                &payload,
                options,
                compression,
                sec,
                private,
                key_id.as_deref(),
                self.signing_embed_public_key,
            ),
            (InputMode::File, Some(sec), None) => {
                encode_secure_bytes(&payload, options, compression, sec)
            }
            (InputMode::File, None, Some((private, key_id))) => encode_signed_bytes(
                &payload,
                options,
                compression,
                private,
                key_id.as_deref(),
                self.signing_embed_public_key,
            ),
            (InputMode::File, None, None) => {
                encode_bytes_with_compression(&payload, options, compression)
            }
        };

        match encoded.and_then(|code| {
            let svg = self.render_svg_preview(&code)?;
            Ok((code, svg))
        }) {
            Ok((code, svg)) => {
                let elapsed = started.elapsed().as_secs_f64() * 1000.0;
                self.generated_revision = self.generated_revision.wrapping_add(1);
                self.generated_svg = Some(Arc::from(svg.into_bytes()));
                self.generated_status = format!(
                    "v{} · {}×{} · ECC {} · mask {} · {} bytes · {:.1}% used · {:.2} ms",
                    code.version,
                    code.size,
                    code.size,
                    code.ecc_level.label(),
                    code.mask_id,
                    code.payload_bytes,
                    code.utilization * 100.0,
                    elapsed
                );
                self.code = Some(code);
            }
            Err(err) => {
                self.generated_error = format!("Generate failed: {err}");
            }
        }
    }

    fn render_svg_preview(&self, code: &QuadQrCode) -> quadqr::Result<String> {
        let svg = render_svg(code, self.render_options())?;
        if let Some(logo) = &self.logo {
            embed_logo_in_svg(svg, code, self.render_options(), logo, self.logo_options())
        } else {
            Ok(svg)
        }
    }

    fn save_svg(&mut self) {
        let Some(svg) = &self.generated_svg else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("quadqr-rust.svg")
            .add_filter("SVG", &["svg"])
            .save_file()
        else {
            return;
        };
        match std::fs::write(&path, svg.as_ref()) {
            Ok(()) => self.generated_status = format!("Saved SVG: {}", path.display()),
            Err(err) => self.generated_error = format!("Save SVG failed: {err}"),
        }
    }

    fn save_png(&mut self) {
        let Some(code) = &self.code else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("quadqr-rust.png")
            .add_filter("PNG", &["png"])
            .save_file()
        else {
            return;
        };
        let rendered = if let Some(logo) = &self.logo {
            render_image_with_logo(code, self.render_options(), logo, self.logo_options())
        } else {
            render_image(code, self.render_options())
        };
        match rendered.and_then(|image| {
            image.save(&path)?;
            Ok(())
        }) {
            Ok(()) => self.generated_status = format!("Saved PNG: {}", path.display()),
            Err(err) => self.generated_error = format!("Save PNG failed: {err}"),
        }
    }

    fn choose_generator_file(&mut self) {
        let Some(path) = rfd::FileDialog::new().pick_file() else {
            return;
        };
        match std::fs::read(&path) {
            Ok(bytes) => {
                self.generator_file_name = path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("selected file")
                    .to_owned();
                self.generator_file_bytes = bytes;
                self.generated_error.clear();
            }
            Err(err) => self.generated_error = format!("Could not read file: {err}"),
        }
    }

    fn choose_logo(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"])
            .pick_file()
        else {
            return;
        };
        match image::open(&path) {
            Ok(image) => {
                self.logo = Some(image);
                self.logo_name = path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("logo")
                    .to_owned();
                self.generate();
            }
            Err(err) => self.generated_error = format!("Logo open failed: {err}"),
        }
    }

    fn open_and_scan_image(&mut self, ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"])
            .pick_file()
        else {
            return;
        };
        let started = Instant::now();
        match image::open(&path) {
            Ok(image) => {
                let rgba = image.to_rgba8();
                self.image_texture = Some(load_texture(ctx, "scan-image", &rgba));
                match scan_dynamic_image(&DynamicImage::ImageRgba8(rgba), &self.scanner_options()) {
                    Ok(scan) => {
                        self.image_diagnostics = Some(scan.diagnostics.clone());
                        self.image_decoded = Some(scan.decoded.clone());
                        self.image_verify_status.clear();
                        self.image_result = decoded_display_text(&scan.decoded);
                        self.image_status = format_scan_status(&scan.decoded, &scan.diagnostics);
                    }
                    Err(err) => {
                        self.image_result.clear();
                        self.image_decoded = None;
                        self.image_diagnostics = None;
                        self.image_status = format!(
                            "Scan failed in {:.2} ms: {err}",
                            started.elapsed().as_secs_f64() * 1000.0
                        );
                    }
                }
            }
            Err(err) => self.image_status = format!("Image open failed: {err}"),
        }
    }

    fn scanner_security_mode(&self) -> std::result::Result<SecurityMode, String> {
        match self.scanner_security_kind {
            SecurityKind::Password => {
                if self.scanner_password.is_empty() {
                    return Err("Enter the password first.".into());
                }
                Ok(SecurityMode::password(self.scanner_password.clone()))
            }
            SecurityKind::RawKey => Ok(SecurityMode::raw_key(
                parse_hex_32(&self.scanner_raw_key_hex).map_err(|e| format!("Raw key: {e}"))?,
            )),
            SecurityKind::None => Err("Choose password or raw-key decryption.".into()),
        }
    }

    fn decrypt_image_result(&mut self) {
        let Some(decoded) = self.image_decoded.clone() else {
            return;
        };
        if !decoded.secure {
            self.image_verify_status = "This result is not encrypted.".into();
            return;
        }
        let mode = match self.scanner_security_mode() {
            Ok(mode) => mode,
            Err(err) => {
                self.image_verify_status = err;
                return;
            }
        };
        let trusted = match optional_public_key(&self.scanner_public_key_hex) {
            Ok(key) => key,
            Err(err) => {
                self.image_verify_status = format!("Public key: {err}");
                return;
            }
        };
        let result = if decoded.signed {
            decrypt_and_verify_application(
                decoded,
                &mode,
                trusted.as_ref(),
                self.scanner_allow_embedded_key,
            )
        } else {
            decrypt_application(decoded, &mode).map(|application| (application, None))
        };
        match result {
            Ok((application, verified)) => {
                self.image_result = application
                    .text
                    .unwrap_or_else(|| format!("{} application bytes", application.payload.len()));
                self.image_verify_status = match verified {
                    Some(true) => {
                        "Authenticated decryption succeeded · Ed25519 signature verified.".into()
                    }
                    Some(false) => {
                        "Decryption succeeded, but signature verification failed.".into()
                    }
                    None => "Authenticated decryption succeeded.".into(),
                };
            }
            Err(err) => self.image_verify_status = format!("Decrypt failed: {err}"),
        }
    }

    fn verify_image_signature(&mut self) {
        let Some(decoded) = self.image_decoded.as_ref() else {
            return;
        };
        if decoded.secure {
            self.image_verify_status =
                "Decrypt secure content first. Signature verification applies to the protected plaintext.".into();
            return;
        }
        let trusted = if self.scanner_public_key_hex.trim().is_empty() {
            None
        } else {
            match parse_hex_32(&self.scanner_public_key_hex) {
                Ok(key) => Some(key),
                Err(err) => {
                    self.image_verify_status = format!("Public key: {err}");
                    return;
                }
            }
        };
        match verify_decoded_signature(decoded, trusted.as_ref(), self.scanner_allow_embedded_key) {
            Ok(true) => self.image_verify_status = "Ed25519 signature verified.".into(),
            Ok(false) => self.image_verify_status = "Signature verification failed.".into(),
            Err(err) => self.image_verify_status = format!("Verify failed: {err}"),
        }
    }

    fn start_camera(&mut self) {
        if self.camera_running || self.cameras.is_empty() {
            return;
        }

        let index = self.cameras[self.camera_selected].1.clone();
        let (tx, rx) = crossbeam_channel::bounded::<CameraEvent>(16);
        let stop = Arc::new(AtomicBool::new(false));
        let latest_frame = Arc::new(Mutex::new(None::<SharedCameraFrame>));
        let worker_stop = stop.clone();
        let worker_latest = latest_frame.clone();
        let options = self.scanner_options();
        let scan_every = self.camera_scan_every.max(1);
        let stop_on_decode = self.camera_stop_on_decode;

        self.camera_stop = Some(stop);
        self.camera_rx = Some(rx);
        self.camera_latest_frame = Some(latest_frame);
        self.camera_running = true;
        self.camera_preview_sequence = 0;
        self.camera_preview_frames = 0;
        self.camera_preview_window_started = Instant::now();
        self.camera_capture_fps = 0.0;
        self.camera_preview_fps = 0.0;
        self.camera_preview_latency_ms = 0.0;
        self.camera_scanner_fps = 0.0;
        self.camera_last_scan_ms = 0.0;
        self.camera_status = "Starting native camera…".into();
        self.camera_result.clear();
        self.camera_decoded = None;
        self.camera_diagnostics = None;
        self.camera_verify_status.clear();
        self.camera_texture = None;

        thread::spawn(move || {
            camera_worker(
                index,
                tx,
                worker_stop,
                worker_latest,
                options,
                scan_every,
                stop_on_decode,
            )
        });
    }

    fn stop_camera(&mut self) {
        if let Some(stop) = &self.camera_stop {
            stop.store(true, Ordering::Relaxed);
        }
        self.camera_status = "Stopping camera…".into();
    }

    fn poll_camera(&mut self, ctx: &egui::Context) {
        // Preview and scanner are intentionally decoupled. The capture thread
        // continuously overwrites this one shared latest-frame slot. The UI
        // samples it independently at repaint speed, while the scanner worker
        // consumes the newest sequence it has not seen yet. No stale frame
        // queue can build up.
        if let Some(shared) = &self.camera_latest_frame {
            let latest = shared.lock().ok().and_then(|guard| (*guard).clone());
            if let Some(frame) = latest {
                if frame.sequence != self.camera_preview_sequence {
                    self.camera_preview_sequence = frame.sequence;
                    self.camera_preview_frames += 1;
                    self.camera_preview_latency_ms =
                        frame.captured_at.elapsed().as_secs_f64() * 1000.0;
                    update_camera_texture(ctx, &mut self.camera_texture, &frame.image);

                    let elapsed = self.camera_preview_window_started.elapsed().as_secs_f64();
                    if elapsed >= 0.5 {
                        self.camera_preview_fps = self.camera_preview_frames as f64 / elapsed;
                        self.camera_preview_frames = 0;
                        self.camera_preview_window_started = Instant::now();
                    }
                }
            }
        }

        let Some(rx) = &self.camera_rx else {
            return;
        };
        let events: Vec<_> = rx.try_iter().collect();
        for event in events {
            match event {
                CameraEvent::CaptureStats { fps, width, height } => {
                    self.camera_capture_fps = fps;
                    if self.camera_running && !self.camera_status.starts_with("Decoded") {
                        self.camera_status =
                            format!("Camera running · {width}×{height} · capture {:.1} FPS", fps);
                    }
                }
                CameraEvent::ScannerStats { fps, last_scan_ms } => {
                    self.camera_scanner_fps = fps;
                    self.camera_last_scan_ms = last_scan_ms;
                }
                CameraEvent::Scan {
                    decoded,
                    diagnostics,
                } => {
                    self.camera_last_scan_ms = diagnostics.elapsed_ms;
                    self.camera_result = decoded_display_text(&decoded);
                    self.camera_status = format_scan_status(&decoded, &diagnostics);
                    self.camera_decoded = Some(decoded);
                    self.camera_diagnostics = Some(diagnostics);
                }
                CameraEvent::Error(err) => self.camera_status = format!("Camera error: {err}"),
                CameraEvent::Stopped => {
                    self.camera_running = false;
                    if !self.camera_status.starts_with("Decoded") {
                        self.camera_status = "Camera stopped".into();
                    }
                    self.camera_stop = None;
                    self.camera_latest_frame = None;
                }
            }
        }
    }

    fn decrypt_camera_result(&mut self) {
        let Some(decoded) = self.camera_decoded.clone() else {
            return;
        };
        if !decoded.secure {
            self.camera_verify_status = "This result is not encrypted.".into();
            return;
        }
        let mode = match self.scanner_security_mode() {
            Ok(mode) => mode,
            Err(err) => {
                self.camera_verify_status = err;
                return;
            }
        };
        let trusted = match optional_public_key(&self.scanner_public_key_hex) {
            Ok(key) => key,
            Err(err) => {
                self.camera_verify_status = format!("Public key: {err}");
                return;
            }
        };
        let result = if decoded.signed {
            decrypt_and_verify_application(
                decoded,
                &mode,
                trusted.as_ref(),
                self.scanner_allow_embedded_key,
            )
        } else {
            decrypt_application(decoded, &mode).map(|application| (application, None))
        };
        match result {
            Ok((application, verified)) => {
                self.camera_result = application
                    .text
                    .unwrap_or_else(|| format!("{} application bytes", application.payload.len()));
                self.camera_verify_status = match verified {
                    Some(true) => {
                        "Authenticated decryption succeeded · Ed25519 signature verified.".into()
                    }
                    Some(false) => {
                        "Decryption succeeded, but signature verification failed.".into()
                    }
                    None => "Authenticated decryption succeeded.".into(),
                };
            }
            Err(err) => self.camera_verify_status = format!("Decrypt failed: {err}"),
        }
    }

    fn verify_camera_signature(&mut self) {
        let Some(decoded) = self.camera_decoded.as_ref() else {
            return;
        };
        if decoded.secure {
            self.camera_verify_status =
                "Decrypt secure content before verifying its inner signature.".into();
            return;
        }
        let trusted = if self.scanner_public_key_hex.trim().is_empty() {
            None
        } else {
            match parse_hex_32(&self.scanner_public_key_hex) {
                Ok(key) => Some(key),
                Err(err) => {
                    self.camera_verify_status = format!("Public key: {err}");
                    return;
                }
            }
        };
        match verify_decoded_signature(decoded, trusted.as_ref(), self.scanner_allow_embedded_key) {
            Ok(true) => self.camera_verify_status = "Ed25519 signature verified.".into(),
            Ok(false) => self.camera_verify_status = "Signature verification failed.".into(),
            Err(err) => self.camera_verify_status = format!("Verify failed: {err}"),
        }
    }

    fn rendered_code_image(&self) -> quadqr::Result<RgbaImage> {
        let code = self.code.as_ref().ok_or_else(|| {
            quadqr::QuadQrError::InvalidArgument("generate a QuadQR first".into())
        })?;
        if let Some(logo) = &self.logo {
            render_image_with_logo(code, self.render_options(), logo, self.logo_options())
        } else {
            render_image(code, self.render_options())
        }
    }

    fn run_reliability(&mut self) {
        let Some(code) = self.code.as_ref() else {
            self.reliability_status = "Generate a QuadQR first.".into();
            return;
        };
        let expected_crc = code.crc32;
        let image = match self.rendered_code_image() {
            Ok(image) => image,
            Err(err) => {
                self.reliability_status = format!("Render failed: {err}");
                return;
            }
        };
        let started = Instant::now();
        match run_reliability_lab(
            &image,
            expected_crc,
            self.reliability_suite,
            &self.scanner_options(),
        ) {
            Ok(report) => {
                self.reliability_status = format!(
                    "{} · {}/{} passed · {:.0} ms",
                    report.rating,
                    report.passed,
                    report.total,
                    started.elapsed().as_secs_f64() * 1000.0
                );
                self.reliability_report = Some(report);
            }
            Err(err) => {
                self.reliability_status = format!("Reliability Lab failed: {err}");
                self.reliability_report = None;
            }
        }
    }

    fn run_sweep(&mut self) {
        let Some(code) = self.code.as_ref() else {
            self.sweep_status = "Generate a QuadQR first.".into();
            return;
        };
        let expected_crc = code.crc32;
        let image = match self.rendered_code_image() {
            Ok(image) => image,
            Err(err) => {
                self.sweep_status = format!("Render failed: {err}");
                return;
            }
        };
        let (axis, angles): (&'static str, Vec<f64>) = match self.sweep_axis {
            SweepAxis::Pitch => ("pitch", vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0]),
            SweepAxis::Yaw => ("yaw", vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0]),
            SweepAxis::Roll => ("roll", vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0]),
        };
        match run_perspective_sweep(&image, expected_crc, axis, &angles, &self.scanner_options()) {
            Ok(result) => {
                let outcomes = result
                    .angles
                    .iter()
                    .zip(result.passed.iter())
                    .map(|(angle, passed)| {
                        format!("{angle:.0}° {}", if *passed { "✓" } else { "×" })
                    })
                    .collect::<Vec<_>>()
                    .join("  ");
                let max = result
                    .largest_passing_angle
                    .map(|value| format!("{value:.0}°"))
                    .unwrap_or_else(|| "none".into());
                self.sweep_status = format!("Largest passing {axis}: {max}\n{outcomes}");
            }
            Err(err) => self.sweep_status = format!("Perspective sweep failed: {err}"),
        }
    }

    fn run_benchmark(&mut self) {
        let encoding = if self.high_density {
            CellEncoding::Triangle16
        } else {
            CellEncoding::Rgbw
        };
        let versions = [1u8, 2, 5, 10, 20, 30, 40];
        let payload_sizes = [24usize, 128, 512, 1024, 2048];
        let started = Instant::now();
        match build_capacity_comparison(self.ecc, encoding, &versions).and_then(|capacity| {
            benchmark_codec(
                self.ecc,
                encoding,
                &payload_sizes,
                self.benchmark_iterations,
                self.benchmark_iterations.min(5),
            )
            .map(|rows| (capacity, rows))
        }) {
            Ok((capacity, rows)) => {
                self.benchmark_capacity = capacity;
                self.benchmark_rows = rows;
                self.benchmark_status = format!(
                    "Native release-style codec benchmark completed in {:.0} ms.",
                    started.elapsed().as_secs_f64() * 1000.0
                );
            }
            Err(err) => {
                self.benchmark_status = format!("Benchmark failed: {err}");
                self.benchmark_rows.clear();
                self.benchmark_capacity.clear();
            }
        }
    }

    fn reliability_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Reliability Lab");
            egui::ComboBox::from_id_salt("reliability-suite")
                .selected_text(match self.reliability_suite {
                    ReliabilitySuite::Quick => "Quick",
                    ReliabilitySuite::Full => "Full",
                    ReliabilitySuite::Extreme => "Extreme",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.reliability_suite,
                        ReliabilitySuite::Quick,
                        "Quick",
                    );
                    ui.selectable_value(
                        &mut self.reliability_suite,
                        ReliabilitySuite::Full,
                        "Full",
                    );
                    ui.selectable_value(
                        &mut self.reliability_suite,
                        ReliabilitySuite::Extreme,
                        "Extreme",
                    );
                });
            if ui.button("Run suite").clicked() {
                self.run_reliability();
            }
        });
        ui.label(&self.reliability_status);
        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            ui.label("Perspective sweep");
            egui::ComboBox::from_id_salt("sweep-axis")
                .selected_text(match self.sweep_axis {
                    SweepAxis::Pitch => "Pitch",
                    SweepAxis::Yaw => "Yaw",
                    SweepAxis::Roll => "Z rotation",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.sweep_axis, SweepAxis::Pitch, "Pitch");
                    ui.selectable_value(&mut self.sweep_axis, SweepAxis::Yaw, "Yaw");
                    ui.selectable_value(&mut self.sweep_axis, SweepAxis::Roll, "Z rotation");
                });
            if ui.button("Run sweep").clicked() {
                self.run_sweep();
            }
        });
        if !self.sweep_status.is_empty() {
            ui.monospace(&self.sweep_status);
        }
        ui.add_space(8.0);

        if let Some(report) = &self.reliability_report {
            ui.horizontal(|ui| {
                ui.heading(format!("{} / 100", report.score));
                ui.label(egui::RichText::new(report.rating).strong());
            });
            egui::Grid::new("reliability-categories")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Category");
                    ui.strong("Score");
                    ui.end_row();
                    for (category, score) in &report.category_scores {
                        ui.label(*category);
                        ui.monospace(format!("{score}%"));
                        ui.end_row();
                    }
                });
            ui.add_space(10.0);
            egui::Grid::new("reliability-scenarios")
                .striped(true)
                .min_col_width(90.0)
                .show(ui, |ui| {
                    ui.strong("Scenario");
                    ui.strong("Result");
                    ui.strong("Time");
                    ui.strong("Corrected");
                    ui.strong("Erasures");
                    ui.strong("Path");
                    ui.end_row();
                    for scenario in &report.scenarios {
                        ui.label(&scenario.name);
                        ui.label(if scenario.passed { "PASS" } else { "FAIL" });
                        ui.monospace(format!("{:.1} ms", scenario.elapsed_ms));
                        ui.monospace(scenario.corrected_symbols.to_string());
                        ui.monospace(scenario.erasure_symbols.to_string());
                        ui.monospace(scenario.scan_path.unwrap_or("—"));
                        ui.end_row();
                    }
                });
        } else {
            ui.label("The suite tests blur, exposure, color casts, resampling, sensor damage and perspective against the final rendered symbol.");
        }
    }

    fn benchmark_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Native Benchmark");
            ui.label(format!("ECC {}", self.ecc.label()));
            ui.label(if self.high_density {
                "Triangle16"
            } else {
                "RGBW"
            });
            ui.add(egui::Slider::new(&mut self.benchmark_iterations, 1..=100).text("Iterations"));
            if ui.button("Run benchmark").clicked() {
                self.run_benchmark();
            }
        });
        ui.label(&self.benchmark_status);
        ui.add_space(10.0);

        if !self.benchmark_capacity.is_empty() {
            ui.label(egui::RichText::new("Capacity comparison").strong());
            egui::Grid::new("capacity-comparison")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Version");
                    ui.strong("Size");
                    ui.strong("QuadQR bytes");
                    ui.strong("Std QR bytes");
                    ui.strong("Gain");
                    ui.end_row();
                    for row in &self.benchmark_capacity {
                        ui.monospace(row.version.to_string());
                        ui.monospace(format!("{}×{}", row.size, row.size));
                        ui.monospace(row.quadqr_bytes.to_string());
                        ui.monospace(row.standard_qr_bytes.to_string());
                        ui.monospace(format!("{:+.1}%", row.gain_percent));
                        ui.end_row();
                    }
                });
            ui.add_space(12.0);
        }

        if !self.benchmark_rows.is_empty() {
            ui.label(egui::RichText::new("Codec timings").strong());
            egui::Grid::new("codec-benchmark")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Payload");
                    ui.strong("Version");
                    ui.strong("Encode mean");
                    ui.strong("Encode p95");
                    ui.strong("Decode mean");
                    ui.strong("Decode p95");
                    ui.end_row();
                    for row in &self.benchmark_rows {
                        ui.monospace(format!("{} B", row.payload_bytes));
                        ui.monospace(format!("v{}", row.version));
                        ui.monospace(format!("{:.3} ms", row.encode.mean_ms));
                        ui.monospace(format!("{:.3} ms", row.encode.p95_ms));
                        ui.monospace(format!("{:.3} ms", row.decode.mean_ms));
                        ui.monospace(format!("{:.3} ms", row.decode.p95_ms));
                        ui.end_row();
                    }
                });
        }
        ui.small("Benchmark is for the native matrix codec. Camera FPS also depends on capture, finder detection, perspective sampling and frame resolution.");
    }

    fn nav(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            nav_button(ui, &mut self.tab, Tab::Generator, "Generator");
            nav_button(ui, &mut self.tab, Tab::ImageScanner, "Image Scanner");
            nav_button(ui, &mut self.tab, Tab::CameraScanner, "Camera Scanner");
            nav_button(ui, &mut self.tab, Tab::Reliability, "Reliability Lab");
            nav_button(ui, &mut self.tab, Tab::Benchmark, "Benchmark");
        });
    }

    fn generator_ui(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();

        // On normal desktop widths the Generator behaves like a real two-pane
        // tool: controls and preview each own their vertical scroll state and
        // both consume the full remaining window height. This avoids egui's
        // content-sized ScrollArea behaviour inside an unconstrained horizontal
        // layout, which previously left a large dead area under the controls.
        if available.x >= 880.0 {
            let controls_width = (available.x * 0.31).clamp(360.0, 440.0);
            let separator_width = 28.0;
            let preview_width = (available.x - controls_width - separator_width).max(360.0);
            let pane_height = available.y.max(240.0);

            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(controls_width, pane_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_width(controls_width);

                        ui.horizontal(|ui| {
                            ui.heading("Generator");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add_sized(
                                            [150.0, 36.0],
                                            egui::Button::new(
                                                egui::RichText::new("Generate QuadQR").strong(),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        self.generate();
                                    }
                                },
                            );
                        });
                        ui.label(
                            egui::RichText::new("Payload, encoding, security and rendering")
                                .weak()
                                .size(12.0),
                        );
                        ui.add_space(6.0);

                        let controls_height = ui.available_height().max(120.0);
                        egui::ScrollArea::vertical()
                            .id_salt("generator-controls")
                            .auto_shrink([false, false])
                            .max_height(controls_height)
                            .show(ui, |ui| {
                                ui.set_min_width(controls_width - 18.0);
                                self.generator_controls(ui);
                                ui.add_space(12.0);
                            });
                    },
                );

                ui.separator();

                ui.allocate_ui_with_layout(
                    egui::vec2(preview_width, pane_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_width(preview_width);
                        let right_height = ui.available_height().max(120.0);
                        egui::ScrollArea::vertical()
                            .id_salt("generator-preview")
                            .auto_shrink([false, false])
                            .max_height(right_height)
                            .show(ui, |ui| {
                                ui.set_min_width((preview_width - 18.0).max(320.0));
                                self.generator_preview_ui(ui, pane_height);
                                ui.add_space(12.0);
                            });
                    },
                );
            });
        } else {
            // Responsive fallback for narrow windows. A single vertical scroll
            // keeps every control reachable instead of forcing two cramped panes.
            egui::ScrollArea::vertical()
                .id_salt("generator-narrow")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Generator");
                        if ui.button("Generate QuadQR").clicked() {
                            self.generate();
                        }
                    });
                    self.generator_controls(ui);
                    ui.separator();
                    self.generator_preview_ui(ui, 620.0);
                });
        }
    }

    fn generator_controls(&mut self, ui: &mut egui::Ui) {
        section(ui, "Payload", |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.input_mode, InputMode::Text, "Text");
                ui.selectable_value(&mut self.input_mode, InputMode::File, "Binary file");
            });
            match self.input_mode {
                InputMode::Text => {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.generator_text)
                            .desired_rows(5)
                            .desired_width(f32::INFINITY)
                            .hint_text("Text to encode"),
                    );
                }
                InputMode::File => {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Choose file").clicked() {
                            self.choose_generator_file();
                        }
                        if !self.generator_file_name.is_empty() {
                            ui.label(&self.generator_file_name);
                        }
                    });
                    if !self.generator_file_bytes.is_empty() {
                        ui.small(format!("{} bytes loaded", self.generator_file_bytes.len()));
                    }
                }
            }
        });

        section(ui, "Format", |ui| {
            ui.horizontal(|ui| {
                ui.label("ECC");
                egui::ComboBox::from_id_salt("ecc")
                    .selected_text(self.ecc.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.ecc, EccLevel::L, "L");
                        ui.selectable_value(&mut self.ecc, EccLevel::M, "M");
                        ui.selectable_value(&mut self.ecc, EccLevel::Q, "Q");
                        ui.selectable_value(&mut self.ecc, EccLevel::H, "H");
                    });
            });
            ui.checkbox(&mut self.high_density, "High Density · Triangle16");
            ui.checkbox(&mut self.version_auto, "Automatic version");
            if self.version_auto {
                ui.horizontal(|ui| {
                    ui.label("Range");
                    ui.add(egui::DragValue::new(&mut self.min_version).range(1..=40));
                    ui.label("to");
                    ui.add(egui::DragValue::new(&mut self.max_version).range(1..=40));
                });
            } else {
                ui.add(egui::Slider::new(&mut self.fixed_version, 1..=40).text("Version"));
            }
        });

        section(ui, "Compression 3.0", |ui| {
            egui::ComboBox::from_id_salt("compression")
                .selected_text(compression_label(self.compression_kind))
                .show_ui(ui, |ui| {
                    for (kind, label) in [
                        (CompressionKind::None, "None"),
                        (CompressionKind::Auto, "Auto"),
                        (CompressionKind::Smart, "Smart"),
                        (CompressionKind::Lz, "LZ"),
                        (CompressionKind::Deflate, "DEFLATE"),
                        (CompressionKind::Brotli, "Brotli"),
                    ] {
                        ui.selectable_value(&mut self.compression_kind, kind, label);
                    }
                });
            match self.compression_kind {
                CompressionKind::Lz | CompressionKind::Deflate => {
                    ui.add(egui::Slider::new(&mut self.compression_level, 1..=9).text("Level"));
                }
                CompressionKind::Brotli => {
                    ui.add(egui::Slider::new(&mut self.compression_level, 0..=11).text("Quality"));
                }
                _ => {}
            }
        });

        section(ui, "Security", |ui| {
            egui::ComboBox::from_id_salt("security")
                .selected_text(security_label(self.security_kind))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.security_kind, SecurityKind::None, "None");
                    ui.selectable_value(
                        &mut self.security_kind,
                        SecurityKind::Password,
                        "Password · AES-256-GCM",
                    );
                    ui.selectable_value(
                        &mut self.security_kind,
                        SecurityKind::RawKey,
                        "Raw 256-bit key · AES-256-GCM",
                    );
                });
            match self.security_kind {
                SecurityKind::None => {}
                SecurityKind::Password => {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.security_password)
                            .password(true)
                            .hint_text("Password"),
                    );
                    ui.add(
                        egui::Slider::new(
                            &mut self.security_iterations,
                            MIN_PBKDF2_ITERATIONS..=MAX_PBKDF2_ITERATIONS,
                        )
                        .logarithmic(true)
                        .text("PBKDF2 iterations"),
                    );
                }
                SecurityKind::RawKey => {
                    ui.horizontal(|ui| {
                        if ui.button("Generate key").clicked() {
                            self.security_raw_key_hex = to_hex(&generate_raw_256_key());
                        }
                        if ui.button("Clear").clicked() {
                            self.security_raw_key_hex.clear();
                        }
                    });
                    ui.add(
                        egui::TextEdit::singleline(&mut self.security_raw_key_hex)
                            .password(true)
                            .hint_text("64-character raw key hex"),
                    );
                }
            }
        });

        section(ui, "Signing", |ui| {
            ui.checkbox(&mut self.signing_enabled, "Sign with Ed25519");
            if self.signing_enabled {
                if ui.button("Generate signing keypair").clicked() {
                    let (private, public, key_id) = generate_signing_keypair();
                    self.signing_private_hex = to_hex(&private);
                    self.signing_public_hex = to_hex(&public);
                    self.signing_key_id = to_hex(&key_id);
                }
                ui.label("Private key");
                ui.add(
                    egui::TextEdit::singleline(&mut self.signing_private_hex)
                        .password(true)
                        .hint_text("32-byte private key in hex"),
                );
                ui.label("Public key");
                ui.add(
                    egui::TextEdit::singleline(&mut self.signing_public_hex)
                        .hint_text("Generated public key"),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.signing_key_id)
                        .hint_text("Optional key ID"),
                );
                ui.checkbox(
                    &mut self.signing_embed_public_key,
                    "Embed public key (compatibility / self-check)",
                );
            }
        });

        section(ui, "Appearance", |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Mode");
                egui::ComboBox::from_id_salt("render-mode")
                    .selected_text(match self.render_mode {
                        RenderMode::Screen => "Screen",
                        RenderMode::Print => "Print",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.render_mode, RenderMode::Screen, "Screen");
                        ui.selectable_value(&mut self.render_mode, RenderMode::Print, "Print");
                    });
                ui.label("Style");
                egui::ComboBox::from_id_salt("render-style")
                    .selected_text(render_style_label(self.render_style))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.render_style,
                            RenderStyle::Classic,
                            "Classic",
                        );
                        ui.selectable_value(&mut self.render_style, RenderStyle::Depth, "Depth");
                        ui.selectable_value(&mut self.render_style, RenderStyle::Soft, "Soft");
                        ui.selectable_value(&mut self.render_style, RenderStyle::Inset, "Inset");
                    });
            });
            ui.small(match self.render_mode {
                RenderMode::Screen => "Uses the standard QuadQR screen palette.",
                RenderMode::Print => {
                    "Uses the print-safe QuadQR palette and Classic style by default."
                }
            });
            ui.add(egui::Slider::new(&mut self.render_quiet_zone, 0..=12).text("Quiet zone"));
            ui.add(egui::Slider::new(&mut self.render_module_size, 2..=24).text("SVG module size"));
            if self.render_mode == RenderMode::Print {
                ui.checkbox(&mut self.allow_styled_print, "Allow styled print output");
            }
        });

        section(ui, "Logo", |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Choose logo").clicked() {
                    self.choose_logo();
                }
                if self.logo.is_some() && ui.button("Remove").clicked() {
                    self.logo = None;
                    self.logo_name.clear();
                    self.generate();
                }
            });
            if self.logo.is_some() {
                ui.small(&self.logo_name);
                ui.checkbox(&mut self.logo_auto_size, "Auto safe size");
                if self.logo_auto_size {
                    if let Some(code) = &self.code {
                        let ratio = estimate_safe_logo_size(
                            code,
                            self.render_mode,
                            self.logo_clear_background,
                        );
                        ui.small(format!(
                            "Estimated safe size: {:.1}% of the code",
                            ratio * 100.0
                        ));
                    } else {
                        ui.small("Auto size is calculated from ECC, utilization and output mode.");
                    }
                } else {
                    ui.add(egui::Slider::new(&mut self.logo_size_ratio, 0.05..=0.30).text("Size"));
                }
                ui.add(
                    egui::Slider::new(&mut self.logo_padding_modules, 0.0..=2.0).text("Padding"),
                );
                ui.add(
                    egui::Slider::new(&mut self.logo_radius_modules, 0.0..=2.0)
                        .text("Background radius"),
                );
                ui.checkbox(
                    &mut self.logo_clear_background,
                    "Clear background behind logo",
                );
            }
        });

        if ui
            .add_sized(
                [ui.available_width(), 40.0],
                egui::Button::new(egui::RichText::new("Generate QuadQR").strong()),
            )
            .clicked()
        {
            self.generate();
        }
    }

    fn generator_preview_ui(&mut self, ui: &mut egui::Ui, pane_height: f32) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading("SVG Preview");
                ui.label(
                    egui::RichText::new("Rendered directly by the Rust SVG renderer")
                        .weak()
                        .size(12.0),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Save PNG").clicked() {
                    self.save_png();
                }
                if ui.button("Save SVG").clicked() {
                    self.save_svg();
                }
            });
        });

        ui.add_space(6.0);

        if !self.generated_status.is_empty() {
            ui.label(egui::RichText::new(&self.generated_status).strong());
        }
        if !self.generated_error.is_empty() {
            ui.colored_label(egui::Color32::LIGHT_RED, &self.generated_error);
        }

        // The old preview always tried to reserve >=420 px vertically. On a
        // 768p display that forced metadata below the viewport. Keep the SVG
        // comfortably visible but cap it relative to the actual pane height.
        let preview_side = ui
            .available_width()
            .min((pane_height * 0.58).clamp(280.0, 520.0))
            .clamp(260.0, 560.0);

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_height(preview_side + 20.0);
            ui.vertical_centered(|ui| {
                if let Some(svg) = &self.generated_svg {
                    let uri = format!("bytes://quadqr-preview-{}.svg", self.generated_revision);
                    ui.add(
                        egui::Image::from_bytes(uri, svg.clone())
                            .max_size(egui::vec2(preview_side, preview_side))
                            .alt_text("Generated QuadQR SVG"),
                    );
                } else {
                    ui.add_space(preview_side / 2.0);
                    ui.label("Generate a QuadQR to preview it.");
                }
            });
        });

        if let Some(code) = &self.code {
            ui.add_space(10.0);
            ui.label(egui::RichText::new("Symbol details").strong().size(15.0));
            code_metadata(ui, code);
        }
    }

    fn image_scanner_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(360.0);
                section(ui, "Image Scanner", |ui| {
                    if ui
                        .add_sized(
                            [ui.available_width(), 38.0],
                            egui::Button::new("Choose image and scan"),
                        )
                        .clicked()
                    {
                        self.open_and_scan_image(ctx);
                    }
                    ui.label(&self.image_status);
                });
                scanner_settings_ui(ui, self);
                scanner_crypto_ui(
                    ui,
                    &mut self.scanner_security_kind,
                    &mut self.scanner_password,
                    &mut self.scanner_raw_key_hex,
                    &mut self.scanner_public_key_hex,
                    &mut self.scanner_allow_embedded_key,
                );
                if let Some(decoded) = &self.image_decoded {
                    let secure = decoded.secure;
                    let signed_plain = decoded.signed && !decoded.secure;
                    section(ui, "Actions", |ui| {
                        if secure && ui.button("Decrypt result").clicked() {
                            self.decrypt_image_result();
                        }
                        if signed_plain && ui.button("Verify signature").clicked() {
                            self.verify_image_signature();
                        }
                        if !self.image_verify_status.is_empty() {
                            ui.label(&self.image_verify_status);
                        }
                    });
                }
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.heading("Scan Result");
                if !self.image_result.is_empty() {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.label(egui::RichText::new("Decoded payload").strong());
                        ui.add(
                            egui::TextEdit::multiline(&mut self.image_result)
                                .desired_rows(6)
                                .desired_width(f32::INFINITY),
                        );
                    });
                }
                if let Some(diag) = &self.image_diagnostics {
                    diagnostics_ui(ui, diag);
                }
                if let Some(texture) = &self.image_texture {
                    let size =
                        fit_size(texture.size_vec2(), ui.available_width().min(800.0), 560.0);
                    ui.add(egui::Image::new((texture.id(), size)).corner_radius(8));
                }
            });
        });
    }

    fn camera_scanner_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(360.0);
                section(ui, "Native Camera", |ui| {
                    let selected = self
                        .cameras
                        .get(self.camera_selected)
                        .map(|v| v.0.as_str())
                        .unwrap_or("No cameras");
                    egui::ComboBox::from_id_salt("camera-list")
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for (i, (name, _)) in self.cameras.iter().enumerate() {
                                ui.selectable_value(&mut self.camera_selected, i, name);
                            }
                        });
                    ui.horizontal(|ui| {
                        if ui.button("Refresh").clicked() && !self.camera_running {
                            self.refresh_cameras();
                        }
                        if !self.camera_running {
                            if ui
                                .add_enabled(!self.cameras.is_empty(), egui::Button::new("Start Camera"))
                                .clicked()
                            {
                                self.start_camera();
                            }
                        } else if ui.button("Stop Camera").clicked() {
                            self.stop_camera();
                        }
                    });
                    ui.checkbox(&mut self.camera_stop_on_decode, "Stop camera after successful scan");
                    ui.add(egui::Slider::new(&mut self.camera_scan_every, 1..=6).text("Scan every N frames"));
                    ui.label(&self.camera_status);
                });
                scanner_settings_ui(ui, self);
                scanner_crypto_ui(
                    ui,
                    &mut self.scanner_security_kind,
                    &mut self.scanner_password,
                    &mut self.scanner_raw_key_hex,
                    &mut self.scanner_public_key_hex,
                    &mut self.scanner_allow_embedded_key,
                );
                if let Some(decoded) = &self.camera_decoded {
                    let secure = decoded.secure;
                    let signed_plain = decoded.signed && !decoded.secure;
                    section(ui, "Actions", |ui| {
                        if secure && ui.button("Decrypt result").clicked() {
                            self.decrypt_camera_result();
                        }
                        if signed_plain && ui.button("Verify signature").clicked() {
                            self.verify_camera_signature();
                        }
                        if !self.camera_verify_status.is_empty() {
                            ui.label(&self.camera_verify_status);
                        }
                    });
                }
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.heading("Live Preview");
                ui.small("Preview is independent from scanning and always displays the newest captured frame.");

                if let Some(texture) = &self.camera_texture {
                    let size = fit_size(texture.size_vec2(), ui.available_width().min(980.0), 610.0);
                    ui.add(egui::Image::new((texture.id(), size)).corner_radius(10));
                } else {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_size(egui::vec2(ui.available_width().min(900.0), 420.0));
                        ui.centered_and_justified(|ui| {
                            ui.label("Start the camera to see the native preview.");
                        });
                    });
                }

                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        metric_chip(ui, "Capture", format!("{:.1} FPS", self.camera_capture_fps));
                        metric_chip(ui, "Preview", format!("{:.1} FPS", self.camera_preview_fps));
                        metric_chip(ui, "Scanner", format!("{:.1} FPS", self.camera_scanner_fps));
                        metric_chip(ui, "Last scan", format!("{:.2} ms", self.camera_last_scan_ms));
                        metric_chip(ui, "Preview age", format!("{:.1} ms", self.camera_preview_latency_ms));
                    });
                });
                ui.add_space(6.0);

                if !self.camera_result.is_empty() {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.label(egui::RichText::new("Decoded payload").strong());
                        ui.add(
                            egui::TextEdit::multiline(&mut self.camera_result)
                                .desired_rows(4)
                                .desired_width(f32::INFINITY),
                        );
                    });
                }
                if let Some(diag) = &self.camera_diagnostics {
                    diagnostics_ui(ui, diag);
                }
            });
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_camera(&ctx);
        if self.camera_running {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading("QuadQR Rust");
                ui.label("Native reference implementation · Format v6");
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("RGBW · Spectrum ECC 2.0 · Triangle16").weak());
            });
        });
        ui.add_space(8.0);
        self.nav(ui);
        ui.separator();
        ui.add_space(6.0);

        match self.tab {
            Tab::Generator => self.generator_ui(ui),
            Tab::ImageScanner => {
                egui::ScrollArea::vertical().show(ui, |ui| self.image_scanner_ui(&ctx, ui));
            }
            Tab::CameraScanner => {
                egui::ScrollArea::vertical().show(ui, |ui| self.camera_scanner_ui(ui));
            }
            Tab::Reliability => {
                egui::ScrollArea::vertical().show(ui, |ui| self.reliability_ui(ui));
            }
            Tab::Benchmark => {
                egui::ScrollArea::vertical().show(ui, |ui| self.benchmark_ui(ui));
            }
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(stop) = &self.camera_stop {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

fn camera_worker(
    index: CameraIndex,
    tx: Sender<CameraEvent>,
    stop: Arc<AtomicBool>,
    latest_frame: Arc<Mutex<Option<SharedCameraFrame>>>,
    options: ScanOptions,
    scan_every: u32,
    stop_on_decode: bool,
) {
    let scanner_stop = stop.clone();
    let scanner_latest = latest_frame.clone();
    let scanner_tx = tx.clone();

    // Scanner is a completely separate consumer from camera capture. It never
    // owns the camera and never blocks preview acquisition. If capture produces
    // frames faster than scanning, scanner_latest simply contains the newest
    // frame and intermediate frames disappear automatically.
    let scanner = thread::spawn(move || {
        let mut last_sequence = 0u64;
        let mut scans_in_window = 0u64;
        let mut window_started = Instant::now();
        let mut miss_streak = 0u32;
        let mut fast_options = options.clone();
        fast_options.finder_recovery = false;
        fast_options.rotation_recovery = false;

        while !scanner_stop.load(Ordering::Relaxed) {
            let frame = scanner_latest
                .lock()
                .ok()
                .and_then(|guard| (*guard).clone());

            let Some(frame) = frame else {
                thread::sleep(Duration::from_millis(1));
                continue;
            };

            if frame.sequence == last_sequence {
                thread::sleep(Duration::from_millis(1));
                continue;
            }
            last_sequence = frame.sequence;

            if (frame.sequence - 1) % scan_every.max(1) as u64 != 0 {
                continue;
            }

            let started = Instant::now();
            // Keep ordinary acquisition extremely cheap. Every fourth miss gets
            // the full parallel recovery stack, which catches rotated/color-
            // damaged symbols without making every empty camera frame expensive.
            let use_deep_recovery = miss_streak >= 3 && miss_streak % 4 == 3;
            let result = scan_rgba(
                frame.image.as_ref(),
                if use_deep_recovery {
                    &options
                } else {
                    &fast_options
                },
            );
            if result.is_ok() {
                miss_streak = 0;
            } else {
                miss_streak = miss_streak.saturating_add(1);
            }
            let last_scan_ms = started.elapsed().as_secs_f64() * 1000.0;
            scans_in_window += 1;

            let stats_elapsed = window_started.elapsed().as_secs_f64();
            if stats_elapsed >= 0.5 {
                let _ = scanner_tx.try_send(CameraEvent::ScannerStats {
                    fps: scans_in_window as f64 / stats_elapsed,
                    last_scan_ms,
                });
                scans_in_window = 0;
                window_started = Instant::now();
            }

            if let Ok(scan) = result {
                let sent = scanner_tx.send(CameraEvent::Scan {
                    decoded: scan.decoded,
                    diagnostics: scan.diagnostics,
                });
                if sent.is_ok() && stop_on_decode {
                    scanner_stop.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }
    });

    let requested =
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
    let mut camera = match Camera::new(index, requested) {
        Ok(camera) => camera,
        Err(err) => {
            stop.store(true, Ordering::Relaxed);
            let _ = tx.send(CameraEvent::Error(err.to_string()));
            let _ = scanner.join();
            let _ = tx.send(CameraEvent::Stopped);
            return;
        }
    };
    if let Err(err) = camera.open_stream() {
        stop.store(true, Ordering::Relaxed);
        let _ = tx.send(CameraEvent::Error(err.to_string()));
        let _ = scanner.join();
        let _ = tx.send(CameraEvent::Stopped);
        return;
    }

    let mut sequence = 0u64;
    let mut captured_in_window = 0u64;
    let mut capture_window_started = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        let buffer = match camera.frame() {
            Ok(frame) => frame,
            Err(err) => {
                let _ = tx.try_send(CameraEvent::Error(err.to_string()));
                continue;
            }
        };
        let rgb = match buffer.decode_image::<RgbFormat>() {
            Ok(img) => img,
            Err(err) => {
                let _ = tx.try_send(CameraEvent::Error(err.to_string()));
                continue;
            }
        };
        let (width, height) = rgb.dimensions();
        let raw = rgb.into_raw();
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for chunk in raw.as_chunks::<3>().0 {
            rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
        }
        let Some(image) = RgbaImage::from_raw(width, height, rgba) else {
            continue;
        };

        sequence += 1;
        captured_in_window += 1;
        let frame = SharedCameraFrame {
            sequence,
            captured_at: Instant::now(),
            image: Arc::new(image),
        };
        if let Ok(mut slot) = latest_frame.lock() {
            *slot = Some(frame);
        }

        let stats_elapsed = capture_window_started.elapsed().as_secs_f64();
        if stats_elapsed >= 0.5 {
            let _ = tx.try_send(CameraEvent::CaptureStats {
                fps: captured_in_window as f64 / stats_elapsed,
                width,
                height,
            });
            captured_in_window = 0;
            capture_window_started = Instant::now();
        }
    }

    let _ = camera.stop_stream();
    stop.store(true, Ordering::Relaxed);
    let _ = scanner.join();
    let _ = tx.send(CameraEvent::Stopped);
}

fn configure_ui(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::dark());
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = egui::vec2(9.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.interact_size.y = 32.0;
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(44, 104, 220);
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(44, 104, 220);
    });
}

fn nav_button(ui: &mut egui::Ui, value: &mut Tab, target: Tab, label: &str) {
    let selected = *value == target;
    if ui
        .selectable_label(selected, egui::RichText::new(label).strong())
        .clicked()
    {
        *value = target;
    }
}

fn metric_chip(ui: &mut egui::Ui, label: &str, value: String) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(label).weak());
            ui.monospace(value);
        });
    });
}

fn section(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(egui::RichText::new(title).strong().size(15.0));
        ui.separator();
        add(ui);
    });
    ui.add_space(6.0);
}

fn scanner_settings_ui(ui: &mut egui::Ui, app: &mut App) {
    section(ui, "Scanner Lab", |ui| {
        ui.horizontal(|ui| {
            ui.label("Versions");
            ui.add(egui::DragValue::new(&mut app.scan_min_version).range(1..=40));
            ui.label("to");
            ui.add(egui::DragValue::new(&mut app.scan_max_version).range(1..=40));
        });
        ui.add(
            egui::Slider::new(&mut app.scan_structural_tolerance, 0.0..=0.40)
                .text("Structure tolerance"),
        );
        ui.checkbox(&mut app.scan_axis_fallback, "Axis-aligned fallback");
        ui.add(egui::Slider::new(&mut app.scan_max_finders, 6..=64).text("Finder candidates"));
        ui.add(
            egui::Slider::new(&mut app.scan_locator_max_dimension, 320..=1920)
                .text("Locator max dimension"),
        );
        ui.collapsing("Sampling & Spectrum ECC", |ui| {
            ui.add(
                egui::Slider::new(&mut app.scan_sample_radius, 0.04..=0.30)
                    .text("RGBW sample radius"),
            );
            ui.add(
                egui::Slider::new(&mut app.scan_high_density_sample_radius, 0.02..=0.14)
                    .text("Triangle16 sample radius"),
            );
            ui.add(
                egui::Slider::new(&mut app.scan_max_erasure_confidence, 0.20..=0.95)
                    .text("Max erasure confidence"),
            );
            ui.checkbox(
                &mut app.scan_soft_decoding,
                "Spectrum ECC 2.0 soft decoding",
            );
            if app.scan_soft_decoding {
                ui.add(
                    egui::Slider::new(&mut app.scan_soft_confidence, 0.20..=0.95)
                        .text("Soft-decode confidence"),
                );
                ui.add(egui::Slider::new(&mut app.scan_soft_max_cells, 2..=16).text("Soft cells"));
                let max_pair = app.scan_soft_max_cells.max(2);
                app.scan_soft_pair_cells = app.scan_soft_pair_cells.min(max_pair);
                ui.add(
                    egui::Slider::new(&mut app.scan_soft_pair_cells, 2..=max_pair)
                        .text("Pair-search cells"),
                );
            }
        });
    });
}

fn scanner_crypto_ui(
    ui: &mut egui::Ui,
    security_kind: &mut SecurityKind,
    password: &mut String,
    raw_key_hex: &mut String,
    public_key_hex: &mut String,
    allow_embedded: &mut bool,
) {
    section(ui, "Secure / Signed Decode", |ui| {
        egui::ComboBox::from_id_salt("scanner-security-mode")
            .selected_text(security_label(*security_kind))
            .show_ui(ui, |ui| {
                ui.selectable_value(security_kind, SecurityKind::Password, "Password");
                ui.selectable_value(security_kind, SecurityKind::RawKey, "Raw 256-bit key");
            });
        match *security_kind {
            SecurityKind::Password => {
                ui.add(
                    egui::TextEdit::singleline(password)
                        .password(true)
                        .hint_text("Decrypt password"),
                );
            }
            SecurityKind::RawKey => {
                ui.add(
                    egui::TextEdit::singleline(raw_key_hex).hint_text("64-character raw key hex"),
                );
            }
            SecurityKind::None => {}
        }
        ui.add(
            egui::TextEdit::singleline(public_key_hex).hint_text("Trusted Ed25519 public key hex"),
        );
        ui.checkbox(allow_embedded, "Allow embedded public key");
    });
}

fn diagnostics_ui(ui: &mut egui::Ui, diag: &ScanDiagnostics) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(egui::RichText::new("Diagnostics").strong());
        egui::Grid::new("scan-diagnostics")
            .striped(true)
            .show(ui, |ui| {
                ui.label("Decode time");
                ui.monospace(format!("{:.2} ms", diag.elapsed_ms));
                ui.end_row();
                ui.label("Path");
                ui.monospace(diag.path);
                ui.end_row();
                ui.label("Finder candidates");
                ui.monospace(diag.finder_candidates.to_string());
                ui.end_row();
                ui.label("Geometry attempts");
                ui.monospace(diag.geometry_attempts.to_string());
                ui.end_row();
                ui.label("Selected version");
                ui.monospace(
                    diag.selected_version
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "—".into()),
                );
                ui.end_row();
            });
    });
    ui.add_space(8.0);
}

fn code_metadata(ui: &mut egui::Ui, code: &QuadQrCode) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        egui::Grid::new("code-metadata")
            .striped(true)
            .show(ui, |ui| {
                for (label, value) in [
                    ("Format", format!("v{}", code.format_version)),
                    ("Version", code.version.to_string()),
                    ("Matrix", format!("{} × {}", code.size, code.size)),
                    ("ECC", code.ecc_level.label().to_string()),
                    ("Parity / block", code.ecc_parity_symbols.to_string()),
                    ("ECC blocks", code.ecc_blocks.to_string()),
                    ("Payload bytes", code.payload_bytes.to_string()),
                    ("Capacity", code.capacity_bytes.to_string()),
                    ("Data cells", code.data_cells.to_string()),
                    ("Mask", code.mask_id.to_string()),
                    ("CRC32", format!("{:08x}", code.crc32)),
                    (
                        "High Density",
                        if code.high_density {
                            "Yes".into()
                        } else {
                            "No".into()
                        },
                    ),
                ] {
                    ui.label(label);
                    ui.monospace(value);
                    ui.end_row();
                }
            });
    });
}

fn decoded_display_text(decoded: &DecodeResult) -> String {
    if decoded.secure {
        return format!("Secure QuadQR · {} encrypted bytes", decoded.payload.len());
    }
    match unwrap_application(decoded.clone()) {
        Ok(application) => application
            .text
            .unwrap_or_else(|| format!("{} application bytes", application.payload.len())),
        Err(_) => decoded
            .text
            .clone()
            .unwrap_or_else(|| format!("{} payload bytes", decoded.payload.len())),
    }
}

fn format_scan_status(decoded: &DecodeResult, diag: &ScanDiagnostics) -> String {
    format!(
        "Decoded in {:.2} ms · {} · v{} · ECC {} · corrected {} · erasures {}{}{}",
        diag.elapsed_ms,
        diag.path,
        decoded.version,
        decoded.ecc_level.label(),
        decoded.corrected_header_symbols + decoded.corrected_body_symbols,
        decoded.erasure_symbols,
        if decoded.secure { " · secure" } else { "" },
        if decoded.signed { " · signed" } else { "" },
    )
}

fn compression_label(kind: CompressionKind) -> &'static str {
    match kind {
        CompressionKind::None => "None",
        CompressionKind::Auto => "Auto",
        CompressionKind::Smart => "Smart",
        CompressionKind::Lz => "LZ",
        CompressionKind::Deflate => "DEFLATE",
        CompressionKind::Brotli => "Brotli",
    }
}

fn security_label(kind: SecurityKind) -> &'static str {
    match kind {
        SecurityKind::None => "None",
        SecurityKind::Password => "Password",
        SecurityKind::RawKey => "Raw 256-bit key",
    }
}

fn render_style_label(style: RenderStyle) -> &'static str {
    match style {
        RenderStyle::Classic => "Classic",
        RenderStyle::Depth => "Depth",
        RenderStyle::Soft => "Soft",
        RenderStyle::Inset => "Inset",
    }
}

fn non_empty_bytes(value: &str) -> Option<Vec<u8>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.as_bytes().to_vec())
    }
}

fn optional_public_key(value: &str) -> std::result::Result<Option<[u8; 32]>, String> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_hex_32(value).map(Some)
    }
}

fn parse_hex_32(value: &str) -> std::result::Result<[u8; 32], String> {
    let clean: String = value
        .trim()
        .trim_start_matches("0x")
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ':' && *c != '-')
        .collect();
    if clean.len() != 64 {
        return Err(format!("expected 64 hex characters, got {}", clean.len()));
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16)
            .map_err(|_| "contains non-hex characters".to_string())?;
    }
    Ok(out)
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn embed_logo_in_svg(
    _svg: String,
    code: &QuadQrCode,
    render: RenderOptions,
    logo: &DynamicImage,
    logo_options: LogoOptions,
) -> quadqr::Result<String> {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    logo.write_to(&mut cursor, image::ImageFormat::Png)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(cursor.into_inner());
    let href = format!("data:image/png;base64,{encoded}");
    render_svg_with_logo_href(code, render, &href, logo_options)
}

fn load_texture(ctx: &egui::Context, name: &str, img: &RgbaImage) -> egui::TextureHandle {
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    ctx.load_texture(name.to_owned(), color, egui::TextureOptions::LINEAR)
}

fn update_camera_texture(
    ctx: &egui::Context,
    texture: &mut Option<egui::TextureHandle>,
    img: &RgbaImage,
) {
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    match texture.as_mut() {
        Some(existing) if existing.size() == size => {
            existing.set(color, egui::TextureOptions::LINEAR);
        }
        _ => {
            *texture = Some(ctx.load_texture(
                "camera-frame".to_owned(),
                color,
                egui::TextureOptions::LINEAR,
            ));
        }
    }
}

fn fit_size(source: egui::Vec2, max_width: f32, max_height: f32) -> egui::Vec2 {
    let scale = (max_width / source.x).min(max_height / source.y).min(1.0);
    source * scale
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1380.0, 920.0])
            .with_min_inner_size([1050.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "QuadQR Rust Reference",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
