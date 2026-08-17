//! Empire definitions, treasuries, and player-owned-system detection.
//!
//! The empire table sits in the first few hundred bytes of the file: one
//! record per empire, each holding a `c1 <id>` prefix, a 4-byte ASCII code,
//! the reversed marker `ECAR` ("RACE"), then two length-prefixed UTF-16BE
//! strings — the picked species (sub-race) name and the empire name. The
//! table ends at the `VSEICEPS` ("SPECIESV") marker. Empire ID 1 is
//! always the human player.
//!
//! Ownership lives (redundantly) in the `VSREYALP` ("PLAYERSV") section:
//! every AI empire's record ends with a shared block of sorted
//! `(system_index, 0x01)` pair lists describing what each empire controls.
//! We fingerprint every plausible list in every record, keep the ones that
//! repeat across records, and pick the list whose systems carry the most of
//! the player species' population. Heuristic, but it needs no knowledge of
//! the record layout between the lists.

use std::collections::BTreeSet;

use az::Az as _;

use crate::cursor::Cursor;
use crate::galaxy::Galaxy;
use crate::Species;

const RACE_MARKER: &[u8] = b"ECAR";
const SPECIES_MARKER: &[u8] = b"VSEICEPS";
const PLAYERS_MARKER: &[u8] = b"VSREYALP";

/// The ID of the human player's empire in every known save.
pub const PLAYER_EMPIRE_ID: u8 = 1;

/// One empire from the header table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Empire {
    pub id: u8,
    /// The picked species/sub-race name, e.g. "Tachidi".
    pub species: String,
    /// The empire's display name.
    pub name: String,
}

fn read_prefixed_utf16(cursor: &mut Cursor) -> Option<String> {
    let len = cursor.u32be().ok()?;
    if len == 0 || len > 50 {
        return None;
    }
    cursor.utf16be(len.az::<usize>()).ok()
}

/// Parse the empire definition table from the file header.
///
/// Returns an empty list when no table is found; a save without one is
/// still scannable, just without empire names or `--mine` support.
#[must_use]
pub fn empires(data: &[u8]) -> Vec<Empire> {
    let table = &data[..data.len().min(0x1000)];
    let end = memchr::memmem::find(table, SPECIES_MARKER).unwrap_or(0x300.min(data.len()));

    let mut out = Vec::new();
    let mut pos = 0;
    while pos < end {
        let Some(found) = memchr::memmem::find(&data[pos..end], RACE_MARKER) else {
            break;
        };
        let marker = pos + found;
        let mut cursor = Cursor::new(data, marker + RACE_MARKER.len());
        let record = read_prefixed_utf16(&mut cursor)
            .zip(read_prefixed_utf16(&mut cursor))
            .zip(marker.checked_sub(5).map(|id_pos| data[id_pos]));
        let Some(((species, name), id)) = record else {
            pos = marker + 1;
            continue;
        };
        out.push(Empire { id, species, name });
        pos = cursor.pos();
    }
    out
}

/// Offset of the treasury `i32` past the end of an empire's name in its
/// `PLAYERSV` record.
///
/// Located by diffing a consecutive autosave series (every empire's value
/// grows each turn by a plausible net income), consistent with Bhruic's
/// `FinanceWraparound` patch (treasury is a signed 32-bit integer capped at
/// `i32::MAX`), and confirmed in-game: patching it changed the displayed
/// AU balance.
const TREASURY_OFFSET: usize = 20;

fn treasury_field(data: &[u8], empire: &Empire) -> Option<usize> {
    let players = memchr::memmem::find(data, PLAYERS_MARKER)?;
    let encoded: Vec<u8> = empire
        .name
        .encode_utf16()
        .flat_map(u16::to_be_bytes)
        .collect();
    let name = players + memchr::memmem::find(&data[players..], &encoded)?;
    let field = name
        .checked_add(encoded.len())?
        .checked_add(TREASURY_OFFSET)?;
    (field.checked_add(4)? <= data.len()).then_some(field)
}

/// Read an empire's treasury in AU, or `None` when its `PLAYERSV` record
/// cannot be located.
///
/// Negative values are real: the game's unpatched finance-wraparound bug
/// stores them when a treasury overflows `i32::MAX`.
#[must_use]
pub fn treasury(data: &[u8], empire: &Empire) -> Option<i32> {
    let at = treasury_field(data, empire)?;
    Some(i32::from_be_bytes([
        data[at],
        data[at + 1],
        data[at + 2],
        data[at + 3],
    ]))
}

