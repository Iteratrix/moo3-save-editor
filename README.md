# MOO3 Save Editor

Save game editor for **Master of Orion 3**. Parses the binary save format and lets you scan and replace species populations on any planet.

Built to solve the Ithkul problem (Harvesters eating your population), but works as a general-purpose species replacement tool.

## Features

- **Scan** your galaxy for any species' populations
- **Replace** one species with another on specific planets or galaxy-wide
- **Auto-detects** save folder on Linux, Windows, and macOS
- **Interactive species picker** or command-line flags
- **Dry run** mode to preview changes
- Creates `.bak` backups before any modification
- No dependencies beyond Python 3.10+

## Quick Start

```bash
# Scan for all species populations
python3 moo3_species.py

# Replace Ithkul with Klackons on shared planets (the classic use case)
python3 moo3_replace.py

# Replace Ithkul with Darloks on a specific planet
python3 moo3_replace.py --replace-with darlok --planet "Alrisha VII"

# Turn everyone on a planet into Humans
python3 moo3_replace.py --target klackon --replace-with human --planet "Psi Tauri I"

# Preview without changing anything
python3 moo3_replace.py --dry-run
```

## Usage: `moo3_species.py`

Scans a save file and reports all species populations, with emphasis on planets where Ithkul cohabit with (and are eating) other species.

```bash
python3 moo3_species.py                     # auto-detect latest save
python3 moo3_species.py path/to/save.gam    # specific save file
```

## Usage: `moo3_replace.py`

Replaces one species with another. Creates a `.bak` backup before patching.

```bash
# Replace Ithkul (default target) with interactive species picker
python3 moo3_replace.py

# Target a specific species to replace
python3 moo3_replace.py --target klackon --replace-with human

# Only replace on specific planets
python3 moo3_replace.py --planet "Alrisha VII"
python3 moo3_replace.py --planet "Alrisha VII" --planet "Phelot II"
python3 moo3_replace.py --planet "Alrisha"          # partial match: all planets in system

# Only replace where a specific species is at risk
python3 moo3_replace.py --protect klackon            # only on planets with Klackons

# Replace ALL instances galaxy-wide
python3 moo3_replace.py --target ithkul --replace-with darlok --all

# Preview changes without modifying
python3 moo3_replace.py --dry-run

# Specify a save file
python3 moo3_replace.py path/to/save.gam
```

### Flags

| Flag | Description |
|------|-------------|
| `--target <species>` | Species to replace (default: Ithkul) |
| `--replace-with <species>` | Species to replace with (interactive if omitted) |
| `--planet "<name>"` | Only affect this planet (repeatable, partial match) |
| `--protect <species>` | Only affect planets where this species is present |
| `--all` | Replace target species galaxy-wide |
| `--dry-run` | Preview changes without modifying the save |

### Windows

1. Install [Python 3.10+](https://www.python.org/downloads/) (check "Add to PATH" during install)
2. Download/clone this repo
3. Double-click `moo3_replace.py` or run `python moo3_replace.py` in a terminal

The tool auto-detects your Steam save folder and keeps the window open on Windows.

## Save File Locations

The tools auto-detect saves in common locations:

| Platform | Path |
|----------|------|
| Linux (Steam) | `~/.steam/steam/steamapps/common/Master of Orion 3/SaveGameFiles/` |
| Windows (Steam) | `C:\Program Files (x86)\Steam\steamapps\common\Master of Orion 3\SaveGameFiles\` |
| macOS (Steam) | `~/Library/Application Support/Steam/steamapps/common/Master of Orion 3/SaveGameFiles/` |

AutoSave files are in the `AutoSaveHistory/` subdirectory.

## The Ithkul Problem

Ithkul are the Harvester species in MOO3. When they share a planet with other species, they **bioharvest (eat) the other populations**. This is hardcoded and cannot be turned off through diplomacy or game settings.

Ithkul appear on your planets via FLU Generator planetary specials, conquest, or migration. Once present, they'll consume your population with no in-game way to remove them selectively.

The default behavior of `moo3_replace.py` (no flags) targets exactly this: find Ithkul on planets shared with other species and replace them.

## Species IDs

From `racemodifiers.txt` in the game data:

| race1 | Species | Type |
|-------|---------|------|
| 0 | Human | Humanoid (Human, Evon, Psilon) |
| 1 | Imsaeis | Etherean (Imsaeis, Eoladi) |
| 2 | Silicoid | Lithic (immune to bioharvesting) |
| 3 | Meklar | Cybernetic (Meklar, Cynoid) |
| 4 | Trilarian | Ichthytosian (Trilarian, Nommo) |
| 5 | **Ithkul** | **Harvester - eats other populations** |
| 6 | Klackon | Insecta (Klackon, Tachidi) |
| 7 | Sakkra | Saurian (Sakkra, Raas, Grendarl) |
| 8 | Darlok | Metashifter (Shapeshifter) |
| 9 | NonCorporeal | Non-corporeal beings |
| 10 | Protoplasmic | Protoplasmic |
| 11 | Plant | Plant species |
| 12 | Fungal | Fungal species |
| 13 | Avian | Avian (includes Alkari) |
| 14 | Gargantua | Giant species |
| 15 | Bulrathi | Ursoid |
| 16 | Mrrshan | Feline |
| 17 | Elerian | Telepathic humanoid |
| 18 | Gnolam | Trader species |
| 19 | Elder | Elder Civilization |
| 20 | ComBot | Combat robots |

## How It Works

MOO3 saves are binary files with big-endian integers, UTF-16BE strings, and a custom fixed-point number format. The format was reverse-engineered from [Bhruic's MOO3 Save Editor](https://web.archive.org/web/2004/http://bhruic.mine.nu/) and the game binary.

Each planet has regions, and each region has a population with a species type (`race1`) and sub-race/magnate (`race2`). The replacement tool changes `race1` at region_offset+10 and sets `race2` to 0 at region_offset+11.

### Key format details

- Header magic: `VS3RDAEH` / Galaxy marker: `VSYXALAG`
- Custom doubles: 6-byte BE signed integer + 2-byte BE uint16 fraction
- Planet slot types: `H` (0x48) = 31 extra bytes, `L`/`O` = 30 extra bytes, `0xFF` = empty orbit
- Special record tags stored reversed (e.g., `SpFLUGen` stored as `neGULFpS`)

## License

MIT
