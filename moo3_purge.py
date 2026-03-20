#!/usr/bin/env python3
"""Purge Ithkul from your planets in a MOO3 save.

Finds all regions where Ithkul (Harvesters) share a planet with another species
and converts them to a species of your choice. Creates a .bak backup before patching.

Usage:
    python3 moo3_purge.py                          # auto-detect, interactive
    python3 moo3_purge.py path/to/save.gam         # specific save file
    python3 moo3_purge.py --replace-with human      # convert Ithkul to Humans
    python3 moo3_purge.py --replace-with sakkra     # convert Ithkul to Sakkra
    python3 moo3_purge.py --protect klackon          # only purge from your Klackon/Tachidi planets
    python3 moo3_purge.py --protect human            # only purge from Human planets
    python3 moo3_purge.py --planet "Alrisha VII"       # purge Ithkul from a specific planet
    python3 moo3_purge.py --planet "Nu Hydrae II" --planet "Phelot II"  # multiple planets
    python3 moo3_purge.py --all                      # purge ALL Ithkul galaxy-wide
    python3 moo3_purge.py --dry-run                  # preview without changing
"""
import shutil
import sys
from collections import defaultdict
from pathlib import Path
from moo3save import (
    ITHKUL_RACE1, KLACKON_RACE1, SPECIES,
    find_latest_save, parse_galaxy, planet_name,
)

# Reverse lookup: name -> race1 ID
SPECIES_BY_NAME = {name.lower(): id for id, name in SPECIES.items() if id != ITHKUL_RACE1}


def parse_args():
    args = sys.argv[1:]
    save_path = None
    replace_with = None
    protect = None
    planet_filters = []
    purge_all = False
    dry_run = False

    i = 0
    while i < len(args):
        if args[i] == '--all':
            purge_all = True
        elif args[i] == '--dry-run':
            dry_run = True
        elif args[i] == '--replace-with' and i + 1 < len(args):
            i += 1
            replace_with = args[i].lower()
        elif args[i] == '--protect' and i + 1 < len(args):
            i += 1
            protect = args[i].lower()
        elif args[i] == '--planet' and i + 1 < len(args):
            i += 1
            planet_filters.append(args[i].lower())
        elif not args[i].startswith('-'):
            save_path = args[i]
        i += 1

    return save_path, replace_with, protect, planet_filters, purge_all, dry_run


def choose_replacement():
    """Interactive species selection for Windows users and anyone who prefers menus."""
    print("\nReplace Ithkul with which species?")
    print()
    choices = sorted(SPECIES_BY_NAME.items(), key=lambda x: x[1])
    for i, (name, race_id) in enumerate(choices):
        print(f"  {i+1:2d}. {name.title():12s} (race1={race_id})")
    print()

    while True:
        try:
            raw = input("Enter number or species name [default: klackon]: ").strip()
            if not raw:
                return KLACKON_RACE1, "Klackon"
            if raw.lower() in SPECIES_BY_NAME:
                rid = SPECIES_BY_NAME[raw.lower()]
                return rid, SPECIES[rid]
            idx = int(raw) - 1
            if 0 <= idx < len(choices):
                name, rid = choices[idx]
                return rid, SPECIES[rid]
            print(f"  Invalid choice. Enter 1-{len(choices)} or a species name.")
        except (ValueError, EOFError):
            print(f"  Invalid input. Enter 1-{len(choices)} or a species name.")


