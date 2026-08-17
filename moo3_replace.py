#!/usr/bin/env python3
"""MOO3 Save Editor — replace species populations.

Replace one species with another on specific planets or galaxy-wide.
Defaults to replacing Ithkul (Harvesters) on shared planets. Creates
a .bak backup before patching.

Usage:
    python3 moo3_replace.py                         # auto-detect, interactive
    python3 moo3_replace.py path/to/save.gam        # specific save file
    python3 moo3_replace.py --replace-with darlok    # convert Ithkul to Darloks
    python3 moo3_replace.py --target klackon --replace-with human --planet "Psi Tauri I"
    python3 moo3_replace.py --protect klackon        # only on planets with Klackons
    python3 moo3_replace.py --planet "Alrisha VII"   # specific planet
    python3 moo3_replace.py --all                    # galaxy-wide
    python3 moo3_replace.py --mine --all             # all Ithkul on YOUR systems only
    python3 moo3_replace.py --dry-run                # preview without changing
"""
import shutil
import sys
from collections import defaultdict
from pathlib import Path
from moo3save import (
    ITHKUL_RACE1, KLACKON_RACE1, SPECIES,
    find_latest_save, parse_galaxy, parse_player_systems, planet_name,
)

# Reverse lookup: name -> race1 ID
SPECIES_BY_NAME = {name.lower(): id for id, name in SPECIES.items()}


def parse_args():
    args = sys.argv[1:]
    save_path = None
    replace_with = None
    protect = None
    target = None
    planet_filters = []
    purge_all = False
    mine_only = False
    dry_run = False

    i = 0
    while i < len(args):
        if args[i] == '--all':
            purge_all = True
        elif args[i] == '--mine':
            mine_only = True
        elif args[i] == '--dry-run':
            dry_run = True
        elif args[i] == '--replace-with' and i + 1 < len(args):
            i += 1
            replace_with = args[i].lower()
        elif args[i] == '--protect' and i + 1 < len(args):
            i += 1
            protect = args[i].lower()
        elif args[i] == '--target' and i + 1 < len(args):
            i += 1
            target = args[i].lower()
        elif args[i] == '--planet' and i + 1 < len(args):
            i += 1
            planet_filters.append(args[i].lower())
        elif not args[i].startswith('-'):
            save_path = args[i]
        i += 1

    return save_path, replace_with, protect, target, planet_filters, purge_all, mine_only, dry_run


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
    save_file, replace_with, protect, target, planet_filters, purge_all, mine_only, dry_run = parse_args()

    # Resolve --target species (defaults to Ithkul)
    all_species_by_name = {name.lower(): id for id, name in SPECIES.items()}
    if target:
        if target not in all_species_by_name:
            print(f"Unknown target species '{target}'. Available:")
            for name in sorted(all_species_by_name):
                print(f"  {name}")
            sys.exit(1)
        target_race1 = all_species_by_name[target]
        target_name = SPECIES[target_race1]
    else:
        target_race1 = ITHKUL_RACE1
        target_name = "Ithkul"

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

    # Player ownership (for --mine flag)
    player_systems = set()
    if mine_only:
        player_systems = parse_player_systems(data)
        if not player_systems:
            print("Could not detect player ownership from save file.")
            sys.exit(1)
        print(f"Player owns {len(player_systems)} systems")

    # Group by planet
    planets = defaultdict(list)
    for r in regions:
        planets[(r['system'], r['sys_idx'], r['planet'])].append(r)

    # Resolve --protect species
    protect_race1 = None
    if protect:
        if protect not in all_species_by_name:
            print(f"\nUnknown species '{protect}'. Available:")
            for name in sorted(all_species_by_name):
                print(f"  {name}")
            sys.exit(1)
        protect_race1 = all_species_by_name[protect]
        if protect_race1 == target_race1:
            print(f"Cannot protect {target_name} from themselves!")
            sys.exit(1)

    # Find target species to replace
    patches = []
    if planet_filters:
        for key, regs in planets.items():
            sys_name, sys_idx, p_idx = key
            if mine_only and sys_idx not in player_systems:
                continue
            pn = planet_name(sys_name, p_idx).lower()
            if any(f in pn for f in planet_filters):
                patches.extend(r for r in regs if r['race1'] == target_race1)
    elif purge_all:
        if mine_only:
            patches = [r for r in regions
                       if r['race1'] == target_race1 and r['sys_idx'] in player_systems]
        else:
            patches = [r for r in regions if r['race1'] == target_race1]
    else:
        for key, regs in planets.items():
            sys_name, sys_idx, p_idx = key
            if mine_only and sys_idx not in player_systems:
                continue
            has_target = any(r['race1'] == target_race1 for r in regs)
            if protect_race1 is not None:
                has_protected = any(r['race1'] == protect_race1 for r in regs)
            else:
                has_protected = any(r['race1'] != target_race1 for r in regs)
            if has_target and has_protected:
                patches.extend(r for r in regs if r['race1'] == target_race1)

    if not patches:
        if purge_all:
            print(f"\nNo {target_name} found anywhere!")
        else:
            print(f"\nNo {target_name} found to replace. Nothing to do!")
        if sys.platform == 'win32':
            input("\nPress Enter to exit...")
        sys.exit(0)

    # Determine replacement species
    if replace_with:
        if replace_with not in all_species_by_name:
            print(f"\nUnknown species '{replace_with}'. Available:")
            for name in sorted(all_species_by_name):
                print(f"  {name}")
            sys.exit(1)
        new_race1 = all_species_by_name[replace_with]
        new_name = SPECIES[new_race1]
    elif sys.stdin.isatty():
        new_race1, new_name = choose_replacement()
    else:
        new_race1 = KLACKON_RACE1
        new_name = "Klackon"

    scope_parts = []
    if planet_filters:
        scope_parts.append("on " + ", ".join(f'"{f}"' for f in planet_filters))
    elif purge_all:
        scope_parts.append("galaxy-wide")
    else:
        scope_parts.append("on shared planets")
    if mine_only:
        scope_parts.append("(your systems only)")
    scope = " ".join(scope_parts)
    action = "Would convert" if dry_run else "Converting"
    print(f"\n{action} {len(patches)} {target_name} regions to {new_name} {scope}:\n")

    total_pop = 0
    patched = 0
    for p in sorted(patches, key=lambda x: (x['system'], x['planet'], x['region'])):
        pn = planet_name(p['system'], p['planet'])
        r1_off = p['offset'] + 10
        r2_off = p['offset'] + 11
        cur_r1 = data[r1_off]

        if cur_r1 != target_race1:
            print(f"  SKIP {pn} R{p['region']}: race1 is {cur_r1}, not {target_race1}")
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
    print(f"\nDone! Converted {patched} {target_name} regions ({total_pop:.2f} pop) to {new_name}.")

    if sys.platform == 'win32':
        input("\nPress Enter to exit...")


if __name__ == "__main__":
    main()
