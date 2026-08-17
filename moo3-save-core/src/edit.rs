//! Field-level edits to region records.
//!
//! All edits are fixed-width in-place writes at offsets carried by a parsed
//! [`Region`], so no offset-table fix-up is needed (the save contains
//! internal file pointers that break when records are resized — Bhruic's
//! v0.51 changelog learned that the hard way).
//!
//! Every write first re-checks that the bytes at the region's offset still
//! hold the parsed species and owner; a stale plan or a different file
//! yields [`Error::StaleRegion`] instead of silent corruption.

use crate::cursor::encode_fixed;
use crate::error::{Error, Result};
use crate::galaxy::{
    Region, ECO_BASE_OFFSET, ECO_MODIFIED_OFFSET, OWNER_OFFSET, POP_OFFSET, RACE1_OFFSET,
    TERRAIN_OFFSET,
};

fn field<'data, const N: usize>(
    data: &'data mut [u8],
    region: &Region,
    offset: usize,
) -> Result<&'data mut [u8; N]> {
    let stale = Error::StaleRegion {
        offset: region.offset,
    };
    let matches = data
        .get(region.offset + RACE1_OFFSET)
        .is_some_and(|&race1| race1 == region.species.race1())
        && data
            .get(region.offset + OWNER_OFFSET)
            .is_some_and(|&owner| owner == region.owner);
    if !matches {
        return Err(stale);
    }
    let start = region.offset + offset;
    let Some(bytes) = start
        .checked_add(N)
        .and_then(|end| data.get_mut(start..end))
    else {
        return Err(stale);
    };
    Ok(bytes.try_into().expect("slice is N bytes"))
}

/// Set a region's population (in the game's fractional population units).
///
/// # Errors
///
/// [`Error::ValueOutOfRange`] for non-finite or absurd values;
/// [`Error::StaleRegion`] when the buffer no longer matches the region.
pub fn set_population(data: &mut [u8], region: &Region, pop: f64) -> Result<()> {
    let Some(encoded) = encode_fixed(pop) else {
        return Err(Error::ValueOutOfRange {
            what: "population",
            value: pop,
        });
    };
    *field::<8>(data, region, POP_OFFSET)? = encoded;
    Ok(())
}

/// Set a region's owning empire id.
///
/// Bhruic's readme warns that a colonization only takes if every region of
/// the planet is flipped and the system lies inside the new owner's space;
/// callers wanting "give me this planet" should flip all its regions.
///
/// # Errors
///
/// [`Error::StaleRegion`] when the buffer no longer matches the region.
pub fn set_owner(data: &mut [u8], region: &Region, owner: u8) -> Result<()> {
    *field::<1>(data, region, OWNER_OFFSET)? = [owner];
    Ok(())
}

/// Set a region's terrain type index.
///
/// # Errors
///
/// [`Error::StaleRegion`] when the buffer no longer matches the region.
pub fn set_terrain(data: &mut [u8], region: &Region, terrain: u8) -> Result<()> {
    *field::<1>(data, region, TERRAIN_OFFSET)? = [terrain];
    Ok(())
}

/// Set a region's base ecosystem rating, preserving the terraforming delta.
///
/// Follows Bhruic's rule: the modified rating becomes
/// `min(base + old_delta, 2)` where `old_delta = modified - old_base`.
///
/// # Errors
///
/// [`Error::StaleRegion`] when the buffer no longer matches the region.
pub fn set_ecosystem(data: &mut [u8], region: &Region, base: i32) -> Result<()> {
    let delta = region.eco_modified - region.eco_base;
    let modified = base.saturating_add(delta).min(2);
    *field::<4>(data, region, ECO_BASE_OFFSET)? = base.to_be_bytes();
    *field::<4>(data, region, ECO_MODIFIED_OFFSET)? = modified.to_be_bytes();
    Ok(())
}