/// Write an empire's treasury in AU.
///
/// # Errors
///
/// [`Error::EmpireRecordNotFound`] when the empire's `PLAYERSV` record
/// cannot be located.
pub fn set_treasury(data: &mut [u8], empire: &Empire, au: i32) -> crate::Result<()> {
    let Some(at) = treasury_field(data, empire) else {
        return Err(crate::Error::EmpireRecordNotFound {
            name: empire.name.clone(),
        });
    };
    data[at..at + 4].copy_from_slice(&au.to_be_bytes());
    Ok(())
}

/// Try to read a sorted `(system_index, 0x01)` pair list at `pos`.
///
/// Returns the system indices and the position past the list, or `None` if
/// the bytes at `pos` don't form one.
fn sorted_pair_list(data: &[u8], pos: usize) -> Option<(Vec<u8>, usize)> {
    let count = *data.get(pos)?;
    if count > 250 {
        return None;
    }
    let mut systems = Vec::with_capacity(usize::from(count));
    let mut p = pos + 1;
    let mut prev: i16 = -1;
    for _ in 0..count {
        let sys_idx = *data.get(p)?;
        let flag = *data.get(p + 1)?;
        if flag != 1 || i16::from(sys_idx) <= prev {
            return None;
        }
        prev = i16::from(sys_idx);
        systems.push(sys_idx);
        p += 2;
    }
    Some((systems, p))
}

/// Detect the system indices the human player owns.
///
/// Returns an empty set when detection fails (missing markers, fewer than
/// three empire records located, or no repeated ownership list) — callers
/// should treat that as "unknown", not "owns nothing".
#[must_use]
pub fn player_systems(data: &[u8], galaxy: &Galaxy) -> BTreeSet<usize> {
    let Some(players) = memchr::memmem::find(data, PLAYERS_MARKER) else {
        return BTreeSet::new();
    };
    let table = empires(data);
    if table.is_empty() {
        return BTreeSet::new();
    }

    let mut positions: Vec<usize> = table
        .iter()
        .filter_map(|empire| {
            let encoded: Vec<u8> = empire
                .name
                .encode_utf16()
                .flat_map(u16::to_be_bytes)
                .collect();
            memchr::memmem::find(&data[players..], &encoded).map(|found| players + found)
        })
        .collect();
    positions.sort_unstable();
    if positions.len() < 3 {
        return BTreeSet::new();
    }

    let mut lists: Vec<(Vec<u8>, usize)> = Vec::new();
    for (index, &start) in positions.iter().enumerate() {
        let end = positions
            .get(index + 1)
            .copied()
            .unwrap_or_else(|| data.len().min(start + 0x0020_0000));
        let mut pos = start;
        while pos + 2 < end {
            match sorted_pair_list(data, pos) {
                Some((systems, next)) if systems.len() >= 10 => {
                    match lists.iter_mut().find(|(known, _)| *known == systems) {
                        Some((_, count)) => *count += 1,
                        None => lists.push((systems, 1)),
                    }
                    pos = next;
                }
                _ => pos += 1,
            }
        }
    }

    let Some(max_count) = lists.iter().map(|(_, count)| *count).max() else {
        return BTreeSet::new();
    };
    let min_freq = 3.max(max_count / 2);
    let candidates: Vec<&Vec<u8>> = lists
        .iter()
        .filter(|(systems, count)| *count >= min_freq && systems.len() >= 10)
        .map(|(systems, _)| systems)
        .collect();
    if candidates.is_empty() {
        return BTreeSet::new();
    }

    let player_species = table
        .iter()
        .find(|empire| empire.id == PLAYER_EMPIRE_ID)
        .and_then(|empire| Species::from_name_or_subrace(&empire.species));

    let score = |systems: &[u8]| -> f64 {
        let Some(species) = player_species else {
            return 0.0;
        };
        galaxy
            .regions
            .iter()
            .filter(|region| {
                region.species == species
                    && systems
                        .iter()
                        .any(|&idx| usize::from(idx) == region.sys_idx)
            })
            .map(|region| region.pop)
            .sum()
    };

    let mut best: Option<(f64, &Vec<u8>)> = None;
    for systems in candidates {
        let scored = score(systems);
        let beats = best.is_none_or(|(best_score, _)| scored > best_score);
        if beats {
            best = Some((scored, systems));
        }
    }
    best.map(|(_, systems)| systems.iter().map(|&idx| usize::from(idx)).collect())
        .unwrap_or_default()
}
