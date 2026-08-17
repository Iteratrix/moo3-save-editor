//! The galaxy section: systems, planets, and population regions.
//!
//! The section starts at the reversed-ASCII marker `VSYXALAG` ("GALAXYSV").
//! After a fixed 65-byte gap comes a one-byte system count, then each system
//! in sequence: a header (UTF-16BE name, orbit slots, two length-prefixed
//! blobs), a planet count, and per planet a list of population regions
//! followed by a large post-region block (buildings, DEAs, queues) that we
//! skip structurally.
//!
//! Region records are where populations live: `race1` (species) at region
//! offset +10 and `race2` (sub-race/magnate) at +11, right after a 2-byte
//! header and the 8-byte fixed-point population. Those two byte offsets are
//! the entire editable surface of this crate — everything else is parsed
//! only to find them.
//!
//! Orbit slot types in the system header: `H` (0x48, habitable) carries 31
//! extra bytes, `L`/`O` (lifeless/other) carry 30, `0xFF` is an empty orbit
//! with none.

use az::Az as _;

use crate::cursor::Cursor;
use crate::error::{Error, Result};
use crate::special::skip_special_record;
use crate::Species;

/// Reversed-ASCII marker that opens the galaxy section.
pub const GALAXY_MARKER: &[u8; 8] = b"VSYXALAG";

/// Offset of the `race1` (species) byte within a region record.
pub const RACE1_OFFSET: usize = 10;
/// Offset of the `race2` (sub-race/magnate) byte within a region record.
pub const RACE2_OFFSET: usize = 11;

/// What kind of body a system's flag byte declares.
///
/// Neutron stars and black holes have no planet list; everything else does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemKind {
    NeutronStar,
    BlackHole,
    Star(u8),
}

impl From<u8> for SystemKind {
    fn from(flag: u8) -> Self {
        match flag {
            0x4E => Self::NeutronStar,
            0x42 => Self::BlackHole,
            other => Self::Star(other),
        }
    }
}

impl SystemKind {
    fn has_planets(self) -> bool {
        match self {
            Self::NeutronStar | Self::BlackHole => false,
            Self::Star(_) => true,
        }
    }
}

/// One star system, as named in the galaxy map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct System {
    pub name: String,
    pub kind: SystemKind,
}

/// One populated region on a planet.
///
/// `offset` is the absolute file offset of the region record; the species
/// byte lives at `offset + RACE1_OFFSET`. Regions with zero population are
/// parsed but not recorded.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub sys_idx: usize,
    pub planet_idx: usize,
    pub region_idx: usize,
    pub species: Species,
    pub race2: u8,
    pub pop: f64,
    pub offset: usize,
}

/// A parsed galaxy: system names plus every populated region.
#[derive(Debug, Clone, PartialEq)]
pub struct Galaxy {
    pub systems: Vec<System>,
    pub regions: Vec<Region>,
}

impl Galaxy {
    /// Parse the galaxy section out of a full save file.
    ///
    /// # Errors
    ///
    /// [`Error::NoGalaxyMarker`] if the file is not a MOO3 save;
    /// [`Error::Truncated`] or [`Error::BadSystemName`] if the parser loses
    /// sync (unknown game version or corrupt file).
    pub fn parse(data: &[u8]) -> Result<Self> {
        let Some(marker) = memchr::memmem::find(data, GALAXY_MARKER) else {
            return Err(Error::NoGalaxyMarker);
        };

        let mut cursor = Cursor::new(data, marker + 8 + 4 + 1 + 60);
        let system_count = cursor.u8()?;

        let mut systems = Vec::with_capacity(usize::from(system_count));
        let mut regions = Vec::new();

        for sys_idx in 0..usize::from(system_count) {
            let system = read_system_header(&mut cursor)?;
            let has_planets = system.kind.has_planets();
            systems.push(system);
            if !has_planets {
                continue;
            }

            let planet_count = cursor.u8()?;
            for planet_idx in 0..usize::from(planet_count) {
                let region_count = cursor.u8()?;
                for region_idx in 0..usize::from(region_count) {
                    let offset = cursor.pos();
                    let RawRegion {
                        species,
                        race2,
                        pop,
                    } = read_region(&mut cursor)?;
                    if pop > 0.0 {
                        regions.push(Region {
                            sys_idx,
                            planet_idx,
                            region_idx,
                            species,
                            race2,
                            pop,
                            offset,
                        });
                    }
                }
                skip_post_region(&mut cursor)?;
            }
        }

        Ok(Self { systems, regions })
    }

