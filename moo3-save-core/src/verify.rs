//! Self-checks used by the corpus runner and the integration tests.
//!
//! [`verify`] exercises the full public surface against one save: parse the
//! galaxy, parse the empire table, detect player ownership, then apply a
//! representative species replacement to a copy and re-parse — asserting
//! that exactly the planned regions changed and nothing else in the parsed
//! structure moved.

use crate::galaxy::Galaxy;
use crate::replace::{self, ApplyOutcome, Scope};
use crate::{empire, Error, Species};

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

    Outcome::Pass(format!(
        "{} systems, {} populated regions, {} empires, {} owned systems, {edit}",
        galaxy.systems.len(),
        galaxy.regions.len(),
        empires.len(),
        owned.len()
    ))
}
