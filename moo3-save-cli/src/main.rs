//! CLI adapter over the core crate.
//!
//! Thin by design: argument handling and I/O live here, logic lives in
//! core. Doubles as a scriptable test harness for the same code the web
//! app runs.

mod locate;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _};

use moo3_save_core::galaxy::{Galaxy, Region};
use moo3_save_core::replace::{self, ApplyOutcome, Scope};
use moo3_save_core::{edit, empire, header, species, verify, Species};

fn usage() -> ! {
    eprintln!(
        "usage:
  moo3-save scan [file.gam] [--target <species>]
      report species populations and cohabitation risks
  moo3-save replace [file.gam] [options]
      replace one species with another (creates a .bak backup)
        --target <species>        species to replace (default: ithkul)
        --replace-with <species>  replacement (default: klackon)
        --planet <name>           only this planet; repeatable, partial match
        --protect <species>       only planets where this species is present
        --all                     replace galaxy-wide
        --mine                    only systems owned by the player
        --dry-run                 preview without modifying
  moo3-save planet \"<name>\" [file.gam]
      inspect a planet's regions (partial name match)
  moo3-save edit [file.gam] --planet \"<name>\" --region <n> [options]
      edit one region (creates a .bak backup)
        --pop <x>        set population
        --owner <id>     set owning empire id
        --terrain <n>    set terrain index
        --eco <n>        set base ecosystem (-2..=2 in practice)
        --dry-run        preview without modifying
  moo3-save turn [file.gam] [--set <n>]
      show or set the turn counter (creates a .bak backup)
  moo3-save corpus <dir>
      run the verification battery on every .gam in <dir>

With no file argument, the newest save in the Steam/GOG folders is used."
    );
    std::process::exit(2)
}

fn parse_species(name: &str) -> anyhow::Result<Species> {
    let Some(found) = Species::from_name(name) else {
        let known: Vec<String> = species::KNOWN
            .iter()
            .map(|species| species.to_string().to_lowercase())
            .collect();
        bail!(
            "unknown species '{name}'; expected one of: {}",
            known.join(", ")
        );
    };
    Ok(found)
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, rest)) = args.split_first() else {
        usage()
    };
    match command.as_str() {
        "scan" => scan(rest),
        "replace" => replace_command(rest),
        "planet" => planet_command(rest),
        "edit" => edit_command(rest),
        "turn" => turn_command(rest),
        "corpus" => match rest {
            [dir] => corpus(Path::new(dir)),
            _ => usage(),
        },
        _ => usage(),
    }
}

fn resolve_save(path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(path) = path {
        return Ok(path);
    }
    locate::latest_save()
        .context("no MOO3 save folder found; pass the save file path as an argument")
}

fn load(path: &Path) -> anyhow::Result<(Vec<u8>, Galaxy)> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    println!("File: {}", path.display());
    println!("Size: {} bytes", bytes.len());
    let galaxy = Galaxy::parse(&bytes).context("parsing save")?;
    Ok((bytes, galaxy))
}

type PlanetKey = (usize, usize);

fn by_planet(galaxy: &Galaxy) -> BTreeMap<PlanetKey, Vec<&Region>> {
    let mut planets: BTreeMap<PlanetKey, Vec<&Region>> = BTreeMap::new();
    for region in &galaxy.regions {
        planets
            .entry((region.sys_idx, region.planet_idx))
            .or_default()
            .push(region);
    }
    planets
}

fn scan(args: &[String]) -> anyhow::Result<()> {
    let mut path = None;
    let mut target = Species::Ithkul;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--target" => {
                let Some(name) = rest.next() else { usage() };
                target = parse_species(name)?;
            }
            other if !other.starts_with('-') => path = Some(PathBuf::from(other)),
            _ => usage(),
        }
    }

    let path = resolve_save(path)?;
    let (bytes, galaxy) = load(&path)?;
    println!(
        "Parsed {} systems, {} populated regions",
        galaxy.systems.len(),
        galaxy.regions.len()
    );
    let owned = empire::player_systems(&bytes, &galaxy);
    if !owned.is_empty() {
        println!("Player owns {} systems", owned.len());
    }

    infestation_report(&galaxy, target, &owned);
    at_risk_report(&galaxy, target);
    Ok(())
}

