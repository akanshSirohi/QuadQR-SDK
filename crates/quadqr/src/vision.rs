#[cfg(not(feature = "parallel"))]
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
#[cfg(feature = "parallel")]
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
#[cfg(feature = "parallel")]
use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

#[cfg(feature = "parallel")]
struct ScanAttemptCounter(AtomicUsize);

#[cfg(not(feature = "parallel"))]
struct ScanAttemptCounter(Cell<usize>);

#[cfg(feature = "parallel")]
impl ScanAttemptCounter {
    fn new() -> Self {
        Self(AtomicUsize::new(0))
    }
    fn increment(&self) {
        self.0.fetch_add(1, AtomicOrdering::Relaxed);
    }
    fn get(&self) -> usize {
        self.0.load(AtomicOrdering::Relaxed)
    }
}

#[cfg(not(feature = "parallel"))]
impl ScanAttemptCounter {
    fn new() -> Self {
        Self(Cell::new(0))
    }
    fn increment(&self) {
        self.0.set(self.0.get() + 1);
    }
    fn get(&self) -> usize {
        self.0.get()
    }
}

#[cfg(feature = "parallel")]
struct ScanErrorCollector(Mutex<Vec<String>>);

#[cfg(not(feature = "parallel"))]
struct ScanErrorCollector(RefCell<Vec<String>>);

#[cfg(feature = "parallel")]
impl ScanErrorCollector {
    fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }
    fn push_limited(&self, error: String, limit: usize) {
        if let Ok(mut collected) = self.0.lock() {
            if collected.len() < limit {
                collected.push(error);
            }
        }
    }
    fn into_vec(self) -> Vec<String> {
        self.0
            .into_inner()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(not(feature = "parallel"))]
impl ScanErrorCollector {
    fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }
    fn push_limited(&self, error: String, limit: usize) {
        let mut collected = self.0.borrow_mut();
        if collected.len() < limit {
            collected.push(error);
        }
    }
    fn into_vec(self) -> Vec<String> {
        self.0.into_inner()
    }
}

use crate::error::{QuadQrError, Result};
use crate::format::{
    decode_matrix_with_options, layout_for_version, DecodeOptions, DecodeResult, SoftDecodeOptions,
    BLACK, BLUE, GREEN, RED, WHITE,
};
use crate::geometry::{
    alignment_patterns, size_for_version, AlignmentProfile, MAX_VERSION, MIN_VERSION,
};

#[derive(Debug, Clone, Copy, Default)]
struct Rgb {
    r: f64,
    g: f64,
    b: f64,
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct FinderPoint {
    pub x: f64,
    pub y: f64,
    pub module_size: f64,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct ScanDiagnostics {
    pub elapsed_ms: f64,
    pub finder_candidates: usize,
    pub geometry_attempts: usize,
    pub selected_version: Option<u8>,
    pub path: &'static str,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub decoded: DecodeResult,
    pub diagnostics: ScanDiagnostics,
    pub finders: Vec<FinderPoint>,
}

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub min_version: u8,
    pub max_version: u8,
    pub structural_tolerance: f64,
    pub axis_aligned_fallback: bool,
    pub max_finder_candidates: usize,
    pub locator_max_dimension: u32,
    /// Optional cap for the full working image. `0` keeps the original size.
    /// Live-camera adapters can set this independently from the locator cap so
    /// all expensive grayscale, geometry and perspective work stays bounded.
    pub working_max_dimension: u32,
    /// Enable threshold/color finder recovery after the cheap locator pass.
    pub finder_recovery: bool,
    /// Enable rotated locator recovery when ordinary finder scans still miss.
    pub rotation_recovery: bool,
    pub sample_radius: f64,
    pub high_density_sample_radius: f64,
    pub max_erasure_confidence: f32,
    pub soft_decoding: bool,
    pub soft_decode: SoftDecodeOptions,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            min_version: MIN_VERSION,
            max_version: MAX_VERSION,
            structural_tolerance: 0.18,
            axis_aligned_fallback: true,
            max_finder_candidates: 24,
            locator_max_dimension: 960,
            working_max_dimension: 0,
            finder_recovery: true,
            rotation_recovery: true,
            sample_radius: 0.16,
            high_density_sample_radius: 0.065,
            max_erasure_confidence: 0.68,
            soft_decoding: true,
            soft_decode: SoftDecodeOptions::default(),
        }
    }
}

fn rgb_distance(a: Rgb, b: Rgb) -> f64 {
    ((a.r - b.r).powi(2) + (a.g - b.g).powi(2) + (a.b - b.b).powi(2)).sqrt()
}

fn pixel_rgb(image: &image::RgbaImage, x: f64, y: f64) -> Rgb {
    let x = x.round().clamp(0.0, image.width().saturating_sub(1) as f64) as u32;
    let y = y
        .round()
        .clamp(0.0, image.height().saturating_sub(1) as f64) as u32;
    let p = image.get_pixel(x, y).0;
    let a = p[3] as f64 / 255.0;
    Rgb {
        r: p[0] as f64 * a + 255.0 * (1.0 - a),
        g: p[1] as f64 * a + 255.0 * (1.0 - a),
        b: p[2] as f64 * a + 255.0 * (1.0 - a),
    }
}

fn bilinear_rgb(image: &image::RgbaImage, x: f64, y: f64) -> Rgb {
    let x0 = x.floor().clamp(0.0, image.width().saturating_sub(1) as f64);
    let y0 = y
        .floor()
        .clamp(0.0, image.height().saturating_sub(1) as f64);
    let x1 = (x0 + 1.0).min(image.width().saturating_sub(1) as f64);
    let y1 = (y0 + 1.0).min(image.height().saturating_sub(1) as f64);
    let fx = (x - x0).clamp(0.0, 1.0);
    let fy = (y - y0).clamp(0.0, 1.0);
    let p00 = pixel_rgb(image, x0, y0);
    let p10 = pixel_rgb(image, x1, y0);
    let p01 = pixel_rgb(image, x0, y1);
    let p11 = pixel_rgb(image, x1, y1);
    let mix = |a: f64, b: f64, t: f64| a + (b - a) * t;
    Rgb {
        r: mix(mix(p00.r, p10.r, fx), mix(p01.r, p11.r, fx), fy),
        g: mix(mix(p00.g, p10.g, fx), mix(p01.g, p11.g, fx), fy),
        b: mix(mix(p00.b, p10.b, fx), mix(p01.b, p11.b, fx), fy),
    }
}

fn build_value_gray(image: &image::RgbaImage) -> Vec<u8> {
    image
        .pixels()
        .map(|p| {
            let a = p[3] as f64 / 255.0;
            let r = p[0] as f64 * a + 255.0 * (1.0 - a);
            let g = p[1] as f64 * a + 255.0 * (1.0 - a);
            let b = p[2] as f64 * a + 255.0 * (1.0 - a);
            r.max(g).max(b).round() as u8
        })
        .collect()
}

fn build_luma_gray(image: &image::RgbaImage) -> Vec<u8> {
    image
        .pixels()
        .map(|p| {
            let a = p[3] as f64 / 255.0;
            let r = p[0] as f64 * a + 255.0 * (1.0 - a);
            let g = p[1] as f64 * a + 255.0 * (1.0 - a);
            let b = p[2] as f64 * a + 255.0 * (1.0 - a);
            (0.2126 * r + 0.7152 * g + 0.0722 * b)
                .round()
                .clamp(0.0, 255.0) as u8
        })
        .collect()
}

fn hybrid_local_binary(gray: &[u8], width: usize, height: usize) -> Option<Vec<u8>> {
    const BLOCK: usize = 8;
    const MIN_DIMENSION: usize = 40;
    const MIN_DYNAMIC_RANGE: i32 = 24;
    if width < MIN_DIMENSION || height < MIN_DIMENSION {
        return None;
    }
    let sub_width = width.div_ceil(BLOCK);
    let sub_height = height.div_ceil(BLOCK);
    let mut black_points = vec![0u16; sub_width * sub_height];
    let max_x_offset = width.saturating_sub(BLOCK);
    let max_y_offset = height.saturating_sub(BLOCK);
    for by in 0..sub_height {
        let y0 = (by * BLOCK).min(max_y_offset);
        for bx in 0..sub_width {
            let x0 = (bx * BLOCK).min(max_x_offset);
            let mut sum = 0u32;
            let mut min_value = 255u8;
            let mut max_value = 0u8;
            for yy in 0..BLOCK {
                let row = (y0 + yy) * width + x0;
                for xx in 0..BLOCK {
                    let value = gray[row + xx];
                    sum += value as u32;
                    min_value = min_value.min(value);
                    max_value = max_value.max(value);
                }
            }
            let mut average = (sum >> 6) as u16;
            if max_value as i32 - min_value as i32 <= MIN_DYNAMIC_RANGE {
                average = (min_value >> 1) as u16;
                if by > 0 && bx > 0 {
                    let above = black_points[(by - 1) * sub_width + bx];
                    let left = black_points[by * sub_width + bx - 1];
                    let diagonal = black_points[(by - 1) * sub_width + bx - 1];
                    let neighbor = (above + 2 * left + diagonal) >> 2;
                    if (min_value as u16) < neighbor {
                        average = neighbor;
                    }
                }
            }
            black_points[by * sub_width + bx] = average;
        }
    }
    let mut binary = vec![0u8; width * height];
    for by in 0..sub_height {
        let y0 = (by * BLOCK).min(max_y_offset);
        let center_y = by.clamp(2, sub_height.saturating_sub(3).max(2));
        for bx in 0..sub_width {
            let x0 = (bx * BLOCK).min(max_x_offset);
            let center_x = bx.clamp(2, sub_width.saturating_sub(3).max(2));
            let mut sum = 0u32;
            let mut count = 0u32;
            for oy in -2isize..=2 {
                let py = (center_y as isize + oy).clamp(0, sub_height as isize - 1) as usize;
                for ox in -2isize..=2 {
                    let px = (center_x as isize + ox).clamp(0, sub_width as isize - 1) as usize;
                    sum += black_points[py * sub_width + px] as u32;
                    count += 1;
                }
            }
            let threshold = (sum / count.max(1)) as u8;
            for yy in 0..BLOCK {
                let y = y0 + yy;
                if y >= height {
                    break;
                }
                for xx in 0..BLOCK {
                    let x = x0 + xx;
                    if x >= width {
                        break;
                    }
                    if gray[y * width + x] <= threshold {
                        binary[y * width + x] = 1;
                    }
                }
            }
        }
    }
    Some(binary)
}

