#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use crate::compression::{prepare_payload, CompressionMode};
use crate::error::{invalid, Result};
use crate::format::{
    decode_matrix, encode_bytes, get_version_info, CellEncoding, EccLevel, EncodeOptions,
};
use crate::geometry::{MAX_VERSION, MIN_VERSION};

pub const STANDARD_QR_BYTE_CAPACITY_L: [usize; 40] = [
    17, 32, 53, 78, 106, 134, 154, 192, 230, 271, 321, 367, 425, 458, 520, 586, 644, 718, 792, 858,
    929, 1003, 1091, 1171, 1273, 1367, 1465, 1528, 1628, 1732, 1840, 1952, 2068, 2188, 2303, 2431,
    2563, 2699, 2809, 2953,
];
pub const STANDARD_QR_BYTE_CAPACITY_M: [usize; 40] = [
    14, 26, 42, 62, 84, 106, 122, 152, 180, 213, 251, 287, 331, 362, 412, 450, 504, 560, 624, 666,
    711, 779, 857, 911, 997, 1059, 1125, 1190, 1264, 1370, 1452, 1538, 1628, 1722, 1809, 1911,
    1989, 2099, 2213, 2331,
];
pub const STANDARD_QR_BYTE_CAPACITY_Q: [usize; 40] = [
    11, 20, 32, 46, 60, 74, 86, 108, 130, 151, 177, 203, 241, 258, 292, 322, 364, 394, 442, 482,
    509, 565, 611, 661, 715, 751, 805, 868, 908, 982, 1030, 1112, 1168, 1228, 1283, 1351, 1423,
    1499, 1579, 1663,
];
pub const STANDARD_QR_BYTE_CAPACITY_H: [usize; 40] = [
    7, 14, 24, 34, 44, 58, 64, 84, 98, 119, 137, 155, 177, 194, 220, 250, 280, 310, 338, 382, 403,
    439, 461, 511, 535, 593, 625, 658, 698, 742, 790, 842, 898, 958, 983, 1051, 1093, 1139, 1219,
    1273,
];

fn standard_capacity_table(ecc: EccLevel) -> &'static [usize; 40] {
    match ecc {
        EccLevel::L => &STANDARD_QR_BYTE_CAPACITY_L,
        EccLevel::M => &STANDARD_QR_BYTE_CAPACITY_M,
        EccLevel::Q => &STANDARD_QR_BYTE_CAPACITY_Q,
        EccLevel::H => &STANDARD_QR_BYTE_CAPACITY_H,
    }
}

pub fn get_standard_qr_byte_capacity(version: u8, ecc: EccLevel) -> Result<usize> {
    if !(MIN_VERSION..=MAX_VERSION).contains(&version) {
        return Err(invalid(format!(
            "version must be {MIN_VERSION}..{MAX_VERSION}"
        )));
    }
    Ok(standard_capacity_table(ecc)[version as usize - 1])
}

#[derive(Debug, Clone)]
pub struct CapacityComparison {
    pub version: u8,
    pub size: usize,
    pub ecc: EccLevel,
    pub quadqr_bytes: usize,
    pub standard_qr_bytes: usize,
    pub difference_bytes: isize,
    pub ratio: f64,
    pub gain_percent: f64,
    pub high_density: bool,
    pub cell_encoding: CellEncoding,
    pub quadqr_bits_per_data_cell: usize,
    pub quadqr_payload_efficiency_percent: f64,
    pub quadqr_payload_bits_per_matrix_cell: f64,
    pub standard_qr_payload_bits_per_matrix_cell: f64,
}

