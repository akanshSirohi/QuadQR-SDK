use std::sync::OnceLock;

use crate::error::{QuadQrError, Result};

pub const FIELD_SIZE: usize = 256;
pub const FIELD_ORDER: usize = 255;
pub const MAX_CODEWORD_SYMBOLS: usize = 255;
const PRIMITIVE_POLYNOMIAL: u16 = 0x11d;

struct GfTables {
    exp: [u8; FIELD_ORDER * 2],
    log: [i16; FIELD_SIZE],
}

fn tables() -> &'static GfTables {
    static TABLES: OnceLock<GfTables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut exp = [0u8; FIELD_ORDER * 2];
        let mut log = [-1i16; FIELD_SIZE];
        let mut value: u16 = 1;
        for (i, entry) in exp.iter_mut().enumerate().take(FIELD_ORDER) {
            *entry = value as u8;
            log[value as usize] = i as i16;
            value <<= 1;
            if value & 0x100 != 0 {
                value ^= PRIMITIVE_POLYNOMIAL;
            }
        }
        for i in FIELD_ORDER..exp.len() {
            exp[i] = exp[i - FIELD_ORDER];
        }
        GfTables { exp, log }
    })
}

#[inline]
pub fn gf_add(a: u8, b: u8) -> u8 {
    a ^ b
}
#[inline]
pub fn gf_sub(a: u8, b: u8) -> u8 {
    a ^ b
}
#[inline]
pub fn gf_neg(a: u8) -> u8 {
    a
}

#[inline]
pub fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let t = tables();
    t.exp[(t.log[a as usize] as usize + t.log[b as usize] as usize) % FIELD_ORDER]
}

#[inline]
pub fn gf_div(a: u8, b: u8) -> Result<u8> {
    if b == 0 {
        return Err(QuadQrError::ReedSolomon(
            "division by zero in GF(256)".into(),
        ));
    }
    if a == 0 {
        return Ok(0);
    }
    let t = tables();
    let mut power = t.log[a as usize] - t.log[b as usize];
    if power < 0 {
        power += FIELD_ORDER as i16;
    }
    Ok(t.exp[power as usize])
}

pub fn gf_alpha_pow(power: i32) -> u8 {
    let mut p = power % FIELD_ORDER as i32;
    if p < 0 {
        p += FIELD_ORDER as i32;
    }
    tables().exp[p as usize]
}

fn poly_eval_high(coeffs: &[u8], x: u8) -> u8 {
    coeffs
        .iter()
        .fold(0u8, |y, &coeff| gf_add(gf_mul(y, x), coeff))
}

fn locator_eval_low(coeffs: &[u8], x: u8) -> u8 {
    let mut y = 0u8;
    let mut xp = 1u8;
    for &coeff in coeffs {
        y = gf_add(y, gf_mul(coeff, xp));
        xp = gf_mul(xp, x);
    }
    y
}

fn build_generator(parity: usize, first_root: i32) -> Vec<u8> {
    let mut generator = vec![1u8];
    for i in 0..parity {
        let root = gf_alpha_pow(first_root + i as i32);
        let factor = [1u8, root];
        let mut next = vec![0u8; generator.len() + 1];
        for a in 0..generator.len() {
            for b in 0..factor.len() {
                next[a + b] = gf_add(next[a + b], gf_mul(generator[a], factor[b]));
            }
        }
        generator = next;
    }
    generator
}

pub fn rs_encode(data: &[u8], parity: usize) -> Result<Vec<u8>> {
    if parity == 0 || data.len() + parity > MAX_CODEWORD_SYMBOLS {
        return Err(QuadQrError::ReedSolomon("invalid RS codeword size".into()));
    }
    let generator = build_generator(parity, 0);
    let mut work = Vec::with_capacity(data.len() + parity);
    work.extend_from_slice(data);
    work.resize(data.len() + parity, 0);
    for i in 0..data.len() {
        let coef = work[i];
        if coef == 0 {
            continue;
        }
        for (j, &g) in generator.iter().enumerate() {
            work[i + j] = gf_sub(work[i + j], gf_mul(coef, g));
        }
    }
    let mut out = data.to_vec();
    out.extend(work[data.len()..].iter().copied().map(gf_neg));
    Ok(out)
}