fn histogram_percentile(histogram: &[u32; 256], total: u32, fraction: f64) -> usize {
    if total == 0 {
        return 0;
    }
    let target = ((total as f64 * fraction.clamp(0.0, 1.0)).ceil() as u32).max(1);
    let mut seen = 0u32;
    for (index, count) in histogram.iter().enumerate() {
        seen += *count;
        if seen >= target {
            return index;
        }
    }
    255
}

fn build_auto_color_value_gray(image: &image::RgbaImage, analysis_inset: f64) -> Vec<u8> {
    let inset = analysis_inset.clamp(0.0, 0.30);
    let x0 = (image.width() as f64 * inset).floor() as u32;
    let y0 = (image.height() as f64 * inset).floor() as u32;
    let x1 = (image.width() as f64 * (1.0 - inset))
        .ceil()
        .max(x0 as f64 + 1.0) as u32;
    let y1 = (image.height() as f64 * (1.0 - inset))
        .ceil()
        .max(y0 as f64 + 1.0) as u32;
    let analysis_width = x1.saturating_sub(x0).max(1);
    let analysis_height = y1.saturating_sub(y0).max(1);
    let pixel_count = analysis_width as usize * analysis_height as usize;
    let step = (pixel_count / 90_000).max(1);
    let mut histograms = [[0u32; 256]; 3];
    let mut samples = 0u32;
    for index in (0..pixel_count).step_by(step) {
        let x = x0 + (index % analysis_width as usize) as u32;
        let y = y0 + (index / analysis_width as usize) as u32;
        let p = image
            .get_pixel(x.min(image.width() - 1), y.min(image.height() - 1))
            .0;
        let a = p[3] as f64 / 255.0;
        let values = [
            p[0] as f64 * a + 255.0 * (1.0 - a),
            p[1] as f64 * a + 255.0 * (1.0 - a),
            p[2] as f64 * a + 255.0 * (1.0 - a),
        ];
        for channel in 0..3 {
            histograms[channel][values[channel].round().clamp(0.0, 255.0) as usize] += 1;
        }
        samples += 1;
    }
    let lows = [
        histogram_percentile(&histograms[0], samples, 0.0001),
        histogram_percentile(&histograms[1], samples, 0.0001),
        histogram_percentile(&histograms[2], samples, 0.0001),
    ];
    let references = [
        histogram_percentile(&histograms[0], samples, 0.95),
        histogram_percentile(&histograms[1], samples, 0.95),
        histogram_percentile(&histograms[2], samples, 0.95),
    ];
    let mut highs = [0.0f64; 3];
    for channel in 0..3 {
        let observed_range = (references[channel] as f64 - lows[channel] as f64).max(72.0);
        highs[channel] = lows[channel] as f64 + observed_range * 255.0 / 190.0;
    }
    let map = |value: f64, channel: usize| {
        ((value - lows[channel] as f64) * 255.0 / (highs[channel] - lows[channel] as f64).max(1.0))
            .clamp(0.0, 255.0)
    };
    image
        .pixels()
        .map(|p| {
            let a = p[3] as f64 / 255.0;
            let r = map(p[0] as f64 * a + 255.0 * (1.0 - a), 0);
            let g = map(p[1] as f64 * a + 255.0 * (1.0 - a), 1);
            let b = map(p[2] as f64 * a + 255.0 * (1.0 - a), 2);
            r.max(g).max(b).round() as u8
        })
        .collect()
}
fn otsu(gray: &[u8]) -> u8 {
    let mut hist = [0u32; 256];
    for &v in gray {
        hist[v as usize] += 1;
    }
    let total = gray.len() as f64;
    let sum: f64 = hist
        .iter()
        .enumerate()
        .map(|(i, &n)| i as f64 * n as f64)
        .sum();
    let mut wb = 0f64;
    let mut sb = 0f64;
    let mut best = -1f64;
    let mut threshold = 127;
    for (t, &count) in hist.iter().enumerate() {
        wb += count as f64;
        if wb == 0.0 {
            continue;
        }
        let wf = total - wb;
        if wf == 0.0 {
            break;
        }
        sb += t as f64 * count as f64;
        let mb = sb / wb;
        let mf = (sum - sb) / wf;
        let var = wb * wf * (mb - mf).powi(2);
        if var > best {
            best = var;
            threshold = t as u8;
        }
    }
    threshold
}
fn binarize(gray: &[u8], threshold: u8) -> Vec<u8> {
    gray.iter()
        .map(|&v| if v <= threshold { 1 } else { 0 })
        .collect()
}

#[derive(Debug, Clone, Copy)]
enum FinderRecoveryStrategy {
    HybridValue,
    ValueHigh,
    ValueLow,
    AutoColor { inset: f64, threshold_offset: u8 },
    Luma,
}

fn build_recovery_binary(
    image: &image::RgbaImage,
    value_gray: &[u8],
    value_threshold: u8,
    strategy: FinderRecoveryStrategy,
) -> Option<Vec<u8>> {
    let width = image.width() as usize;
    let height = image.height() as usize;
    match strategy {
        FinderRecoveryStrategy::HybridValue => hybrid_local_binary(value_gray, width, height),
        FinderRecoveryStrategy::ValueHigh => {
            let threshold = value_threshold.saturating_add(18).clamp(8, 247);
            Some(binarize(value_gray, threshold))
        }
        FinderRecoveryStrategy::ValueLow => {
            let threshold = value_threshold.saturating_sub(14).clamp(8, 247);
            Some(binarize(value_gray, threshold))
        }
        FinderRecoveryStrategy::AutoColor {
            inset,
            threshold_offset,
        } => {
            let gray = build_auto_color_value_gray(image, inset);
            let threshold = otsu(&gray).saturating_add(threshold_offset).clamp(8, 247);
            Some(binarize(&gray, threshold))
        }
        FinderRecoveryStrategy::Luma => {
            let gray = build_luma_gray(image);
            Some(binarize(&gray, otsu(&gray)))
        }
    }
}

fn build_alignment_recovery_binary(image: &image::RgbaImage) -> Option<Vec<u8>> {
    let width = image.width() as usize;
    let height = image.height() as usize;
    let gray = build_auto_color_value_gray(image, 0.10);
    // One combined recovery image: channel-level normalization first, local
    // threshold second. This targets exposure/cast/shadow damage without
    // spawning a matrix of full geometry retries.
    hybrid_local_binary(&gray, width, height).or_else(|| {
        let threshold = otsu(&gray);
        Some(binarize(&gray, threshold))
    })
}

fn rotate_binary(binary: &[u8], width: usize, height: usize, angle_radians: f64) -> Vec<u8> {
    let mut rotated = vec![0u8; width * height];
    let cx = (width as f64 - 1.0) * 0.5;
    let cy = (height as f64 - 1.0) * 0.5;
    let cos = angle_radians.cos();
    let sin = angle_radians.sin();
    for y in 0..height {
        let dy = y as f64 - cy;
        let row = y * width;
        for x in 0..width {
            let dx = x as f64 - cx;
            // Inverse map destination -> source.
            let sx = cx + cos * dx + sin * dy;
            let sy = cy - sin * dx + cos * dy;
            if sx >= 0.0 && sy >= 0.0 && sx < width as f64 && sy < height as f64 {
                let source_x = sx.round().clamp(0.0, width.saturating_sub(1) as f64) as usize;
                let source_y = sy.round().clamp(0.0, height.saturating_sub(1) as f64) as usize;
                rotated[row + x] = binary[source_y * width + source_x];
            }
        }
    }
    rotated
}

fn unrotate_finders(finders: &mut [FinderPoint], width: usize, height: usize, angle_radians: f64) {
    let cx = (width as f64 - 1.0) * 0.5;
    let cy = (height as f64 - 1.0) * 0.5;
    let cos = angle_radians.cos();
    let sin = angle_radians.sin();
    for finder in finders {
        let dx = finder.x - cx;
        let dy = finder.y - cy;
        // A source feature was moved by +angle into the rotated image. Map the
        // detected rotated coordinate back with -angle.
        finder.x = cx + cos * dx + sin * dy;
        finder.y = cy - sin * dx + cos * dy;
    }
}

fn finder_quality(finders: &[FinderPoint]) -> f64 {
    finders.iter().take(6).map(|finder| finder.score).sum()
}

fn ratio_score(lengths: [usize; 5]) -> Option<f64> {
    let total: usize = lengths.iter().sum();
    if total < 7 {
        return None;
    }
    let module = total as f64 / 7.0;
    let expected = [module, module, 3.0 * module, module, module];
    let mut score = 0.0;
    for i in 0..5 {
        let tolerance = if i == 2 { module * 1.25 } else { module * 0.8 };
        let diff = (lengths[i] as f64 - expected[i]).abs();
        if diff > tolerance {
            return None;
        }
        score += diff / expected[i].max(1.0);
    }
    Some(score)
}
fn center_from_end(s: [usize; 5], end: usize) -> f64 {
    end as f64 - s[4] as f64 - s[3] as f64 - s[2] as f64 / 2.0
}