    /// The display name of a region's planet, e.g. `"Alrisha VII"`.
    #[must_use]
    pub fn planet_name(&self, region: &Region) -> String {
        let system = self
            .systems
            .get(region.sys_idx)
            .map_or("?", |system| system.name.as_str());
        format!("{system} {}", roman(region.planet_idx + 1))
    }
}

/// Lowercase Roman numeral for planet numbering (1-based).
#[must_use]
pub fn roman(mut n: usize) -> String {
    let mut out = String::new();
    for (value, digits) in [(10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")] {
        while n >= value {
            out.push_str(digits);
            n -= value;
        }
    }
    out
}

struct RawRegion {
    species: Species,
    race2: u8,
    pop: f64,
}

fn read_region(cursor: &mut Cursor) -> Result<RawRegion> {
    cursor.skip(1 + 1)?;
    let pop = cursor.fixed()?;
    let species = Species::from(cursor.u8()?);
    let race2 = cursor.u8()?;
    cursor.skip(1 + 8 + 1 + 4 + 4 + 8 + 8 + 8)?;

    let specials = cursor.u8()?;
    for _ in 0..specials {
        skip_special_record(cursor)?;
    }

    let entries = cursor.u8()?;
    for _ in 0..entries {
        cursor.skip(1 + 8)?;
        let inner = cursor.u8()?;
        cursor.skip(usize::from(inner) * 17)?;
        cursor.skip(7)?;
    }

    for index in 0..3_u8 {
        if cursor.peek_u8()? != index {
            continue;
        }
        cursor.skip(1)?;
        let switch_val = cursor.u32be()?;
        cursor.skip(2 + 1 + 1)?;
        let entries = cursor.u8()?;
        for _ in 0..entries {
            cursor.skip(1 + 8)?;
            let inner = cursor.u8()?;
            cursor.skip(usize::from(inner) * 17)?;
            cursor.skip(7)?;
        }
        let orbits = cursor.u8()?;
        for _ in 0..orbits {
            let tag = cursor.u8()?;
            if tag == 0x4F {
                cursor.skip(1 + 4 + 1 + 1)?;
            }
        }
        cursor.skip(8 + 1)?;
        match i64::from(switch_val) - 1 {
            0 | 1 | 3 | 4 | 5 => cursor.skip(8)?,
            2 => cursor.skip(0x30)?,
            6 => cursor.skip(0x10)?,
            7 => cursor.skip(0x28)?,
            _ => {}
        }
    }

    cursor.skip(1)?;
    Ok(RawRegion {
        species,
        race2,
        pop,
    })
}

fn skip_typed_field(cursor: &mut Cursor) -> Result<()> {
    let field_type = cursor.u8()?;
    match field_type {
        0 => cursor.skip(1 + 4)?,
        3 | 4 => {
            cursor.skip(1 + 4)?;
            let entries = cursor.u8()?;
            for _ in 0..entries {
                let sub_type = cursor.u8()?;
                if sub_type == 7 {
                    cursor.skip(1 + 4)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn skip_length_prefixed(cursor: &mut Cursor) -> Result<()> {
    let len = cursor.u32be()?;
    cursor.skip(len.az::<usize>())
}

fn skip_post_region(cursor: &mut Cursor) -> Result<()> {
    cursor.skip(1)?;
    skip_length_prefixed(cursor)?;
    skip_length_prefixed(cursor)?;
    cursor.skip(1 + 1 + 1 + 8 + 1 + 1 + 8 + 8 + 1 + 1 + 1 + 2 + 1)?;
    let name_len = cursor.u32be()?;
    cursor.skip(name_len.az::<usize>() * 2)?;
    let specials = cursor.u8()?;
    for _ in 0..specials {
        skip_special_record(cursor)?;
    }
    cursor.skip(8 + 8 + 2)?;
    let fields = cursor.u8()?;
    for _ in 0..fields {
        skip_typed_field(cursor)?;
    }

    let big_flag = cursor.u8()?;
    if big_flag > 0 {
        cursor.skip(1)?;
        for tail in [14, 10, 10, 5] {
            let entries = cursor.u8()?;
            cursor.skip(usize::from(entries) * 42)?;
            cursor.skip(tail)?;
        }
        let entries = cursor.u8()?;
        cursor.skip(usize::from(entries) * 9)?;
        cursor.skip(2 + 1 + 1 + 8 + 4 + 4)?;
        cursor.skip(7 * 2 + 7 * 2 + 5)?;
        let groups = cursor.u8()?;
        for _ in 0..groups {
            cursor.skip(1 + 8)?;
            let inner = cursor.u8()?;
            cursor.skip(usize::from(inner) * 17)?;
            cursor.skip(1 + 1 + 1 + 4)?;
        }
        cursor.skip(9 + 1 + 0x38)?;
        cursor.skip(8 * 4)?;
        cursor.skip(0x80)?;
        cursor.skip(8 * 4)?;
        cursor.skip(0x3C)?;
        for _ in 0..5 {
            let entries = cursor.u32be()?;
            cursor.skip(entries.az::<usize>() * 12)?;
        }
    }

    cursor.skip(0x17)?;
    let queue = cursor.u8()?;
    cursor.skip(usize::from(queue) * 8)?;
    cursor.skip(5)?;
    loop {
        let sentinel = cursor.u8()?;
        if sentinel == 0xFF {
            break;
        }
        cursor.skip(8)?;
    }
    skip_length_prefixed(cursor)?;
    cursor.skip(1)?;
    let fragments = cursor.u8()?;
    for _ in 0..fragments {
        cursor.skip(1)?;
        skip_length_prefixed(cursor)?;
        skip_length_prefixed(cursor)?;
        cursor.skip(1 + 1 + 1 + 8 + 1 + 1 + 8 + 8 + 1 + 1 + 1 + 2 + 1)?;
        let entries = cursor.u8()?;
        cursor.skip(usize::from(entries) * 6)?;
    }
    Ok(())
}

fn read_system_header(cursor: &mut Cursor) -> Result<System> {
    let start = cursor.pos();
    let kind = SystemKind::from(cursor.u8()?);
    cursor.skip(2)?;
    let name_len = cursor.u32be()?;
    if name_len == 0 || name_len > 200 {
        return Err(Error::BadSystemName {
            offset: start,
            len: name_len,
        });
    }
    let name = cursor.utf16be(name_len.az::<usize>())?;
    cursor.skip(2 + 8 + 8 + 8 + 8)?;

    let slots = cursor.u8()?;
    for _ in 0..slots {
        let slot_type = cursor.u8()?;
        match slot_type {
            0x48 => cursor.skip(31)?,
            0x4C | 0x4F => cursor.skip(30)?,
            _ => {}
        }
    }

    cursor.skip(2)?;
    let count = cursor.u8()?;
    cursor.skip(usize::from(count))?;
    cursor.skip(2)?;
    let count = cursor.u8()?;
    cursor.skip(usize::from(count) * 8)?;
    cursor.skip(3)?;
    skip_length_prefixed(cursor)?;
    skip_length_prefixed(cursor)?;
    cursor.skip(0x20)?;

    Ok(System { name, kind })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roman_numerals() {
        assert_eq!(roman(1), "I");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(9), "IX");
        assert_eq!(roman(14), "XIV");
    }

    #[test]
    fn non_save_bytes_are_rejected() {
        assert!(matches!(
            Galaxy::parse(b"definitely not a save"),
            Err(Error::NoGalaxyMarker)
        ));
    }
}
