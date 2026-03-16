"""MOO3 binary save file parser.

Reverse-engineered from Bhruic's MOO3 Save Editor v0.51 and the game binary.

Format notes:
- Big-endian integers throughout
- Header magic: VS3RDAEH ("HEADERS3V" reversed)
- Galaxy marker: VSYXALAG ("GALAXYSV" reversed)
- Custom "double": 6-byte BE signed integer + 2-byte BE uint16 fraction
- System names: UTF-16BE
- Special record tags stored reversed (e.g. "SpFLUGen" stored as "neGULFpS")
- Planet slot types: H (0x48) = 31 extra bytes, L (0x4C) and O (0x4F) = 30
- Region race fields: race1 (species type) at offset+10, race2 (sub-race) at offset+11
"""
from collections import defaultdict
from pathlib import Path
import platform

# Species type IDs (race1 field)
SPECIES = {
    0: "Human",
    1: "Sakkra",
    2: "Meklar",
    3: "Silicoid",
    4: "Psilon",
    5: "Ithkul",     # Harvesters - eat other populations
    6: "Klackon",     # Includes Tachidi sub-races
    7: "Raas",
    8: "Nommo",
    9: "Grendarl",
    10: "Cynoid",
    11: "Imsaeis",
    12: "Eoladi",
    13: "Geodic",
}

ITHKUL_RACE1 = 5
KLACKON_RACE1 = 6  # Tachidi are a Klackon sub-race


def find_save_dirs():
    """Find common MOO3 save directories across platforms."""
    candidates = []

    home = Path.home()
    system = platform.system()

    if system == "Linux":
        # Steam on Linux (native and flatpak)
        for steam_root in [
            home / ".steam" / "debian-installation",
            home / ".steam" / "steam",
            home / ".local" / "share" / "Steam",
            home / ".var" / "app" / "com.valvesoftware.Steam" / ".local" / "share" / "Steam",
        ]:
            candidates.append(steam_root / "steamapps" / "common" / "Master of Orion 3" / "SaveGameFiles")
    elif system == "Windows":
        # Default Steam install
        for drive in ["C", "D", "E"]:
            candidates.append(Path(f"{drive}:\\Program Files (x86)\\Steam\\steamapps\\common\\Master of Orion 3\\SaveGameFiles"))
            candidates.append(Path(f"{drive}:\\Program Files\\Steam\\steamapps\\common\\Master of Orion 3\\SaveGameFiles"))
        # GOG
        candidates.append(Path("C:\\GOG Games\\Master of Orion 3\\SaveGameFiles"))
    elif system == "Darwin":
        candidates.append(home / "Library" / "Application Support" / "Steam" / "steamapps" / "common" / "Master of Orion 3" / "SaveGameFiles")

    return [p for p in candidates if p.exists()]


def find_latest_save(save_dir=None):
    """Find the most recent .gam save file."""
    if save_dir:
        dirs = [Path(save_dir)]
    else:
        dirs = find_save_dirs()

    if not dirs:
        return None

    newest = None
    newest_mtime = 0

    for d in dirs:
        for pattern in ["AutoSaveHistory/*.gam", "*.gam"]:
            for gam in d.glob(pattern):
                if gam.stat().st_mtime > newest_mtime:
                    newest = gam
                    newest_mtime = gam.stat().st_mtime

    return newest


# ============================================================
# Binary format primitives
# ============================================================

def read_u32(data, offset):
    return (data[offset] << 24) | (data[offset+1] << 16) | (data[offset+2] << 8) | data[offset+3]


def read_utf16be(data, offset, nchars):
    return data[offset:offset + nchars * 2].decode('utf-16-be', errors='replace')


def read_double(data, offset):
    """MOO3's custom fixed-point: 6-byte BE signed int + 2-byte BE uint16 fraction."""
    int_part = int.from_bytes(data[offset:offset+6], 'big', signed=True)
    frac = int.from_bytes(data[offset+6:offset+8], 'big', signed=False)
    return int_part + frac / 65536.0


# ============================================================
# Save format structure parsers
# ============================================================

def read_special_sub(data, pos):
    pos += 8 + 8 + 1 + 1 + 4 + 4 + 4
    count1 = data[pos]; pos += 1
    pos += count1 * 17
    count2 = data[pos]; pos += 1
    pos += count2 * 17
    return pos