fn vertical_cross(
    binary: &[u8],
    w: usize,
    h: usize,
    start_y: f64,
    x: f64,
    max_count: usize,
    original_total: usize,
) -> Option<(f64, f64, f64)> {
    let x = x.round().clamp(0.0, (w - 1) as f64) as usize;
    let mut s = [0usize; 5];
    let mut y = start_y.round().clamp(0.0, (h - 1) as f64) as isize;
    while y >= 0 && binary[y as usize * w + x] != 0 {
        s[2] += 1;
        y -= 1;
    }
    if y < 0 {
        return None;
    }
    while y >= 0 && binary[y as usize * w + x] == 0 && s[1] <= max_count {
        s[1] += 1;
        y -= 1;
    }
    if y < 0 || s[1] > max_count {
        return None;
    }
    while y >= 0 && binary[y as usize * w + x] != 0 && s[0] <= max_count {
        s[0] += 1;
        y -= 1;
    }
    if s[0] > max_count {
        return None;
    }
    y = start_y.round().clamp(0.0, (h - 1) as f64) as isize + 1;
    while y < h as isize && binary[y as usize * w + x] != 0 {
        s[2] += 1;
        y += 1;
    }
    if y == h as isize {
        return None;
    }
    while y < h as isize && binary[y as usize * w + x] == 0 && s[3] < max_count {
        s[3] += 1;
        y += 1;
    }
    if y == h as isize || s[3] >= max_count {
        return None;
    }
    while y < h as isize && binary[y as usize * w + x] != 0 && s[4] < max_count {
        s[4] += 1;
        y += 1;
    }
    if s[4] >= max_count {
        return None;
    }
    let total: usize = s.iter().sum();
    if (total as isize - original_total as isize).unsigned_abs() as f64
        > original_total as f64 * 0.4
    {
        return None;
    }
    let score = ratio_score(s)?;
    Some((center_from_end(s, y as usize), total as f64 / 7.0, score))
}
fn horizontal_cross(
    binary: &[u8],
    w: usize,
    h: usize,
    start_x: f64,
    y: f64,
    max_count: usize,
    original_total: usize,
) -> Option<(f64, f64, f64)> {
    let y = y.round().clamp(0.0, (h - 1) as f64) as usize;
    let mut s = [0usize; 5];
    let mut x = start_x.round().clamp(0.0, (w - 1) as f64) as isize;
    while x >= 0 && binary[y * w + x as usize] != 0 {
        s[2] += 1;
        x -= 1;
    }
    if x < 0 {
        return None;
    }
    while x >= 0 && binary[y * w + x as usize] == 0 && s[1] <= max_count {
        s[1] += 1;
        x -= 1;
    }
    if x < 0 || s[1] > max_count {
        return None;
    }
    while x >= 0 && binary[y * w + x as usize] != 0 && s[0] <= max_count {
        s[0] += 1;
        x -= 1;
    }
    if s[0] > max_count {
        return None;
    }
    x = start_x.round().clamp(0.0, (w - 1) as f64) as isize + 1;
    while x < w as isize && binary[y * w + x as usize] != 0 {
        s[2] += 1;
        x += 1;
    }
    if x == w as isize {
        return None;
    }
    while x < w as isize && binary[y * w + x as usize] == 0 && s[3] < max_count {
        s[3] += 1;
        x += 1;
    }
    if x == w as isize || s[3] >= max_count {
        return None;
    }
    while x < w as isize && binary[y * w + x as usize] != 0 && s[4] < max_count {
        s[4] += 1;
        x += 1;
    }
    if s[4] >= max_count {
        return None;
    }
    let total: usize = s.iter().sum();
    if (total as isize - original_total as isize).unsigned_abs() as f64
        > original_total as f64 * 0.4
    {
        return None;
    }
    let score = ratio_score(s)?;
    Some((center_from_end(s, x as usize), total as f64 / 7.0, score))
}

fn finder_candidates(binary: &[u8], w: usize, h: usize, max_candidates: usize) -> Vec<FinderPoint> {
    if w == 0 || h == 0 || binary.len() < w.saturating_mul(h) {
        return Vec::new();
    }

    let row_step = if h > 900 { 2 } else { 1 };
    let rows: Vec<usize> = (0..h).step_by(row_step).collect();
    let scan_row = |y: usize| {
        let mut row_candidates = Vec::new();
        let mut runs: Vec<(u8, usize, usize)> = Vec::new();
        let mut start = 0usize;
        let mut color = binary[y * w];
        for x in 1..=w {
            if x == w || binary[y * w + x] != color {
                runs.push((color, start, x - start));
                if x < w {
                    start = x;
                    color = binary[y * w + x];
                }
            }
        }
        if runs.len() < 5 {
            return row_candidates;
        }
        for i in 0..=runs.len() - 5 {
            if [
                runs[i].0,
                runs[i + 1].0,
                runs[i + 2].0,
                runs[i + 3].0,
                runs[i + 4].0,
            ] != [1, 0, 1, 0, 1]
            {
                continue;
            }
            let lengths = [
                runs[i].2,
                runs[i + 1].2,
                runs[i + 2].2,
                runs[i + 3].2,
                runs[i + 4].2,
            ];
            let Some(rs) = ratio_score(lengths) else {
                continue;
            };
            let end = runs[i + 4].1 + runs[i + 4].2;
            let cx = center_from_end(lengths, end);
            let total: usize = lengths.iter().sum();
            let max_count = (lengths[2] * 2).max(3);
            let Some((cy, vm, vs)) = vertical_cross(binary, w, h, y as f64, cx, max_count, total)
            else {
                continue;
            };
            let Some((hx, hm, hs)) = horizontal_cross(binary, w, h, cx, cy, max_count, total)
            else {
                continue;
            };
            if (hx - cx).abs() > vm * 1.5 {
                continue;
            }
            row_candidates.push(FinderPoint {
                x: (hx + cx) / 2.0,
                y: cy,
                module_size: (total as f64 / 7.0 + vm + hm) / 3.0,
                score: rs + vs + hs,
            });
        }
        row_candidates
    };

    // Finder rows are independent, so the ordinary locator pass can use all
    // cores too. Keep tiny images serial to avoid spending more on scheduling
    // than on the scan itself.
    #[cfg(feature = "parallel")]
    let mut raw: Vec<FinderPoint> = if rows.len() >= 96 {
        let per_row: Vec<Vec<FinderPoint>> = rows.par_iter().map(|&y| scan_row(y)).collect();
        per_row.into_iter().flatten().collect()
    } else {
        rows.into_iter().flat_map(scan_row).collect()
    };
    #[cfg(not(feature = "parallel"))]
    let mut raw: Vec<FinderPoint> = rows.into_iter().flat_map(scan_row).collect();

    raw.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal));
    let mut clustered: Vec<FinderPoint> = Vec::new();
    for p in raw {
        if let Some(existing) = clustered.iter_mut().find(|q| {
            ((q.x - p.x).powi(2) + (q.y - p.y).powi(2)).sqrt()
                < q.module_size.max(p.module_size) * 2.2
        }) {
            if p.score < existing.score {
                *existing = p;
            }
        } else {
            clustered.push(p);
        }
        if clustered.len() > max_candidates * 3 {
            break;
        }
    }
    clustered.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal));
    clustered.truncate(max_candidates);
    clustered
}

fn dist(a: FinderPoint, b: FinderPoint) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// Return all plausible interpretations of three finder centers.
///
/// The old scanner assumed the top-left finder was always opposite the longest
/// side of an almost-right triangle. That is fast for a front-facing symbol,
/// but a projective camera view can compress one side enough to invalidate that
/// assumption. Keeping the three plausible corner interpretations is cheap, and
/// the structural decoder will reject mirrored/incorrect hypotheses later.
fn order_triple_candidates(
    a: FinderPoint,
    b: FinderPoint,
    c: FinderPoint,
) -> Vec<(FinderPoint, FinderPoint, FinderPoint, f64)> {
    let pts = [a, b, c];
    let max_module = a.module_size.max(b.module_size).max(c.module_size).max(1.0);
    let min_module = a.module_size.min(b.module_size).min(c.module_size);
    let module_spread = (max_module - min_module) / max_module;
    if module_spread > 0.82 {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(3);
    for tl_index in 0..3 {
        let tl = pts[tl_index];
        let mut others = [pts[(tl_index + 1) % 3], pts[(tl_index + 2) % 3]];
        let mut v1 = (others[0].x - tl.x, others[0].y - tl.y);
        let mut v2 = (others[1].x - tl.x, others[1].y - tl.y);
        let d1 = (v1.0 * v1.0 + v1.1 * v1.1).sqrt();
        let d2 = (v2.0 * v2.0 + v2.1 * v2.1).sqrt();
        if d1 < 8.0 || d2 < 8.0 {
            continue;
        }

        // Reject only genuinely degenerate triangles. Under perspective, the
        // two logical perpendicular edges can look far from 90 degrees.
        let cross = v1.0 * v2.1 - v1.1 * v2.0;
        let sine = (cross.abs() / (d1 * d2).max(1e-9)).clamp(0.0, 1.0);
        if sine < 0.16 {
            continue;
        }

        let arm_ratio = d1.min(d2) / d1.max(d2);
        if arm_ratio < 0.18 {
            continue;
        }

        if cross < 0.0 {
            others.swap(0, 1);
            v1 = (others[0].x - tl.x, others[0].y - tl.y);
            v2 = (others[1].x - tl.x, others[1].y - tl.y);
        }
        let tr = others[0];
        let bl = others[1];

        // Prefer square-ish candidates but do not require affine-square
        // geometry. Finder quality remains the dominant ranking signal.
        let perspective_penalty = (1.0 - sine) * 0.45 + (1.0 - arm_ratio) * 0.20;
        let score = perspective_penalty + module_spread * 0.30 + a.score + b.score + c.score;
        let _ = (v1, v2); // keep the orientation calculation explicit above
        out.push((tl, tr, bl, score));
    }
    out
}

fn version_candidates(
    tl: FinderPoint,
    tr: FinderPoint,
    bl: FinderPoint,
    options: &ScanOptions,
) -> Vec<u8> {
    let module = (tl.module_size + tr.module_size + bl.module_size) / 3.0;
    let estimate = ((dist(tl, tr) + dist(tl, bl)) / 2.0 / module + 7.0).round();
    let mut versions: Vec<(f64, u8)> = (options.min_version..=options.max_version)
        .map(|v| ((size_for_version(v).unwrap() as f64 - estimate).abs(), v))
        .collect();
    versions.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    versions.into_iter().take(8).map(|(_, v)| v).collect()
}

fn solve_linear(mut a: [[f64; 9]; 8]) -> Option<[f64; 8]> {
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
            a.swap(pivot, col);
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
    for (index, row) in a.iter().enumerate() {
        out[index] = row[8];
    }
    Some(out)
}
fn homography(src: [(f64, f64); 4], dst: [(f64, f64); 4]) -> Option<[f64; 9]> {
    let mut a = [[0.0; 9]; 8];
    for i in 0..4 {
        let (x, y) = src[i];
        let (u, v) = dst[i];
        let r = 2 * i;
        a[r] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y, u];
        a[r + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y, v];
    }
    let h = solve_linear(a)?;
    Some([h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], 1.0])
}
fn project(h: [f64; 9], x: f64, y: f64) -> (f64, f64) {
    let d = h[6] * x + h[7] * y + h[8];
    (
        (h[0] * x + h[1] * y + h[2]) / d,
        (h[3] * x + h[4] * y + h[5]) / d,
    )
}