fn infestation_report(galaxy: &Galaxy, target: Species, owned: &BTreeSet<usize>) {
    let mut infested: BTreeMap<usize, Vec<&Region>> = BTreeMap::new();
    for region in galaxy
        .regions
        .iter()
        .filter(|region| region.species == target)
    {
        infested.entry(region.sys_idx).or_default().push(region);
    }

    let bar = "=".repeat(70);
    println!("\n{bar}");
    println!(
        "{} REPORT - {} systems",
        target.to_string().to_uppercase(),
        infested.len()
    );
    println!("{bar}");

    let system_pop = |regions: &[&Region]| regions.iter().map(|region| region.pop).sum::<f64>();
    let mut systems: Vec<(&usize, &Vec<&Region>)> = infested.iter().collect();
    systems.sort_by(|(_, a), (_, b)| system_pop(b).total_cmp(&system_pop(a)));

    let mut grand_pop = 0.0;
    for (&sys_idx, regions) in &systems {
        let name = galaxy
            .systems
            .get(sys_idx)
            .map_or("?", |system| system.name.as_str());
        let mine = if owned.contains(&sys_idx) {
            " [YOURS]"
        } else {
            ""
        };
        let planet_indices: BTreeSet<usize> =
            regions.iter().map(|region| region.planet_idx).collect();
        let pop = system_pop(regions);
        grand_pop += pop;
        println!(
            "\n  {name}{mine}: {pop:.1} pop in {} regions on {} planet(s)",
            regions.len(),
            planet_indices.len()
        );
        for planet_idx in planet_indices {
            let on_planet: Vec<&&Region> = regions
                .iter()
                .filter(|region| region.planet_idx == planet_idx)
                .collect();
            let planet = galaxy.planet_name(on_planet[0]);
            let pop: f64 = on_planet.iter().map(|region| region.pop).sum();
            println!("    {planet}: {pop:.2} in {} region(s)", on_planet.len());
        }
    }

    let total_regions: usize = systems.iter().map(|(_, regions)| regions.len()).sum();
    println!("\n{bar}");
    println!(
        "TOTALS: {grand_pop:.1} {target} in {total_regions} regions across {} systems",
        systems.len()
    );
    if !owned.is_empty() {
        let mine = systems
            .iter()
            .filter(|(sys_idx, _)| owned.contains(sys_idx))
            .count();
        println!("  YOUR systems with {target}: {mine}");
    }
    println!("{bar}");
}

fn at_risk_report(galaxy: &Galaxy, target: Species) {
    let bar = "=".repeat(70);
    println!("\n{bar}");
    println!(
        "PLANETS WITH {} + OTHER SPECIES (at risk of bioharvesting)",
        target.to_string().to_uppercase()
    );
    println!("{bar}");

    let planets = by_planet(galaxy);
    let mut at_risk: Vec<&Vec<&Region>> = planets
        .values()
        .filter(|regions| {
            regions.iter().any(|region| region.species == target)
                && regions.iter().any(|region| region.species != target)
        })
        .collect();
    let target_pop = |regions: &[&Region]| {
        regions
            .iter()
            .filter(|region| region.species == target)
            .map(|region| region.pop)
            .sum::<f64>()
    };
    at_risk.sort_by(|a, b| target_pop(b).total_cmp(&target_pop(a)));

    if at_risk.is_empty() {
        println!("  None found - your planets are safe!");
        return;
    }
    for regions in at_risk {
        let planet = galaxy.planet_name(regions[0]);
        let others: Vec<&&Region> = regions
            .iter()
            .filter(|region| region.species != target)
            .collect();
        let other_pop: f64 = others.iter().map(|region| region.pop).sum();
        let mut other_names: Vec<String> = others
            .iter()
            .map(|region| region.species.to_string())
            .collect();
        other_names.sort_unstable();
        other_names.dedup();
        println!(
            "\n  {planet}: {target} {:.2} vs {} {other_pop:.2}",
            target_pop(regions),
            other_names.join(", ")
        );
        for region in regions {
            let marker = if region.species == target {
                format!(" <-- {}", target.to_string().to_uppercase())
            } else {
                String::new()
            };
            println!(
                "    Region {}: {:<10} pop={:.4}{marker}",
                region.region_idx,
                region.species.to_string(),
                region.pop
            );
        }
    }
}