def read_special_record(data, pos):
    tag = bytes(reversed(data[pos:pos+8])).decode('ascii', errors='replace')
    pos += 8

    EXTRA_BYTES = {
        "SpGenerc": (1, 0), "SpTerfrm": (2, 8), "SpDeplet": (2, 4),
        "SpPrtDep": (2, 4), "SpAbnCol": (2, 0), "SpSplCol": (2, 12),
        "SpFLUGen": (2, 12), " SpEvent": (2, 16), " SpRuins": (2, 8),
        "SpAntarX": (2, 1),
    }

    if tag == "SpGuardn":
        pos += 2; pos = read_special_sub(data, pos); pos += 1
        vlen = read_u32(data, pos); pos += 4; pos += vlen
    elif tag in EXTRA_BYTES:
        pre, post = EXTRA_BYTES[tag]
        pos += pre; pos = read_special_sub(data, pos); pos += post
    else:
        pos += 2; pos = read_special_sub(data, pos)

    return pos


def read_region(data, pos):
    """Parse a population region. Returns (new_pos, race1, race2, pop)."""
    pos += 1 + 1
    pop = read_double(data, pos); pos += 8
    race1 = data[pos]; pos += 1
    race2 = data[pos]; pos += 1
    pos += 1 + 8 + 1 + 4 + 4 + 8 + 8 + 8

    count1 = data[pos]; pos += 1
    for _ in range(count1):
        pos = read_special_record(data, pos)

    count2 = data[pos]; pos += 1
    for _ in range(count2):
        pos += 1 + 8
        ic = data[pos]; pos += 1
        pos += ic * 17
        pos += 7

    for i in range(3):
        if data[pos] != i:
            continue
        pos += 1
        switch_val = read_u32(data, pos); pos += 4
        pos += 2 + 1 + 1
        sc = data[pos]; pos += 1
        for _ in range(sc):
            pos += 1 + 8
            ic2 = data[pos]; pos += 1
            pos += ic2 * 17
            pos += 7
        ac = data[pos]; pos += 1
        for _ in range(ac):
            tb = data[pos]; pos += 1
            if tb == 0x4F:
                pos += 1 + 4 + 1 + 1
        pos += 8 + 1
        sv = switch_val - 1
        if sv in (0, 1, 3, 4, 5): pos += 8
        elif sv == 2: pos += 0x30
        elif sv == 6: pos += 0x10
        elif sv == 7: pos += 0x28

    pos += 1
    return pos, race1, race2, pop


def read_typed_field(data, pos):
    ft = data[pos]; pos += 1
    if ft == 0:
        pos += 1 + 4
    elif ft in (1, 2):
        pass
    elif ft in (3, 4):
        pos += 1 + 4
        sc = data[pos]; pos += 1
        for _ in range(sc):
            st = data[pos]; pos += 1
            if st == 7:
                pos += 1 + 4
    return pos