def main():
    save_file, replace_with, protect, planet_filters, purge_all, dry_run = parse_args()

    if save_file:
        save_path = Path(save_file)
    else:
        save_path = find_latest_save()
        if not save_path:
            print("Could not find MOO3 save files. Pass the path as an argument:")
            print("  python3 moo3_purge.py \"path/to/savefile.gam\"")
            if sys.platform == 'win32':
                input("\nPress Enter to exit...")
            sys.exit(1)

    print(f"File: {save_path}")
    data = bytearray(save_path.read_bytes())
    print(f"Size: {len(data):,} bytes")

    try:
        regions, system_count = parse_galaxy(data)
    except Exception as e:
        print(f"Parse error: {e}")
        if sys.platform == 'win32':
            input("\nPress Enter to exit...")
        sys.exit(1)

    # Group by planet
    planets = defaultdict(list)
    for r in regions:
        planets[(r['system'], r['sys_idx'], r['planet'])].append(r)

    # Resolve --protect species
    protect_race1 = None
    if protect:
        if protect not in SPECIES_BY_NAME and protect != 'ithkul':
            print(f"\nUnknown species '{protect}'. Available:")
            for name in sorted(SPECIES_BY_NAME):
                print(f"  {name}")
            sys.exit(1)
        protect_race1 = SPECIES_BY_NAME.get(protect)
        if protect_race1 is None:
            print("Cannot protect Ithkul from themselves!")
            sys.exit(1)

    # Find Ithkul to purge
    patches = []
    if planet_filters:
        # Target specific planets by name (e.g. "Alrisha VII")
        for key, regs in planets.items():
            sys_name, _, p_idx = key
            pn = planet_name(sys_name, p_idx).lower()
            if any(f in pn for f in planet_filters):
                patches.extend(r for r in regs if r['race1'] == ITHKUL_RACE1)
    elif purge_all:
        patches = [r for r in regions if r['race1'] == ITHKUL_RACE1]
    else:
        for key, regs in planets.items():
            has_ithkul = any(r['race1'] == ITHKUL_RACE1 for r in regs)
            if protect_race1 is not None:
                has_protected = any(r['race1'] == protect_race1 for r in regs)
            else:
                has_protected = any(r['race1'] != ITHKUL_RACE1 for r in regs)
            if has_ithkul and has_protected:
                patches.extend(r for r in regs if r['race1'] == ITHKUL_RACE1)

    if not patches:
        if purge_all:
            print("\nNo Ithkul found anywhere. The galaxy is clean!")
        else:
            print("\nNo Ithkul found sharing planets with other species. Nothing to do!")
        if sys.platform == 'win32':
            input("\nPress Enter to exit...")
        sys.exit(0)

    # Determine replacement species
    if replace_with:
        if replace_with not in SPECIES_BY_NAME:
            print(f"\nUnknown species '{replace_with}'. Available:")
            for name in sorted(SPECIES_BY_NAME):
                print(f"  {name}")
            sys.exit(1)
        new_race1 = SPECIES_BY_NAME[replace_with]
        new_name = SPECIES[new_race1]
    elif sys.stdin.isatty():
        new_race1, new_name = choose_replacement()
    else:
        new_race1 = KLACKON_RACE1
        new_name = "Klackon"

    mode = "galaxy-wide" if purge_all else "on shared planets"
    action = "Would convert" if dry_run else "Converting"
    print(f"\n{action} {len(patches)} Ithkul regions to {new_name} {mode}:\n")

    total_pop = 0
    patched = 0
    for p in sorted(patches, key=lambda x: (x['system'], x['planet'], x['region'])):
        pn = planet_name(p['system'], p['planet'])
        r1_off = p['offset'] + 10
        r2_off = p['offset'] + 11
        cur_r1 = data[r1_off]

        if cur_r1 != ITHKUL_RACE1:
            print(f"  SKIP {pn} R{p['region']}: race1 is {cur_r1}, not {ITHKUL_RACE1}")
            continue

        if not dry_run:
            data[r1_off] = new_race1
            data[r2_off] = 0

        total_pop += p['pop']
        patched += 1
        status = "WOULD" if dry_run else "OK  "
        print(f"  {status} {pn} R{p['region']}: pop={p['pop']:.4f}")

    if dry_run:
        print(f"\nDry run: {patched} regions ({total_pop:.2f} pop) would be converted to {new_name}.")
        print("Run without --dry-run to apply.")
        if sys.platform == 'win32':
            input("\nPress Enter to exit...")
        return

    backup = str(save_path) + ".bak"
    print(f"\nBacking up to {backup}")
    shutil.copy2(save_path, backup)

    print("Writing patched save...")
    save_path.write_bytes(data)
    print(f"\nDone! Converted {patched} Ithkul regions ({total_pop:.2f} pop) to {new_name}.")

    if sys.platform == 'win32':
        input("\nPress Enter to exit...")


if __name__ == "__main__":
    main()