fn next_value<'args>(rest: &mut std::slice::Iter<'args, String>) -> &'args String {
    match rest.next() {
        Some(value) => value,
        None => usage(),
    }
}

struct ReplaceArgs {
    path: Option<PathBuf>,
    target: Species,
    replacement: Species,
    planets: Vec<String>,
    protect: Option<Species>,
    everywhere: bool,
    mine: bool,
    dry_run: bool,
}

fn parse_replace_args(args: &[String]) -> anyhow::Result<ReplaceArgs> {
    let mut parsed = ReplaceArgs {
        path: None,
        target: Species::Ithkul,
        replacement: Species::Klackon,
        planets: Vec::new(),
        protect: None,
        everywhere: false,
        mine: false,
        dry_run: false,
    };
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--target" => parsed.target = parse_species(next_value(&mut rest))?,
            "--replace-with" => parsed.replacement = parse_species(next_value(&mut rest))?,
            "--planet" => parsed.planets.push(next_value(&mut rest).clone()),
            "--protect" => parsed.protect = Some(parse_species(next_value(&mut rest))?),
            "--all" => parsed.everywhere = true,
            "--mine" => parsed.mine = true,
            "--dry-run" => parsed.dry_run = true,
            other if !other.starts_with('-') => parsed.path = Some(PathBuf::from(other)),
            _ => usage(),
        }
    }
    if parsed.protect == Some(parsed.target) {
        bail!("cannot protect {} from themselves", parsed.target);
    }
    if parsed.target == parsed.replacement {
        bail!("target and replacement are both {}", parsed.target);
    }
    Ok(parsed)
}

fn replace_command(args: &[String]) -> anyhow::Result<()> {
    let ReplaceArgs {
        path,
        target,
        replacement,
        planets,
        protect,
        everywhere,
        mine,
        dry_run,
    } = parse_replace_args(args)?;

    let path = resolve_save(path)?;
    let (mut bytes, galaxy) = load(&path)?;

    let owned = if mine {
        let owned = empire::player_systems(&bytes, &galaxy);
        if owned.is_empty() {
            bail!("could not detect player ownership from this save");
        }
        println!("Player owns {} systems", owned.len());
        Some(owned)
    } else {
        None
    };

    let scope = if planets.is_empty() {
        if everywhere {
            Scope::Everywhere
        } else {
            Scope::Shared { protect }
        }
    } else {
        Scope::Planets(planets.clone())
    };

    let planned = replace::plan(&galaxy, target, &scope, owned.as_ref());
    if planned.is_empty() {
        println!("\nNo {target} found to replace. Nothing to do!");
        return Ok(());
    }

    let mut scope_note = match &scope {
        Scope::Planets(names) => format!(
            "on {}",
            names
                .iter()
                .map(|name| format!("\"{name}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Scope::Everywhere => "galaxy-wide".to_owned(),
        Scope::Shared { .. } => "on shared planets".to_owned(),
    };
    if mine {
        scope_note.push_str(" (your systems only)");
    }
    let action = if dry_run {
        "Would convert"
    } else {
        "Converting"
    };
    println!(
        "\n{action} {} {target} regions to {replacement} {scope_note}:\n",
        planned.len()
    );

    let mut sorted = planned.clone();
    sorted.sort_by(|a, b| {
        galaxy
            .planet_name(a)
            .cmp(&galaxy.planet_name(b))
            .then(a.region_idx.cmp(&b.region_idx))
    });
    for region in &sorted {
        let status = if dry_run { "WOULD" } else { "OK   " };
        println!(
            "  {status} {} R{}: pop={:.4}",
            galaxy.planet_name(region),
            region.region_idx,
            region.pop
        );
    }

    if dry_run {
        let pop: f64 = planned.iter().map(|region| region.pop).sum();
        println!(
            "\nDry run: {} regions ({pop:.2} pop) would be converted to {replacement}.",
            planned.len()
        );
        println!("Run without --dry-run to apply.");
        return Ok(());
    }

    let backup = path.with_extension("gam.bak");
    println!("\nBacking up to {}", backup.display());
    std::fs::copy(&path, &backup).context("writing backup")?;

    let ApplyOutcome {
        patched,
        skipped,
        pop,
    } = replace::apply(&mut bytes, &planned, target, replacement);
    if skipped > 0 {
        println!("Skipped {skipped} regions that no longer held {target}.");
    }
    std::fs::write(&path, &bytes).with_context(|| format!("writing {}", path.display()))?;
    println!("\nDone! Converted {patched} {target} regions ({pop:.2} pop) to {replacement}.");
    Ok(())
}

fn backup_and_write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let backup = path.with_extension("gam.bak");
    println!("\nBacking up to {}", backup.display());
    std::fs::copy(path, &backup).context("writing backup")?;
    std::fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))
}

