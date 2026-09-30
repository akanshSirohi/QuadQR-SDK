#![cfg(feature = "image")]

use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use crate::error::{invalid, Result};
use crate::format::QuadQrCode;
use crate::render::{render_image, RenderOptions};
use crate::vision::{scan_rgba, ScanOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReliabilitySuite {
    Quick,
    Full,
    Extreme,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StressDistortion {
    Blur,
    MotionBlur,
    LowExposure,
    HighExposure,
    Shadow,
    Glare,
    WarmCast,
    CoolCast,
    ContrastLoss,
    SensorNoise,
    JpegLike,
    Downscale,
    Perspective,
    Perspective3d,
    Rotation,
}

#[derive(Debug, Clone, Copy)]
pub struct PerspectiveOptions {
    pub pitch_degrees: f64,
    pub yaw_degrees: f64,
    pub roll_degrees: f64,
}

impl Default for PerspectiveOptions {
    fn default() -> Self {
        Self {
            pitch_degrees: 0.0,
            yaw_degrees: 0.0,
            roll_degrees: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StressScenarioResult {
    pub name: String,
    pub category: &'static str,
    pub passed: bool,
    pub elapsed_ms: f64,
    pub corrected_symbols: usize,
    pub erasure_symbols: usize,
    pub scan_path: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ReliabilityReport {
    pub score: u8,
    pub rating: &'static str,
    pub passed: usize,
    pub total: usize,
    pub scenarios: Vec<StressScenarioResult>,
    pub category_scores: BTreeMap<&'static str, u8>,
}

#[derive(Debug, Clone)]
pub struct ScanabilityAssessment {
    pub score: u8,
    pub rating: &'static str,
    pub report: ReliabilityReport,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PerspectiveSweepResult {
    pub axis: &'static str,
    pub angles: Vec<f64>,
    pub passed: Vec<bool>,
    pub largest_passing_angle: Option<f64>,
}

fn clamp_byte(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

fn map_pixels(
    image: &image::RgbaImage,
    mut transform: impl FnMut(u32, u32, [u8; 4]) -> [u8; 4],
) -> image::RgbaImage {
    let mut out = image.clone();
    for (x, y, pixel) in out.enumerate_pixels_mut() {
        *pixel = image::Rgba(transform(x, y, pixel.0));
    }
    out
}

fn bilinear_sample(image: &image::RgbaImage, x: f64, y: f64) -> [u8; 4] {
    let max_x = image.width().saturating_sub(1) as f64;
    let max_y = image.height().saturating_sub(1) as f64;
    if x < 0.0 || y < 0.0 || x > max_x || y > max_y {
        return [255, 255, 255, 255];
    }
    let x0 = x.floor();
    let y0 = y.floor();
    let x1 = (x0 + 1.0).min(max_x);
    let y1 = (y0 + 1.0).min(max_y);
    let fx = x - x0;
    let fy = y - y0;
    let p00 = image.get_pixel(x0 as u32, y0 as u32).0;
    let p10 = image.get_pixel(x1 as u32, y0 as u32).0;
    let p01 = image.get_pixel(x0 as u32, y1 as u32).0;
    let p11 = image.get_pixel(x1 as u32, y1 as u32).0;
    let mut out = [0u8; 4];
    for channel in 0..4 {
        let top = p00[channel] as f64 + (p10[channel] as f64 - p00[channel] as f64) * fx;
        let bottom = p01[channel] as f64 + (p11[channel] as f64 - p01[channel] as f64) * fx;
        out[channel] = clamp_byte(top + (bottom - top) * fy);
    }
    out
}

fn solve_linear_8(mut a: [[f64; 9]; 8]) -> Option<[f64; 8]> {
    for col in 0..8 {
        let mut pivot = col;
        for row in (col + 1)..8 {
            if a[row][col].abs() > a[pivot][col].abs() {
                pivot = row;
            }
        }
        if a[pivot][col].abs() < 1e-10 {
            return None;
        }
        if pivot != col {
            a.swap(col, pivot);
        }
        let divisor = a[col][col];
        for value in &mut a[col][col..] {
            *value /= divisor;
        }
        let pivot_row = a[col];
        for (row_index, row) in a.iter_mut().enumerate() {
            if row_index == col {
                continue;
            }
            let factor = row[col];
            for (value, &pivot_value) in row[col..].iter_mut().zip(pivot_row[col..].iter()) {
                *value -= factor * pivot_value;
            }
        }
    }
    let mut out = [0.0; 8];
    for row in 0..8 {
        out[row] = a[row][8];
    }
    Some(out)
}

fn homography(from: [(f64, f64); 4], to: [(f64, f64); 4]) -> Option<[f64; 9]> {
    let mut system = [[0.0; 9]; 8];
    for i in 0..4 {
        let (x, y) = from[i];
        let (u, v) = to[i];
        system[i * 2] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y, u];
        system[i * 2 + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y, v];
    }
    let solution = solve_linear_8(system)?;
    Some([
        solution[0],
        solution[1],
        solution[2],
        solution[3],
        solution[4],
        solution[5],
        solution[6],
        solution[7],
        1.0,
    ])
}

fn project(h: [f64; 9], x: f64, y: f64) -> Option<(f64, f64)> {
    let denominator = h[6] * x + h[7] * y + h[8];
    if denominator.abs() < 1e-12 {
        return None;
    }
    Some((
        (h[0] * x + h[1] * y + h[2]) / denominator,
        (h[3] * x + h[4] * y + h[5]) / denominator,
    ))
}

fn perspective_corners(width: f64, height: f64, options: PerspectiveOptions) -> [(f64, f64); 4] {
    let pitch = options.pitch_degrees.to_radians();
    let yaw = options.yaw_degrees.to_radians();
    let roll = options.roll_degrees.to_radians();
    let cx = width / 2.0;
    let cy = height / 2.0;
    let scale_x = yaw.cos().abs().max(0.18);
    let scale_y = pitch.cos().abs().max(0.18);
    let yaw_shift = yaw.sin() * width * 0.20;
    let pitch_shift = pitch.sin() * height * 0.20;
    let base = [
        (-cx * scale_x - yaw_shift, -cy * scale_y - pitch_shift),
        (cx * scale_x + yaw_shift, -cy * scale_y + pitch_shift),
        (-cx * scale_x + yaw_shift, cy * scale_y - pitch_shift),
        (cx * scale_x - yaw_shift, cy * scale_y + pitch_shift),
    ];
    let cos_r = roll.cos();
    let sin_r = roll.sin();
    base.map(|(x, y)| (cx + x * cos_r - y * sin_r, cy + x * sin_r + y * cos_r))
}

fn warp_perspective(image: &image::RgbaImage, options: PerspectiveOptions) -> image::RgbaImage {
    let width = image.width();
    let height = image.height();
    let source = [
        (0.0, 0.0),
        (width.saturating_sub(1) as f64, 0.0),
        (0.0, height.saturating_sub(1) as f64),
        (
            width.saturating_sub(1) as f64,
            height.saturating_sub(1) as f64,
        ),
    ];
    let destination = perspective_corners(width as f64, height as f64, options);
    let Some(inverse) = homography(destination, source) else {
        return image.clone();
    };
    let mut out = image::RgbaImage::from_pixel(width, height, image::Rgba([255, 255, 255, 255]));
    for y in 0..height {
        for x in 0..width {
            if let Some((sx, sy)) = project(inverse, x as f64, y as f64) {
                out.put_pixel(x, y, image::Rgba(bilinear_sample(image, sx, sy)));
            }
        }
    }
    out
}

fn motion_blur(image: &image::RgbaImage, radius: u32) -> image::RgbaImage {
    if radius == 0 {
        return image.clone();
    }
    let mut out = image.clone();
    for y in 0..image.height() {
        for x in 0..image.width() {
            let start = x.saturating_sub(radius);
            let end = (x + radius).min(image.width().saturating_sub(1));
            let count = end - start + 1;
            let mut sum = [0u32; 4];
            for sx in start..=end {
                let p = image.get_pixel(sx, y).0;
                for c in 0..4 {
                    sum[c] += p[c] as u32;
                }
            }
            out.put_pixel(
                x,
                y,
                image::Rgba([
                    (sum[0] / count) as u8,
                    (sum[1] / count) as u8,
                    (sum[2] / count) as u8,
                    (sum[3] / count) as u8,
                ]),
            );
        }
    }
    out
}

pub fn apply_stress_distortion(
    image: &image::RgbaImage,
    distortion: StressDistortion,
    amount: f64,
    perspective: PerspectiveOptions,
) -> Result<image::RgbaImage> {
    let amount = amount.clamp(0.0, 1.0);
    let result = match distortion {
        StressDistortion::Blur => image::imageops::blur(image, (0.4 + amount * 2.8) as f32),
        StressDistortion::MotionBlur => motion_blur(image, (1.0 + amount * 7.0).round() as u32),
        StressDistortion::LowExposure => map_pixels(image, |_, _, mut p| {
            let factor = 1.0 - amount * 0.58;
            for channel in &mut p[..3] {
                *channel = clamp_byte(*channel as f64 * factor);
            }
            p
        }),
        StressDistortion::HighExposure => map_pixels(image, |_, _, mut p| {
            let factor = amount * 100.0;
            for channel in &mut p[..3] {
                *channel = clamp_byte(*channel as f64 + factor);
            }
            p
        }),
        StressDistortion::Shadow => {
            let width = image.width().max(1) as f64;
            map_pixels(image, |x, y, mut p| {
                let gradient = ((x as f64 / width) * 0.65
                    + (y as f64 / image.height().max(1) as f64) * 0.35)
                    .clamp(0.0, 1.0);
                let factor = 1.0 - amount * 0.68 * gradient;
                for channel in &mut p[..3] {
                    *channel = clamp_byte(*channel as f64 * factor);
                }
                p
            })
        }
        StressDistortion::Glare => {
            let cx = image.width() as f64 * 0.62;
            let cy = image.height() as f64 * 0.34;
            let radius = image.width().min(image.height()) as f64 * (0.08 + amount * 0.12);
            map_pixels(image, |x, y, mut p| {
                let distance = ((x as f64 - cx).powi(2) + (y as f64 - cy).powi(2)).sqrt();
                let gain = (1.0 - distance / radius).clamp(0.0, 1.0) * amount * 210.0;
                for channel in &mut p[..3] {
                    *channel = clamp_byte(*channel as f64 + gain);
                }
                p
            })
        }
        StressDistortion::WarmCast => map_pixels(image, |_, _, mut p| {
            p[0] = clamp_byte(p[0] as f64 * (1.0 + amount * 0.18));
            p[1] = clamp_byte(p[1] as f64 * (1.0 + amount * 0.06));
            p[2] = clamp_byte(p[2] as f64 * (1.0 - amount * 0.24));
            p
        }),
        StressDistortion::CoolCast => map_pixels(image, |_, _, mut p| {
            p[0] = clamp_byte(p[0] as f64 * (1.0 - amount * 0.18));
            p[1] = clamp_byte(p[1] as f64 * (1.0 + amount * 0.02));
            p[2] = clamp_byte(p[2] as f64 * (1.0 + amount * 0.20));
            p
        }),
        StressDistortion::ContrastLoss => {
            let factor = 1.0 - amount * 0.65;
            map_pixels(image, |_, _, mut p| {
                for channel in &mut p[..3] {
                    *channel = clamp_byte(128.0 + (*channel as f64 - 128.0) * factor);
                }
                p
            })
        }
        StressDistortion::SensorNoise => {
            let mut state = 0x51a9_7f3du32;
            map_pixels(image, |x, y, mut p| {
                state ^= x.wrapping_mul(0x9e37_79b9) ^ y.wrapping_mul(0x85eb_ca6b);
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let noise = ((state & 0xff) as f64 / 255.0 - 0.5) * amount * 55.0;
                for channel in &mut p[..3] {
                    *channel = clamp_byte(*channel as f64 + noise);
                }
                p
            })
        }
        StressDistortion::JpegLike => map_pixels(image, |x, y, mut p| {
            let step = (2.0 + amount * 22.0).round().max(2.0);
            let block_bias = (((x / 8 + y / 8) % 3) as f64 - 1.0) * amount * 4.0;
            for channel in &mut p[..3] {
                *channel = clamp_byte(((*channel as f64 / step).round() * step) + block_bias);
            }
            p
        }),
        StressDistortion::Downscale => {
            let factor = (1.0 - amount * 0.72).clamp(0.18, 1.0);
            let width = (image.width() as f64 * factor).round().max(16.0) as u32;
            let height = (image.height() as f64 * factor).round().max(16.0) as u32;
            let small = image::imageops::resize(
                image,
                width,
                height,
                image::imageops::FilterType::Triangle,
            );
            image::imageops::resize(
                &small,
                image.width(),
                image.height(),
                image::imageops::FilterType::Triangle,
            )
        }
        StressDistortion::Perspective => warp_perspective(
            image,
            PerspectiveOptions {
                pitch_degrees: perspective.pitch_degrees + amount * 18.0,
                yaw_degrees: perspective.yaw_degrees + amount * 26.0,
                roll_degrees: perspective.roll_degrees,
            },
        ),
        StressDistortion::Perspective3d => warp_perspective(image, perspective),
        StressDistortion::Rotation => warp_perspective(
            image,
            PerspectiveOptions {
                roll_degrees: perspective.roll_degrees + amount * 28.0,
                ..perspective
            },
        ),
    };
    Ok(result)
}

fn rating(score: u8) -> &'static str {
    match score {
        90..=100 => "Excellent",
        75..=89 => "Good",
        50..=74 => "Risky",
        _ => "Likely unscannable",
    }
}

fn scenarios_for_suite(
    suite: ReliabilitySuite,
) -> Vec<(
    &'static str,
    &'static str,
    StressDistortion,
    f64,
    PerspectiveOptions,
)> {
    let mut scenarios = vec![
        (
            "baseline",
            "optics",
            StressDistortion::Blur,
            0.0,
            PerspectiveOptions::default(),
        ),
        (
            "lens-blur",
            "optics",
            StressDistortion::Blur,
            0.35,
            PerspectiveOptions::default(),
        ),
        (
            "low-exposure",
            "lighting",
            StressDistortion::LowExposure,
            0.35,
            PerspectiveOptions::default(),
        ),
        (
            "warm-cast",
            "color",
            StressDistortion::WarmCast,
            0.45,
            PerspectiveOptions::default(),
        ),
        (
            "downscale",
            "resampling",
            StressDistortion::Downscale,
            0.45,
            PerspectiveOptions::default(),
        ),
        (
            "perspective",
            "perspective",
            StressDistortion::Perspective3d,
            1.0,
            PerspectiveOptions {
                pitch_degrees: 12.0,
                yaw_degrees: 22.0,
                roll_degrees: 4.0,
            },
        ),
    ];
    if suite != ReliabilitySuite::Quick {
        scenarios.extend([
            (
                "motion-blur",
                "optics",
                StressDistortion::MotionBlur,
                0.35,
                PerspectiveOptions::default(),
            ),
            (
                "high-exposure",
                "lighting",
                StressDistortion::HighExposure,
                0.35,
                PerspectiveOptions::default(),
            ),
            (
                "gradient-shadow",
                "lighting",
                StressDistortion::Shadow,
                0.50,
                PerspectiveOptions::default(),
            ),
            (
                "glare",
                "lighting",
                StressDistortion::Glare,
                0.45,
                PerspectiveOptions::default(),
            ),
            (
                "cool-cast",
                "color",
                StressDistortion::CoolCast,
                0.45,
                PerspectiveOptions::default(),
            ),
            (
                "contrast-loss",
                "color",
                StressDistortion::ContrastLoss,
                0.45,
                PerspectiveOptions::default(),
            ),
            (
                "sensor-noise",
                "sensor",
                StressDistortion::SensorNoise,
                0.40,
                PerspectiveOptions::default(),
            ),
            (
                "jpeg-like",
                "sensor",
                StressDistortion::JpegLike,
                0.45,
                PerspectiveOptions::default(),
            ),
            (
                "rotation",
                "perspective",
                StressDistortion::Rotation,
                0.55,
                PerspectiveOptions {
                    roll_degrees: 8.0,
                    ..PerspectiveOptions::default()
                },
            ),
            (
                "3d-tilt",
                "perspective",
                StressDistortion::Perspective3d,
                1.0,
                PerspectiveOptions {
                    pitch_degrees: 20.0,
                    yaw_degrees: 32.0,
                    roll_degrees: 8.0,
                },
            ),
        ]);
    }
    if suite == ReliabilitySuite::Extreme {
        scenarios.extend([
            (
                "extreme-yaw",
                "perspective",
                StressDistortion::Perspective3d,
                1.0,
                PerspectiveOptions {
                    pitch_degrees: 10.0,
                    yaw_degrees: 48.0,
                    roll_degrees: 0.0,
                },
            ),
            (
                "extreme-pitch",
                "perspective",
                StressDistortion::Perspective3d,
                1.0,
                PerspectiveOptions {
                    pitch_degrees: 42.0,
                    yaw_degrees: 12.0,
                    roll_degrees: 7.0,
                },
            ),
            (
                "extreme-combined",
                "perspective",
                StressDistortion::Perspective3d,
                1.0,
                PerspectiveOptions {
                    pitch_degrees: 34.0,
                    yaw_degrees: 42.0,
                    roll_degrees: 14.0,
                },
            ),
        ]);
    }
    scenarios
}

pub fn run_reliability_lab(
    image: &image::RgbaImage,
    expected_crc32: u32,
    suite: ReliabilitySuite,
    scan_options: &ScanOptions,
) -> Result<ReliabilityReport> {
    let mut results = Vec::new();
    for (name, category, distortion, amount, perspective) in scenarios_for_suite(suite) {
        let transformed = if name == "baseline" {
            image.clone()
        } else {
            apply_stress_distortion(image, distortion, amount, perspective)?
        };
        let started = Instant::now();
        let scan = scan_rgba(&transformed, scan_options);
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        let (passed, corrected_symbols, erasure_symbols, scan_path) = match scan {
            Ok(result) => (
                result.decoded.crc32 == expected_crc32,
                result.decoded.corrected_header_symbols + result.decoded.corrected_body_symbols,
                result.decoded.erasure_symbols,
                Some(result.diagnostics.path),
            ),
            Err(_) => (false, 0, 0, None),
        };
        results.push(StressScenarioResult {
            name: name.to_owned(),
            category,
            passed,
            elapsed_ms,
            corrected_symbols,
            erasure_symbols,
            scan_path,
        });
    }
    let total = results.len();
    if total == 0 {
        return Err(invalid("reliability suite has no scenarios"));
    }
    let passed = results.iter().filter(|result| result.passed).count();
    let score = ((passed as f64 / total as f64) * 100.0).round() as u8;
    let mut grouped: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    for result in &results {
        let entry = grouped.entry(result.category).or_insert((0, 0));
        entry.1 += 1;
        if result.passed {
            entry.0 += 1;
        }
    }
    let category_scores = grouped
        .into_iter()
        .map(|(category, (category_passed, category_total))| {
            (
                category,
                ((category_passed as f64 / category_total as f64) * 100.0).round() as u8,
            )
        })
        .collect();
    Ok(ReliabilityReport {
        score,
        rating: rating(score),
        passed,
        total,
        scenarios: results,
        category_scores,
    })
}

pub fn run_image_stress_test(
    image: &image::RgbaImage,
    expected_crc32: u32,
    scan_options: &ScanOptions,
) -> Result<ReliabilityReport> {
    run_reliability_lab(image, expected_crc32, ReliabilitySuite::Quick, scan_options)
}

pub fn assess_scanability(
    code: &QuadQrCode,
    render_options: RenderOptions,
    suite: ReliabilitySuite,
    scan_options: &ScanOptions,
) -> Result<ScanabilityAssessment> {
    let image = render_image(code, render_options)?;
    let report = run_reliability_lab(&image, code.crc32, suite, scan_options)?;
    let mut recommendations = Vec::new();
    if report.score < 90 {
        recommendations
            .push("Increase rendered pixels per module or physical module size.".to_owned());
    }
    if code.utilization > 0.82 {
        recommendations.push(
            "Use a larger QuadQR version or higher-density mode only after camera validation."
                .to_owned(),
        );
    }
    if code.high_density && report.score < 90 {
        recommendations.push(
            "Triangle16 needs more pixels per module than normal RGBW under perspective and blur."
                .to_owned(),
        );
    }
    if render_options.mode == crate::render::RenderMode::Print {
        recommendations.push(
            "Validate the final printer, paper, lighting and camera combination physically."
                .to_owned(),
        );
    }
    Ok(ScanabilityAssessment {
        score: report.score,
        rating: report.rating,
        report,
        recommendations,
    })
}

pub fn run_perspective_sweep(
    image: &image::RgbaImage,
    expected_crc32: u32,
    axis: &'static str,
    angles: &[f64],
    scan_options: &ScanOptions,
) -> Result<PerspectiveSweepResult> {
    if !matches!(axis, "pitch" | "yaw" | "roll") {
        return Err(invalid(
            "perspective sweep axis must be pitch, yaw, or roll",
        ));
    }
    let mut passed = Vec::with_capacity(angles.len());
    let mut largest_passing_angle = None;
    for &angle in angles {
        let perspective = match axis {
            "pitch" => PerspectiveOptions {
                pitch_degrees: angle,
                ..PerspectiveOptions::default()
            },
            "yaw" => PerspectiveOptions {
                yaw_degrees: angle,
                ..PerspectiveOptions::default()
            },
            _ => PerspectiveOptions {
                roll_degrees: angle,
                ..PerspectiveOptions::default()
            },
        };
        let transformed =
            apply_stress_distortion(image, StressDistortion::Perspective3d, 1.0, perspective)?;
        let ok = scan_rgba(&transformed, scan_options)
            .map(|result| result.decoded.crc32 == expected_crc32)
            .unwrap_or(false);
        passed.push(ok);
        if ok {
            largest_passing_angle = Some(angle);
        }
    }
    Ok(PerspectiveSweepResult {
        axis,
        angles: angles.to_vec(),
        passed,
        largest_passing_angle,
    })
}
