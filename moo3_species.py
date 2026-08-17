#!/usr/bin/env python3
"""MOO3 Save Editor — scan species populations.

Reports all species populations galaxy-wide, with emphasis on planets where
Ithkul (Harvesters) cohabit with other species (bioharvesting risk).

Usage:
    python3 moo3_species.py                    # auto-detect latest save
    python3 moo3_species.py path/to/save.gam   # specific save file
"""
import sys
from collections import defaultdict
from pathlib import Path
from moo3save import (
    ITHKUL_RACE1, KLACKON_RACE1, SPECIES,
    find_latest_save, parse_galaxy, parse_player_systems, planet_name,
)


def main():
    if len(sys.argv) > 1:
        save_path = Path(sys.argv[1])
    else:
        save_path = find_latest_save()
        if not save_path:
            print("Could not find MOO3 save files. Pass the path as an argument:")
            print("  python3 moo3_scan.py /path/to/savefile.gam")
            sys.exit(1)

    print(f"File: {save_path}")
    data = save_path.read_bytes()
    print(f"Size: {len(data):,} bytes")

    try:
        regions, system_count = parse_galaxy(data)
    except Exception as e:
        print(f"Parse error: {e}")
        sys.exit(1)

    print(f"Parsed {system_count} systems, {len(regions)} populated regions")

    player_systems = parse_player_systems(data)
    if player_systems:
        print(f"Player owns {len(player_systems)} systems")
    print()

    # Group by planet
    planets = defaultdict(list)
    for r in regions:
        planets[(r['system'], r['sys_idx'], r['planet'])].append(r)

    # Ithkul report per system
    ithkul_by_sys = defaultdict(lambda: defaultdict(list))
    for r in regions:
        if r['race1'] == ITHKUL_RACE1:
            ithkul_by_sys[r['system']][r['planet']].append(r)

    print(f"{'=' * 70}")
    print(f"ITHKUL INFESTATION REPORT - {len(ithkul_by_sys)} systems")
    print(f"{'=' * 70}")

    grand_pop = 0
    grand_regions = 0

    for sys_name in sorted(ithkul_by_sys, key=lambda s: -sum(
            r['pop'] for pl in ithkul_by_sys[s].values() for r in pl)):
        pmap = ithkul_by_sys[sys_name]
        spop = sum(r['pop'] for pl in pmap.values() for r in pl)
        sregs = sum(len(pl) for pl in pmap.values())
        grand_pop += spop
        grand_regions += sregs
        # Check if any region in this system is player-owned
        sys_idx = next(r['sys_idx'] for pl in pmap.values() for r in pl)
        mine = " [YOURS]" if player_systems and sys_idx in player_systems else ""
        print(f"\n  {sys_name}{mine}: {spop:.1f} pop in {sregs} regions on {len(pmap)} planet(s)")
        for p_idx in sorted(pmap):
            regs = pmap[p_idx]
            pn = planet_name(sys_name, p_idx)
            pp = sum(r['pop'] for r in regs)
            print(f"    {pn}: {pp:.2f} in {len(regs)} region(s)")

    print(f"\n{'=' * 70}")
    print(f"TOTALS: {grand_pop:.1f} Ithkul in {grand_regions} regions across {len(ithkul_by_sys)} systems")
    if player_systems:
        player_ithkul = {s for s in ithkul_by_sys if any(
            r['sys_idx'] in player_systems for pl in ithkul_by_sys[s].values() for r in pl)}
        print(f"  YOUR systems with Ithkul: {len(player_ithkul)}")
    print(f"{'=' * 70}")

    # Find player's planets with Ithkul (any species cohabiting with Ithkul)
    # We detect the player by looking for Klackon (Tachidi) but also report any
    # planet where Ithkul share space with non-Ithkul populations.
    print(f"\n{'=' * 70}")
    print(f"PLANETS WITH ITHKUL + OTHER SPECIES (at risk of bioharvesting)")
    print(f"{'=' * 70}")

    at_risk = []
    for key, regs in planets.items():
        has_ithkul = any(r['race1'] == ITHKUL_RACE1 for r in regs)
        has_other = any(r['race1'] != ITHKUL_RACE1 for r in regs)
        if has_ithkul and has_other:
            at_risk.append((key, regs))

    if at_risk:
        for key, regs in sorted(at_risk, key=lambda x: -sum(
                r['pop'] for r in x[1] if r['race1'] == ITHKUL_RACE1)):
            sys_name, _, p_idx = key
            ith = [r for r in regs if r['race1'] == ITHKUL_RACE1]
            others = [r for r in regs if r['race1'] != ITHKUL_RACE1]
            pn = planet_name(sys_name, p_idx)
            other_species = {SPECIES.get(r['race1'], f"race1={r['race1']}") for r in others}
            print(f"\n  {pn}: Ithkul {sum(r['pop'] for r in ith):.2f} vs "
                  f"{', '.join(other_species)} {sum(r['pop'] for r in others):.2f}")
            for r in sorted(regs, key=lambda x: x['region']):
                sp = SPECIES.get(r['race1'], f"race1={r['race1']}")
                marker = " <-- ITHKUL" if r['race1'] == ITHKUL_RACE1 else ""
                print(f"    Region {r['region']}: {sp:10s} pop={r['pop']:.4f}{marker}")
    else:
        print("  None found - your planets are safe!")


if __name__ == "__main__":
    main()