fn alignment_score(
    binary: &[u8],
    w: usize,
    h: usize,
    cx: f64,
    cy: f64,
    vx: (f64, f64),
    vy: (f64, f64),
) -> f64 {
    let mut mismatch: f64 = 0.0;
    let mut total: f64 = 0.0;
    for ro in -2i32..=2 {
        for co in -2i32..=2 {
            let expected = ro.abs() == 2 || co.abs() == 2 || (ro == 0 && co == 0);
            let x = cx + vx.0 * co as f64 + vy.0 * ro as f64;
            let y = cy + vx.1 * co as f64 + vy.1 * ro as f64;
            if x < 0.0 || y < 0.0 || x >= w as f64 || y >= h as f64 {
                mismatch += 1.0;
                total += 1.0;
                continue;
            }

            // A tiny 5-point vote is much less brittle than one rounded pixel
            // when the alignment pattern is blurred, resampled or oblique.
            let ix = x.round() as isize;
            let iy = y.round() as isize;
            let mut black_votes = 0usize;
            let mut votes = 0usize;
            for (ox, oy) in [(0isize, 0isize), (-1, 0), (1, 0), (0, -1), (0, 1)] {
                let px = ix + ox;
                let py = iy + oy;
                if px >= 0 && py >= 0 && px < w as isize && py < h as isize {
                    black_votes += (binary[py as usize * w + px as usize] != 0) as usize;
                    votes += 1;
                }
            }
            let observed = black_votes * 2 >= votes.max(1);
            if observed != expected {
                mismatch += 1.0;
            }
            total += 1.0;
        }
    }
    mismatch / total.max(1.0)
}

#[derive(Debug, Clone, Copy)]
struct ThreeFinderProjectiveModel {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
    g: f64,
    h: f64,
}

impl ThreeFinderProjectiveModel {
    fn project(&self, u: f64, v: f64) -> Option<(f64, f64)> {
        let denominator = 1.0 + self.g * u + self.h * v;
        if denominator.abs() < 1e-8 {
            return None;
        }
        Some((
            (self.a * u + self.b * v + self.c) / denominator,
            (self.d * u + self.e * v + self.f) / denominator,
        ))
    }
}

/// Estimate a full projective mapping from the three finder centres plus their
/// observed module scales. Along a projective line the local module scale
/// changes with the square of the homography denominator, so the endpoint
/// scale ratio gives the missing projective coefficient. This is much more
/// accurate under yaw/pitch than independently warping the top and left arms
/// and then adding them as an affine parallelogram.
fn three_finder_projective_model(
    tl: FinderPoint,
    tr: FinderPoint,
    bl: FinderPoint,
    version: u8,
) -> Option<ThreeFinderProjectiveModel> {
    let size = size_for_version(version).ok()? as f64;
    let logical_span = size - 7.0;
    if logical_span <= 0.0 {
        return None;
    }

    let tl_module = tl.module_size.max(0.25);
    let tr_module = tr.module_size.max(0.25);
    let bl_module = bl.module_size.max(0.25);
    let top_denominator = (tl_module / tr_module).sqrt().clamp(0.20, 5.0);
    let left_denominator = (tl_module / bl_module).sqrt().clamp(0.20, 5.0);
    let g = (top_denominator - 1.0) / logical_span;
    let h = (left_denominator - 1.0) / logical_span;

    let a = (tr.x * top_denominator - tl.x) / logical_span;
    let d = (tr.y * top_denominator - tl.y) / logical_span;
    let b = (bl.x * left_denominator - tl.x) / logical_span;
    let e = (bl.y * left_denominator - tl.y) / logical_span;

    Some(ThreeFinderProjectiveModel {
        a,
        b,
        c: tl.x,
        d,
        e,
        f: tl.y,
        g,
        h,
    })
}

fn predicted_alignment_from_finders(
    tl: FinderPoint,
    tr: FinderPoint,
    bl: FinderPoint,
    version: u8,
) -> Option<(f64, f64)> {
    let patterns = alignment_patterns(version, AlignmentProfile::Standard5).ok()?;
    let primary = patterns
        .iter()
        .find(|p| p.primary)
        .or_else(|| patterns.last())?;
    let model = three_finder_projective_model(tl, tr, bl, version)?;
    // Finder centres are logical (3.5, 3.5), while alignment centres are
    // sampled at (col + 0.5, row + 0.5), hence the relative col/row - 3.0.
    model.project(primary.col as f64 - 3.0, primary.row as f64 - 3.0)
}

fn find_alignment(
    binary: &[u8],
    w: usize,
    h: usize,
    tl: FinderPoint,
    tr: FinderPoint,
    bl: FinderPoint,
    version: u8,
) -> Option<(f64, f64)> {
    let patterns = alignment_patterns(version, AlignmentProfile::Standard5).ok()?;
    let primary = patterns
        .iter()
        .find(|p| p.primary)
        .or_else(|| patterns.last())?;

    let dx = primary.col as f64 - 3.0;
    let dy = primary.row as f64 - 3.0;
    let model = three_finder_projective_model(tl, tr, bl, version)?;
    let predicted = model.project(dx, dy)?;

    // Use the projective model's local Jacobian instead of an affine top/left
    // basis. Under yaw/pitch the module direction and scale near the primary
    // alignment eye can differ substantially from the finder-corner average.
    let px = model.project(dx + 1.0, dy)?;
    let py = model.project(dx, dy + 1.0)?;
    let local_bx = (px.0 - predicted.0, px.1 - predicted.1);
    let local_by = (py.0 - predicted.0, py.1 - predicted.1);

    let bx_len = (local_bx.0 * local_bx.0 + local_bx.1 * local_bx.1).sqrt();
    let by_len = (local_by.0 * local_by.0 + local_by.1 * local_by.1).sqrt();
    let module = ((bx_len + by_len) * 0.5).max(0.75);
    let shorter_arm = dist(tl, tr).min(dist(tl, bl));
    let mut best = (1.0f64, predicted.0, predicted.1, 1.0f64);

    // Stage 1: the common case. Search tightly around the projective prediction
    // with a few local scale hypotheses. Most front-facing and moderate-angle
    // symbols should finish here.
    let near_radius = (module * 4.5).clamp(3.0, 28.0);
    let near_step = (module * 0.85).clamp(1.0, 4.0);
    let near_scales = [0.76, 0.90, 1.0, 1.12, 1.30];
    let mut oy = -near_radius;
    while oy <= near_radius {
        let mut ox = -near_radius;
        while ox <= near_radius {
            let cx = predicted.0 + ox;
            let cy = predicted.1 + oy;
            for scale in near_scales {
                let score = alignment_score(
                    binary,
                    w,
                    h,
                    cx,
                    cy,
                    (local_bx.0 * scale, local_bx.1 * scale),
                    (local_by.0 * scale, local_by.1 * scale),
                );
                if score < best.0 {
                    best = (score, cx, cy, scale);
                }
            }
            ox += near_step;
        }
        oy += near_step;
    }

    // Stage 2 only runs when the tight search was not convincing. Strong
    // perspective can shift the bottom-right alignment pattern substantially,
    // so expand the center search while using a coarser stride.
    if best.0 > 0.28 {
        let broad_radius = (module * 11.0).max(shorter_arm * 0.13).clamp(8.0, 110.0);
        let broad_step = (module * 2.1).clamp(2.5, 10.0);
        let broad_scales = [0.60, 0.78, 0.94, 1.0, 1.16, 1.38, 1.62];
        let mut oy = -broad_radius;
        while oy <= broad_radius {
            let mut ox = -broad_radius;
            while ox <= broad_radius {
                let cx = predicted.0 + ox;
                let cy = predicted.1 + oy;
                for scale in broad_scales {
                    let score = alignment_score(
                        binary,
                        w,
                        h,
                        cx,
                        cy,
                        (local_bx.0 * scale, local_bx.1 * scale),
                        (local_by.0 * scale, local_by.1 * scale),
                    );
                    if score < best.0 {
                        best = (score, cx, cy, scale);
                    }
                }
                ox += broad_step;
            }
            oy += broad_step;
        }
    }

    // Final sub-module refinement around the best coarse hypothesis.
    let refine_radius = (module * 2.2).clamp(2.0, 16.0);
    let refine_step = (module * 0.34).clamp(0.75, 2.25);
    let mut oy = -refine_radius;
    while oy <= refine_radius {
        let mut ox = -refine_radius;
        while ox <= refine_radius {
            let cx = best.1 + ox;
            let cy = best.2 + oy;
            for delta in [-0.16f64, -0.08, 0.0, 0.08, 0.16] {
                let scale = (best.3 + delta).clamp(0.48, 1.85);
                let score = alignment_score(
                    binary,
                    w,
                    h,
                    cx,
                    cy,
                    (local_bx.0 * scale, local_bx.1 * scale),
                    (local_by.0 * scale, local_by.1 * scale),
                );
                if score < best.0 {
                    best = (score, cx, cy, scale);
                }
            }
            ox += refine_step;
        }
        oy += refine_step;
    }

    (best.0 <= 0.46).then_some((best.1, best.2))
}

fn projected_sample(image: &image::RgbaImage, h: [f64; 9], mx: f64, my: f64, radius: f64) -> Rgb {
    let offsets = [
        (0.0, 0.0),
        (-radius, 0.0),
        (radius, 0.0),
        (0.0, -radius),
        (0.0, radius),
    ];
    let mut sum = Rgb::default();
    let mut n = 0.0;
    for (dx, dy) in offsets {
        let (x, y) = project(h, mx + dx, my + dy);
        if x >= 0.0 && y >= 0.0 && x < image.width() as f64 && y < image.height() as f64 {
            let p = bilinear_rgb(image, x, y);
            sum.r += p.r;
            sum.g += p.g;
            sum.b += p.b;
            n += 1.0;
        }
    }
    if n == 0.0 {
        Rgb {
            r: 255.0,
            g: 255.0,
            b: 255.0,
        }
    } else {
        Rgb {
            r: sum.r / n,
            g: sum.g / n,
            b: sum.b / n,
        }
    }
}
fn sample_perspective(
    image: &image::RgbaImage,
    h: [f64; 9],
    size: usize,
    radius: f64,
) -> Vec<Vec<Rgb>> {
    (0..size)
        .map(|r| {
            (0..size)
                .map(|c| projected_sample(image, h, c as f64 + 0.5, r as f64 + 0.5, radius))
                .collect()
        })
        .collect()
}
fn robust_rgb(mut values: Vec<Rgb>) -> Rgb {
    if values.is_empty() {
        return Rgb {
            r: 255.0,
            g: 255.0,
            b: 255.0,
        };
    }
    let med = |vals: &mut Vec<f64>| {
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        vals[vals.len() / 2]
    };
    let mut r = values.iter().map(|x| x.r).collect();
    let mut g = values.iter().map(|x| x.g).collect();
    let mut b = values.drain(..).map(|x| x.b).collect();
    Rgb {
        r: med(&mut r),
        g: med(&mut g),
        b: med(&mut b),
    }
}

