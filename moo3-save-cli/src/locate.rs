//! Find MOO3 save files in the usual install locations.

use std::path::PathBuf;
use std::time::SystemTime;

fn save_dirs() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let suffix = ["steamapps", "common", "Master of Orion 3", "SaveGameFiles"];

    if cfg!(target_os = "linux") {
        if let Some(home) = std::env::home_dir() {
            for root in [
                home.join(".steam/debian-installation"),
                home.join(".steam/steam"),
                home.join(".local/share/Steam"),
                home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
            ] {
                candidates.push(suffix.iter().fold(root, |path, part| path.join(part)));
            }
        }
    } else if cfg!(target_os = "windows") {
        for drive in ["C", "D", "E"] {
            for program_files in ["Program Files (x86)", "Program Files"] {
                let root = PathBuf::from(format!("{drive}:\\{program_files}\\Steam"));
                candidates.push(suffix.iter().fold(root, |path, part| path.join(part)));
            }
        }
        candidates.push(PathBuf::from(
            "C:\\GOG Games\\Master of Orion 3\\SaveGameFiles",
        ));
    } else if cfg!(target_os = "macos") {
        if let Some(home) = std::env::home_dir() {
            let root = home.join("Library/Application Support/Steam");
            candidates.push(suffix.iter().fold(root, |path, part| path.join(part)));
        }
    }

    candidates
        .into_iter()
        .filter(|path| path.is_dir())
        .collect()
}

fn gam_files(dir: &PathBuf) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("gam"))
        })
        .collect()
}

/// The most recently modified `.gam` file across all known save folders
/// (including `AutoSaveHistory/`), or `None` if nothing was found.
pub fn latest_save() -> Option<PathBuf> {
    let mut newest: Option<(SystemTime, PathBuf)> = None;
    for dir in save_dirs() {
        let mut files = gam_files(&dir);
        files.extend(gam_files(&dir.join("AutoSaveHistory")));
        for path in files {
            let Ok(modified) = path.metadata().and_then(|meta| meta.modified()) else {
                continue;
            };
            let is_newer = newest
                .as_ref()
                .is_none_or(|(current, _)| modified > *current);
            if is_newer {
                newest = Some((modified, path));
            }
        }
    }
    newest.map(|(_, path)| path)
}
