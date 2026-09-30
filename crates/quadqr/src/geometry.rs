use crate::error::{invalid, Result};

pub const MIN_VERSION: u8 = 1;
pub const MAX_VERSION: u8 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentProfile {
    Standard5,
    Legacy3,
}

#[derive(Debug, Clone)]
pub struct AlignmentPattern {
    pub row: usize,
    pub col: usize,
    pub size: usize,
    pub primary: bool,
    pub bootstrap: bool,
    pub separator: bool,
}

const ALIGNMENT_AXES: [&[u8]; 41] = [
    &[],
    &[],
    &[6, 18],
    &[6, 22],
    &[6, 26],
    &[6, 30],
    &[6, 34],
    &[6, 22, 38],
    &[6, 24, 42],
    &[6, 26, 46],
    &[6, 28, 50],
    &[6, 30, 54],
    &[6, 32, 58],
    &[6, 34, 62],
    &[6, 26, 46, 66],
    &[6, 26, 48, 70],
    &[6, 26, 50, 74],
    &[6, 30, 54, 78],
    &[6, 30, 56, 82],
    &[6, 30, 58, 86],
    &[6, 34, 62, 90],
    &[6, 28, 50, 72, 94],
    &[6, 26, 50, 74, 98],
    &[6, 30, 54, 78, 102],
    &[6, 28, 54, 80, 106],
    &[6, 32, 58, 84, 110],
    &[6, 30, 58, 86, 114],
    &[6, 34, 62, 90, 118],
    &[6, 26, 50, 74, 98, 122],
    &[6, 30, 54, 78, 102, 126],
    &[6, 26, 52, 78, 104, 130],
    &[6, 30, 56, 82, 108, 134],
    &[6, 34, 60, 86, 112, 138],
    &[6, 30, 58, 86, 114, 142],
    &[6, 34, 62, 90, 118, 146],
    &[6, 30, 54, 78, 102, 126, 150],
    &[6, 24, 50, 76, 102, 128, 154],
    &[6, 28, 54, 80, 106, 132, 158],
    &[6, 32, 58, 84, 110, 136, 162],
    &[6, 26, 54, 82, 110, 138, 166],
    &[6, 30, 58, 86, 114, 142, 170],
];

pub fn validate_version(version: u8) -> Result<()> {
    if !(MIN_VERSION..=MAX_VERSION).contains(&version) {
        return Err(invalid(format!(
            "version must be {MIN_VERSION}..{MAX_VERSION}"
        )));
    }
    Ok(())
}

pub fn size_for_version(version: u8) -> Result<usize> {
    validate_version(version)?;
    Ok(21 + 4 * (version as usize - 1))
}

pub fn version_from_size(size: usize) -> Option<u8> {
    if size < 21 || !(size - 21).is_multiple_of(4) {
        return None;
    }
    let version = ((size - 21) / 4 + 1) as u8;
    (MIN_VERSION..=MAX_VERSION)
        .contains(&version)
        .then_some(version)
}

pub fn alignment_patterns(version: u8, profile: AlignmentProfile) -> Result<Vec<AlignmentPattern>> {
    validate_version(version)?;
    let size = size_for_version(version)?;
    if version == 1 {
        let center = size - 4;
        return Ok(vec![AlignmentPattern {
            row: center,
            col: center,
            size: 5,
            primary: true,
            bootstrap: true,
            separator: true,
        }]);
    }

    let axes = ALIGNMENT_AXES[version as usize];
    let last = *axes
        .last()
        .ok_or_else(|| invalid("missing alignment axes"))? as usize;
    let mut out = Vec::new();
    for &r in axes {
        for &c in axes {
            let row = r as usize;
            let col = c as usize;
            if (row == 6 && col == 6) || (row == 6 && col == last) || (row == last && col == 6) {
                continue;
            }
            let primary = row == last && col == last;
            out.push(AlignmentPattern {
                row,
                col,
                size: if profile == AlignmentProfile::Legacy3 && !primary {
                    3
                } else {
                    5
                },
                primary,
                bootstrap: false,
                separator: false,
            });
        }
    }
    Ok(out)
}

pub fn alignment_radius(pattern: &AlignmentPattern) -> isize {
    if pattern.size == 3 {
        1
    } else {
        2
    }
}

pub fn alignment_is_black(
    pattern: &AlignmentPattern,
    row_offset: isize,
    col_offset: isize,
) -> Option<bool> {
    let radius = alignment_radius(pattern);
    if row_offset.abs() > radius || col_offset.abs() > radius {
        return None;
    }
    if pattern.size == 3 {
        return Some(row_offset != 0 || col_offset != 0);
    }
    let outer = row_offset.abs() == 2 || col_offset.abs() == 2;
    let center = row_offset == 0 && col_offset == 0;
    Some(outer || center)
}
