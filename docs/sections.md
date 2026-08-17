# Save file section map

Every top-level section is fenced by a marker stored as reversed ASCII.
Offsets below are from `The Synthesis Turn 0115.gam` (6,845,293 bytes) and
shift between saves; find markers by search, never by offset.

| Bytes in file | Reversed (meaning) | First offset | Notes |
|---|---|---|---|
| `VS3RDAEH` | `HEADR3SV` | 0x0 | file header |
| `VSEICEPS` | `SPECIESV` | 0x248 | ends the empire table (`ECAR` records) |
| `VSGREWOP` | `POWERGSV` | 0x6181 | power graph history? |
| `VSRGMMAG` | `GAMMGRSV` | 0x1B1AA | game manager (turn state?) |
| `VSYXALAG` | `GALAXYSV` | 0x1B1DE | galaxy: systems, planets, regions — parsed by `galaxy.rs` |
| `VSREYALP` | `PLAYERSV` | 0x288307 | per-empire records; ownership lists live here (`empire.rs`); likely treasury/economy too |
| `VSONHCET` | `TECHNOSV` | 0x5C56EB | technology state |
| `VSLRNGRF` | `FRGNRLSV` | 0x66DB24 | foreign relations / diplomacy |
| `VSNESNRO` | `ORNSENSV` | 0x672124 | Orion Senate |
| `VSTSHRDL` | `LDRHSTSV` | 0x6724D1 | leader history |
| `VSXRATNA` | `ANTARXSV` | 0x672609 | Antaran expedition ("Antaran X") |
| `VSSTNEVE` | `EVENTSSV` | 0x672671 | events |
| `VSCNYSON` | `NOSYNCSV` | 0x672829 | non-synced state (bulk of the file in autosaves?) |
| `VSRETSAM` | `MASTERSV` | 0x68503D | master record |
| `VSPERTIS` | `SITREPSV` | 0x685241 | situation reports |

Planetary specials inside the galaxy section use reversed 8-char `Sp*` tags
(`neGULFpS` = `SpFLUGen`); see `special.rs`.

Candidate expansion targets, pending format work: treasury and research
allocation (PLAYERSV), known techs (TECHNOSV), diplomacy states (FRGNRLSV),
Senate votes (ORNSENSV), leaders (LDRHSTSV).