pub fn compare_capacity(
    version: u8,
    ecc: EccLevel,
    encoding: CellEncoding,
) -> Result<CapacityComparison> {
    let quadqr = get_version_info(version, ecc, encoding)?;
    let standard_qr_bytes = get_standard_qr_byte_capacity(version, ecc)?;
    let difference_bytes = quadqr.capacity_bytes as isize - standard_qr_bytes as isize;
    let ratio = if standard_qr_bytes > 0 {
        quadqr.capacity_bytes as f64 / standard_qr_bytes as f64
    } else {
        0.0
    };
    let gain_percent = if standard_qr_bytes > 0 {
        difference_bytes as f64 / standard_qr_bytes as f64 * 100.0
    } else {
        0.0
    };
    let theoretical_bits = quadqr.theoretical_bits.max(1) as f64;
    let matrix_cells = (quadqr.size * quadqr.size) as f64;
    Ok(CapacityComparison {
        version,
        size: quadqr.size,
        ecc,
        quadqr_bytes: quadqr.capacity_bytes,
        standard_qr_bytes,
        difference_bytes,
        ratio,
        gain_percent,
        high_density: encoding == CellEncoding::Triangle16,
        cell_encoding: encoding,
        quadqr_bits_per_data_cell: quadqr.bits_per_data_cell,
        quadqr_payload_efficiency_percent: quadqr.capacity_bytes as f64 * 8.0 / theoretical_bits
            * 100.0,
        quadqr_payload_bits_per_matrix_cell: quadqr.capacity_bytes as f64 * 8.0 / matrix_cells,
        standard_qr_payload_bits_per_matrix_cell: standard_qr_bytes as f64 * 8.0 / matrix_cells,
    })
}

pub fn build_capacity_comparison(
    ecc: EccLevel,
    encoding: CellEncoding,
    versions: &[u8],
) -> Result<Vec<CapacityComparison>> {
    versions
        .iter()
        .map(|&version| compare_capacity(version, ecc, encoding))
        .collect()
}

#[derive(Debug, Clone)]
pub struct CapacityPlan {
    pub ecc: EccLevel,
    pub high_density: bool,
    pub cell_encoding: CellEncoding,
    pub source_bytes: usize,
    pub encoded_bytes: usize,
    pub stored_bytes: usize,
    pub extension_overhead_bytes: usize,
    pub compression: crate::compression::CompressionAlgorithm,
    pub compression_level: Option<u32>,
    pub compressed: bool,
    pub quadqr_version: Option<u8>,
    pub quadqr_size: Option<usize>,
    pub quadqr_capacity_bytes: Option<usize>,
    pub remaining_bytes: Option<usize>,
    pub utilization_percent: Option<f64>,
    pub standard_qr_version: Option<u8>,
    pub standard_qr_size: Option<usize>,
    pub matrix_width_savings_percent: Option<f64>,
}