#[derive(Clone, Copy)]
struct Palette5 {
    black: Rgb,
    white: Rgb,
    red: Rgb,
    green: Rgb,
    blue: Rgb,
}

fn observed_palette(grid: &[Vec<Rgb>], version: u8) -> Result<Palette5> {
    let layout = layout_for_version(version)?;
    let take = |positions: &[(usize, usize)]| {
        robust_rgb(
            positions
                .iter()
                .step_by((positions.len() / 36).max(1))
                .take(36)
                .map(|&(r, c)| grid[r][c])
                .collect(),
        )
    };
    let p = Palette5 {
        black: take(&layout.calibration.black),
        white: take(&layout.calibration.white),
        red: take(&layout.calibration.red),
        green: take(&layout.calibration.green),
        blue: take(&layout.calibration.blue),
    };
    let colors = [p.black, p.white, p.red, p.green, p.blue];
    let mut min = f64::INFINITY;
    for i in 0..colors.len() {
        for j in i + 1..colors.len() {
            min = min.min(rgb_distance(colors[i], colors[j]));
        }
    }
    if min < 20.0 {
        return Err(QuadQrError::Decode(
            "RGBW calibration references are not separable".into(),
        ));
    }
    Ok(p)
}

fn calibration_reference_points(
    grid: &[Vec<Rgb>],
    positions: &[(usize, usize)],
    limit: usize,
) -> Vec<(usize, usize, Rgb)> {
    if positions.is_empty() {
        return Vec::new();
    }
    let step = (positions.len() / limit.max(1)).max(1);
    positions
        .iter()
        .step_by(step)
        .take(limit)
        .map(|&(row, col)| (row, col, grid[row][col]))
        .collect()
}

fn estimate_local_reference(refs: &[(usize, usize, Rgb)], row: usize, col: usize) -> Rgb {
    let mut weighted = Rgb::default();
    let mut total = 0.0;
    for &(ref_row, ref_col, rgb) in refs {
        let dr = row as f64 - ref_row as f64;
        let dc = col as f64 - ref_col as f64;
        let distance_sq = dr * dr + dc * dc;
        if distance_sq < 1e-9 {
            return rgb;
        }
        // Smooth inverse-distance weighting. The +4 keeps one slightly blurred
        // structural sample from dominating a whole local region.
        let weight = 1.0 / (distance_sq + 4.0);
        weighted.r += rgb.r * weight;
        weighted.g += rgb.g * weight;
        weighted.b += rgb.b * weight;
        total += weight;
    }
    if total <= 0.0 {
        return Rgb::default();
    }
    Rgb {
        r: weighted.r / total,
        g: weighted.g / total,
        b: weighted.b / total,
    }
}

fn spatially_normalize_rgb_grid(grid: &[Vec<Rgb>], version: u8) -> Result<Vec<Vec<Rgb>>> {
    let layout = layout_for_version(version)?;
    // 24 + 24 reference points keeps this recovery bounded even on large
    // versions. It runs only after the ordinary Raw/Balanced/Affine attempts
    // have failed for an already-sampled geometry candidate.
    let black_refs = calibration_reference_points(grid, &layout.calibration.black, 24);
    let white_refs = calibration_reference_points(grid, &layout.calibration.white, 24);
    if black_refs.is_empty() || white_refs.is_empty() {
        return Err(QuadQrError::Decode(
            "black/white references unavailable for spatial normalization".into(),
        ));
    }
    let size = grid.len();
    let mut normalized = vec![vec![Rgb::default(); size]; size];
    for row in 0..size {
        for col in 0..size {
            let rgb = grid[row][col];
            let black = estimate_local_reference(&black_refs, row, col);
            let white = estimate_local_reference(&white_refs, row, col);
            let channel = |value: f64, low: f64, high: f64| {
                let range = (high - low).max(28.0);
                ((value - low) * 255.0 / range).clamp(-96.0, 384.0)
            };
            normalized[row][col] = Rgb {
                r: channel(rgb.r, black.r, white.r),
                g: channel(rgb.g, black.g, white.g),
                b: channel(rgb.b, black.b, white.b),
            };
        }
    }
    Ok(normalized)
}

#[derive(Clone, Copy)]
enum ClassifierMode {
    Raw,
    Balanced,
    Affine,
}
#[derive(Clone, Copy)]
struct AffineTransform {
    c: [[f64; 4]; 3],
}

fn white_balance(rgb: Rgb, p: Palette5) -> Rgb {
    let channel =
        |v: f64, b: f64, w: f64| ((v - b) * 255.0 / (w - b).max(32.0)).clamp(-96.0, 384.0);
    Rgb {
        r: channel(rgb.r, p.black.r, p.white.r),
        g: channel(rgb.g, p.black.g, p.white.g),
        b: channel(rgb.b, p.black.b, p.white.b),
    }
}
fn solve4(mut a: [[f64; 5]; 4]) -> Option<[f64; 4]> {
    for col in 0..4 {
        let mut pivot = col;
        for row in (col + 1)..4 {
            if a[row][col].abs() > a[pivot][col].abs() {
                pivot = row;
            }
        }
        if a[pivot][col].abs() < 1e-9 {
            return None;
        }
        if pivot != col {
            a.swap(pivot, col);
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
    Some([a[0][4], a[1][4], a[2][4], a[3][4]])
}
fn affine_transform(p: Palette5) -> Option<AffineTransform> {
    let observed = [p.black, p.white, p.red, p.green, p.blue];
    let ideal = [
        Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        },
        Rgb {
            r: 255.0,
            g: 255.0,
            b: 255.0,
        },
        Rgb {
            r: 255.0,
            g: 0.0,
            b: 0.0,
        },
        Rgb {
            r: 0.0,
            g: 255.0,
            b: 0.0,
        },
        Rgb {
            r: 0.0,
            g: 0.0,
            b: 255.0,
        },
    ];
    let rows: [[f64; 4]; 5] = observed.map(|x| [x.r, x.g, x.b, 1.0]);
    let mut normal = [[0.0; 4]; 4];
    for row in rows {
        for i in 0..4 {
            for j in 0..4 {
                normal[i][j] += row[i] * row[j];
            }
        }
    }
    for (i, row) in normal.iter_mut().enumerate() {
        row[i] += 1e-4;
    }
    let mut coeff = [[0.0; 4]; 3];
    for (channel, coefficients) in coeff.iter_mut().enumerate() {
        let mut rhs = [0.0; 4];
        for (ri, row) in rows.iter().enumerate() {
            let y = match channel {
                0 => ideal[ri].r,
                1 => ideal[ri].g,
                _ => ideal[ri].b,
            };
            for i in 0..4 {
                rhs[i] += row[i] * y;
            }
        }
        let mut aug = [[0.0; 5]; 4];
        for i in 0..4 {
            for j in 0..4 {
                aug[i][j] = normal[i][j];
            }
            aug[i][4] = rhs[i];
        }
        *coefficients = solve4(aug)?;
    }
    Some(AffineTransform { c: coeff })
}
fn apply_affine(rgb: Rgb, t: AffineTransform) -> Rgb {
    let x = [rgb.r, rgb.g, rgb.b, 1.0];
    let channel = |c: [f64; 4]| {
        c.iter()
            .zip(x)
            .map(|(a, b)| a * b)
            .sum::<f64>()
            .clamp(-96.0, 384.0)
    };
    Rgb {
        r: channel(t.c[0]),
        g: channel(t.c[1]),
        b: channel(t.c[2]),
    }
}
fn transformed_palette(p: Palette5, mode: ClassifierMode) -> (Palette5, Option<AffineTransform>) {
    match mode {
        ClassifierMode::Raw => (p, None),
        ClassifierMode::Balanced => (
            Palette5 {
                black: white_balance(p.black, p),
                white: white_balance(p.white, p),
                red: white_balance(p.red, p),
                green: white_balance(p.green, p),
                blue: white_balance(p.blue, p),
            },
            None,
        ),
        ClassifierMode::Affine => {
            let t = affine_transform(p);
            if let Some(t) = t {
                (
                    Palette5 {
                        black: Rgb {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                        },
                        white: Rgb {
                            r: 255.0,
                            g: 255.0,
                            b: 255.0,
                        },
                        red: Rgb {
                            r: 255.0,
                            g: 0.0,
                            b: 0.0,
                        },
                        green: Rgb {
                            r: 0.0,
                            g: 255.0,
                            b: 0.0,
                        },
                        blue: Rgb {
                            r: 0.0,
                            g: 0.0,
                            b: 255.0,
                        },
                    },
                    Some(t),
                )
            } else {
                (p, None)
            }
        }
    }
}
fn transform_sample(
    rgb: Rgb,
    raw: Palette5,
    mode: ClassifierMode,
    affine: Option<AffineTransform>,
) -> Rgb {
    match mode {
        ClassifierMode::Raw => rgb,
        ClassifierMode::Balanced => white_balance(rgb, raw),
        ClassifierMode::Affine => affine.map(|t| apply_affine(rgb, t)).unwrap_or(rgb),
    }
}
fn classify(rgb: Rgb, colors: &[(i16, Rgb)]) -> (i16, f32, i16) {
    let mut d: Vec<(f64, i16)> = colors
        .iter()
        .map(|&(v, c)| (rgb_distance(rgb, c), v))
        .collect();
    d.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(Ordering::Equal)
            .then(a.1.cmp(&b.1))
    });
    let second = d.get(1).copied().unwrap_or((d[0].0 + 1.0, d[0].1));
    let conf = ((second.0 - d[0].0) / second.0.max(1e-6)).clamp(0.0, 1.0) as f32;
    (d[0].1, conf, second.1)
}
type ClassifiedGrid = (Vec<Vec<i16>>, Vec<Vec<f32>>, Vec<Vec<i16>>);

fn classify_rgbw_grid(
    grid: &[Vec<Rgb>],
    version: u8,
    mode: ClassifierMode,
) -> Result<ClassifiedGrid> {
    let layout = layout_for_version(version)?;
    let raw = observed_palette(grid, version)?;
    let (p, t) = transformed_palette(raw, mode);
    let size = grid.len();
    let mut m = vec![vec![WHITE; size]; size];
    let mut conf = vec![vec![1.0; size]; size];
    let mut alt = vec![vec![WHITE; size]; size];
    for r in 0..size {
        for c in 0..size {
            let colors = if layout.reserved[r][c] {
                vec![
                    (BLACK, p.black),
                    (WHITE, p.white),
                    (RED, p.red),
                    (GREEN, p.green),
                    (BLUE, p.blue),
                ]
            } else {
                vec![
                    (RED, p.red),
                    (GREEN, p.green),
                    (BLUE, p.blue),
                    (WHITE, p.white),
                ]
            };
            let sample = transform_sample(grid[r][c], raw, mode, t);
            let (v, q, a) = classify(sample, &colors);
            m[r][c] = v;
            conf[r][c] = q;
            alt[r][c] = a;
        }
    }
    Ok((m, conf, alt))
}