pub fn rs_syndromes(codeword: &[u8], parity: usize) -> Vec<u8> {
    (0..parity)
        .map(|i| poly_eval_high(codeword, gf_alpha_pow(i as i32)))
        .collect()
}

fn all_zero(values: &[u8]) -> bool {
    values.iter().all(|&v| v == 0)
}

fn berlekamp_massey(syndromes: &[u8]) -> Result<(Vec<u8>, usize)> {
    let n = syndromes.len();
    let mut c = vec![0u8; n + 1];
    let mut b_poly = vec![0u8; n + 1];
    c[0] = 1;
    b_poly[0] = 1;
    let mut l = 0usize;
    let mut m = 1usize;
    let mut b = 1u8;

    for index in 0..n {
        let mut discrepancy = syndromes[index];
        for i in 1..=l {
            discrepancy = gf_add(discrepancy, gf_mul(c[i], syndromes[index - i]));
        }
        if discrepancy == 0 {
            m += 1;
            continue;
        }
        let t = c.clone();
        let scale = gf_div(discrepancy, b)?;
        for i in 0..c.len().saturating_sub(m) {
            if b_poly[i] != 0 {
                c[i + m] = gf_sub(c[i + m], gf_mul(scale, b_poly[i]));
            }
        }
        if 2 * l <= index {
            l = index + 1 - l;
            b_poly = t;
            b = discrepancy;
            m = 1;
        } else {
            m += 1;
        }
    }
    Ok((c[..=l].to_vec(), l))
}

fn find_error_positions(locator: &[u8], codeword_len: usize) -> Vec<usize> {
    let mut positions = Vec::new();
    for power in 0..codeword_len {
        let x = gf_alpha_pow(-(power as i32));
        if locator_eval_low(locator, x) == 0 {
            positions.push(codeword_len - 1 - power);
        }
    }
    positions
}

fn solve_linear_system(mut matrix: Vec<Vec<u8>>, vector: Vec<u8>) -> Result<Vec<u8>> {
    let n = vector.len();
    for r in 0..n {
        matrix[r].push(vector[r]);
    }
    for col in 0..n {
        let mut pivot = col;
        while pivot < n && matrix[pivot][col] == 0 {
            pivot += 1;
        }
        if pivot == n {
            return Err(QuadQrError::ReedSolomon(
                "singular GF(256) correction system".into(),
            ));
        }
        if pivot != col {
            matrix.swap(pivot, col);
        }
        let inv = gf_div(1, matrix[col][col])?;
        for value in &mut matrix[col][col..=n] {
            *value = gf_mul(*value, inv);
        }
        let pivot_row = matrix[col].clone();
        for (row_index, row) in matrix.iter_mut().enumerate() {
            if row_index == col || row[col] == 0 {
                continue;
            }
            let factor = row[col];
            for (value, &pivot_value) in row[col..=n].iter_mut().zip(pivot_row[col..=n].iter()) {
                *value = gf_sub(*value, gf_mul(factor, pivot_value));
            }
        }
    }
    Ok(matrix.into_iter().map(|row| row[n]).collect())
}

fn solve_error_magnitudes(
    syndromes: &[u8],
    positions: &[usize],
    codeword_len: usize,
) -> Result<Vec<u8>> {
    let count = positions.len();
    let mut matrix = vec![vec![0u8; count]; count];
    let vector = syndromes[..count].to_vec();
    for (row_index, row) in matrix.iter_mut().enumerate() {
        let root_power = row_index as i32;
        for (cell, &position) in row.iter_mut().zip(positions.iter()) {
            let polynomial_power = codeword_len - 1 - position;
            *cell = gf_alpha_pow(root_power * polynomial_power as i32);
        }
    }
    solve_linear_system(matrix, vector)
}