def read_post_region(data, pos):
    """Parse the post-region portion of planet data."""
    pos += 1
    slen1 = read_u32(data, pos); pos += 4; pos += slen1
    slen2 = read_u32(data, pos); pos += 4; pos += slen2
    pos += 1 + 1 + 1 + 8 + 1 + 1 + 8 + 8 + 1 + 1 + 1 + 2 + 1
    wlen = read_u32(data, pos); pos += 4; pos += wlen * 2
    sp_count = data[pos]; pos += 1
    for _ in range(sp_count):
        pos = read_special_record(data, pos)
    pos += 8 + 8 + 2
    tf_count = data[pos]; pos += 1
    for _ in range(tf_count):
        pos = read_typed_field(data, pos)
    big_flag = data[pos]; pos += 1
    if big_flag > 0:
        pos += 1
        cnt1 = data[pos]; pos += 1; pos += cnt1 * 42
        pos += 1 + 4 + 4 + 1 + 4                     # 14 bytes after array 1
        cnt2 = data[pos]; pos += 1; pos += cnt2 * 42
        pos += 5 + 1 + 4                              # 10 bytes after array 2
        cnt3 = data[pos]; pos += 1; pos += cnt3 * 42
        pos += 5 + 1 + 4                              # 10 bytes after array 3
        cnt4 = data[pos]; pos += 1; pos += cnt4 * 42
        pos += 5                                       # 5 bytes after array 4
        cnt5 = data[pos]; pos += 1; pos += cnt5 * 9
        pos += 2 + 1 + 1 + 8 + 4 + 4
        pos += 7 * 2 + 7 * 2 + 5
        tg = data[pos]; pos += 1
        for _ in range(tg):
            pos += 1 + 8
            sc = data[pos]; pos += 1; pos += sc * 17
            pos += 1 + 1 + 1 + 4
        pos += 9 + 1 + 0x38
        for _ in range(8): pos += 4
        pos += 0x80
        for _ in range(8): pos += 4
        pos += 0x3C
        for _ in range(5):
            ec = read_u32(data, pos); pos += 4; pos += ec * 12
    pos += 0x17
    qc = data[pos]; pos += 1; pos += qc * 8
    pos += 5
    while True:
        s = data[pos]; pos += 1
        if s == 0xFF: break
        pos += 8
    bc = read_u32(data, pos); pos += 4; pos += bc
    pos += 1
    fc = data[pos]; pos += 1
    for _ in range(fc):
        pos += 1
        cl_a = read_u32(data, pos); pos += 4; pos += cl_a
        cl_b = read_u32(data, pos); pos += 4; pos += cl_b
        pos += 1 + 1 + 1 + 8 + 1 + 1 + 8 + 8 + 1 + 1 + 1 + 2 + 1
        ssc = data[pos]; pos += 1; pos += ssc * 6
    return pos


def read_system_header(data, pos):
    start = pos
    flag = data[pos]; pos += 1
    pos += 2
    name_len = read_u32(data, pos); pos += 4
    if name_len > 200 or name_len == 0:
        raise ValueError(f"Bad name_len={name_len} at 0x{start:X}")
    name = read_utf16be(data, pos, name_len); pos += name_len * 2
    pos += 2 + 8 + 8 + 8 + 8
    slot_count = data[pos]; pos += 1
    for _ in range(slot_count):
        ptype = data[pos]; pos += 1
        if ptype == 0x48:       # H: habitable
            pos += 31
        elif ptype in (0x4C, 0x4F):  # L: lifeless, O: other
            pos += 30
        # 0xFF = empty orbit, no extra data
    pos += 2
    sl_count = data[pos]; pos += 1; pos += sl_count
    pos += 2
    sl_data = data[pos]; pos += 1; pos += sl_data * 8
    pos += 3
    b1 = read_u32(data, pos); pos += 4; pos += b1
    b2 = read_u32(data, pos); pos += 4; pos += b2
    pos += 0x20
    return pos, name, flag


# ============================================================
# High-level API
# ============================================================

def parse_galaxy(data):
    """Parse all systems and return list of populated region records.

    Each record: {system, sys_idx, planet, region, pop, race1, race2, offset}
    """
    marker = data.find(b'VSYXALAG')
    if marker < 0:
        raise ValueError("Galaxy marker VSYXALAG not found - not a valid MOO3 save")

    pos = marker + 8 + 4 + 1 + 60
    system_count = data[pos]; pos += 1

    regions = []

    for sys_idx in range(system_count):
        new_pos, name, flag = read_system_header(data, pos)
        pos = new_pos

        if flag in (0x4E, 0x42):  # Neutron star / Black hole
            continue

        planet_count = data[pos]; pos += 1
        for p_idx in range(planet_count):
            region_count = data[pos]; pos += 1
            for r_idx in range(region_count):
                reg_start = pos
                new_pos, race1, race2, pop = read_region(data, pos)
                if pop > 0:
                    regions.append({
                        'system': name, 'sys_idx': sys_idx,
                        'planet': p_idx, 'region': r_idx,
                        'pop': pop, 'race1': race1, 'race2': race2,
                        'offset': reg_start,
                    })
                pos = new_pos
            pos = read_post_region(data, pos)

    return regions, system_count


def roman(n):
    vals = [(10, 'X'), (9, 'IX'), (5, 'V'), (4, 'IV'), (1, 'I')]
    r = ''
    for v, s in vals:
        while n >= v:
            r += s; n -= v
    return r


def planet_name(system, planet_idx):
    return f"{system} {roman(planet_idx + 1)}"