fn sample_triangle_perspective(
    image: &image::RgbaImage,
    h: [f64; 9],
    size: usize,
    radius: f64,
) -> Vec<Vec<(Rgb, Rgb)>> {
    let inset = 0.27;
    let side = inset - 0.06;
    let middle = 0.50 - inset * 0.04;
    let a = [(inset, inset), (side, middle), (middle, side)];
    (0..size)
        .map(|r| {
            (0..size)
                .map(|c| {
                    let first = robust_rgb(
                        a.iter()
                            .map(|&(x, y)| {
                                projected_sample(image, h, c as f64 + x, r as f64 + y, radius)
                            })
                            .collect(),
                    );
                    let second = robust_rgb(
                        a.iter()
                            .map(|&(x, y)| {
                                projected_sample(
                                    image,
                                    h,
                                    c as f64 + 1.0 - x,
                                    r as f64 + 1.0 - y,
                                    radius,
                                )
                            })
                            .collect(),
                    );
                    (first, second)
                })
                .collect()
        })
        .collect()
}
fn classify_triangle_grid(
    center_grid: &[Vec<Rgb>],
    tri: &[Vec<(Rgb, Rgb)>],
    version: u8,
    mode: ClassifierMode,
) -> Result<ClassifiedGrid> {
    let layout = layout_for_version(version)?;
    let raw = observed_palette(center_grid, version)?;
    let (p, t) = transformed_palette(raw, mode);
    let size = center_grid.len();
    let mut m = vec![vec![WHITE; size]; size];
    let mut conf = vec![vec![1.0; size]; size];
    let mut alt = vec![vec![WHITE; size]; size];
    let data_colors = [
        (RED, p.red),
        (GREEN, p.green),
        (BLUE, p.blue),
        (WHITE, p.white),
    ];
    for r in 0..size {
        for c in 0..size {
            if layout.reserved[r][c] {
                let sample = transform_sample(center_grid[r][c], raw, mode, t);
                let (v, q, a) = classify(
                    sample,
                    &[
                        (BLACK, p.black),
                        (WHITE, p.white),
                        (RED, p.red),
                        (GREEN, p.green),
                        (BLUE, p.blue),
                    ],
                );
                m[r][c] = v;
                conf[r][c] = q;
                alt[r][c] = a;
            } else {
                let (a, b) = tri[r][c];
                let a = transform_sample(a, raw, mode, t);
                let b = transform_sample(b, raw, mode, t);
                let (va, qa, aa) = classify(a, &data_colors);
                let (vb, qb, ab) = classify(b, &data_colors);
                m[r][c] = ((va & 3) << 2) | (vb & 3);
                conf[r][c] = qa.min(qb);
                alt[r][c] = if qa <= qb {
                    ((aa & 3) << 2) | (vb & 3)
                } else {
                    ((va & 3) << 2) | (ab & 3)
                };
            }
        }
    }
    Ok((m, conf, alt))
}
fn active_bounds(image: &image::RgbaImage) -> Result<Bounds> {
    let mut minx = image.width() as usize;
    let mut miny = image.height() as usize;
    let mut maxx = 0usize;
    let mut maxy = 0usize;
    let mut found = false;
    for (y, row) in image.rows().enumerate() {
        for (x, p) in row.enumerate() {
            if p[3] > 16 && (p[0] < 238 || p[1] < 238 || p[2] < 238) {
                minx = minx.min(x);
                miny = miny.min(y);
                maxx = maxx.max(x);
                maxy = maxy.max(y);
                found = true;
            }
        }
    }
    if !found {
        return Err(QuadQrError::Decode(
            "no code-like non-white area found".into(),
        ));
    }
    Ok(Bounds {
        x: minx,
        y: miny,
        width: maxx - minx + 1,
        height: maxy - miny + 1,
    })
}
fn sample_axis(image: &image::RgbaImage, b: Bounds, size: usize) -> Vec<Vec<Rgb>> {
    let mw = b.width as f64 / size as f64;
    let mh = b.height as f64 / size as f64;
    let rad = mw.min(mh) * 0.18;
    (0..size)
        .map(|r| {
            (0..size)
                .map(|c| {
                    let cx = b.x as f64 + (c as f64 + 0.5) * mw;
                    let cy = b.y as f64 + (r as f64 + 0.5) * mh;
                    let mut vals = Vec::new();
                    for dy in [-rad, 0.0, rad] {
                        for dx in [-rad, 0.0, rad] {
                            vals.push(pixel_rgb(image, cx + dx, cy + dy));
                        }
                    }
                    robust_rgb(vals)
                })
                .collect()
        })
        .collect()
}
fn sample_axis_tri(image: &image::RgbaImage, b: Bounds, size: usize) -> Vec<Vec<(Rgb, Rgb)>> {
    let mw = b.width as f64 / size as f64;
    let mh = b.height as f64 / size as f64;
    let inset = 0.27;
    let side = inset - 0.06;
    let middle = 0.50 - inset * 0.04;
    let anchors = [(inset, inset), (side, middle), (middle, side)];
    (0..size)
        .map(|r| {
            (0..size)
                .map(|c| {
                    let samp = |x: f64, y: f64| {
                        pixel_rgb(
                            image,
                            b.x as f64 + (c as f64 + x) * mw,
                            b.y as f64 + (r as f64 + y) * mh,
                        )
                    };
                    let first = robust_rgb(anchors.iter().map(|&(x, y)| samp(x, y)).collect());
                    let second = robust_rgb(
                        anchors
                            .iter()
                            .map(|&(x, y)| samp(1.0 - x, 1.0 - y))
                            .collect(),
                    );
                    (first, second)
                })
                .collect()
        })
        .collect()
}

fn decode_sampled(
    center: &[Vec<Rgb>],
    tri: Option<&[Vec<(Rgb, Rgb)>]>,
    version: u8,
    options: &ScanOptions,
) -> Result<DecodeResult> {
    let mut errors = Vec::new();
    let decode = |matrix: &[Vec<i16>], confidence: &[Vec<f32>], alternatives: &[Vec<i16>]| {
        decode_matrix_with_options(
            matrix,
            Some(confidence),
            if options.soft_decoding {
                Some(alternatives)
            } else {
                None
            },
            DecodeOptions {
                structure_tolerance: options.structural_tolerance,
                max_erasure_confidence: options.max_erasure_confidence,
                soft_decoding: options.soft_decoding,
                soft: options.soft_decode,
            },
        )
    };
    // A failed palette/classification attempt should not abort the entire
    // geometry candidate. Continue through the existing bounded classifier
    // modes; this adds no new geometry/recovery passes.
    let calibration_plausible = observed_palette(center, version).is_ok();
    for mode in [
        ClassifierMode::Raw,
        ClassifierMode::Balanced,
        ClassifierMode::Affine,
    ] {
        match classify_rgbw_grid(center, version, mode) {
            Ok((matrix, confidence, alternatives)) => {
                match decode(&matrix, &confidence, &alternatives) {
                    Ok(value) => return Ok(value),
                    Err(error) => errors.push(error.to_string()),
                }
            }
            Err(error) => errors.push(error.to_string()),
        }
    }
    // Uneven shadow / exposure recovery. This is deliberately a sampled-grid
    // fallback, not another finder/geometry pass, so it cannot multiply the
    // expensive perspective search. It closely mirrors the JS scanner's local
    // black/white normalization and only runs after the three cheap classifiers
    // failed on this geometry.
    if calibration_plausible {
        match spatially_normalize_rgb_grid(center, version) {
            Ok(normalized) => {
                for mode in [
                    ClassifierMode::Raw,
                    ClassifierMode::Balanced,
                    ClassifierMode::Affine,
                ] {
                    match classify_rgbw_grid(&normalized, version, mode) {
                        Ok((matrix, confidence, alternatives)) => {
                            match decode(&matrix, &confidence, &alternatives) {
                                Ok(value) => return Ok(value),
                                Err(error) => errors.push(error.to_string()),
                            }
                        }
                        Err(error) => errors.push(error.to_string()),
                    }
                }
            }
            Err(error) => errors.push(error.to_string()),
        }
    }

    if let Some(triangle) = tri {
        for mode in [
            ClassifierMode::Raw,
            ClassifierMode::Balanced,
            ClassifierMode::Affine,
        ] {
            match classify_triangle_grid(center, triangle, version, mode) {
                Ok((matrix, confidence, alternatives)) => {
                    match decode(&matrix, &confidence, &alternatives) {
                        Ok(value) => return Ok(value),
                        Err(error) => errors.push(error.to_string()),
                    }
                }
                Err(error) => errors.push(error.to_string()),
            }
        }
    }
    Err(QuadQrError::Decode(errors.join(" | ")))
}

