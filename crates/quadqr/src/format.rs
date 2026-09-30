use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::error::{invalid, QuadQrError, Result};
use crate::geometry::{
    alignment_is_black, alignment_patterns, alignment_radius, size_for_version, validate_version,
    version_from_size, AlignmentPattern, AlignmentProfile, MAX_VERSION, MIN_VERSION,
};
use crate::rs::{rs_decode, rs_encode, MAX_CODEWORD_SYMBOLS};

pub const FORMAT_VERSION: u8 = 6;
pub const LEGACY_FORMAT_VERSION: u8 = 5;
pub const DEFAULT_ECC_LEVEL: EccLevel = EccLevel::M;

pub const BLACK: i16 = -1;
pub const RED: i16 = 0;
pub const GREEN: i16 = 1;
pub const BLUE: i16 = 2;
pub const WHITE: i16 = 3;

const MAGIC: [u8; 4] = *b"QQRW";
const HEADER_BYTES: usize = 10;
const HEADER_RS_PARITY: usize = 8;
const HEADER_CODEWORD_BYTES: usize = HEADER_BYTES + HEADER_RS_PARITY;
const RGBW_CELLS_PER_BYTE: usize = 4;
const TRIANGLE16_CELLS_PER_BYTE: usize = 2;
const HEADER_CODEWORD_CELLS: usize = HEADER_CODEWORD_BYTES * RGBW_CELLS_PER_BYTE;
const COMPACT_HEADER_MAGIC: u8 = 0xc3;
const COMPACT_HEADER_BYTES: usize = 4;
const COMPACT_HEADER_RS_PARITY: usize = 4;
const COMPACT_HEADER_CODEWORD_BYTES: usize = COMPACT_HEADER_BYTES + COMPACT_HEADER_RS_PARITY;
const COMPACT_HEADER_CODEWORD_CELLS: usize = COMPACT_HEADER_CODEWORD_BYTES * RGBW_CELLS_PER_BYTE;
const CRC_BYTES: usize = 4;

pub const TEXT_FLAG: u8 = 1;
pub const SECURE_FLAG: u8 = 1 << 3;
pub const EXTENDED_PAYLOAD_FLAG: u8 = 1 << 4;
pub const SIGNED_FLAG: u8 = 1 << 5;
pub const TRIANGLE16_FLAG: u8 = 1 << 6;
const ECC_SHIFT: u8 = 1;
const ECC_MASK: u8 = 0b0000_0110;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EccLevel {
    L,
    M,
    Q,
    H,
}

