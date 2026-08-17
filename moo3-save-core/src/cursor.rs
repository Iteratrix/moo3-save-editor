//! Bounds-checked sequential reader over raw save bytes.
//!
//! Every read goes through [`Cursor::take`], so a mis-parse or hostile file
//! surfaces as [`Error::Truncated`] instead of a panic. The parser modules
//! rely on this: they skip fixed-size runs without validating content, and
//! the cursor is what keeps that safe.

use az::{Az as _, SaturatingAs as _};

use crate::error::{Error, Result};

pub(crate) struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let truncated = Error::Truncated {
            offset: self.pos,
            needed: len,
            len: self.data.len(),
        };
        let Some(end) = self.pos.checked_add(len) else {
            return Err(truncated);
        };
        let Some(bytes) = self.data.get(self.pos..end) else {
            return Err(truncated);
        };
        self.pos = end;
        Ok(bytes)
    }

    pub fn skip(&mut self, len: usize) -> Result<()> {
        self.take(len).map(|_| ())
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    /// Read the next byte without consuming it.
    pub fn peek_u8(&self) -> Result<u8> {
        self.data.get(self.pos).copied().ok_or(Error::Truncated {
            offset: self.pos,
            needed: 1,
            len: self.data.len(),
        })
    }

    pub fn u32be(&mut self) -> Result<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn i32be(&mut self) -> Result<i32> {
        let bytes = self.take(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// MOO3's custom fixed-point number: a 6-byte big-endian signed integer
    /// part followed by a 2-byte big-endian 1/65536 fraction.
    pub fn fixed(&mut self) -> Result<f64> {
        let bytes = self.take(8)?;
        let extend = if bytes[0] & 0x80 == 0 { 0x00 } else { 0xFF };
        let int_part = i64::from_be_bytes([
            extend, extend, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
        ]);
        let frac = u16::from_be_bytes([bytes[6], bytes[7]]);
        Ok(int_part.az::<f64>() + f64::from(frac) / 65536.0)
    }

    /// A UTF-16BE string of `chars` code units (no length prefix, no
    /// terminator). Unpaired surrogates decode to U+FFFD.
    pub fn utf16be(&mut self, chars: usize) -> Result<String> {
        let Some(byte_len) = chars.checked_mul(2) else {
            return Err(Error::Truncated {
                offset: self.pos,
                needed: usize::MAX,
                len: self.data.len(),
            });
        };
        let bytes = self.take(byte_len)?;
        let units = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]));
        Ok(char::decode_utf16(units)
            .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect())
    }
}

/// Encode a value into MOO3's 8-byte fixed-point wire format.
///
/// Returns `None` when the integer part does not fit the 6-byte signed
/// range (`|v| >= 2^47`).
pub(crate) fn encode_fixed(value: f64) -> Option<[u8; 8]> {
    const LIMIT: f64 = 140_737_488_355_328.0; // 2^47
    if !value.is_finite() || value <= -LIMIT || value >= LIMIT {
        return None;
    }
    let int_part = value.floor().az::<i64>();
    let frac = ((value - value.floor()) * 65536.0).saturating_as::<u16>();
    let int_bytes = int_part.to_be_bytes();
    let frac_bytes = frac.to_be_bytes();
    Some([
        int_bytes[2],
        int_bytes[3],
        int_bytes[4],
        int_bytes[5],
        int_bytes[6],
        int_bytes[7],
        frac_bytes[0],
        frac_bytes[1],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        for value in [0.0, 1.5, -1.5, 123.4375, 140_000.0, 7077.0] {
            let bytes = encode_fixed(value).expect("in range");
            let mut cursor = Cursor::new(&bytes, 0);
            let decoded = cursor.fixed().expect("8 bytes");
            assert!(
                (decoded - value).abs() < 1.0 / 65536.0,
                "{value} -> {decoded}"
            );
        }
        assert!(encode_fixed(1e30).is_none());
        assert!(encode_fixed(f64::NAN).is_none());
    }

    #[test]
    fn fixed_point_decodes_sign_and_fraction() {
        let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0x80, 0x00];
        let mut cursor = Cursor::new(&data, 0);
        let value = cursor.fixed().expect("in bounds");
        assert!((value - (-1.5)).abs() < 1e-9);
    }

    #[test]
    fn reads_past_end_are_errors() {
        let mut cursor = Cursor::new(&[1, 2], 0);
        assert!(cursor.u32be().is_err());
    }
}
