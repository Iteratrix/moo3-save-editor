//! Planetary special records (`Sp*` tags), skipped structurally.
//!
//! Specials appear inside region records and again in the post-region planet
//! data. Their 8-byte ASCII tags are stored reversed (`SpFLUGen` on disk is
//! `neGULFpS`). Each tag has a known amount of extra data before and after a
//! common sub-record; the amounts were mapped empirically from Bhruic's
//! editor and real saves. We never edit specials — parsing them exists only
//! to keep the cursor in sync on the way to the next region.

use az::Az as _;

use crate::cursor::Cursor;
use crate::error::Result;

fn skip_sub_record(cursor: &mut Cursor) -> Result<()> {
    cursor.skip(8 + 8 + 1 + 1 + 4 + 4 + 4)?;
    let count = cursor.u8()?;
    cursor.skip(usize::from(count) * 17)?;
    let count = cursor.u8()?;
    cursor.skip(usize::from(count) * 17)?;
    Ok(())
}

/// Extra bytes (before, after) the common sub-record, per known tag.
fn extra_bytes(tag: [u8; 8]) -> Option<(usize, usize)> {
    match tag.as_slice() {
        b"SpGenerc" => Some((1, 0)),
        b"SpTerfrm" | b" SpRuins" => Some((2, 8)),
        b"SpDeplet" | b"SpPrtDep" => Some((2, 4)),
        b"SpAbnCol" => Some((2, 0)),
        b"SpSplCol" | b"SpFLUGen" => Some((2, 12)),
        b" SpEvent" => Some((2, 16)),
        b"SpAntarX" => Some((2, 1)),
        _ => None,
    }
}

/// Skip one special record starting at the cursor.
pub(crate) fn skip_special_record(cursor: &mut Cursor) -> Result<()> {
    let stored = cursor.take(8)?;
    let mut tag = [0_u8; 8];
    for (out, byte) in tag.iter_mut().zip(stored.iter().rev()) {
        *out = *byte;
    }

    if tag.as_slice() == b"SpGuardn" {
        cursor.skip(2)?;
        skip_sub_record(cursor)?;
        cursor.skip(1)?;
        let len = cursor.u32be()?;
        cursor.skip(len.az::<usize>())?;
    } else if let Some((before, after)) = extra_bytes(tag) {
        cursor.skip(before)?;
        skip_sub_record(cursor)?;
        cursor.skip(after)?;
    } else {
        cursor.skip(2)?;
        skip_sub_record(cursor)?;
    }
    Ok(())
}