impl EccLevel {
    pub fn id(self) -> u8 {
        match self {
            Self::L => 0,
            Self::M => 1,
            Self::Q => 2,
            Self::H => 3,
        }
    }
    pub fn from_id(id: u8) -> Result<Self> {
        match id {
            0 => Ok(Self::L),
            1 => Ok(Self::M),
            2 => Ok(Self::Q),
            3 => Ok(Self::H),
            _ => Err(invalid(format!("unknown ECC id {id}"))),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::L => "L",
            Self::M => "M",
            Self::Q => "Q",
            Self::H => "H",
        }
    }
    pub fn parity(self, version: u8) -> usize {
        if version == 1 {
            match self {
                Self::L => 4,
                Self::M => 8,
                Self::Q => 12,
                Self::H => 16,
            }
        } else {
            match self {
                Self::L => 12,
                Self::M => 24,
                Self::Q => 36,
                Self::H => 48,
            }
        }
    }
    pub fn correctable(self, version: u8) -> usize {
        self.parity(version) / 2
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellEncoding {
    Rgbw,
    Triangle16,
}

impl CellEncoding {
    pub fn cells_per_body_byte(self) -> usize {
        match self {
            Self::Rgbw => RGBW_CELLS_PER_BYTE,
            Self::Triangle16 => TRIANGLE16_CELLS_PER_BYTE,
        }
    }
    pub fn high_density(self) -> bool {
        self == Self::Triangle16
    }
}

#[derive(Debug, Clone)]
pub struct EncodeOptions {
    pub ecc: EccLevel,
    pub version: Option<u8>,
    pub min_version: u8,
    pub max_version: u8,
    pub high_density: bool,
    pub text: bool,
    pub secure: bool,
    pub extended: bool,
    pub signed: bool,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            ecc: DEFAULT_ECC_LEVEL,
            version: None,
            min_version: MIN_VERSION,
            max_version: MAX_VERSION,
            high_density: false,
            text: false,
            secure: false,
            extended: false,
            signed: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: u8,
    pub format_version: u8,
    pub ecc_level: EccLevel,
    pub size: usize,
    pub data_cells: usize,
    pub theoretical_bits: usize,
    pub capacity_bytes: usize,
    pub header_cells: usize,
    pub header_bytes: usize,
    pub header_parity_symbols: usize,
    pub body_parity_symbols: usize,
    pub correctable_header_symbols: usize,
    pub correctable_symbols_per_block: usize,
    pub compact_small_symbol: bool,
    pub calibration_cells: usize,
    pub has_alignment_marker: bool,
    pub alignment_patterns: usize,
    pub alignment_centers: Vec<(usize, usize)>,
    pub bits_per_data_cell: usize,
    pub high_density: bool,
    pub cell_encoding: CellEncoding,
    pub colors: usize,
    pub states_per_data_cell: usize,
    pub spectral_interleaving: bool,
    pub confidence_aware_ecc: bool,
}

#[derive(Debug, Clone)]
pub struct QuadQrCode {
    pub format_version: u8,
    pub version: u8,
    pub size: usize,
    pub matrix: Vec<Vec<i16>>,
    pub mask_id: u8,
    pub payload_bytes: usize,
    pub meaningful_cells: usize,
    pub data_cells: usize,
    pub capacity_bytes: usize,
    pub utilization: f64,
    pub high_density: bool,
    pub cell_encoding: CellEncoding,
    pub ecc_level: EccLevel,
    pub ecc_parity_symbols: usize,
    pub ecc_blocks: usize,
    pub correctable_symbols_per_block: usize,
    pub crc32: u32,
}

#[derive(Debug, Clone)]
pub struct DecodeResult {
    pub format_version: u8,
    pub version: u8,
    pub size: usize,
    pub mask_id: u8,
    pub rotation: u8,
    pub flags: u8,
    pub high_density: bool,
    pub cell_encoding: CellEncoding,
    pub ecc_level: EccLevel,
    pub ecc_parity_symbols: usize,
    pub ecc_blocks: usize,
    pub corrected_header_symbols: usize,
    pub corrected_body_symbols: usize,
    pub erasure_symbols: usize,
    pub spectral_interleaving: bool,
    pub spectrum_ecc_version: u8,
    pub soft_decoded: bool,
    pub soft_substitutions: usize,
    pub soft_decode_attempts: usize,
    pub soft_decode_cells_considered: usize,
    pub payload: Vec<u8>,
    pub text: Option<String>,
    pub secure: bool,
    pub extended: bool,
    pub signed: bool,
    pub crc32: u32,
}

#[derive(Debug, Clone)]
pub struct CalibrationPositions {
    pub red: Vec<(usize, usize)>,
    pub green: Vec<(usize, usize)>,
    pub blue: Vec<(usize, usize)>,
    pub black: Vec<(usize, usize)>,
    pub white: Vec<(usize, usize)>,
}

impl CalibrationPositions {
    fn new() -> Self {
        Self {
            red: vec![],
            green: vec![],
            blue: vec![],
            black: vec![],
            white: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub version: u8,
    pub size: usize,
    pub alignment_profile: AlignmentProfile,
    pub matrix: Vec<Vec<i16>>,
    pub reserved: Vec<Vec<bool>>,
    pub data_positions: Vec<(usize, usize)>,
    pub calibration: CalibrationPositions,
    pub alignments: Vec<AlignmentPattern>,
}

#[derive(Debug, Clone)]
struct HeaderPlan {
    parity_symbols: usize,
    codeword_bytes: usize,
    codeword_cells: usize,
}

#[derive(Debug, Clone)]
struct BodyPlan {
    body_byte_count: usize,
    data_symbols: usize,
    parity_symbols: usize,
    data_block_lengths: Vec<usize>,
    codeword_block_lengths: Vec<usize>,
    encoded_symbols: usize,
    encoded_cells: usize,
}

fn header_plan(version: u8) -> HeaderPlan {
    if version == 1 {
        HeaderPlan {
            parity_symbols: COMPACT_HEADER_RS_PARITY,
            codeword_bytes: COMPACT_HEADER_CODEWORD_BYTES,
            codeword_cells: COMPACT_HEADER_CODEWORD_CELLS,
        }
    } else {
        HeaderPlan {
            parity_symbols: HEADER_RS_PARITY,
            codeword_bytes: HEADER_CODEWORD_BYTES,
            codeword_cells: HEADER_CODEWORD_CELLS,
        }
    }
}

pub fn alignment_profile_for_format(format_version: u8) -> AlignmentProfile {
    if format_version == LEGACY_FORMAT_VERSION {
        AlignmentProfile::Legacy3
    } else {
        AlignmentProfile::Standard5
    }
}

pub fn layout_for_version(version: u8) -> Result<Layout> {
    create_layout(version, AlignmentProfile::Standard5)
}

fn create_layout(version: u8, profile: AlignmentProfile) -> Result<Layout> {
    validate_version(version)?;
    let size = size_for_version(version)?;
    let mut matrix = vec![vec![WHITE; size]; size];
    let mut reserved = vec![vec![false; size]; size];
    let mut calibration = CalibrationPositions::new();

    #[allow(clippy::too_many_arguments)]
    fn reserve(
        size: usize,
        matrix: &mut [Vec<i16>],
        reserved: &mut [Vec<bool>],
        calibration: &mut CalibrationPositions,
        row: isize,
        col: isize,
        value: i16,
        key: Option<&str>,
    ) {
        if row < 0 || col < 0 || row >= size as isize || col >= size as isize {
            return;
        }
        let r = row as usize;
        let c = col as usize;
        reserved[r][c] = true;
        matrix[r][c] = value;
        match key {
            Some("red") => calibration.red.push((r, c)),
            Some("green") => calibration.green.push((r, c)),
            Some("blue") => calibration.blue.push((r, c)),
            Some("black") => calibration.black.push((r, c)),
            Some("white") => calibration.white.push((r, c)),
            _ => {}
        }
    }

    let draw_finder = |top: usize,
                       left: usize,
                       matrix: &mut Vec<Vec<i16>>,
                       reserved: &mut Vec<Vec<bool>>,
                       calibration: &mut CalibrationPositions| {
        for r in -1isize..=7 {
            for c in -1isize..=7 {
                let rr = top as isize + r;
                let cc = left as isize + c;
                if r == -1 || c == -1 || r == 7 || c == 7 {
                    reserve(
                        size,
                        matrix,
                        reserved,
                        calibration,
                        rr,
                        cc,
                        WHITE,
                        Some("white"),
                    );
                }
            }
        }
        for r in 0..7 {
            for c in 0..7 {
                let outer = r == 0 || c == 0 || r == 6 || c == 6;
                let center = (2..=4).contains(&r) && (2..=4).contains(&c);
                let black = outer || center;
                reserve(
                    size,
                    matrix,
                    reserved,
                    calibration,
                    (top + r) as isize,
                    (left + c) as isize,
                    if black { BLACK } else { WHITE },
                    Some(if black { "black" } else { "white" }),
                );
            }
        }
    };

    draw_finder(0, 0, &mut matrix, &mut reserved, &mut calibration);
    draw_finder(0, size - 7, &mut matrix, &mut reserved, &mut calibration);
    draw_finder(size - 7, 0, &mut matrix, &mut reserved, &mut calibration);

    for col in 8..size.saturating_sub(8) {
        reserve(
            size,
            &mut matrix,
            &mut reserved,
            &mut calibration,
            6,
            col as isize,
            if col % 2 == 0 { BLACK } else { WHITE },
            None,
        );
    }
    for row in 8..size.saturating_sub(8) {
        reserve(
            size,
            &mut matrix,
            &mut reserved,
            &mut calibration,
            row as isize,
            6,
            if row % 2 == 0 { BLACK } else { WHITE },
            None,
        );
    }

    let alignments = alignment_patterns(version, profile)?;
    for pattern in &alignments {
        let radius = alignment_radius(pattern);
        if pattern.separator {
            let sep = radius + 1;
            for ro in -sep..=sep {
                for co in -sep..=sep {
                    if ro.abs() == sep || co.abs() == sep {
                        reserve(
                            size,
                            &mut matrix,
                            &mut reserved,
                            &mut calibration,
                            pattern.row as isize + ro,
                            pattern.col as isize + co,
                            WHITE,
                            Some("white"),
                        );
                    }
                }
            }
        }
        for ro in -radius..=radius {
            for co in -radius..=radius {
                let black = alignment_is_black(pattern, ro, co).unwrap_or(false);
                reserve(
                    size,
                    &mut matrix,
                    &mut reserved,
                    &mut calibration,
                    pattern.row as isize + ro,
                    pattern.col as isize + co,
                    if black { BLACK } else { WHITE },
                    Some(if black { "black" } else { "white" }),
                );
            }
        }
    }

    let is_free =
        |row0: usize, col0: usize, h: usize, w: usize, reserved: &Vec<Vec<bool>>| -> bool {
            if row0 + h > size || col0 + w > size {
                return false;
            }
            for row in reserved.iter().skip(row0).take(h) {
                if row.iter().skip(col0).take(w).any(|&v| v) {
                    return false;
                }
            }
            true
        };

    let preferred = (size.saturating_sub(6), size.saturating_sub(13));
    let origin = if is_free(preferred.0, preferred.1, 2, 6, &reserved) {
        preferred
    } else {
        let mut found = None;
        if size >= 16 {
            'outer: for row in (8..=size - 8).rev() {
                for col in (8..=size - 8).rev() {
                    if is_free(row, col, 2, 6, &reserved) {
                        found = Some((row, col));
                        break 'outer;
                    }
                }
            }
        }
        found.ok_or_else(|| {
            invalid(format!(
                "unable to reserve calibration strip for version {version}"
            ))
        })?
    };

    for (key, cell, offset) in [("red", RED, 0usize), ("green", GREEN, 2), ("blue", BLUE, 4)] {
        for row in [origin.0, origin.0 + 1] {
            for dc in 0..2 {
                reserve(
                    size,
                    &mut matrix,
                    &mut reserved,
                    &mut calibration,
                    row as isize,
                    (origin.1 + offset + dc) as isize,
                    cell,
                    Some(key),
                );
            }
        }
    }

    let mut data_positions = Vec::new();
    let mut upward = true;
    let mut col = size as isize - 1;
    while col >= 0 {
        if col == 6 {
            col -= 1;
        }
        for i in 0..size {
            let row = if upward { size - 1 - i } else { i };
            for dx in 0..2isize {
                let c = col - dx;
                if c >= 0 && !reserved[row][c as usize] {
                    data_positions.push((row, c as usize));
                }
            }
        }
        upward = !upward;
        col -= 2;
    }

    Ok(Layout {
        version,
        size,
        alignment_profile: profile,
        matrix,
        reserved,
        data_positions,
        calibration,
        alignments,
    })
}

fn mask_value(row: usize, col: usize, mask_id: u8) -> Result<i16> {
    let v = match mask_id {
        0 => (row + col) & 3,
        1 => (2 * row + col) & 3,
        2 => (row + 2 * col) & 3,
        3 => (row * col + row + col) & 3,
        _ => return Err(invalid(format!("unknown mask {mask_id}"))),
    };
    Ok(v as i16)
}

fn pack_triangle(a: i16, b: i16) -> Result<i16> {
    if !(0..=3).contains(&a) || !(0..=3).contains(&b) {
        return Err(invalid("invalid Triangle16 color"));
    }
    Ok(((a & 3) << 2) | (b & 3))
}

fn unpack_triangle(value: i16) -> Result<(i16, i16)> {
    if !(0..=15).contains(&value) {
        return Err(invalid("invalid Triangle16 cell"));
    }
    Ok(((value >> 2) & 3, value & 3))
}

fn solid_triangle(color: i16) -> Result<i16> {
    pack_triangle(color, color)
}

fn triangle_mask_value(row: usize, col: usize, mask_id: u8) -> Result<i16> {
    let first = mask_value(row, col, mask_id)?;
    let second = mask_value(row + 1, col + 2, (mask_id + 1) & 3)?;
    pack_triangle(first, second)
}

fn apply_cell_mask(
    value: i16,
    row: usize,
    col: usize,
    mask_id: u8,
    encoding: CellEncoding,
    protected_header: bool,
) -> Result<i16> {
    if encoding == CellEncoding::Rgbw {
        return Ok(value ^ mask_value(row, col, mask_id)?);
    }
    if protected_header {
        let (a, b) = unpack_triangle(value)?;
        if a != b {
            return Err(invalid("Triangle16 protected header must be solid"));
        }
        return solid_triangle(a ^ mask_value(row, col, mask_id)?);
    }
    Ok(value ^ triangle_mask_value(row, col, mask_id)?)
}

type SpectralCache = Mutex<HashMap<(usize, u8), Vec<usize>>>;

fn spectral_cache() -> &'static SpectralCache {
    static CACHE: OnceLock<SpectralCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn spectral_permutation(length: usize, version: u8) -> Vec<usize> {
    if let Some(v) = spectral_cache()
        .lock()
        .ok()
        .and_then(|m| m.get(&(length, version)).cloned())
    {
        return v;
    }
    let mut permutation: Vec<usize> = (0..length).collect();
    let mut state = 0x9e37_79b9u32
        ^ (version as u32 + 1).wrapping_mul(0x85eb_ca6b)
        ^ (length as u32 + 17).wrapping_mul(0xc2b2_ae35);
    let mut next_random = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };
    for index in (1..length).rev() {
        let swap_index = (next_random() as usize) % (index + 1);
        permutation.swap(index, swap_index);
    }
    if let Ok(mut cache) = spectral_cache().lock() {
        cache.insert((length, version), permutation.clone());
    }
    permutation
}

fn bytes_to_cells(bytes: &[u8], encoding: CellEncoding) -> Vec<i16> {
    let mut out = Vec::with_capacity(bytes.len() * encoding.cells_per_body_byte());
    match encoding {
        CellEncoding::Rgbw => {
            for &byte in bytes {
                out.push(((byte >> 6) & 3) as i16);
                out.push(((byte >> 4) & 3) as i16);
                out.push(((byte >> 2) & 3) as i16);
                out.push((byte & 3) as i16);
            }
        }
        CellEncoding::Triangle16 => {
            for &byte in bytes {
                out.push(((byte >> 4) & 0x0f) as i16);
                out.push((byte & 0x0f) as i16);
            }
        }
    }
    out
}

fn bytes_to_header_cells(bytes: &[u8], encoding: CellEncoding) -> Result<Vec<i16>> {
    let base = bytes_to_cells(bytes, CellEncoding::Rgbw);
    if encoding == CellEncoding::Rgbw {
        return Ok(base);
    }
    base.into_iter().map(solid_triangle).collect()
}

fn cells_to_bytes(cells: &[i16], byte_count: usize, encoding: CellEncoding) -> Result<Vec<u8>> {
    let need = byte_count * encoding.cells_per_body_byte();
    if cells.len() < need {
        return Err(QuadQrError::Decode("not enough data cells".into()));
    }
    let mut out = vec![0u8; byte_count];
    match encoding {
        CellEncoding::Rgbw => {
            for (i, byte) in out.iter_mut().enumerate() {
                let o = i * 4;
                let vals = [cells[o], cells[o + 1], cells[o + 2], cells[o + 3]];
                if vals.iter().any(|v| !(0..=3).contains(v)) {
                    return Err(QuadQrError::Decode("invalid RGBW data cell".into()));
                }
                *byte = ((vals[0] as u8) << 6)
                    | ((vals[1] as u8) << 4)
                    | ((vals[2] as u8) << 2)
                    | vals[3] as u8;
            }
        }
        CellEncoding::Triangle16 => {
            for (i, byte) in out.iter_mut().enumerate() {
                let o = i * 2;
                if !(0..=15).contains(&cells[o]) || !(0..=15).contains(&cells[o + 1]) {
                    return Err(QuadQrError::Decode("invalid Triangle16 data cell".into()));
                }
                *byte = ((cells[o] as u8) << 4) | cells[o + 1] as u8;
            }
        }
    }
    Ok(out)
}

fn header_cells_to_bytes(
    cells: &[i16],
    byte_count: usize,
    encoding: CellEncoding,
) -> Result<Vec<u8>> {
    if encoding == CellEncoding::Rgbw {
        return cells_to_bytes(cells, byte_count, CellEncoding::Rgbw);
    }
    let mut colors = Vec::with_capacity(cells.len());
    for &value in cells {
        let (a, b) = unpack_triangle(value)?;
        if a != b {
            return Err(QuadQrError::Decode(
                "Triangle16 protected header lost solid-color integrity".into(),
            ));
        }
        colors.push(a);
    }
    cells_to_bytes(&colors, byte_count, CellEncoding::Rgbw)
}

pub fn crc32(bytes: &[u8]) -> u32 {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = [0u32; 256];
        for (n, entry) in table.iter_mut().enumerate() {
            let mut c = n as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xedb8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            *entry = c;
        }
        table
    });
    let mut crc = 0xffff_ffffu32;
    for &byte in bytes {
        crc = table[((crc ^ byte as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffff_ffff
}

fn body_plan(
    payload_length: usize,
    ecc: EccLevel,
    version: u8,
    encoding: CellEncoding,
) -> BodyPlan {
    let parity = ecc.parity(version);
    let body_byte_count = payload_length + CRC_BYTES;
    let max_data_per_block = MAX_CODEWORD_SYMBOLS - parity;
    let mut data_block_lengths = Vec::new();
    let mut remaining = body_byte_count;
    while remaining > 0 {
        let length = remaining.min(max_data_per_block);
        data_block_lengths.push(length);
        remaining -= length;
    }
    let codeword_block_lengths: Vec<_> = data_block_lengths.iter().map(|&l| l + parity).collect();
    let encoded_symbols = codeword_block_lengths.iter().sum();
    BodyPlan {
        body_byte_count,
        data_symbols: body_byte_count,
        parity_symbols: parity,
        data_block_lengths,
        codeword_block_lengths,
        encoded_symbols,
        encoded_cells: encoded_symbols * encoding.cells_per_body_byte(),
    }
}

fn stream_cell_count(
    payload_length: usize,
    ecc: EccLevel,
    version: u8,
    encoding: CellEncoding,
) -> usize {
    header_plan(version).codeword_cells
        + body_plan(payload_length, ecc, version, encoding).encoded_cells
}

pub fn capacity_bytes(version: u8, ecc: EccLevel, encoding: CellEncoding) -> Result<usize> {
    let layout = create_layout(version, AlignmentProfile::Standard5)?;
    if stream_cell_count(0, ecc, version, encoding) > layout.data_positions.len() {
        return Ok(0);
    }
    let mut low = 0usize;
    let mut high = layout.data_positions.len() / encoding.cells_per_body_byte();
    while low < high {
        let mid = (low + high).div_ceil(2);
        if stream_cell_count(mid, ecc, version, encoding) <= layout.data_positions.len() {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    Ok(low)
}

pub fn get_version_info(version: u8, ecc: EccLevel, encoding: CellEncoding) -> Result<VersionInfo> {
    validate_version(version)?;
    let layout = create_layout(version, AlignmentProfile::Standard5)?;
    let header = header_plan(version);
    let patterns = alignment_patterns(version, AlignmentProfile::Standard5)?;
    let bits_per_data_cell = if encoding == CellEncoding::Triangle16 {
        4
    } else {
        2
    };
    Ok(VersionInfo {
        version,
        format_version: FORMAT_VERSION,
        ecc_level: ecc,
        size: layout.size,
        data_cells: layout.data_positions.len(),
        theoretical_bits: layout.data_positions.len() * bits_per_data_cell,
        capacity_bytes: capacity_bytes(version, ecc, encoding)?,
        header_cells: header.codeword_cells,
        header_bytes: if version == 1 {
            COMPACT_HEADER_BYTES
        } else {
            HEADER_BYTES
        },
        header_parity_symbols: header.parity_symbols,
        body_parity_symbols: ecc.parity(version),
        correctable_header_symbols: header.parity_symbols / 2,
        correctable_symbols_per_block: ecc.correctable(version),
        compact_small_symbol: version == 1,
        calibration_cells: 12,
        has_alignment_marker: !patterns.is_empty(),
        alignment_patterns: patterns.len(),
        alignment_centers: patterns.iter().map(|p| (p.row, p.col)).collect(),
        bits_per_data_cell,
        high_density: encoding == CellEncoding::Triangle16,
        cell_encoding: encoding,
        colors: 4,
        states_per_data_cell: if encoding == CellEncoding::Triangle16 {
            16
        } else {
            4
        },
        spectral_interleaving: true,
        confidence_aware_ecc: true,
    })
}

pub fn rotate_matrix(matrix: &[Vec<i16>], quarter_turns: i32) -> Result<Vec<Vec<i16>>> {
    if matrix.is_empty() || matrix.iter().any(|row| row.len() != matrix.len()) {
        return Err(invalid("matrix must be non-empty and square"));
    }
    let mut out = matrix.to_vec();
    let turns = quarter_turns.rem_euclid(4);
    for _ in 0..turns {
        let size = out.len();
        let mut next = vec![vec![WHITE; size]; size];
        for row in 0..size {
            for col in 0..size {
                next[col][size - 1 - row] = out[row][col];
            }
        }
        out = next;
    }
    Ok(out)
}

pub fn encode_uint8_array(payload: &[u8], options: EncodeOptions) -> Result<QuadQrCode> {
    encode_bytes(payload, options)
}

pub fn decode_uint8_array(matrix: &[Vec<i16>]) -> Result<Vec<u8>> {
    Ok(decode_matrix(matrix)?.payload)
}

fn choose_version(
    payload_length: usize,
    options: &EncodeOptions,
    encoding: CellEncoding,
) -> Result<u8> {
    validate_version(options.min_version)?;
    validate_version(options.max_version)?;
    if options.min_version > options.max_version {
        return Err(invalid("min_version must be <= max_version"));
    }
    if let Some(version) = options.version {
        validate_version(version)?;
        if version < options.min_version || version > options.max_version {
            return Err(invalid("requested version outside bounds"));
        }
        let layout = create_layout(version, AlignmentProfile::Standard5)?;
        if stream_cell_count(payload_length, options.ecc, version, encoding)
            <= layout.data_positions.len()
        {
            return Ok(version);
        }
        return Err(QuadQrError::Capacity);
    }
    for version in options.min_version..=options.max_version {
        let layout = create_layout(version, AlignmentProfile::Standard5)?;
        if stream_cell_count(payload_length, options.ecc, version, encoding)
            <= layout.data_positions.len()
        {
            return Ok(version);
        }
    }
    Err(QuadQrError::Capacity)
}

fn make_header(payload_length: usize, flags: u8, version: u8) -> Result<Vec<u8>> {
    if version == 1 {
        if payload_length > 255 {
            return Err(QuadQrError::Capacity);
        }
        return Ok(vec![
            COMPACT_HEADER_MAGIC,
            flags,
            payload_length as u8,
            (payload_length as u8) ^ 0xff,
        ]);
    }
    if payload_length > u32::MAX as usize {
        return Err(QuadQrError::Capacity);
    }
    let mut header = vec![0u8; HEADER_BYTES];
    header[..4].copy_from_slice(&MAGIC);
    header[4] = FORMAT_VERSION;
    header[5] = flags;
    header[6..10].copy_from_slice(&(payload_length as u32).to_be_bytes());
    Ok(header)
}

fn parse_header(header: &[u8], version: u8) -> Result<(u8, u8, usize)> {
    if version == 1 {
        if header.len() != 4 || header[0] != COMPACT_HEADER_MAGIC || header[3] != (header[2] ^ 0xff)
        {
            return Err(QuadQrError::Decode("compact header mismatch".into()));
        }
        return Ok((FORMAT_VERSION, header[1], header[2] as usize));
    }
    if header.len() != HEADER_BYTES || header[..4] != MAGIC {
        return Err(QuadQrError::Decode("QuadQR magic mismatch".into()));
    }
    let format_version = header[4];
    if format_version != FORMAT_VERSION && format_version != LEGACY_FORMAT_VERSION {
        return Err(QuadQrError::Decode(format!(
            "unsupported format version {format_version}"
        )));
    }
    let payload_length = u32::from_be_bytes([header[6], header[7], header[8], header[9]]) as usize;
    Ok((format_version, header[5], payload_length))
}

fn flags_for(options: &EncodeOptions, encoding: CellEncoding) -> u8 {
    (if options.text { TEXT_FLAG } else { 0 })
        | ((options.ecc.id() << ECC_SHIFT) & ECC_MASK)
        | (if options.secure { SECURE_FLAG } else { 0 })
        | (if options.extended {
            EXTENDED_PAYLOAD_FLAG
        } else {
            0
        })
        | (if options.signed { SIGNED_FLAG } else { 0 })
        | (if encoding == CellEncoding::Triangle16 {
            TRIANGLE16_FLAG
        } else {
            0
        })
}

fn make_padding_cells(count: usize, seed: u32, encoding: CellEncoding) -> Vec<i16> {
    let mut state = if seed == 0 { 0x6d2b_79f5 } else { seed };
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        out.push(if encoding == CellEncoding::Triangle16 {
            (state & 0x0f) as i16
        } else {
            (state & 3) as i16
        });
    }
    out
}

fn interleave_blocks(blocks: &[Vec<u8>]) -> Vec<u8> {
    let max = blocks.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = Vec::new();
    for index in 0..max {
        for block in blocks {
            if index < block.len() {
                out.push(block[index]);
            }
        }
    }
    out
}

fn deinterleave_blocks<T: Copy>(stream: &[T], lengths: &[usize]) -> Result<Vec<Vec<T>>> {
    let mut blocks: Vec<Vec<Option<T>>> = lengths.iter().map(|&l| vec![None; l]).collect();
    let mut cursor = 0usize;
    let max = lengths.iter().copied().max().unwrap_or(0);
    for index in 0..max {
        for (block, &len) in blocks.iter_mut().zip(lengths.iter()) {
            if index < len {
                if cursor >= stream.len() {
                    return Err(QuadQrError::Decode(
                        "interleaved RS stream too short".into(),
                    ));
                }
                block[index] = Some(stream[cursor]);
                cursor += 1;
            }
        }
    }
    if cursor != stream.len() {
        return Err(QuadQrError::Decode(
            "interleaved RS stream length mismatch".into(),
        ));
    }
    Ok(blocks
        .into_iter()
        .map(|b| b.into_iter().map(Option::unwrap).collect())
        .collect())
}

fn encode_header(header: &[u8], version: u8, encoding: CellEncoding) -> Result<Vec<i16>> {
    let plan = header_plan(version);
    let codeword = rs_encode(header, plan.parity_symbols)?;
    if codeword.len() != plan.codeword_bytes {
        return Err(QuadQrError::Decode("header RS sizing mismatch".into()));
    }
    bytes_to_header_cells(&codeword, encoding)
}

fn encode_body(
    body: &[u8],
    payload_length: usize,
    ecc: EccLevel,
    version: u8,
    encoding: CellEncoding,
) -> Result<(Vec<i16>, BodyPlan)> {
    let plan = body_plan(payload_length, ecc, version, encoding);
    if body.len() != plan.data_symbols {
        return Err(invalid("body RS symbol calculation mismatch"));
    }
    let mut blocks = Vec::new();
    let mut offset = 0usize;
    for &length in &plan.data_block_lengths {
        blocks.push(rs_encode(
            &body[offset..offset + length],
            plan.parity_symbols,
        )?);
        offset += length;
    }
    Ok((bytes_to_cells(&interleave_blocks(&blocks), encoding), plan))
}

fn apply_data(
    layout: &Layout,
    raw_cells: &[i16],
    mask_id: u8,
    spectral: bool,
    encoding: CellEncoding,
) -> Result<Vec<Vec<i16>>> {
    let mut matrix = layout.matrix.clone();
    let permutation = if spectral {
        Some(spectral_permutation(
            layout.data_positions.len(),
            layout.version,
        ))
    } else {
        None
    };
    let header_cells = header_plan(layout.version).codeword_cells;
    for logical_index in 0..layout.data_positions.len() {
        let physical_index = permutation
            .as_ref()
            .map(|p| p[logical_index])
            .unwrap_or(logical_index);
        let (row, col) = layout.data_positions[physical_index];
        matrix[row][col] = apply_cell_mask(
            raw_cells[logical_index],
            row,
            col,
            mask_id,
            encoding,
            encoding == CellEncoding::Triangle16 && logical_index < header_cells,
        )?;
    }
    Ok(matrix)
}

fn data_penalty(
    matrix: &[Vec<i16>],
    reserved: &[Vec<bool>],
    encoding: CellEncoding,
) -> Result<f64> {
    let size = matrix.len();
    let mut penalty = 0f64;
    let mut counts = [0usize; 4];
    let mut color_count = 0usize;
    let mut add_colors = |value: i16| -> Result<()> {
        if encoding == CellEncoding::Triangle16 {
            let (a, b) = unpack_triangle(value)?;
            counts[a as usize] += 1;
            counts[b as usize] += 1;
            color_count += 2;
        } else {
            if !(0..=3).contains(&value) {
                return Err(invalid("invalid data cell while scoring mask"));
            }
            counts[value as usize] += 1;
            color_count += 1;
        }
        Ok(())
    };
    for r in 0..size {
        let mut previous: Option<i16> = None;
        let mut run = 0;
        for c in 0..size {
            if reserved[r][c] {
                previous = None;
                run = 0;
                continue;
            }
            let value = matrix[r][c];
            add_colors(value)?;
            if previous == Some(value) {
                run += 1;
                if run >= 4 {
                    penalty += 2.0;
                }
            } else {
                previous = Some(value);
                run = 1;
            }
        }
    }
    for c in 0..size {
        let mut previous: Option<i16> = None;
        let mut run = 0;
        for r in 0..size {
            if reserved[r][c] {
                previous = None;
                run = 0;
                continue;
            }
            let value = matrix[r][c];
            if previous == Some(value) {
                run += 1;
                if run >= 4 {
                    penalty += 2.0;
                }
            } else {
                previous = Some(value);
                run = 1;
            }
        }
    }
    if color_count > 0 {
        let ideal = color_count as f64 / 4.0;
        penalty += counts
            .iter()
            .map(|&n| (n as f64 - ideal).abs())
            .sum::<f64>()
            / 2.0;
    }
    Ok(penalty)
}

pub fn encode_prepared_bytes(payload: &[u8], options: &EncodeOptions) -> Result<QuadQrCode> {
    let encoding = if options.high_density {
        CellEncoding::Triangle16
    } else {
        CellEncoding::Rgbw
    };
    let version = choose_version(payload.len(), options, encoding)?;
    let layout = create_layout(version, AlignmentProfile::Standard5)?;
    let flags = flags_for(options, encoding);
    let header = make_header(payload.len(), flags, version)?;
    let mut crc_input = header.clone();
    crc_input.extend_from_slice(payload);
    let crc = crc32(&crc_input);
    let header_cells = encode_header(&header, version, encoding)?;
    let mut body = payload.to_vec();
    body.extend_from_slice(&crc.to_be_bytes());
    let (body_cells, plan) = encode_body(&body, payload.len(), options.ecc, version, encoding)?;
    let mut meaningful = header_cells;
    meaningful.extend(body_cells);
    if meaningful.len() > layout.data_positions.len() {
        return Err(QuadQrError::Capacity);
    }
    let seed =
        crc ^ payload.len() as u32 ^ ((version as u32) << 24) ^ ((options.ecc.id() as u32) << 16);
    let padding = make_padding_cells(
        layout.data_positions.len() - meaningful.len(),
        seed,
        encoding,
    );
    let meaningful_len = meaningful.len();
    meaningful.extend(padding);

    let mut best_mask = 0u8;
    let mut best_matrix = None;
    let mut best_penalty = f64::INFINITY;
    for mask in 0..4u8 {
        let matrix = apply_data(&layout, &meaningful, mask, true, encoding)?;
        let penalty = data_penalty(&matrix, &layout.reserved, encoding)?;
        if penalty < best_penalty {
            best_penalty = penalty;
            best_mask = mask;
            best_matrix = Some(matrix);
        }
    }
    let capacity = capacity_bytes(version, options.ecc, encoding)?;
    Ok(QuadQrCode {
        format_version: FORMAT_VERSION,
        version,
        size: layout.size,
        matrix: best_matrix.ok_or_else(|| invalid("unable to select mask"))?,
        mask_id: best_mask,
        payload_bytes: payload.len(),
        meaningful_cells: meaningful_len,
        data_cells: layout.data_positions.len(),
        capacity_bytes: capacity,
        utilization: meaningful_len as f64 / layout.data_positions.len() as f64,
        high_density: encoding == CellEncoding::Triangle16,
        cell_encoding: encoding,
        ecc_level: options.ecc,
        ecc_parity_symbols: plan.parity_symbols,
        ecc_blocks: plan.data_block_lengths.len(),
        correctable_symbols_per_block: options.ecc.correctable(version),
        crc32: crc,
    })
}

pub fn encode_bytes(payload: &[u8], mut options: EncodeOptions) -> Result<QuadQrCode> {
    options.text = false;
    encode_prepared_bytes(payload, &options)
}

pub fn encode_text(text: &str, mut options: EncodeOptions) -> Result<QuadQrCode> {
    options.text = true;
    encode_prepared_bytes(text.as_bytes(), &options)
}

fn rotate90(matrix: &[Vec<i16>]) -> Vec<Vec<i16>> {
    let size = matrix.len();
    let mut out = vec![vec![WHITE; size]; size];
    for r in 0..size {
        for c in 0..size {
            out[c][size - 1 - r] = matrix[r][c];
        }
    }
    out
}

fn rotate90_f32(matrix: &[Vec<f32>]) -> Vec<Vec<f32>> {
    let size = matrix.len();
    let mut out = vec![vec![1.0; size]; size];
    for r in 0..size {
        for c in 0..size {
            out[c][size - 1 - r] = matrix[r][c];
        }
    }
    out
}

fn finder_mismatch(matrix: &[Vec<i16>], top: usize, left: usize) -> f64 {
    let mut mismatch = 0usize;
    for r in 0..7 {
        for c in 0..7 {
            let outer = r == 0 || c == 0 || r == 6 || c == 6;
            let center = (2..=4).contains(&r) && (2..=4).contains(&c);
            let expected = if outer || center { BLACK } else { WHITE };
            if matrix
                .get(top + r)
                .and_then(|row| row.get(left + c))
                .copied()
                != Some(expected)
            {
                mismatch += 1;
            }
        }
    }
    mismatch as f64 / 49.0
}

fn alignment_mismatch(matrix: &[Vec<i16>], pattern: &AlignmentPattern) -> f64 {
    let radius = alignment_radius(pattern);
    let mut mismatch = 0usize;
    let mut total = 0usize;
    for ro in -radius..=radius {
        for co in -radius..=radius {
            let expected = if alignment_is_black(pattern, ro, co).unwrap_or(false) {
                BLACK
            } else {
                WHITE
            };
            total += 1;
            let row = (pattern.row as isize + ro) as usize;
            let col = (pattern.col as isize + co) as usize;
            if matrix.get(row).and_then(|r| r.get(col)).copied() != Some(expected) {
                mismatch += 1;
            }
        }
    }
    mismatch as f64 / total.max(1) as f64
}

fn validate_structure(matrix: &[Vec<i16>], tolerance: f64, profile: AlignmentProfile) -> bool {
    let size = matrix.len();
    if size < 21 || matrix.iter().any(|r| r.len() != size) {
        return false;
    }
    let Some(version) = version_from_size(size) else {
        return false;
    };
    let finder_ratios = [
        finder_mismatch(matrix, 0, 0),
        finder_mismatch(matrix, 0, size - 7),
        finder_mismatch(matrix, size - 7, 0),
    ];
    if finder_ratios.iter().any(|&x| x > tolerance) {
        return false;
    }
    let Ok(patterns) = alignment_patterns(version, profile) else {
        return false;
    };
    let align_tol = tolerance.max(0.12);
    if let Some(primary) = patterns.last() {
        if alignment_mismatch(matrix, primary) > align_tol {
            return false;
        }
    }
    let mut weighted = 0f64;
    let mut weight = 0f64;
    for p in patterns {
        let w = if p.primary { 2.0 } else { 1.0 };
        weighted += alignment_mismatch(matrix, &p) * w;
        weight += w;
    }
    weight == 0.0 || weighted / weight <= align_tol
}

fn extract_visible(
    matrix: &[Vec<i16>],
    layout: &Layout,
    encoding: CellEncoding,
) -> Result<Vec<i16>> {
    let max = if encoding == CellEncoding::Triangle16 {
        15
    } else {
        3
    };
    let mut out = Vec::with_capacity(layout.data_positions.len());
    for &(r, c) in &layout.data_positions {
        let value = matrix[r][c];
        if value < 0 || value > max {
            return Err(QuadQrError::Decode(
                "data region contains invalid cell".into(),
            ));
        }
        out.push(value);
    }
    Ok(out)
}

fn unmask_restore(
    visible: &[i16],
    positions: &[(usize, usize)],
    mask: u8,
    version: u8,
    spectral: bool,
    encoding: CellEncoding,
) -> Result<Vec<i16>> {
    let permutation = if spectral {
        Some(spectral_permutation(visible.len(), version))
    } else {
        None
    };
    let mut logical = vec![0i16; visible.len()];
    let header_cells = header_plan(version).codeword_cells;
    for logical_index in 0..visible.len() {
        let physical_index = permutation
            .as_ref()
            .map(|p| p[logical_index])
            .unwrap_or(logical_index);
        let (row, col) = positions[physical_index];
        logical[logical_index] = apply_cell_mask(
            visible[physical_index],
            row,
            col,
            mask,
            encoding,
            encoding == CellEncoding::Triangle16 && logical_index < header_cells,
        )?;
    }
    Ok(logical)
}

fn restore_confidence(physical: &[f32], version: u8, spectral: bool) -> Vec<f32> {
    if !spectral {
        return physical.to_vec();
    }
    let permutation = spectral_permutation(physical.len(), version);
    let mut logical = vec![1.0f32; physical.len()];
    for i in 0..permutation.len() {
        logical[i] = physical[permutation[i]];
    }
    logical
}

fn symbol_confidence(
    confidences: Option<&[f32]>,
    bytes: usize,
    cells_per_byte: usize,
) -> Option<Vec<f32>> {
    let c = confidences?;
    if c.len() < bytes * cells_per_byte {
        return None;
    }
    Some(
        (0..bytes)
            .map(|i| {
                let start = i * cells_per_byte;
                c[start..start + cells_per_byte]
                    .iter()
                    .copied()
                    .fold(1.0f32, f32::min)
            })
            .collect(),
    )
}

fn decode_rs_adaptive(
    codeword: &[u8],
    parity: usize,
    confidences: Option<&[f32]>,
    max_erasure_confidence: f32,
) -> Result<crate::rs::RsDecode> {
    match rs_decode(codeword, parity, &[]) {
        Ok(v) => Ok(v),
        Err(hard) => {
            let Some(confidences) = confidences else {
                return Err(hard);
            };
            if confidences.len() != codeword.len() {
                return Err(hard);
            }
            let mut ranked: Vec<(f32, usize)> = confidences
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(p, c)| (c <= max_erasure_confidence).then_some((c, p)))
                .collect();
            ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let limit = parity.min(ranked.len());
            let mut last = hard;
            for count in 1..=limit {
                let erasures: Vec<usize> = ranked[..count].iter().map(|x| x.1).collect();
                match rs_decode(codeword, parity, &erasures) {
                    Ok(v) => return Ok(v),
                    Err(e) => last = e,
                }
            }
            Err(last)
        }
    }
}

fn decode_header(
    raw: &[i16],
    version: u8,
    confidences: Option<&[f32]>,
    encoding: CellEncoding,
    max_erasure_confidence: f32,
) -> Result<(u8, u8, usize, usize, usize)> {
    let plan = header_plan(version);
    let codeword = header_cells_to_bytes(
        &raw[..plan.codeword_cells.min(raw.len())],
        plan.codeword_bytes,
        encoding,
    )?;
    let symbol_conf = symbol_confidence(
        confidences.map(|x| &x[..plan.codeword_cells.min(x.len())]),
        plan.codeword_bytes,
        RGBW_CELLS_PER_BYTE,
    );
    let decoded = decode_rs_adaptive(
        &codeword,
        plan.parity_symbols,
        symbol_conf.as_deref(),
        max_erasure_confidence,
    )?;
    let (format_version, flags, payload_length) = parse_header(&decoded.data, version)?;
    Ok((
        format_version,
        flags,
        payload_length,
        decoded.corrected_symbols,
        decoded.erasure_symbols,
    ))
}

fn decode_body(
    raw: &[i16],
    payload_length: usize,
    ecc: EccLevel,
    version: u8,
    confidences: Option<&[f32]>,
    encoding: CellEncoding,
    max_erasure_confidence: f32,
) -> Result<(Vec<u8>, usize, usize, BodyPlan)> {
    let plan = body_plan(payload_length, ecc, version, encoding);
    let start = header_plan(version).codeword_cells;
    if raw.len() < start + plan.encoded_cells {
        return Err(QuadQrError::Decode("protected body incomplete".into()));
    }
    let symbols = cells_to_bytes(
        &raw[start..start + plan.encoded_cells],
        plan.encoded_symbols,
        encoding,
    )?;
    let symbol_conf = symbol_confidence(
        confidences.and_then(|c| c.get(start..start + plan.encoded_cells)),
        plan.encoded_symbols,
        encoding.cells_per_body_byte(),
    );
    let blocks = deinterleave_blocks(&symbols, &plan.codeword_block_lengths)?;
    let confidence_blocks = if let Some(c) = symbol_conf {
        Some(deinterleave_blocks(&c, &plan.codeword_block_lengths)?)
    } else {
        None
    };
    let mut data = Vec::new();
    let mut corrected = 0;
    let mut erasures = 0;
    for (i, block) in blocks.iter().enumerate() {
        let conf = confidence_blocks.as_ref().map(|v| v[i].as_slice());
        let decoded = decode_rs_adaptive(block, plan.parity_symbols, conf, max_erasure_confidence)?;
        corrected += decoded.corrected_symbols;
        erasures += decoded.erasure_symbols;
        data.extend_from_slice(&decoded.data);
    }
    data.truncate(plan.body_byte_count);
    Ok((data, corrected, erasures, plan))
}

fn decode_canonical(
    matrix: &[Vec<i16>],
    rotation: u8,
    confidence: Option<&[Vec<f32>]>,
    tolerance: f64,
    profile: AlignmentProfile,
    max_erasure_confidence: f32,
) -> Result<DecodeResult> {
    let size = matrix.len();
    let version = version_from_size(size)
        .ok_or_else(|| QuadQrError::Decode(format!("unsupported matrix size {size}")))?;
    if !validate_structure(matrix, tolerance, profile) {
        return Err(QuadQrError::Structure(
            "finder/alignment structure mismatch".into(),
        ));
    }
    let layout = create_layout(version, profile)?;
    let has_extended = layout.data_positions.iter().any(|&(r, c)| matrix[r][c] > 3);
    let attempts = if has_extended {
        [CellEncoding::Triangle16, CellEncoding::Rgbw]
    } else {
        [CellEncoding::Rgbw, CellEncoding::Triangle16]
    };
    let mut errors = Vec::new();
    for encoding in attempts {
        let visible = match extract_visible(matrix, &layout, encoding) {
            Ok(v) => v,
            Err(e) => {
                errors.push(e.to_string());
                continue;
            }
        };
        let physical_conf = confidence.map(|m| {
            layout
                .data_positions
                .iter()
                .map(|&(r, c)| m[r][c])
                .collect::<Vec<_>>()
        });
        for spectral in [true, false] {
            for mask in 0..4u8 {
                let attempt = (|| -> Result<DecodeResult> {
                    let raw = unmask_restore(
                        &visible,
                        &layout.data_positions,
                        mask,
                        version,
                        spectral,
                        encoding,
                    )?;
                    let raw_conf = physical_conf
                        .as_ref()
                        .map(|c| restore_confidence(c, version, spectral));
                    let (format_version, flags, payload_length, header_corrected, header_erasures) =
                        decode_header(
                            &raw,
                            version,
                            raw_conf.as_deref(),
                            encoding,
                            max_erasure_confidence,
                        )?;
                    if version >= 7 && alignment_profile_for_format(format_version) != profile {
                        return Err(QuadQrError::Decode(
                            "alignment profile does not match decoded format version".into(),
                        ));
                    }
                    let declared = if flags & TRIANGLE16_FLAG != 0 {
                        CellEncoding::Triangle16
                    } else {
                        CellEncoding::Rgbw
                    };
                    if declared != encoding {
                        return Err(QuadQrError::Decode("header cell encoding mismatch".into()));
                    }
                    let ecc = EccLevel::from_id((flags & ECC_MASK) >> ECC_SHIFT)?;
                    if stream_cell_count(payload_length, ecc, version, encoding) > raw.len() {
                        return Err(QuadQrError::Decode(
                            "declared payload exceeds matrix capacity".into(),
                        ));
                    }
                    let (body, body_corrected, body_erasures, plan) = decode_body(
                        &raw,
                        payload_length,
                        ecc,
                        version,
                        raw_conf.as_deref(),
                        encoding,
                        max_erasure_confidence,
                    )?;
                    if body.len() < payload_length + 4 {
                        return Err(QuadQrError::Decode("decoded body too short".into()));
                    }
                    let payload = body[..payload_length].to_vec();
                    let expected = u32::from_be_bytes([
                        body[payload_length],
                        body[payload_length + 1],
                        body[payload_length + 2],
                        body[payload_length + 3],
                    ]);
                    let header = make_header(payload_length, flags, version)?;
                    let mut crc_input = header;
                    crc_input.extend_from_slice(&payload);
                    let actual = crc32(&crc_input);
                    if expected != actual {
                        return Err(QuadQrError::Decode("CRC mismatch after ECC".into()));
                    }
                    let text = if flags & TEXT_FLAG != 0 && flags & SECURE_FLAG == 0 {
                        Some(String::from_utf8_lossy(&payload).into_owned())
                    } else {
                        None
                    };
                    Ok(DecodeResult {
                        format_version,
                        version,
                        size,
                        mask_id: mask,
                        rotation,
                        flags,
                        high_density: encoding == CellEncoding::Triangle16,
                        cell_encoding: encoding,
                        ecc_level: ecc,
                        ecc_parity_symbols: plan.parity_symbols,
                        ecc_blocks: plan.data_block_lengths.len(),
                        corrected_header_symbols: header_corrected,
                        corrected_body_symbols: body_corrected,
                        erasure_symbols: header_erasures + body_erasures,
                        spectral_interleaving: spectral,
                        spectrum_ecc_version: 2,
                        soft_decoded: false,
                        soft_substitutions: 0,
                        soft_decode_attempts: 0,
                        soft_decode_cells_considered: 0,
                        payload,
                        text,
                        secure: flags & SECURE_FLAG != 0,
                        extended: flags & EXTENDED_PAYLOAD_FLAG != 0,
                        signed: flags & SIGNED_FLAG != 0,
                        crc32: actual,
                    })
                })();
                match attempt {
                    Ok(v) => return Ok(v),
                    Err(e) => {
                        errors.push(format!("{encoding:?}/spectral={spectral}/mask={mask}: {e}"))
                    }
                }
            }
        }
    }
    Err(QuadQrError::Decode(errors.join(" | ")))
}

#[derive(Debug, Clone, Copy)]
pub struct SoftDecodeOptions {
    pub confidence_threshold: f32,
    pub max_cells: usize,
    pub pair_cells: usize,
}

impl Default for SoftDecodeOptions {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.72,
            max_cells: 10,
            pair_cells: 6,
        }
    }
}

fn rotate90_i16(matrix: &[Vec<i16>]) -> Vec<Vec<i16>> {
    rotate90(matrix)
}

#[allow(clippy::too_many_arguments)]
fn try_soft_decode(
    matrix: &[Vec<i16>],
    alternatives: &[Vec<i16>],
    confidence: &[Vec<f32>],
    rotation: u8,
    tolerance: f64,
    profile: AlignmentProfile,
    options: SoftDecodeOptions,
    max_erasure_confidence: f32,
) -> Option<DecodeResult> {
    let version = version_from_size(matrix.len())?;
    let mut positions = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for p in [AlignmentProfile::Standard5, AlignmentProfile::Legacy3] {
        let layout = create_layout(version, p).ok()?;
        for pos in layout.data_positions {
            if seen.insert(pos) {
                positions.push(pos);
            }
        }
    }
    let threshold = options.confidence_threshold.clamp(0.05, 0.95);
    let max_cells = options.max_cells.clamp(2, 16);
    let pair_cells = options.pair_cells.clamp(2, max_cells);
    let mut ranked = Vec::<(f32, usize, usize, i16)>::new();
    for (row, col) in positions {
        let alt = *alternatives.get(row)?.get(col)?;
        let current = *matrix.get(row)?.get(col)?;
        let conf = *confidence.get(row)?.get(col)?;
        if alt != current && conf <= threshold {
            ranked.push((conf, row, col, alt));
        }
    }
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    ranked.truncate(max_cells);
    let considered = ranked.len();
    let mut attempts = 0usize;
    let mut attempt = |changes: &[(usize, usize, i16)]| -> Option<DecodeResult> {
        let mut trial = matrix.to_vec();
        for &(r, c, v) in changes {
            trial[r][c] = v;
        }
        attempts += 1;
        match decode_canonical(
            &trial,
            rotation,
            Some(confidence),
            tolerance,
            profile,
            max_erasure_confidence,
        ) {
            Ok(mut decoded) => {
                decoded.spectrum_ecc_version = 2;
                decoded.soft_decoded = true;
                decoded.soft_substitutions = changes.len();
                decoded.soft_decode_attempts = attempts;
                decoded.soft_decode_cells_considered = considered;
                Some(decoded)
            }
            Err(_) => None,
        }
    };
    for &(_, r, c, alt) in &ranked {
        if let Some(v) = attempt(&[(r, c, alt)]) {
            return Some(v);
        }
    }
    let pair_limit = pair_cells.min(ranked.len());
    for i in 0..pair_limit {
        for j in i + 1..pair_limit {
            let (_, r1, c1, a1) = ranked[i];
            let (_, r2, c2, a2) = ranked[j];
            if let Some(v) = attempt(&[(r1, c1, a1), (r2, c2, a2)]) {
                return Some(v);
            }
        }
    }
    None
}

pub fn decode_matrix(matrix: &[Vec<i16>]) -> Result<DecodeResult> {
    decode_matrix_with_options(matrix, None, None, DecodeOptions::default())
}

pub fn decode_matrix_with_confidence(
    matrix: &[Vec<i16>],
    confidence: Option<&[Vec<f32>]>,
    tolerance: f64,
) -> Result<DecodeResult> {
    decode_matrix_with_options(
        matrix,
        confidence,
        None,
        DecodeOptions {
            structure_tolerance: tolerance,
            ..DecodeOptions::default()
        },
    )
}

pub fn decode_matrix_with_soft(
    matrix: &[Vec<i16>],
    confidence: Option<&[Vec<f32>]>,
    alternatives: Option<&[Vec<i16>]>,
    tolerance: f64,
    soft_options: SoftDecodeOptions,
) -> Result<DecodeResult> {
    decode_matrix_with_options(
        matrix,
        confidence,
        alternatives,
        DecodeOptions {
            structure_tolerance: tolerance,
            soft_decoding: true,
            soft: soft_options,
            ..DecodeOptions::default()
        },
    )
}

#[derive(Debug, Clone, Copy)]
pub struct DecodeOptions {
    pub structure_tolerance: f64,
    pub max_erasure_confidence: f32,
    pub soft_decoding: bool,
    pub soft: SoftDecodeOptions,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            structure_tolerance: 0.0,
            max_erasure_confidence: 0.68,
            soft_decoding: true,
            soft: SoftDecodeOptions::default(),
        }
    }
}