pub fn scan_rgba(image: &image::RgbaImage, options: &ScanOptions) -> Result<ScanResult> {
    let start = Instant::now();

    // Camera adapters may bound the *entire* scanner working image, not only
    // the cheap locator image. This prevents a 1080p/4K camera frame from
    // falling back to full-resolution grayscale, geometry and perspective
    // work after finder acquisition. Still-image scanning keeps this disabled
    // by default.
    let input_max_dimension = image.width().max(image.height());
    if options.working_max_dimension > 0 && input_max_dimension > options.working_max_dimension {
        let scale = options.working_max_dimension as f64 / input_max_dimension as f64;
        let resized = image::imageops::resize(
            image,
            (image.width() as f64 * scale).round().max(1.0) as u32,
            (image.height() as f64 * scale).round().max(1.0) as u32,
            image::imageops::FilterType::Triangle,
        );
        let mut nested_options = options.clone();
        nested_options.working_max_dimension = 0;
        let mut result = scan_rgba(&resized, &nested_options)?;
        let inverse_scale = 1.0 / scale;
        for finder in &mut result.finders {
            finder.x *= inverse_scale;
            finder.y *= inverse_scale;
            finder.module_size *= inverse_scale;
        }
        // Include the resize cost in camera diagnostics as well.
        result.diagnostics.elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        return Ok(result);
    }
    let max_dim = image.width().max(image.height());
    let locator_scale =
        if options.locator_max_dimension > 0 && max_dim > options.locator_max_dimension {
            options.locator_max_dimension as f64 / max_dim as f64
        } else {
            1.0
        };
    let locator_owned = if locator_scale < 0.999 {
        Some(image::imageops::resize(
            image,
            (image.width() as f64 * locator_scale).round().max(1.0) as u32,
            (image.height() as f64 * locator_scale).round().max(1.0) as u32,
            image::imageops::FilterType::Triangle,
        ))
    } else {
        None
    };
    let locator = locator_owned.as_ref().unwrap_or(image);
    let locator_width = locator.width() as usize;
    let locator_height = locator.height() as usize;
    let locator_value_gray = build_value_gray(locator);
    let locator_threshold = otsu(&locator_value_gray);
    let locator_binary = binarize(&locator_value_gray, locator_threshold);

    let mut finders = finder_candidates(
        &locator_binary,
        locator_width,
        locator_height,
        options.max_finder_candidates,
    );
    let mut selected_recovery: Option<FinderRecoveryStrategy> = None;
    let mut best_recovery_strategy: Option<FinderRecoveryStrategy> = None;
    let mut best_recovery_binary: Option<Vec<u8>> = None;

    // Finder recovery operates on the downscaled locator image, not the full
    // camera frame. Independent threshold/color strategies run concurrently.
    // Once one strategy wins, only that strategy is rebuilt at full resolution
    // for alignment-pattern geometry.
    if options.finder_recovery && finders.len() < 3 {
        #[derive(Debug)]
        struct RecoveryCandidate {
            strategy: FinderRecoveryStrategy,
            binary: Vec<u8>,
            finders: Vec<FinderPoint>,
        }

        let strategies = [
            FinderRecoveryStrategy::HybridValue,
            FinderRecoveryStrategy::ValueHigh,
            FinderRecoveryStrategy::ValueLow,
            FinderRecoveryStrategy::AutoColor {
                inset: 0.10,
                threshold_offset: 0,
            },
            FinderRecoveryStrategy::AutoColor {
                inset: 0.10,
                threshold_offset: 12,
            },
            FinderRecoveryStrategy::AutoColor {
                inset: 0.20,
                threshold_offset: 0,
            },
            FinderRecoveryStrategy::AutoColor {
                inset: 0.20,
                threshold_offset: 12,
            },
            FinderRecoveryStrategy::AutoColor {
                inset: 0.04,
                threshold_offset: 0,
            },
            FinderRecoveryStrategy::AutoColor {
                inset: 0.04,
                threshold_offset: 12,
            },
            FinderRecoveryStrategy::Luma,
        ];

        let evaluate_recovery = |strategy: FinderRecoveryStrategy| {
            let binary =
                build_recovery_binary(locator, &locator_value_gray, locator_threshold, strategy)?;
            let candidates = finder_candidates(
                &binary,
                locator_width,
                locator_height,
                options.max_finder_candidates,
            );
            Some(RecoveryCandidate {
                strategy,
                binary,
                finders: candidates,
            })
        };
        let choose_recovery = |a: RecoveryCandidate, b: RecoveryCandidate| {
            let a_valid = a.finders.len() >= 3;
            let b_valid = b.finders.len() >= 3;
            if a_valid != b_valid {
                return if a_valid { a } else { b };
            }
            if a.finders.len() != b.finders.len() {
                return if a.finders.len() > b.finders.len() {
                    a
                } else {
                    b
                };
            }
            if finder_quality(&a.finders) <= finder_quality(&b.finders) {
                a
            } else {
                b
            }
        };

        #[cfg(feature = "parallel")]
        let best = strategies
            .par_iter()
            .filter_map(|&strategy| evaluate_recovery(strategy))
            .reduce_with(choose_recovery);
        #[cfg(not(feature = "parallel"))]
        let best = strategies
            .iter()
            .filter_map(|&strategy| evaluate_recovery(strategy))
            .reduce(choose_recovery);

        if let Some(best) = best {
            let should_take = best.finders.len() > finders.len()
                || (best.finders.len() == finders.len()
                    && finder_quality(&best.finders) < finder_quality(&finders));
            if should_take {
                finders = best.finders.clone();
                selected_recovery = Some(best.strategy);
            }
            best_recovery_strategy = Some(best.strategy);
            best_recovery_binary = Some(best.binary);
        }
    }

    // Rotation recovery is intentionally second-stage. The normal finder path
    // remains very cheap, while rotated/oblique symbols can be normalized on
    // several CPU cores only when the ordinary row/column scan did not yield
    // three reliable finders.
    if options.rotation_recovery && finders.len() < 3 {
        let mut sources: Vec<(&[u8], Option<FinderRecoveryStrategy>)> =
            vec![(&locator_binary, None)];
        if let Some(binary) = best_recovery_binary.as_deref() {
            sources.push((binary, best_recovery_strategy));
        }
        let angles = [
            -54.0f64, -45.0, -36.0, -24.0, -12.0, 12.0, 24.0, 36.0, 45.0, 54.0,
        ];
        let tasks: Vec<(usize, f64)> = (0..sources.len())
            .flat_map(|source_index| {
                angles
                    .iter()
                    .copied()
                    .map(move |angle| (source_index, angle))
            })
            .collect();

        #[derive(Debug)]
        struct RotatedCandidate {
            strategy: Option<FinderRecoveryStrategy>,
            finders: Vec<FinderPoint>,
        }

        let evaluate_rotated = |(source_index, angle_degrees): (usize, f64)| {
            let (source, strategy) = sources[source_index];
            let radians = angle_degrees.to_radians();
            let rotated = rotate_binary(source, locator_width, locator_height, radians);
            let mut candidates = finder_candidates(
                &rotated,
                locator_width,
                locator_height,
                options.max_finder_candidates,
            );
            if candidates.is_empty() {
                return None;
            }
            unrotate_finders(&mut candidates, locator_width, locator_height, radians);
            Some(RotatedCandidate {
                strategy,
                finders: candidates,
            })
        };
        let choose_rotated = |a: RotatedCandidate, b: RotatedCandidate| {
            let a_valid = a.finders.len() >= 3;
            let b_valid = b.finders.len() >= 3;
            if a_valid != b_valid {
                return if a_valid { a } else { b };
            }
            if a.finders.len() != b.finders.len() {
                return if a.finders.len() > b.finders.len() {
                    a
                } else {
                    b
                };
            }
            if finder_quality(&a.finders) <= finder_quality(&b.finders) {
                a
            } else {
                b
            }
        };

        #[cfg(feature = "parallel")]
        let best_rotated = tasks
            .par_iter()
            .filter_map(|&(source_index, angle_degrees)| {
                evaluate_rotated((source_index, angle_degrees))
            })
            .reduce_with(choose_rotated);
        #[cfg(not(feature = "parallel"))]
        let best_rotated = tasks
            .iter()
            .filter_map(|&(source_index, angle_degrees)| {
                evaluate_rotated((source_index, angle_degrees))
            })
            .reduce(choose_rotated);

        if let Some(best_rotated) = best_rotated {
            let should_take = best_rotated.finders.len() > finders.len()
                || (best_rotated.finders.len() == finders.len()
                    && finder_quality(&best_rotated.finders) < finder_quality(&finders));
            if should_take {
                finders = best_rotated.finders;
                selected_recovery = best_rotated.strategy;
            }
        }
    }

    if locator_scale < 0.999 {
        for finder in &mut finders {
            finder.x /= locator_scale;
            finder.y /= locator_scale;
            finder.module_size /= locator_scale;
        }
    }

    // With fewer than two finders there is no perspective hypothesis and the
    // guarded axis fallback will not run either. Return before touching the
    // full-resolution frame again. This is the dominant live-camera no-code
    // path and keeps it intentionally cheap.
    if finders.len() < 2 {
        return Err(QuadQrError::Decode(format!(
            "finder search found only {} candidate(s)",
            finders.len()
        )));
    }

    let width = image.width() as usize;
    let height = image.height() as usize;
    let value_gray = build_value_gray(image);
    let value_threshold = otsu(&value_gray);
    let default_full_binary = binarize(&value_gray, value_threshold);
    let full_binary = selected_recovery
        .and_then(|strategy| build_recovery_binary(image, &value_gray, value_threshold, strategy))
        .unwrap_or(default_full_binary);

    let mut triples = Vec::new();
    for i in 0..finders.len() {
        for j in (i + 1)..finders.len() {
            for k in (j + 1)..finders.len() {
                triples.extend(order_triple_candidates(finders[i], finders[j], finders[k]));
            }
        }
    }
    triples.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(Ordering::Equal));
    triples.truncate(30);

    #[derive(Debug, Clone, Copy)]
    struct GeometryTask {
        tl: FinderPoint,
        tr: FinderPoint,
        bl: FinderPoint,
        version: u8,
    }

    let mut geometry_tasks = Vec::new();
    for &(tl, tr, bl, _) in &triples {
        for version in version_candidates(tl, tr, bl, options) {
            geometry_tasks.push(GeometryTask {
                tl,
                tr,
                bl,
                version,
            });
        }
    }

    let geometry_attempt_counter = ScanAttemptCounter::new();
    let errors = ScanErrorCollector::new();

    let try_geometry_on = |task: &GeometryTask,
                           binary: &[u8],
                           alignment_override: Option<(f64, f64)>,
                           path: &'static str| {
        geometry_attempt_counter.increment();
        let size = size_for_version(task.version).ok()? as usize;
        let alignment = match alignment_override {
            Some(value) => value,
            None => find_alignment(
                binary,
                width,
                height,
                task.tl,
                task.tr,
                task.bl,
                task.version,
            )?,
        };
        let patterns = alignment_patterns(task.version, AlignmentProfile::Standard5).ok()?;
        let primary = patterns
            .iter()
            .find(|pattern| pattern.primary)
            .or_else(|| patterns.last())?;
        let src = [
            (3.5, 3.5),
            (size as f64 - 3.5, 3.5),
            (3.5, size as f64 - 3.5),
            (primary.col as f64 + 0.5, primary.row as f64 + 0.5),
        ];
        let dst = [
            (task.tl.x, task.tl.y),
            (task.tr.x, task.tr.y),
            (task.bl.x, task.bl.y),
            alignment,
        ];
        let hm = homography(src, dst)?;
        let center = sample_perspective(image, hm, size, options.sample_radius.clamp(0.0, 0.30));

        // Most RGBW symbols do not need triangle sampling. Keep it lazy so a
        // successful ordinary decode avoids roughly half of the perspective
        // sampling work.
        let decoded = match decode_sampled(&center, None, task.version, options) {
            Ok(decoded) => Ok(decoded),
            Err(center_error) => {
                let triangle = sample_triangle_perspective(
                    image,
                    hm,
                    size,
                    options.high_density_sample_radius.clamp(0.0, 0.20),
                );
                decode_sampled(&center, Some(&triangle), task.version, options)
                    .map_err(|error| QuadQrError::Decode(format!("{center_error} | {error}")))
            }
        };

        match decoded {
            Ok(decoded) => Some((decoded, task.version, vec![task.tl, task.tr, task.bl], path)),
            Err(error) => {
                errors.push_limited(error.to_string(), 8);
                None
            }
        }
    };

    // Run the best-ranked hypotheses in parallel waves. This remains the same
    // fast/normal geometry path as before.
    let mut perspective_result = None;
    for wave in geometry_tasks.chunks(24) {
        #[cfg(feature = "parallel")]
        {
            perspective_result = wave.par_iter().find_map_any(|task| {
                try_geometry_on(task, &full_binary, None, "perspective-parallel")
            });
        }
        #[cfg(not(feature = "parallel"))]
        {
            perspective_result = wave
                .iter()
                .find_map(|task| try_geometry_on(task, &full_binary, None, "perspective-parallel"));
        }
        if perspective_result.is_some() {
            break;
        }
    }

    // Bounded recovery #1: after the normal search is exhausted, retry only
    // the three best geometry hypotheses on one stronger alignment binary.
    // Auto-color + local thresholding targets low exposure, color cast,
    // contrast loss and gradient shadow without multiplying the entire search.
    if perspective_result.is_none() && options.finder_recovery {
        if let Some(enhanced_binary) = build_alignment_recovery_binary(image) {
            for task in geometry_tasks.iter().take(3) {
                if let Some(result) = try_geometry_on(
                    task,
                    &enhanced_binary,
                    None,
                    "perspective-enhanced-alignment",
                ) {
                    perspective_result = Some(result);
                    break;
                }
            }
        }
    }

    // Bounded recovery #2: rotation and projective views can retain excellent
    // three-finder geometry while the small alignment eye is too distorted to
    // validate. Direct projection is cheap because it skips the alignment-area
    // search entirely, so cover the first three triple interpretations (up
    // to eight version hypotheses each) instead of only the first two tasks.
    // Every candidate still has to pass structure, Reed-Solomon and CRC.
    if perspective_result.is_none() {
        for task in geometry_tasks.iter().take(24) {
            let Some(estimated_alignment) =
                predicted_alignment_from_finders(task.tl, task.tr, task.bl, task.version)
            else {
                continue;
            };
            if let Some(result) = try_geometry_on(
                task,
                &full_binary,
                Some(estimated_alignment),
                "perspective-estimated-alignment",
            ) {
                perspective_result = Some(result);
                break;
            }
        }
    }

    let mut geometry_attempts = geometry_attempt_counter.get();
    if let Some((decoded, version, successful_finders, path)) = perspective_result {
        return Ok(ScanResult {
            decoded,
            diagnostics: ScanDiagnostics {
                elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
                finder_candidates: finders.len(),
                geometry_attempts,
                selected_version: Some(version),
                path,
            },
            finders: successful_finders,
        });
    }

    // Axis fallback is useful for clean exported images, but it used to run up
    // to 40 full decodes even on camera frames with no credible code. Require
    // finder evidence before paying that cost.
    if options.axis_aligned_fallback && finders.len() >= 2 {
        if let Ok(bounds) = active_bounds(image) {
            let aspect = bounds.width as f64 / bounds.height.max(1) as f64;
            if (0.62..=1.62).contains(&aspect) && bounds.width >= 21 && bounds.height >= 21 {
                let versions: Vec<u8> = (options.min_version..=options.max_version).collect();
                let axis_attempt_counter = ScanAttemptCounter::new();
                let try_axis = |version: u8| {
                    axis_attempt_counter.increment();
                    let size = size_for_version(version).ok()? as usize;
                    let center = sample_axis(image, bounds, size);
                    if let Ok(decoded) = decode_sampled(&center, None, version, options) {
                        return Some((decoded, version));
                    }
                    let triangle = sample_axis_tri(image, bounds, size);
                    decode_sampled(&center, Some(&triangle), version, options)
                        .ok()
                        .map(|decoded| (decoded, version))
                };
                #[cfg(feature = "parallel")]
                let axis_result = versions
                    .par_iter()
                    .find_map_any(|&version| try_axis(version));
                #[cfg(not(feature = "parallel"))]
                let axis_result = versions.iter().find_map(|&version| try_axis(version));
                geometry_attempts += axis_attempt_counter.get();
                if let Some((decoded, version)) = axis_result {
                    return Ok(ScanResult {
                        decoded,
                        diagnostics: ScanDiagnostics {
                            elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
                            finder_candidates: finders.len(),
                            geometry_attempts,
                            selected_version: Some(version),
                            path: "axis-aligned-parallel",
                        },
                        finders: finders.iter().take(3).copied().collect(),
                    });
                }
            }
        }
    }

    let error_text = errors
        .into_vec()
        .into_iter()
        .take(8)
        .collect::<Vec<_>>()
        .join(" || ");
    Err(QuadQrError::Decode(format!(
        "image scan failed after {geometry_attempts} geometry attempts; {error_text}"
    )))
}

