//! The file header (`VS3RDAEH`, i.e. "HEADR3SV" reversed).
//!
//! The only mapped field so far is the turn counter, located by diffing a
//! consecutive autosave series (turns 180-184) and confirmed against a
//! manually named turn-115 save.

use crate::error::{Error, Result};

/// Reversed-ASCII magic at offset 0 of every save.
pub const MAGIC: &[u8; 8] = b"VS3RDAEH";

const TURN_OFFSET: usize = 0xD;

fn turn_bytes(data: &[u8]) -> Result<usize> {
    if data.get(..MAGIC.len()) != Some(MAGIC.as_slice()) {
        return Err(Error::NoHeaderMagic);
    }
    let end = TURN_OFFSET + 4;
    if data.len() < end {
        return Err(Error::Truncated {
            offset: TURN_OFFSET,
            needed: 4,
            len: data.len(),
        });
    }
    Ok(TURN_OFFSET)
}

/// Read the current turn number.
///
/// # Errors
///
/// [`Error::NoHeaderMagic`] or [`Error::Truncated`] on non-save input.
pub fn turn(data: &[u8]) -> Result<u32> {
    let at = turn_bytes(data)?;
    let bytes = [data[at], data[at + 1], data[at + 2], data[at + 3]];
    Ok(u32::from_be_bytes(bytes))
}

/// Write the turn number.
///
/// # Errors
///
/// [`Error::NoHeaderMagic`] or [`Error::Truncated`] on non-save input.
pub fn set_turn(data: &mut [u8], turn: u32) -> Result<()> {
    let at = turn_bytes(data)?;
    data[at..at + 4].copy_from_slice(&turn.to_be_bytes());
    Ok(())
}