pub fn decode_matrix_with_options(
    matrix: &[Vec<i16>],
    confidence: Option<&[Vec<f32>]>,
    alternatives: Option<&[Vec<i16>]>,
    options: DecodeOptions,
) -> Result<DecodeResult> {
    if matrix.is_empty() || matrix.iter().any(|r| r.len() != matrix.len()) {
        return Err(invalid("square matrix required"));
    }
    if let Some(c) = confidence {
        if c.len() != matrix.len() || c.iter().any(|r| r.len() != matrix.len()) {
            return Err(invalid("confidence matrix must match QuadQR matrix"));
        }
    }
    if let Some(a) = alternatives {
        if a.len() != matrix.len() || a.iter().any(|r| r.len() != matrix.len()) {
            return Err(invalid("alternative matrix must match QuadQR matrix"));
        }
    }

    let tolerance = options.structure_tolerance;
    let max_erasure_confidence = options.max_erasure_confidence.clamp(0.0, 1.0);
    let mut current = matrix.to_vec();
    let mut conf_current = confidence.map(|c| c.to_vec());
    let mut alt_current = alternatives.map(|a| a.to_vec());
    let mut errors = Vec::new();

    for rotation in 0..4u8 {
        for profile in [AlignmentProfile::Standard5, AlignmentProfile::Legacy3] {
            match decode_canonical(
                &current,
                rotation,
                conf_current.as_deref(),
                tolerance,
                profile,
                max_erasure_confidence,
            ) {
                Ok(v) => return Ok(v),
                Err(e) => {
                    errors.push(format!("rot={rotation}/{profile:?}: {e}"));
                    if options.soft_decoding {
                        if let (Some(conf), Some(alts)) =
                            (conf_current.as_deref(), alt_current.as_deref())
                        {
                            if let Some(v) = try_soft_decode(
                                &current,
                                alts,
                                conf,
                                rotation,
                                tolerance,
                                profile,
                                options.soft,
                                max_erasure_confidence,
                            ) {
                                return Ok(v);
                            }
                        }
                    }
                }
            }
        }
        current = rotate90(&current);
        if let Some(c) = conf_current.take() {
            conf_current = Some(rotate90_f32(&c));
        }
        if let Some(a) = alt_current.take() {
            alt_current = Some(rotate90_i16(&a));
        }
    }
    Err(QuadQrError::Decode(errors.join(" || ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_rgbw() {
        let code = encode_text("Hello QuadQR Rust", EncodeOptions::default()).unwrap();
        let decoded = decode_matrix(&code.matrix).unwrap();
        assert_eq!(decoded.text.as_deref(), Some("Hello QuadQR Rust"));
        assert_eq!(decoded.version, code.version);
    }

    #[test]
    fn round_trip_triangle16() {
        let mut options = EncodeOptions::default();
        options.high_density = true;
        let code = encode_text("Triangle16 compatibility", options).unwrap();
        let decoded = decode_matrix(&code.matrix).unwrap();
        assert_eq!(decoded.text.as_deref(), Some("Triangle16 compatibility"));
        assert!(decoded.high_density);
    }

    #[test]
    fn crc_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }
}