fn forney_syndromes(syndromes: &[u8], erasures: &[usize], codeword_len: usize) -> Vec<u8> {
    let mut reduced = syndromes.to_vec();
    for &position in erasures {
        let polynomial_power = codeword_len - 1 - position;
        let x = gf_alpha_pow(polynomial_power as i32);
        for i in 0..reduced.len().saturating_sub(1) {
            reduced[i] = gf_add(gf_mul(reduced[i], x), reduced[i + 1]);
        }
        reduced.pop();
    }
    reduced
}

#[derive(Debug, Clone)]
pub struct RsDecode {
    pub corrected: Vec<u8>,
    pub data: Vec<u8>,
    pub corrected_symbols: usize,
    pub erasure_symbols: usize,
    pub unknown_error_symbols: usize,
    pub error_positions: Vec<usize>,
}

pub fn rs_decode(codeword: &[u8], parity: usize, erasure_positions: &[usize]) -> Result<RsDecode> {
    if codeword.len() > MAX_CODEWORD_SYMBOLS || parity == 0 || parity >= codeword.len() {
        return Err(QuadQrError::ReedSolomon(
            "invalid Reed-Solomon parity/codeword length".into(),
        ));
    }
    let mut erasures = erasure_positions.to_vec();
    erasures.sort_unstable();
    erasures.dedup();
    if erasures.iter().any(|&p| p >= codeword.len()) || erasures.len() > parity {
        return Err(QuadQrError::ReedSolomon("invalid erasure positions".into()));
    }

    let syndromes = rs_syndromes(codeword, parity);
    if all_zero(&syndromes) {
        return Ok(RsDecode {
            corrected: codeword.to_vec(),
            data: codeword[..codeword.len() - parity].to_vec(),
            corrected_symbols: 0,
            erasure_symbols: 0,
            unknown_error_symbols: 0,
            error_positions: vec![],
        });
    }

    let mut unknown_positions = Vec::new();
    if erasures.len() < parity {
        let reduced = if erasures.is_empty() {
            syndromes.clone()
        } else {
            forney_syndromes(&syndromes, &erasures, codeword.len())
        };
        if !all_zero(&reduced) {
            let (locator, error_count) = berlekamp_massey(&reduced)?;
            let limit = (parity - erasures.len()) / 2;
            if error_count == 0 || error_count > limit {
                return Err(QuadQrError::ReedSolomon(format!(
                    "found {error_count} unknown errors with {} erasures; limit is {limit}",
                    erasures.len()
                )));
            }
            unknown_positions = find_error_positions(&locator, codeword.len())
                .into_iter()
                .filter(|p| !erasures.contains(p))
                .collect();
            if unknown_positions.len() != error_count {
                return Err(QuadQrError::ReedSolomon(format!(
                    "locator found {}/{} unknown positions",
                    unknown_positions.len(),
                    error_count
                )));
            }
        }
    }

    let mut positions = erasures.clone();
    positions.extend_from_slice(&unknown_positions);
    positions.sort_unstable();
    positions.dedup();
    if positions.is_empty() || 2 * unknown_positions.len() + erasures.len() > parity {
        return Err(QuadQrError::ReedSolomon(
            "error/erasure budget exceeded".into(),
        ));
    }

    let magnitudes = solve_error_magnitudes(&syndromes, &positions, codeword.len())?;
    let mut corrected = codeword.to_vec();
    for (i, &position) in positions.iter().enumerate() {
        corrected[position] = gf_sub(corrected[position], magnitudes[i]);
    }
    if !all_zero(&rs_syndromes(&corrected, parity)) {
        return Err(QuadQrError::ReedSolomon(
            "correction failed syndrome verification".into(),
        ));
    }

    Ok(RsDecode {
        data: corrected[..corrected.len() - parity].to_vec(),
        corrected,
        corrected_symbols: positions.len(),
        erasure_symbols: erasures.len(),
        unknown_error_symbols: unknown_positions.len(),
        error_positions: positions,
    })
}
