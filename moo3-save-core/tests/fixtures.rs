use std::io::Read as _;

use moo3_save_core::galaxy::Galaxy;
use moo3_save_core::replace::{self, Scope};
use moo3_save_core::{empire, verify, Species};

fn fixture() -> Vec<u8> {
    let path = format!(
        "{}/../test-data/synthesis-turn115.gam.gz",
        env!("CARGO_MANIFEST_DIR")
    );
    let compressed = std::fs::read(path).expect("fixture present");
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(compressed.as_slice())
        .read_to_end(&mut bytes)
        .expect("fixture inflates");
    bytes
}

#[test]
fn fixture_parses_and_verifies() {
    let bytes = fixture();
    let galaxy = Galaxy::parse(&bytes).expect("parses");
    assert_eq!(galaxy.systems.len(), 250);
    assert_eq!(galaxy.regions.len(), 2794);

    let empires = empire::empires(&bytes);
    assert_eq!(empires.len(), 11);
    assert!(
        empires.iter().any(|e| e.id == empire::PLAYER_EMPIRE_ID),
        "player empire present"
    );

    let owned = empire::player_systems(&bytes, &galaxy);
    assert_eq!(owned.len(), 20);

    let player = empires
        .iter()
        .find(|e| e.id == empire::PLAYER_EMPIRE_ID)
        .expect("player empire");
    assert_eq!(empire::treasury(&bytes, player), Some(64_573));

    let mut edited = bytes.clone();
    empire::set_treasury(&mut edited, player, 7_777_777).expect("treasury writes");
    assert_eq!(empire::treasury(&edited, player), Some(7_777_777));

    let verify::Outcome::Pass(summary) = verify::verify(&bytes) else {
        panic!("expected pass");
    };
    assert!(summary.contains("edit ok"), "{summary}");
}

#[test]
fn shared_scope_replacement_clears_cohabitation() {
    let mut bytes = fixture();
    let galaxy = Galaxy::parse(&bytes).expect("parses");

    let scope = Scope::Shared { protect: None };
    let target = moo3_save_core::species::KNOWN
        .into_iter()
        .find(|&species| !replace::plan(&galaxy, species, &scope, None).is_empty())
        .expect("some species cohabits somewhere");
    let replacement = if target == Species::Silicoid {
        Species::Klackon
    } else {
        Species::Silicoid
    };

    let planned = replace::plan(&galaxy, target, &scope, None);
    let everywhere = replace::plan(&galaxy, target, &Scope::Everywhere, None);
    assert!(
        planned.len() <= everywhere.len(),
        "shared scope never exceeds galaxy-wide"
    );

    let outcome = replace::apply(&mut bytes, &planned, target, replacement);
    assert_eq!(outcome.patched, planned.len());
    assert_eq!(outcome.skipped, 0);

    let reparsed = Galaxy::parse(&bytes).expect("reparses");
    assert_eq!(reparsed.regions.len(), galaxy.regions.len());
    let remaining = replace::plan(&reparsed, target, &scope, None);
    assert!(remaining.is_empty(), "no shared-planet {target} remain");
}

#[test]
fn planet_scope_matches_partial_names() {
    let bytes = fixture();
    let galaxy = Galaxy::parse(&bytes).expect("parses");
    let Some(first) = replace::plan(&galaxy, Species::Ithkul, &Scope::Everywhere, None)
        .first()
        .copied()
        .cloned()
    else {
        panic!("fixture has Ithkul");
    };
    let planet = galaxy.planet_name(&first);
    let system = planet
        .rsplit_once(' ')
        .map(|(system, _)| system)
        .expect("has numeral");

    let exact = replace::plan(
        &galaxy,
        Species::Ithkul,
        &Scope::Planets(vec![planet.to_lowercase()]),
        None,
    );
    assert!(exact.iter().any(|region| region.offset == first.offset));

    let by_system = replace::plan(
        &galaxy,
        Species::Ithkul,
        &Scope::Planets(vec![system.to_lowercase()]),
        None,
    );
    assert!(by_system.len() >= exact.len());
}

#[test]
fn stale_plans_are_skipped_not_applied() {
    let mut bytes = fixture();
    let galaxy = Galaxy::parse(&bytes).expect("parses");
    let planned = replace::plan(&galaxy, Species::Ithkul, &Scope::Everywhere, None);
    assert!(!planned.is_empty());

    let first = replace::apply(&mut bytes, &planned, Species::Ithkul, Species::Darlok);
    assert_eq!(first.skipped, 0);
    let again = replace::apply(&mut bytes, &planned, Species::Ithkul, Species::Darlok);
    assert_eq!(again.patched, 0);
    assert_eq!(again.skipped, planned.len());
}

#[test]
fn corpus_env_dir_all_pass() {
    let Ok(dir) = std::env::var("MOO3_CORPUS_DIR") else {
        eprintln!("MOO3_CORPUS_DIR unset; skipping corpus test");
        return;
    };
    let mut checked = 0;
    for entry in std::fs::read_dir(dir).expect("corpus dir readable") {
        let path = entry.expect("dir entry").path();
        if path
            .extension()
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("gam"))
        {
            continue;
        }
        let bytes = std::fs::read(&path).expect("readable");
        match verify::verify(&bytes) {
            verify::Outcome::Pass(_) | verify::Outcome::NotSave => checked += 1,
            verify::Outcome::Fail(reason) => panic!("{}: {reason}", path.display()),
        }
    }
    assert!(checked > 0, "corpus dir contained no .gam files");
}
