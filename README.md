# MOO3 Ithkul Purge

Save game editor for **Master of Orion 3** that removes Ithkul (Harvester) populations from your planets.

## The Problem

Ithkul are the Harvester species in MOO3. When they share a planet with other species, they **bioharvest (eat) the other populations**. This is hardcoded to their species type and cannot be turned off through diplomacy or game settings.

Ithkul can appear on your planets through several mechanisms:
- **FLU Generator** planetary specials randomly spawn pre-warp populations of any species
- Conquest of Ithkul-inhabited worlds
- Migration from nearby Ithkul systems

Once Ithkul are on your planet, they'll eat your population until nothing is left. There is no in-game way to selectively remove a species from a planet.

## What This Does

- **Scans** your save file and reports all Ithkul infestations galaxy-wide
- **Purges** Ithkul from planets where they cohabit with other species by converting them to Klackons
- Creates a `.bak` backup before making any changes
- Works with Steam saves on Linux, Windows, and macOS
- No dependencies beyond Python 3.10+

## Usage

```bash
# Scan for Ithkul (auto-detects latest save)
python3 moo3_scan.py

# Scan a specific save file
python3 moo3_scan.py "path/to/The Synthesis Turn 0150.gam"

# Purge Ithkul from shared planets (interactive species picker)
python3 moo3_purge.py

# Replace Ithkul with a specific species
python3 moo3_purge.py --replace-with human
python3 moo3_purge.py --replace-with sakkra
python3 moo3_purge.py --replace-with psilon

# Only purge Ithkul from planets with your species
python3 moo3_purge.py --protect klackon
python3 moo3_purge.py --protect human

# Preview what would be changed without modifying anything
python3 moo3_purge.py --dry-run

# Purge ALL Ithkul galaxy-wide (nuclear option)
python3 moo3_purge.py --all

# Purge a specific save
python3 moo3_purge.py "path/to/save.gam"
```

### Windows

1. Install [Python 3.10+](https://www.python.org/downloads/) (check "Add to PATH" during install)
2. Download/clone this repo
3. Double-click `moo3_purge.py` or open a terminal in the folder and run `python moo3_purge.py`

The tool auto-detects your Steam save folder and includes "Press Enter to exit" prompts on Windows so the window doesn't vanish.

## Save File Locations

The tools auto-detect saves in common locations:

| Platform | Path |
|----------|------|
| Linux (Steam) | `~/.steam/steam/steamapps/common/Master of Orion 3/SaveGameFiles/` |
| Windows (Steam) | `C:\Program Files (x86)\Steam\steamapps\common\Master of Orion 3\SaveGameFiles\` |
| macOS (Steam) | `~/Library/Application Support/Steam/steamapps/common/Master of Orion 3/SaveGameFiles/` |

AutoSave files are in the `AutoSaveHistory/` subdirectory.

## How It Works

MOO3 save files are binary with big-endian integers, UTF-16BE strings, and a custom fixed-point number format. The save format was reverse-engineered from [Bhruic's MOO3 Save Editor](https://web.archive.org/web/2004/http://bhruic.mine.nu/) and the game binary.

Each planet has regions, and each region has a population with a species type (`race1`) and sub-race (`race2`). The purge tool changes Ithkul regions (`race1=5`) to Klackons (`race1=6`) with a neutral sub-race, which the game handles gracefully.

### Key format details

- Header magic: `VS3RDAEH` / Galaxy marker: `VSYXALAG`
- Custom doubles: 6-byte BE signed integer + 2-byte BE uint16 fraction
- Region race fields: `race1` at region_offset+10, `race2` at region_offset+11
- Planet slot types: `H` (0x48) = 31 extra bytes, `L`/`O` = 30 extra bytes
- Special record tags stored reversed (e.g., `SpFLUGen` stored as `neGULFpS`)

## Species Type IDs

| race1 | Species | Notes |
|-------|---------|-------|
| 0 | Human | |
| 1 | Sakkra | |
| 2 | Meklar | |
| 3 | Silicoid | Immune to bioharvesting |
| 4 | Psilon | |
| 5 | **Ithkul** | **Harvesters - eat other populations** |
| 6 | Klackon | Includes Tachidi sub-races |
| 7 | Raas | |
| 8 | Nommo | |
| 9 | Grendarl | |

## License

MIT