pub fn scan_dynamic_image(
    image: &image::DynamicImage,
    options: &ScanOptions,
) -> Result<ScanResult> {
    scan_rgba(&image.to_rgba8(), options)
}

pub fn decode_image_buffer(input: &[u8]) -> Result<image::RgbaImage> {
    Ok(image::load_from_memory(input)?.to_rgba8())
}

pub fn scan_buffer(input: &[u8], options: &ScanOptions) -> Result<ScanResult> {
    let image = decode_image_buffer(input)?;
    scan_rgba(&image, options)
}
pub fn scan_file(path: impl AsRef<std::path::Path>, options: &ScanOptions) -> Result<ScanResult> {
    let image = image::open(path)?;
    scan_dynamic_image(&image, options)
}

/// Diagnostic alias matching the JavaScript `debugScanImageData` capability.
/// Rust `ScanResult` always includes finder and timing diagnostics, so no
/// separate debug-mode decode path is required.
pub fn debug_scan_rgba(image: &image::RgbaImage, options: &ScanOptions) -> Result<ScanResult> {
    scan_rgba(image, options)
}

#[derive(Debug, Clone)]
pub struct RectifiedCode {
    pub image: image::RgbaImage,
    pub version: u8,
    pub finders: Vec<FinderPoint>,
    pub path: &'static str,
}

fn rectify_from_homography(
    image: &image::RgbaImage,
    hm: [f64; 9],
    size: usize,
    module_size: u32,
) -> image::RgbaImage {
    let module_size = module_size.max(1);
    let output_size = size as u32 * module_size;
    let mut output = image::RgbaImage::new(output_size, output_size);
    for y in 0..output_size {
        for x in 0..output_size {
            let module_x = (x as f64 + 0.5) / module_size as f64;
            let module_y = (y as f64 + 0.5) / module_size as f64;
            let (source_x, source_y) = project(hm, module_x, module_y);
            let rgb = bilinear_rgb(image, source_x, source_y);
            output.put_pixel(
                x,
                y,
                image::Rgba([
                    rgb.r.round().clamp(0.0, 255.0) as u8,
                    rgb.g.round().clamp(0.0, 255.0) as u8,
                    rgb.b.round().clamp(0.0, 255.0) as u8,
                    255,
                ]),
            );
        }
    }
    output
}

/// Detect and perspective-rectify the strongest QuadQR candidate.
/// This is the native equivalent of `rectifyDetectedCode()` from the JS API.
pub fn rectify_detected_code(
    image: &image::RgbaImage,
    options: &ScanOptions,
    module_size: u32,
) -> Result<RectifiedCode> {
    let decoded = scan_rgba(image, options)?;
    let target_version = decoded.decoded.version;
    let gray = build_value_gray(image);
    let threshold = otsu(&gray);
    let binary = binarize(&gray, threshold);
    let width = image.width() as usize;
    let height = image.height() as usize;
    // Reuse the finder points from the successful scan first. They may have
    // come from color/rotation recovery that a raw Otsu pass would miss.
    let mut finders = decoded.finders.clone();
    if finders.len() < 3 {
        finders = finder_candidates(&binary, width, height, options.max_finder_candidates);
    }
    let mut triples = Vec::new();
    for i in 0..finders.len() {
        for j in (i + 1)..finders.len() {
            for k in (j + 1)..finders.len() {
                triples.extend(order_triple_candidates(finders[i], finders[j], finders[k]));
            }
        }
    }
    triples.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(Ordering::Equal));
    for (tl, tr, bl, _) in triples.into_iter().take(30) {
        let Some(alignment) = find_alignment(&binary, width, height, tl, tr, bl, target_version)
        else {
            continue;
        };
        let size = size_for_version(target_version)? as usize;
        let patterns = alignment_patterns(target_version, AlignmentProfile::Standard5)?;
        let Some(primary) = patterns
            .iter()
            .find(|pattern| pattern.primary)
            .or_else(|| patterns.last())
        else {
            continue;
        };
        let src = [
            (3.5, 3.5),
            (size as f64 - 3.5, 3.5),
            (3.5, size as f64 - 3.5),
            (primary.col as f64 + 0.5, primary.row as f64 + 0.5),
        ];
        let dst = [(tl.x, tl.y), (tr.x, tr.y), (bl.x, bl.y), alignment];
        if let Some(hm) = homography(src, dst) {
            return Ok(RectifiedCode {
                image: rectify_from_homography(image, hm, size, module_size),
                version: target_version,
                finders: vec![tl, tr, bl],
                path: "perspective",
            });
        }
    }

    if options.axis_aligned_fallback {
        let bounds = active_bounds(image)?;
        let size = size_for_version(target_version)? as u32;
        let cropped = image::imageops::crop_imm(
            image,
            bounds.x as u32,
            bounds.y as u32,
            bounds.width as u32,
            bounds.height as u32,
        )
        .to_image();
        return Ok(RectifiedCode {
            image: image::imageops::resize(
                &cropped,
                size * module_size.max(1),
                size * module_size.max(1),
                image::imageops::FilterType::Lanczos3,
            ),
            version: target_version,
            finders: decoded.finders,
            path: "axis-aligned",
        });
    }

    Err(QuadQrError::Decode(
        "could not rectify detected QuadQR".into(),
    ))
}
