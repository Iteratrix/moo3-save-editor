//! WASM bridge: JSON-in/JSON-out API over the core crate.
//!
//! Thin by design: (de)serialization lives here, logic lives in core. A
//! string API keeps the JS side trivial (`JSON.parse` and go). If the
//! boundary grows rich enough that stringly typing hurts, upgrade to
//! `tsify` for generated TypeScript types.
//!
//! Three calls mirror the CLI's flow: [`summarize`] (scan), [`plan_replace`]
//! (dry run / preview), and [`apply_replace`] (patch and return new bytes).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use moo3_save_core::galaxy::Galaxy;
use moo3_save_core::replace::{self, ApplyOutcome, Scope};
use moo3_save_core::{empire, species, Species};

#[derive(Serialize)]
struct EmpireJson {
    id: u8,
    species: String,
    name: String,
}

#[derive(Serialize)]
struct SpeciesTotal {
    name: String,
    pop: f64,
    regions: usize,
    systems: usize,
}

#[derive(Serialize)]
struct SummaryJson {
    systems: usize,
    regions: usize,
    player_systems: usize,
    empires: Vec<EmpireJson>,
    species: Vec<SpeciesTotal>,
    known_species: Vec<String>,
}

fn parse_galaxy(bytes: &[u8]) -> Result<Galaxy, JsError> {
    Galaxy::parse(bytes).map_err(|error| JsError::new(&error.to_string()))
}

/// Scan a save: species totals, empires, and player ownership.
///
/// # Errors
///
/// When the bytes are not a parseable MOO3 save.
///
/// # Panics
///
/// Only if JSON serialization fails, which cannot happen for these types.
#[wasm_bindgen]
pub fn summarize(bytes: &[u8]) -> Result<String, JsError> {
    let galaxy = parse_galaxy(bytes)?;
    let empires = empire::empires(bytes);
    let owned = empire::player_systems(bytes, &galaxy);

    let mut totals: BTreeMap<u8, SpeciesTotal> = BTreeMap::new();
    let mut seen_systems: BTreeSet<(u8, usize)> = BTreeSet::new();
    for region in &galaxy.regions {
        let race1 = region.species.race1();
        let entry = totals.entry(race1).or_insert_with(|| SpeciesTotal {
            name: region.species.to_string(),
            pop: 0.0,
            regions: 0,
            systems: 0,
        });
        entry.pop += region.pop;
        entry.regions += 1;
        if seen_systems.insert((race1, region.sys_idx)) {
            entry.systems += 1;
        }
    }
    let mut totals: Vec<SpeciesTotal> = totals.into_values().collect();
    totals.sort_by(|a, b| b.pop.total_cmp(&a.pop));

    let summary = SummaryJson {
        systems: galaxy.systems.len(),
        regions: galaxy.regions.len(),
        player_systems: owned.len(),
        empires: empires
            .into_iter()
            .map(|empire::Empire { id, species, name }| EmpireJson { id, species, name })
            .collect(),
        species: totals,
        known_species: species::KNOWN.iter().map(ToString::to_string).collect(),
    };
    Ok(serde_json::to_string(&summary).expect("summary serializes"))
}

#[derive(Deserialize)]
struct Options {
    target: String,
    replacement: String,
    scope: String,
    #[serde(default)]
    planets: Vec<String>,
    #[serde(default)]
    protect: Option<String>,
    #[serde(default)]
    mine: bool,
}

struct Resolved {
    target: Species,
    replacement: Species,
    scope: Scope,
    mine: bool,
}

fn resolve(options: &str) -> Result<Resolved, JsError> {
    let Options {
        target,
        replacement,
        scope,
        planets,
        protect,
        mine,
    } = serde_json::from_str(options).map_err(|error| JsError::new(&error.to_string()))?;

    let species_named = |name: &str| -> Result<Species, JsError> {
        Species::from_name(name).ok_or_else(|| JsError::new(&format!("unknown species '{name}'")))
    };
    let target = species_named(&target)?;
    let replacement = species_named(&replacement)?;
    if target == replacement {
        return Err(JsError::new("target and replacement are the same species"));
    }
    let scope = match scope.as_str() {
        "shared" => Scope::Shared {
            protect: match protect {
                Some(name) if !name.is_empty() => Some(species_named(&name)?),
                _ => None,
            },
        },
        "everywhere" => Scope::Everywhere,
        "planets" => {
            if planets.is_empty() {
                return Err(JsError::new("no planet names given"));
            }
            Scope::Planets(planets)
        }
        other => return Err(JsError::new(&format!("unknown scope '{other}'"))),
    };
    Ok(Resolved {
        target,
        replacement,
        scope,
        mine,
    })
}

fn owned_systems(
    bytes: &[u8],
    galaxy: &Galaxy,
    mine: bool,
) -> Result<Option<BTreeSet<usize>>, JsError> {
    if !mine {
        return Ok(None);
    }
    let owned = empire::player_systems(bytes, galaxy);
    if owned.is_empty() {
        return Err(JsError::new(
            "could not detect player ownership from this save",
        ));
    }
    Ok(Some(owned))
}

#[derive(Serialize)]
struct PlannedRegion {
    planet: String,
    region: usize,
    pop: f64,
}

#[derive(Serialize)]
struct PlanJson {
    count: usize,
    pop: f64,
    regions: Vec<PlannedRegion>,
}

/// Preview which regions a replacement would touch.
///
/// # Errors
///
/// On unparseable saves, malformed options, or unknown species names.
///
/// # Panics
///
/// Only if JSON serialization fails, which cannot happen for these types.
#[wasm_bindgen]
pub fn plan_replace(bytes: &[u8], options: &str) -> Result<String, JsError> {
    let Resolved {
        target,
        scope,
        mine,
        ..
    } = resolve(options)?;
    let galaxy = parse_galaxy(bytes)?;
    let owned = owned_systems(bytes, &galaxy, mine)?;
    let planned = replace::plan(&galaxy, target, &scope, owned.as_ref());

    let mut regions: Vec<PlannedRegion> = planned
        .iter()
        .map(|region| PlannedRegion {
            planet: galaxy.planet_name(region),
            region: region.region_idx,
            pop: region.pop,
        })
        .collect();
    regions.sort_by(|a, b| a.planet.cmp(&b.planet).then(a.region.cmp(&b.region)));
    let plan = PlanJson {
        count: planned.len(),
        pop: planned.iter().map(|region| region.pop).sum(),
        regions,
    };
    Ok(serde_json::to_string(&plan).expect("plan serializes"))
}

/// Apply a replacement and return the patched save bytes.
///
/// # Errors
///
/// Same conditions as [`plan_replace`], plus when the plan matches nothing.
#[wasm_bindgen]
pub fn apply_replace(bytes: &[u8], options: &str) -> Result<Vec<u8>, JsError> {
    let Resolved {
        target,
        replacement,
        scope,
        mine,
    } = resolve(options)?;
    let galaxy = parse_galaxy(bytes)?;
    let owned = owned_systems(bytes, &galaxy, mine)?;
    let planned = replace::plan(&galaxy, target, &scope, owned.as_ref());
    if planned.is_empty() {
        return Err(JsError::new(&format!("no {target} regions match")));
    }

    let mut edited = bytes.to_vec();
    let ApplyOutcome {
        patched, skipped, ..
    } = replace::apply(&mut edited, &planned, target, replacement);
    if skipped > 0 || patched != planned.len() {
        return Err(JsError::new(&format!(
            "apply patched {patched} of {} planned regions",
            planned.len()
        )));
    }
    Ok(edited)
}