fn region_row(region: &Region) {
    println!(
        "    R{}: owner={:<3} {:<12} race2={:<3} pop={:<10.4} terrain={:<3} eco={}/{}",
        region.region_idx,
        region.owner,
        region.species.to_string(),
        region.race2,
        region.pop,
        region.terrain,
        region.eco_base,
        region.eco_modified,
    );
}

fn matching_planets<'galaxy>(
    galaxy: &'galaxy Galaxy,
    name: &str,
) -> BTreeMap<PlanetKey, Vec<&'galaxy Region>> {
    let name = name.to_lowercase();
    by_planet(galaxy)
        .into_iter()
        .filter(|(_, regions)| {
            galaxy
                .planet_name(regions[0])
                .to_lowercase()
                .contains(&name)
        })
        .collect()
}

fn planet_command(args: &[String]) -> anyhow::Result<()> {
    let mut positional = args.iter().filter(|arg| !arg.starts_with('-'));
    let Some(name) = positional.next() else {
        usage()
    };
    let path = resolve_save(positional.next().map(PathBuf::from))?;
    let (_, galaxy) = load(&path)?;

    let planets = matching_planets(&galaxy, name);
    if planets.is_empty() {
        bail!("no planet matching \"{name}\" has populated regions");
    }
    for regions in planets.values() {
        println!("\n  {}:", galaxy.planet_name(regions[0]));
        for region in regions {
            region_row(region);
        }
    }
    println!(
        "\n{} planet(s) matched. Only populated regions are listed.",
        planets.len()
    );
    Ok(())
}

struct EditArgs {
    path: Option<PathBuf>,
    planet: Option<String>,
    region: Option<usize>,
    pop: Option<f64>,
    owner: Option<u8>,
    terrain: Option<u8>,
    eco: Option<i32>,
    dry_run: bool,
}

fn parse_edit_args(args: &[String]) -> anyhow::Result<EditArgs> {
    let mut parsed = EditArgs {
        path: None,
        planet: None,
        region: None,
        pop: None,
        owner: None,
        terrain: None,
        eco: None,
        dry_run: false,
    };
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--planet" => parsed.planet = Some(next_value(&mut rest).clone()),
            "--region" => parsed.region = Some(next_value(&mut rest).parse()?),
            "--pop" => parsed.pop = Some(next_value(&mut rest).parse()?),
            "--owner" => parsed.owner = Some(next_value(&mut rest).parse()?),
            "--terrain" => parsed.terrain = Some(next_value(&mut rest).parse()?),
            "--eco" => parsed.eco = Some(next_value(&mut rest).parse()?),
            "--dry-run" => parsed.dry_run = true,
            other if !other.starts_with('-') => parsed.path = Some(PathBuf::from(other)),
            _ => usage(),
        }
    }
    Ok(parsed)
}

