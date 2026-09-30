#[cfg(feature = "compression")]
use std::io::{Read, Write};

use crate::error::{invalid, QuadQrError, Result};

pub const PAYLOAD_ENVELOPE_MAGIC: [u8; 4] = *b"QPX1";
pub const PAYLOAD_ENVELOPE_VERSION: u8 = 3;
pub const PAYLOAD_ENVELOPE_HEADER_BYTES: usize = 16;

pub const PAYLOAD_ENVELOPE_SIGNED_FLAG: u8 = 1;
pub const PAYLOAD_ENVELOPE_COMPRESSED_FLAG: u8 = 1 << 1;
pub const PAYLOAD_ENVELOPE_EMBEDDED_KEY_FLAG: u8 = 1 << 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    None,
    Lz,
    Deflate,
    Brotli,
}

impl CompressionAlgorithm {
    pub fn id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Lz => 1,
            Self::Deflate => 2,
            Self::Brotli => 3,
        }
    }

    pub fn from_id(id: u8) -> Result<Self> {
        match id {
            0 => Ok(Self::None),
            1 => Ok(Self::Lz),
            2 => Ok(Self::Deflate),
            3 => Ok(Self::Brotli),
            _ => Err(invalid(format!("unsupported QuadQR compression id {id}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionMode {
    #[default]
    None,
    Auto,
    Smart,
    Lz(u8),
    Deflate(u32),
    Brotli(u32),
}

#[derive(Debug, Clone)]
pub struct PreparedPayload {
    pub payload: Vec<u8>,
    pub extended: bool,
    pub compression: CompressionAlgorithm,
    pub compression_level: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct PayloadEnvelope {
    pub header: [u8; PAYLOAD_ENVELOPE_HEADER_BYTES],
    pub version: u8,
    pub flags: u8,
    pub compression: CompressionAlgorithm,
    pub signature_algorithm: u8,
    pub original_length: usize,
    pub key_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
    pub stored_payload: Vec<u8>,
    pub payload: Vec<u8>,
}

impl PayloadEnvelope {
    pub fn signed(&self) -> bool {
        self.flags & PAYLOAD_ENVELOPE_SIGNED_FLAG != 0
    }
    pub fn compressed(&self) -> bool {
        self.flags & PAYLOAD_ENVELOPE_COMPRESSED_FLAG != 0
    }
    pub fn has_embedded_public_key(&self) -> bool {
        self.flags & PAYLOAD_ENVELOPE_EMBEDDED_KEY_FLAG != 0
    }
}

pub fn compress_lz(input: &[u8], level: u8) -> Result<Vec<u8>> {
    if !(1..=9).contains(&level) {
        return Err(invalid("LZ level must be 1..9"));
    }
    let candidate_depth = [0usize, 4, 8, 12, 16, 24, 32, 48, 64, 96][level as usize];
    let lazy_depth = [0usize, 0, 0, 0, 0, 0, 0, 12, 24, 48][level as usize];
    let lazy_gain = if level >= 9 {
        0
    } else if level >= 7 {
        1
    } else {
        2
    };
    let history_limit = candidate_depth.max(lazy_depth).max(4);

    use std::collections::HashMap;
    let mut recent: HashMap<u32, Vec<usize>> = HashMap::new();
    let key_at = |i: usize| -> Option<u32> {
        if i + 2 < input.len() {
            Some(((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | input[i + 2] as u32)
        } else {
            None
        }
    };

    fn remember(
        recent: &mut std::collections::HashMap<u32, Vec<usize>>,
        input: &[u8],
        i: usize,
        history_limit: usize,
    ) {
        if i + 2 >= input.len() {
            return;
        }
        let key = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | input[i + 2] as u32;
        let list = recent.entry(key).or_default();
        list.push(i);
        while list.len() > history_limit {
            list.remove(0);
        }
        while !list.is_empty() && i.saturating_sub(list[0]) > 4095 {
            list.remove(0);
        }
    }

    fn find_best(
        recent: &std::collections::HashMap<u32, Vec<usize>>,
        input: &[u8],
        position: usize,
        max_candidates: usize,
    ) -> (usize, usize) {
        if position + 2 >= input.len() {
            return (0, 0);
        }
        let key = ((input[position] as u32) << 16)
            | ((input[position + 1] as u32) << 8)
            | input[position + 2] as u32;
        let Some(candidates) = recent.get(&key) else {
            return (0, 0);
        };
        let mut best_len = 0usize;
        let mut best_offset = 0usize;
        for (checked, &candidate) in candidates.iter().rev().enumerate() {
            if checked >= max_candidates {
                break;
            }
            let offset = position.saturating_sub(candidate);
            if offset == 0 || offset > 4095 {
                continue;
            }
            let mut len = 0usize;
            while len < 18
                && position + len < input.len()
                && input[candidate + len] == input[position + len]
            {
                len += 1;
            }
            if len >= 3 && len > best_len {
                best_len = len;
                best_offset = offset;
                if len == 18 {
                    break;
                }
            }
        }
        (best_len, best_offset)
    }

    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < input.len() {
        let flag_index = out.len();
        out.push(0);
        let mut flags = 0u8;
        for token in 0..8u8 {
            if pos >= input.len() {
                break;
            }
            let (mut best_len, mut best_offset) = find_best(&recent, input, pos, candidate_depth);
            if best_len >= 3 && lazy_depth > 0 && pos + 1 < input.len() {
                let (next_len, _) = find_best(&recent, input, pos + 1, lazy_depth);
                if next_len > best_len + lazy_gain {
                    best_len = 0;
                    best_offset = 0;
                }
            }
            if best_len >= 3 {
                flags |= 1 << token;
                let encoded =
                    (((best_offset as u16) & 0x0fff) << 4) | (((best_len - 3) as u16) & 0x0f);
                out.push((encoded >> 8) as u8);
                out.push(encoded as u8);
                for i in 0..best_len {
                    remember(&mut recent, input, pos + i, history_limit);
                }
                pos += best_len;
            } else {
                out.push(input[pos]);
                remember(&mut recent, input, pos, history_limit);
                pos += 1;
            }
        }
        out[flag_index] = flags;
    }
    let _ = key_at;
    Ok(out)
}

pub fn decompress_lz(input: &[u8], expected_length: Option<usize>) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < input.len() {
        let flags = input[pos];
        pos += 1;
        for token in 0..8u8 {
            if pos >= input.len() {
                break;
            }
            if flags & (1 << token) != 0 {
                if pos + 1 >= input.len() {
                    return Err(QuadQrError::Compression(
                        "compressed payload is truncated".into(),
                    ));
                }
                let encoded = ((input[pos] as u16) << 8) | input[pos + 1] as u16;
                pos += 2;
                let offset = ((encoded >> 4) & 0x0fff) as usize;
                let length = ((encoded & 0x0f) + 3) as usize;
                if offset == 0 || offset > out.len() {
                    return Err(QuadQrError::Compression("invalid LZ back-reference".into()));
                }
                for _ in 0..length {
                    let value = out[out.len() - offset];
                    out.push(value);
                }
            } else {
                out.push(input[pos]);
                pos += 1;
            }
            if expected_length.is_some_and(|n| out.len() >= n) {
                break;
            }
        }
    }
    if let Some(expected) = expected_length {
        if out.len() != expected {
            return Err(QuadQrError::Compression(
                "decompressed payload length mismatch".into(),
            ));
        }
    }
    Ok(out)
}

#[cfg(feature = "compression")]
pub fn compress_deflate(input: &[u8], level: u32) -> Result<Vec<u8>> {
    use flate2::{write::DeflateEncoder, Compression};
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(level.min(9)));
    encoder.write_all(input)?;
    encoder.finish().map_err(QuadQrError::Io)
}

#[cfg(feature = "compression")]
pub fn decompress_deflate(input: &[u8], expected_length: Option<usize>) -> Result<Vec<u8>> {
    use flate2::read::DeflateDecoder;
    let mut decoder = DeflateDecoder::new(input);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    if let Some(expected) = expected_length {
        if out.len() != expected {
            return Err(QuadQrError::Compression(
                "DEFLATE payload length mismatch".into(),
            ));
        }
    }
    Ok(out)
}

#[cfg(feature = "compression")]
pub fn compress_brotli(input: &[u8], quality: u32) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut writer = brotli::CompressorWriter::new(&mut out, 4096, quality.min(11), 22);
        writer.write_all(input)?;
    }
    Ok(out)
}

#[cfg(feature = "compression")]
pub fn decompress_brotli(input: &[u8], expected_length: Option<usize>) -> Result<Vec<u8>> {
    let mut decoder = brotli::Decompressor::new(input, 4096);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    if let Some(expected) = expected_length {
        if out.len() != expected {
            return Err(QuadQrError::Compression(
                "Brotli payload length mismatch".into(),
            ));
        }
    }
    Ok(out)
}

fn make_candidate(input: &[u8], algorithm: CompressionAlgorithm, level: u32) -> Result<Vec<u8>> {
    match algorithm {
        CompressionAlgorithm::None => Ok(input.to_vec()),
        CompressionAlgorithm::Lz => compress_lz(input, level.clamp(1, 9) as u8),
        CompressionAlgorithm::Deflate => {
            #[cfg(feature = "compression")]
            {
                compress_deflate(input, level.clamp(1, 9))
            }
            #[cfg(not(feature = "compression"))]
            {
                Err(QuadQrError::Compression(
                    "compression feature disabled".into(),
                ))
            }
        }
        CompressionAlgorithm::Brotli => {
            #[cfg(feature = "compression")]
            {
                compress_brotli(input, level.min(11))
            }
            #[cfg(not(feature = "compression"))]
            {
                Err(QuadQrError::Compression(
                    "compression feature disabled".into(),
                ))
            }
        }
    }
}

pub fn make_envelope_header(
    compression: CompressionAlgorithm,
    flags: u8,
    original_length: usize,
    key_id_length: usize,
    public_key_length: usize,
    signature_length: usize,
    signature_algorithm: u8,
) -> Result<[u8; PAYLOAD_ENVELOPE_HEADER_BYTES]> {
    if key_id_length > 255 || public_key_length > 255 || signature_length > 255 {
        return Err(invalid("payload extension metadata field too large"));
    }
    if original_length > u32::MAX as usize {
        return Err(invalid("payload is too large"));
    }
    let mut header = [0u8; PAYLOAD_ENVELOPE_HEADER_BYTES];
    header[..4].copy_from_slice(&PAYLOAD_ENVELOPE_MAGIC);
    header[4] = PAYLOAD_ENVELOPE_VERSION;
    header[5] = flags;
    header[6] = compression.id();
    header[7] = signature_algorithm;
    header[8..12].copy_from_slice(&(original_length as u32).to_be_bytes());
    header[12] = key_id_length as u8;
    header[13] = public_key_length as u8;
    header[14] = signature_length as u8;
    header[15] = 0;
    Ok(header)
}

pub fn pack_compressed_envelope(
    input: &[u8],
    algorithm: CompressionAlgorithm,
    body: &[u8],
) -> Result<Vec<u8>> {
    let flags = if algorithm == CompressionAlgorithm::None {
        0
    } else {
        PAYLOAD_ENVELOPE_COMPRESSED_FLAG
    };
    let header = make_envelope_header(algorithm, flags, input.len(), 0, 0, 0, 0)?;
    let mut out = Vec::with_capacity(header.len() + body.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(body);
    Ok(out)
}

pub fn prepare_payload(input: &[u8], mode: CompressionMode) -> Result<PreparedPayload> {
    match mode {
        CompressionMode::None => Ok(PreparedPayload {
            payload: input.to_vec(),
            extended: false,
            compression: CompressionAlgorithm::None,
            compression_level: None,
        }),
        CompressionMode::Lz(level) => {
            let body = make_candidate(input, CompressionAlgorithm::Lz, level as u32)?;
            Ok(PreparedPayload {
                payload: pack_compressed_envelope(input, CompressionAlgorithm::Lz, &body)?,
                extended: true,
                compression: CompressionAlgorithm::Lz,
                compression_level: Some(level as u32),
            })
        }
        CompressionMode::Deflate(level) => {
            let body = make_candidate(input, CompressionAlgorithm::Deflate, level)?;
            Ok(PreparedPayload {
                payload: pack_compressed_envelope(input, CompressionAlgorithm::Deflate, &body)?,
                extended: true,
                compression: CompressionAlgorithm::Deflate,
                compression_level: Some(level),
            })
        }
        CompressionMode::Brotli(level) => {
            let body = make_candidate(input, CompressionAlgorithm::Brotli, level)?;
            Ok(PreparedPayload {
                payload: pack_compressed_envelope(input, CompressionAlgorithm::Brotli, &body)?,
                extended: true,
                compression: CompressionAlgorithm::Brotli,
                compression_level: Some(level),
            })
        }
        CompressionMode::Auto | CompressionMode::Smart => {
            let mut candidates = vec![(CompressionAlgorithm::None, input.to_vec(), None)];
            candidates.push((
                CompressionAlgorithm::Lz,
                make_candidate(input, CompressionAlgorithm::Lz, 6)?,
                Some(6),
            ));
            #[cfg(feature = "compression")]
            {
                candidates.push((
                    CompressionAlgorithm::Deflate,
                    make_candidate(input, CompressionAlgorithm::Deflate, 6)?,
                    Some(6),
                ));
                candidates.push((
                    CompressionAlgorithm::Brotli,
                    make_candidate(input, CompressionAlgorithm::Brotli, 6)?,
                    Some(6),
                ));
                if mode == CompressionMode::Smart {
                    candidates.push((
                        CompressionAlgorithm::Deflate,
                        make_candidate(input, CompressionAlgorithm::Deflate, 8)?,
                        Some(8),
                    ));
                    candidates.push((
                        CompressionAlgorithm::Brotli,
                        make_candidate(input, CompressionAlgorithm::Brotli, 9)?,
                        Some(9),
                    ));
                    candidates.push((
                        CompressionAlgorithm::Deflate,
                        make_candidate(input, CompressionAlgorithm::Deflate, 9)?,
                        Some(9),
                    ));
                    candidates.push((
                        CompressionAlgorithm::Brotli,
                        make_candidate(input, CompressionAlgorithm::Brotli, 11)?,
                        Some(11),
                    ));
                }
            }
            candidates.sort_by_key(|(alg, body, _)| {
                if *alg == CompressionAlgorithm::None {
                    body.len()
                } else {
                    PAYLOAD_ENVELOPE_HEADER_BYTES + body.len()
                }
            });
            let (algorithm, body, level) = candidates.remove(0);
            if algorithm == CompressionAlgorithm::None {
                Ok(PreparedPayload {
                    payload: input.to_vec(),
                    extended: false,
                    compression: algorithm,
                    compression_level: None,
                })
            } else {
                Ok(PreparedPayload {
                    payload: pack_compressed_envelope(input, algorithm, &body)?,
                    extended: true,
                    compression: algorithm,
                    compression_level: level,
                })
            }
        }
    }
}

pub fn parse_payload_envelope(input: &[u8]) -> Result<PayloadEnvelope> {
    if input.len() < PAYLOAD_ENVELOPE_HEADER_BYTES || input[..4] != PAYLOAD_ENVELOPE_MAGIC {
        return Err(QuadQrError::Compression(
            "payload extension envelope is not recognized".into(),
        ));
    }
    let mut header = [0u8; PAYLOAD_ENVELOPE_HEADER_BYTES];
    header.copy_from_slice(&input[..PAYLOAD_ENVELOPE_HEADER_BYTES]);
    let version = input[4];
    if !(1..=PAYLOAD_ENVELOPE_VERSION).contains(&version) {
        return Err(QuadQrError::Compression(format!(
            "unsupported payload extension version {version}"
        )));
    }
    let flags = input[5];
    let compression = CompressionAlgorithm::from_id(input[6])?;
    let signature_algorithm = input[7];
    let original_length = u32::from_be_bytes([input[8], input[9], input[10], input[11]]) as usize;
    let key_id_len = input[12] as usize;
    let public_key_len = input[13] as usize;
    let signature_len = input[14] as usize;
    let metadata_len = key_id_len + public_key_len + signature_len;
    if input.len() < PAYLOAD_ENVELOPE_HEADER_BYTES + metadata_len {
        return Err(QuadQrError::Compression(
            "payload extension envelope is truncated".into(),
        ));
    }
    let mut cursor = PAYLOAD_ENVELOPE_HEADER_BYTES;
    let key_id = input[cursor..cursor + key_id_len].to_vec();
    cursor += key_id_len;
    let public_key = input[cursor..cursor + public_key_len].to_vec();
    cursor += public_key_len;
    let signature = input[cursor..cursor + signature_len].to_vec();
    cursor += signature_len;
    let stored_payload = input[cursor..].to_vec();
    let payload = match compression {
        CompressionAlgorithm::None => stored_payload.clone(),
        CompressionAlgorithm::Lz => decompress_lz(&stored_payload, Some(original_length))?,
        CompressionAlgorithm::Deflate => {
            #[cfg(feature = "compression")]
            {
                decompress_deflate(&stored_payload, Some(original_length))?
            }
            #[cfg(not(feature = "compression"))]
            {
                return Err(QuadQrError::Compression(
                    "compression feature disabled".into(),
                ));
            }
        }
        CompressionAlgorithm::Brotli => {
            #[cfg(feature = "compression")]
            {
                decompress_brotli(&stored_payload, Some(original_length))?
            }
            #[cfg(not(feature = "compression"))]
            {
                return Err(QuadQrError::Compression(
                    "compression feature disabled".into(),
                ));
            }
        }
    };
    if payload.len() != original_length {
        return Err(QuadQrError::Compression(
            "payload extension original length mismatch".into(),
        ));
    }
    Ok(PayloadEnvelope {
        header,
        version,
        flags,
        compression,
        signature_algorithm,
        original_length,
        key_id,
        public_key,
        signature,
        stored_payload,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lz_round_trip() {
        let source = b"quadqr quadqr quadqr quadqr 123456 quadqr";
        let compressed = compress_lz(source, 6).unwrap();
        assert_eq!(
            decompress_lz(&compressed, Some(source.len())).unwrap(),
            source
        );
    }

    #[test]
    fn extension_round_trip() {
        let source = b"repetitive repetitive repetitive repetitive repetitive";
        let prepared = prepare_payload(source, CompressionMode::Auto).unwrap();
        if prepared.extended {
            let env = parse_payload_envelope(&prepared.payload).unwrap();
            assert_eq!(env.payload, source);
        } else {
            assert_eq!(prepared.payload, source);
        }
    }
}
