//! Raster images (clause 9.10): read the size of a JPEG file, with no imaging library.
//!
//! A reader needs only the frame header (SOF) of the file: the width, the height, the number of
//! components and the sample precision. The pixels are never decoded. The SVG and the PDF carry the
//! JPEG bytes as they are (clause 15).

use sha2::{Digest, Sha256};

/// The frame header of a JPEG that clause 9.10.1 accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JpegInfo {
    pub width_px: u32,
    pub height_px: u32,
    /// 1 grey, 3 colour.
    pub components: u8,
}

/// Clause 9.10.1: the frame header of a JPEG with 8-bit samples and 1 or 3 components, coded as
/// baseline, extended or progressive DCT; `None` for anything else.
pub fn jpeg_info(data: &[u8]) -> Option<JpegInfo> {
    if data.len() < 4 || data[0..2] != [0xFF, 0xD8] {
        return None;
    }
    let mut i = 2;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            return None;
        }
        let marker = data[i + 1];
        if marker == 0xFF {
            i += 1; // a fill byte
            continue;
        }
        if marker == 0x01 || (0xD0..0xD8).contains(&marker) {
            i += 2; // TEM and RSTn have no length field
            continue;
        }
        if marker == 0xD9 || marker == 0xDA {
            return None; // end of image, or start of scan before any frame header
        }
        let length = usize::from(u16::from_be_bytes([data[i + 2], data[i + 3]]));
        if length < 2 || i + 2 + length > data.len() {
            return None;
        }
        if matches!(marker, 0xC0..=0xC2) {
            let seg = &data[i + 4..i + 2 + length];
            if seg.len() < 6 {
                return None;
            }
            let height = u32::from(u16::from_be_bytes([seg[1], seg[2]]));
            let width = u32::from(u16::from_be_bytes([seg[3], seg[4]]));
            let n = seg[5];
            if seg[0] != 8 || !matches!(n, 1 | 3) || width == 0 || height == 0 {
                return None;
            }
            return Some(JpegInfo {
                width_px: width,
                height_px: height,
                components: n,
            });
        }
        if (0xC3..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            return None; // another kind of frame: lossless or arithmetic coding
        }
        i += 2 + length;
    }
    None
}

/// The SHA-256 of some bytes, as 64 lower-case hex digits (clause 9.10.2).
pub fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sof(marker: u8, precision: u8, w: u16, h: u16, n: u8) -> Vec<u8> {
        let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, marker];
        let len = 8 + 3 * u16::from(n);
        v.extend(len.to_be_bytes());
        v.push(precision);
        v.extend(h.to_be_bytes());
        v.extend(w.to_be_bytes());
        v.push(n);
        v.extend(std::iter::repeat_n(0, 3 * usize::from(n)));
        v.extend([0xFF, 0xD9]);
        v
    }

    #[test]
    fn reads_baseline_extended_and_progressive_frames() {
        for marker in [0xC0, 0xC1, 0xC2] {
            assert_eq!(
                jpeg_info(&sof(marker, 8, 640, 480, 3)),
                Some(JpegInfo {
                    width_px: 640,
                    height_px: 480,
                    components: 3
                })
            );
        }
        assert_eq!(jpeg_info(&sof(0xC0, 8, 2, 1, 1)).unwrap().components, 1);
    }

    #[test]
    fn refuses_what_clause_9_10_1_refuses() {
        assert_eq!(jpeg_info(&sof(0xC3, 8, 4, 4, 3)), None); // lossless
        assert_eq!(jpeg_info(&sof(0xC9, 8, 4, 4, 3)), None); // arithmetic
        assert_eq!(jpeg_info(&sof(0xC0, 12, 4, 4, 3)), None); // 12-bit
        assert_eq!(jpeg_info(&sof(0xC0, 8, 4, 4, 4)), None); // CMYK
        assert_eq!(jpeg_info(&sof(0xC0, 8, 0, 4, 3)), None);
        assert_eq!(jpeg_info(b"\x89PNG\r\n\x1a\n"), None);
        assert_eq!(jpeg_info(&[]), None);
    }

    #[test]
    fn hashes_as_lower_case_hex() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