pub fn calculate_capacity_plan(
    payload: &[u8],
    ecc: EccLevel,
    encoding: CellEncoding,
    compression: CompressionMode,
) -> Result<CapacityPlan> {
    let prepared = prepare_payload(payload, compression)?;
    let encoded_bytes = prepared.payload.len();
    let stored_bytes = if prepared.extended {
        let envelope = crate::compression::parse_payload_envelope(&prepared.payload)?;
        envelope.stored_payload.len()
    } else {
        payload.len()
    };
    let mut quadqr_version = None;
    let mut quadqr_size = None;
    let mut quadqr_capacity_bytes = None;
    for version in MIN_VERSION..=MAX_VERSION {
        let info = get_version_info(version, ecc, encoding)?;
        if encoded_bytes <= info.capacity_bytes {
            quadqr_version = Some(version);
            quadqr_size = Some(info.size);
            quadqr_capacity_bytes = Some(info.capacity_bytes);
            break;
        }
    }
    let standard_qr_version = standard_capacity_table(ecc)
        .iter()
        .position(|&capacity| payload.len() <= capacity)
        .map(|index| index as u8 + 1);
    let standard_qr_size = standard_qr_version.map(|version| 21 + 4 * (version as usize - 1));
    let matrix_width_savings_percent = match (quadqr_size, standard_qr_size) {
        (Some(quad), Some(standard)) if standard > 0 => {
            Some((1.0 - quad as f64 / standard as f64) * 100.0)
        }
        _ => None,
    };
    Ok(CapacityPlan {
        ecc,
        high_density: encoding == CellEncoding::Triangle16,
        cell_encoding: encoding,
        source_bytes: payload.len(),
        encoded_bytes,
        stored_bytes,
        extension_overhead_bytes: encoded_bytes.saturating_sub(stored_bytes),
        compression: prepared.compression,
        compression_level: prepared.compression_level,
        compressed: prepared.compression != crate::compression::CompressionAlgorithm::None,
        quadqr_version,
        quadqr_size,
        quadqr_capacity_bytes,
        remaining_bytes: quadqr_capacity_bytes
            .map(|capacity| capacity.saturating_sub(encoded_bytes)),
        utilization_percent: quadqr_capacity_bytes.map(|capacity| {
            if capacity == 0 {
                0.0
            } else {
                encoded_bytes as f64 / capacity as f64 * 100.0
            }
        }),
        standard_qr_version,
        standard_qr_size,
        matrix_width_savings_percent,
    })
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TimingSummary {
    pub mean_ms: f64,
    pub median_ms: f64,
    pub p95_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((p * sorted.len() as f64).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    sorted[index]
}

fn summarize(mut values: Vec<f64>) -> TimingSummary {
    if values.is_empty() {
        return TimingSummary::default();
    }
    values.sort_by(|a, b| a.total_cmp(b));
    TimingSummary {
        mean_ms: values.iter().sum::<f64>() / values.len() as f64,
        median_ms: percentile(&values, 0.5),
        p95_ms: percentile(&values, 0.95),
        min_ms: values[0],
        max_ms: values[values.len() - 1],
    }
}

fn benchmark_payload(length: usize, seed: u32) -> Vec<u8> {
    let mut state = seed ^ length as u32;
    (0..length)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as u8
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct CodecBenchmarkRow {
    pub payload_bytes: usize,
    pub version: u8,
    pub size: usize,
    pub capacity_bytes: usize,
    pub utilization_percent: f64,
    pub encode: TimingSummary,
    pub decode: TimingSummary,
    pub iterations: usize,
}

pub fn benchmark_codec(
    ecc: EccLevel,
    encoding: CellEncoding,
    payload_sizes: &[usize],
    iterations: usize,
    warmup: usize,
) -> Result<Vec<CodecBenchmarkRow>> {
    let iterations = iterations.max(1);
    let mut rows = Vec::new();
    for &payload_bytes in payload_sizes {
        let payload = benchmark_payload(payload_bytes, 0x51);
        let probe = encode_bytes(
            &payload,
            EncodeOptions {
                ecc,
                high_density: encoding == CellEncoding::Triangle16,
                ..EncodeOptions::default()
            },
        )?;
        for _ in 0..warmup {
            let encoded = encode_bytes(
                &payload,
                EncodeOptions {
                    ecc,
                    version: Some(probe.version),
                    high_density: encoding == CellEncoding::Triangle16,
                    ..EncodeOptions::default()
                },
            )?;
            let _ = decode_matrix(&encoded.matrix)?;
        }
        let mut encode_samples = Vec::with_capacity(iterations);
        let mut decode_samples = Vec::with_capacity(iterations);
        let mut last = probe;
        for _ in 0..iterations {
            let started = Instant::now();
            last = encode_bytes(
                &payload,
                EncodeOptions {
                    ecc,
                    version: Some(last.version),
                    high_density: encoding == CellEncoding::Triangle16,
                    ..EncodeOptions::default()
                },
            )?;
            encode_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            let started = Instant::now();
            let decoded = decode_matrix(&last.matrix)?;
            decode_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            if decoded.payload.len() != payload.len() {
                return Err(invalid("benchmark decode length mismatch"));
            }
        }
        let info = get_version_info(last.version, ecc, encoding)?;
        rows.push(CodecBenchmarkRow {
            payload_bytes,
            version: last.version,
            size: last.size,
            capacity_bytes: info.capacity_bytes,
            utilization_percent: if info.capacity_bytes == 0 {
                0.0
            } else {
                payload_bytes as f64 / info.capacity_bytes as f64 * 100.0
            },
            encode: summarize(encode_samples),
            decode: summarize(decode_samples),
            iterations,
        });
    }
    Ok(rows)
}

#[derive(Debug, Clone)]
pub struct BenchmarkReport {
    pub capacity: Vec<CapacityComparison>,
    pub performance: Vec<CodecBenchmarkRow>,
}

pub fn benchmark_report(
    ecc: EccLevel,
    encoding: CellEncoding,
    versions: &[u8],
    payload_sizes: &[usize],
    iterations: usize,
    warmup: usize,
) -> Result<BenchmarkReport> {
    Ok(BenchmarkReport {
        capacity: build_capacity_comparison(ecc, encoding, versions)?,
        performance: benchmark_codec(ecc, encoding, payload_sizes, iterations, warmup)?,
    })
}
