//! Species replacement: plan which regions to touch, then patch them.
//!
//! The edit itself is two bytes per region — `race1` becomes the
//! replacement species and `race2` (sub-race/magnate) resets to 0 so the
//! game falls back to the species' default sub-race. Population, buildings,
//! and everything else stay untouched; the game simply treats the people as
//! the new species from the next turn on.
//!
//! Planning is separate from patching so callers (CLI `--dry-run`, the web
//! preview) can show exactly what would change before committing.

use std::collections::{BTreeMap, BTreeSet};

use crate::galaxy::{Galaxy, Region, RACE1_OFFSET, RACE2_OFFSET};
use crate::Species;

/// Which planets a replacement applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Only planets where the target cohabits with someone else — the
    /// classic anti-Ithkul case. With `protect` set, only planets where
    /// that particular species is present.
    Shared { protect: Option<Species> },
    /// Only planets whose display name contains one of these strings
    /// (case-insensitive; `"Alrisha"` matches every planet in the system).
    Planets(Vec<String>),
    /// Every region of the target species, galaxy-wide.
    Everywhere,
}

/// Select the regions a replacement would touch.
///
/// `owned` restricts to systems in the set (the `--mine` flag); `None`
/// means no ownership filter. Regions come back in file order.
#[must_use]
pub fn plan<'galaxy>(
    galaxy: &'galaxy Galaxy,
    target: Species,
    scope: &Scope,
    owned: Option<&BTreeSet<usize>>,
) -> Vec<&'galaxy Region> {
    let in_owned = |region: &Region| owned.is_none_or(|systems| systems.contains(&region.sys_idx));

    match scope {
        Scope::Everywhere => galaxy
            .regions
            .iter()
            .filter(|region| region.species == target && in_owned(region))
            .collect(),
        Scope::Planets(names) => {
            let names: Vec<String> = names.iter().map(|name| name.to_lowercase()).collect();
            galaxy
                .regions
                .iter()
                .filter(|region| {
                    region.species == target
                        && in_owned(region)
                        && names
                            .iter()
                            .any(|name| galaxy.planet_name(region).to_lowercase().contains(name))
                })
                .collect()
        }
        Scope::Shared { protect } => {
            let mut planets: BTreeMap<(usize, usize), Vec<&Region>> = BTreeMap::new();
            for region in &galaxy.regions {
                planets
                    .entry((region.sys_idx, region.planet_idx))
                    .or_default()
                    .push(region);
            }
            let mut out = Vec::new();
            for regions in planets.values() {
                let has_target = regions.iter().any(|region| region.species == target);
                let has_protected = match protect {
                    Some(protected) => regions.iter().any(|region| region.species == *protected),
                    None => regions.iter().any(|region| region.species != target),
                };
                if has_target && has_protected {
                    out.extend(
                        regions
                            .iter()
                            .copied()
                            .filter(|region| region.species == target && in_owned(region)),
                    );
                }
            }
            out.sort_by_key(|region| region.offset);
            out
        }
    }
}

/// What [`apply`] did to the buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApplyOutcome {
    /// Regions actually patched.
    pub patched: usize,
    /// Regions skipped because the byte at `race1` no longer held the
    /// target species (stale plan, or a plan from a different file).
    pub skipped: usize,
    /// Total population of the patched regions.
    pub pop: f64,
}

/// Patch the planned regions in place.
///
/// Each region is re-checked against `target` before writing, so applying a
/// stale plan degrades to skips instead of corrupting unrelated bytes.
pub fn apply(
    data: &mut [u8],
    planned: &[&Region],
    target: Species,
    replacement: Species,
) -> ApplyOutcome {
    let mut outcome = ApplyOutcome {
        patched: 0,
        skipped: 0,
        pop: 0.0,
    };
    for region in planned {
        let race1 = region.offset + RACE1_OFFSET;
        let race2 = region.offset + RACE2_OFFSET;
        let current = data.get(race1).copied();
        if current != Some(target.race1()) || race2 >= data.len() {
            outcome.skipped += 1;
            continue;
        }
        data[race1] = replacement.race1();
        data[race2] = 0;
        outcome.patched += 1;
        outcome.pop += region.pop;
    }
    outcome
}