fn edit_command(args: &[String]) -> anyhow::Result<()> {
    let EditArgs {
        path,
        planet,
        region,
        pop,
        owner,
        terrain,
        eco,
        dry_run,
    } = parse_edit_args(args)?;
    let (Some(planet), Some(region_idx)) = (planet, region) else {
        usage()
    };
    if pop.is_none() && owner.is_none() && terrain.is_none() && eco.is_none() {
        bail!("nothing to edit: pass at least one of --pop, --owner, --terrain, --eco");
    }

    let path = resolve_save(path)?;
    let (mut bytes, galaxy) = load(&path)?;

    let mut planets = matching_planets(&galaxy, &planet);
    if planets.len() > 1 {
        let wanted = planet.to_lowercase();
        let exact: BTreeMap<PlanetKey, Vec<&Region>> = planets
            .iter()
            .filter(|(_, regions)| galaxy.planet_name(regions[0]).to_lowercase() == wanted)
            .map(|(key, regions)| (*key, regions.clone()))
            .collect();
        if exact.len() == 1 {
            planets = exact;
        } else {
            for regions in planets.values() {
                println!("  {}", galaxy.planet_name(regions[0]));
            }
            bail!(
                "\"{planet}\" matches {} planets; be more specific",
                planets.len()
            );
        }
    }
    let Some(regions) = planets.values().next() else {
        bail!("no planet matching \"{planet}\" has populated regions");
    };
    let Some(target) = regions
        .iter()
        .find(|region| region.region_idx == region_idx)
    else {
        println!("Populated regions on {}:", galaxy.planet_name(regions[0]));
        for region in regions {
            region_row(region);
        }
        bail!("region {region_idx} is not a populated region of this planet");
    };

    println!("\nBefore:");
    region_row(target);

    if dry_run {
        println!("\nDry run: no changes written.");
        return Ok(());
    }

    if let Some(pop) = pop {
        edit::set_population(&mut bytes, target, pop)?;
    }
    if let Some(owner) = owner {
        edit::set_owner(&mut bytes, target, owner)?;
    }
    if let Some(terrain) = terrain {
        edit::set_terrain(&mut bytes, target, terrain)?;
    }
    if let Some(eco) = eco {
        edit::set_ecosystem(&mut bytes, target, eco)?;
    }

    let reparsed = Galaxy::parse(&bytes).context("re-parsing after edit")?;
    let Some(after) = reparsed
        .regions
        .iter()
        .find(|region| region.offset == target.offset)
    else {
        bail!("edited region no longer parses; aborting without writing");
    };
    println!("\nAfter:");
    region_row(after);

    backup_and_write(&path, &bytes)?;
    println!("\nDone.");
    Ok(())
}

fn turn_command(args: &[String]) -> anyhow::Result<()> {
    let mut path = None;
    let mut set: Option<u32> = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--set" => set = Some(next_value(&mut rest).parse()?),
            other if !other.starts_with('-') => path = Some(PathBuf::from(other)),
            _ => usage(),
        }
    }
    let path = resolve_save(path)?;
    let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    println!("File: {}", path.display());
    let turn = header::turn(&bytes)?;
    println!("Turn: {turn}");

    let Some(new_turn) = set else {
        return Ok(());
    };
    let mut bytes = bytes;
    header::set_turn(&mut bytes, new_turn)?;
    backup_and_write(&path, &bytes)?;
    println!("Turn set to {new_turn}.");
    Ok(())
}

enum Verdict {
    Pass(String),
    Skip,
    Fail(String),
}

fn verdict_for(path: &Path) -> Verdict {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => return Verdict::Fail(format!("read: {error}")),
    };
    match verify::verify(&bytes) {
        verify::Outcome::Pass(summary) => Verdict::Pass(summary),
        verify::Outcome::NotSave => Verdict::Skip,
        verify::Outcome::Fail(reason) => Verdict::Fail(reason),
    }
}

fn corpus(dir: &Path) -> anyhow::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("gam"))
        })
        .collect();
    entries.sort();

    let mut passed = 0_usize;
    let mut skipped = 0_usize;
    let mut failed = 0_usize;
    for path in &entries {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        match verdict_for(path) {
            Verdict::Pass(summary) => {
                passed += 1;
                println!("PASS {name}: {summary}");
            }
            Verdict::Skip => {
                skipped += 1;
                println!("SKIP {name}: not a MOO3 save");
            }
            Verdict::Fail(reason) => {
                failed += 1;
                println!("FAIL {name}: {reason}");
            }
        }
    }
    println!(
        "\n{passed} passed, {skipped} skipped, {failed} failed, {} total",
        entries.len()
    );
    if failed > 0 {
        bail!("{failed} corpus files failed");
    }
    Ok(())
}
