//! Self-checks used by the corpus runner and the integration tests.
//!
//! [`verify`] exercises the full public surface against one save: parse the
//! galaxy, parse the empire table, detect player ownership, then apply a
//! representative species replacement to a copy and re-parse — asserting
//! that exactly the planned regions changed and nothing else in the parsed
//! structure moved.

use crate::galaxy::Galaxy;
use crate::replace::{self, ApplyOutcome, Scope};
use crate::{edit, empire, header, Error, Species};

/// Result of verifying one file.
#[derive(Debug)]
pub enum Outcome {
    /// All checks passed; the string summarizes what was exercised.
    Pass(String),
    /// The file has no galaxy marker (not a MOO3 save).
    NotSave,
    /// A check failed.
    Fail(String),
}

fn check_edit(bytes: &[u8], galaxy: &Galaxy) -> Result<String, String> {
    let Some(first) = galaxy.regions.first() else {
        return Ok("edit skipped (no populated regions)".to_owned());
    };
    let target = first.species;
    let replacement = if target == Species::Silicoid {
        Species::Human
    } else {
        Species::Silicoid
    };

    let planned = replace::plan(galaxy, target, &Scope::Everywhere, None);
    if planned.is_empty() {
        return Err("plan for the first region's species selected nothing".to_owned());
    }

    let mut edited = bytes.to_vec();
    let ApplyOutcome {
        patched, skipped, ..
    } = replace::apply(&mut edited, &planned, target, replacement);
    if patched != planned.len() || skipped != 0 {
        return Err(format!(
            "apply patched {patched}/{} with {skipped} skips",
            planned.len()
        ));
    }

    let reparsed =
        Galaxy::parse(&edited).map_err(|error| format!("reparse after edit: {error}"))?;
    if reparsed.systems != galaxy.systems {
        return Err("system list changed across edit".to_owned());
    }
    if reparsed.regions.len() != galaxy.regions.len() {
        return Err(format!(
            "region count changed across edit: {} -> {}",
            galaxy.regions.len(),
            reparsed.regions.len()
        ));
    }
    let planned_offsets: Vec<usize> = planned.iter().map(|region| region.offset).collect();
    for (before, after) in galaxy.regions.iter().zip(&reparsed.regions) {
        let expected = if planned_offsets.contains(&before.offset) {
            let mut expected = before.clone();
            expected.species = replacement;
            expected.race2 = 0;
            expected
        } else {
            before.clone()
        };
        if *after != expected {
            return Err(format!(
                "region at {:#X} changed unexpectedly: {before:?} -> {after:?}",
                before.offset
            ));
        }
    }
    Ok(format!(
        "edit ok ({patched} {target} regions -> {replacement})"
    ))
}

fn check_field_edits(bytes: &[u8], galaxy: &Galaxy) -> Result<String, String> {
    let turn = header::turn(bytes).map_err(|error| format!("turn read: {error}"))?;
    let mut edited = bytes.to_vec();
    header::set_turn(&mut edited, turn + 1).map_err(|error| format!("turn write: {error}"))?;
    let reread_turn = header::turn(&edited).map_err(|error| format!("turn reread: {error}"))?;
    if reread_turn != turn + 1 {
        return Err(format!("turn edit did not stick: {turn} -> {reread_turn}"));
    }

    let Some(first) = galaxy.regions.first() else {
        return Ok(format!("turn {turn} ok, pop edit skipped (no regions)"));
    };
    let target_pop = first.pop + 1.5;
    edit::set_population(&mut edited, first, target_pop)
        .map_err(|error| format!("pop write: {error}"))?;

    let reparsed =
        Galaxy::parse(&edited).map_err(|error| format!("reparse after field edits: {error}"))?;
    if reparsed.regions.len() != galaxy.regions.len() {
        return Err("region count changed across field edits".to_owned());
    }
    let Some(reread) = reparsed.regions.first() else {
        return Err("first region vanished across field edits".to_owned());
    };
    if (reread.pop - target_pop).abs() >= 1.0 / 65536.0 {
        return Err(format!(
            "pop edit did not stick: wanted {target_pop}, got {}",
            reread.pop
        ));
    }
    let mut expected = first.clone();
    expected.pop = reread.pop;
    if *reread != expected {
        return Err("pop edit disturbed sibling fields".to_owned());
    }
    for (before, after) in galaxy.regions.iter().zip(&reparsed.regions).skip(1) {
        if before != after {
            return Err(format!(
                "region at {:#X} changed unexpectedly during field edits",
                before.offset
            ));
        }
    }
    Ok(format!("turn {turn} ok, pop edit ok"))
}

/// Run the full check battery against raw save bytes.
#[must_use]
pub fn verify(bytes: &[u8]) -> Outcome {
    let galaxy = match Galaxy::parse(bytes) {
        Ok(galaxy) => galaxy,
        Err(Error::NoGalaxyMarker) => return Outcome::NotSave,
        Err(error) => return Outcome::Fail(format!("parse: {error}")),
    };
    if galaxy.systems.is_empty() {
        return Outcome::Fail("galaxy parsed to zero systems".to_owned());
    }

    let empires = empire::empires(bytes);
    let owned = empire::player_systems(bytes, &galaxy);

    let edit = match check_edit(bytes, &galaxy) {
        Ok(edit) => edit,
        Err(reason) => return Outcome::Fail(reason),
    };
    let fields = match check_field_edits(bytes, &galaxy) {
        Ok(fields) => fields,
        Err(reason) => return Outcome::Fail(reason),
    };

    Outcome::Pass(format!(
        "{} systems, {} populated regions, {} empires, {} owned systems, {edit}, {fields}",
        galaxy.systems.len(),
        galaxy.regions.len(),
        empires.len(),
        owned.len()
    ))
}
