# Region and planet record layouts (via Bhruic's editor v0.51)

Recovered 2026-08 by disassembling Bhruic's Master of Orion 3 Savegame
Editor v0.51 (the only save editor ever built for MOO3; still downloadable
at `http://www.orionsector.com/pages/moo3/downloads/bhruic/Moo3Editv0.51.zip`).
The layouts below were read out of its parser and independently confirmed
against its serializer, which writes the identical byte sequence.

## Region record (offsets from record start)

| Offset | Size | Type | Field |
|---|---|---|---|
| +0 | 1 | — | unknown, untouched |
| +1 | 1 | u8 | **Owner** (empire id) |
| +2 | 8 | fixed-point | **Population** |
| +10 | 1 | u8 | **race1** (species) |
| +11 | 1 | u8 | **race2** (sub-race/magnate) |
| +12 | 1 | — | unknown, untouched |
| +13 | 8 | fixed-point | unmapped #1 |
| +21 | 1 | u8 | **Terrain** (index into terrain table) |
| +22 | 4 | i32 BE | **Base ecosystem** (≈ −2…+2) |
| +26 | 4 | i32 BE | **Modified ecosystem** (base + delta, clamped ≤ 2) |
| +30 | 8 | fixed-point | unmapped #2 |
| +38 | 8 | fixed-point | unmapped #3 |
| +46 | 8 | fixed-point | unmapped #4 |
| +54 | 1 | u8 | flag |
| +55… | var | | sub-records (specials etc.) |

Unmapped #1–#4 are candidates for morale/food/infrastructure — the dev
cheat table names engine fields `MdUnrest`, `mPopGrow`, `ManufEff`.

Bhruic's ecosystem rule: when editing base, set modified =
`clamp(base + old_delta, ≤ 2)`.

## Planet post-region block

After the last region record (`P` = end of regions):

```
P+0        1 byte
P+1        u32 BE len, ASCII string A
           u32 BE len, ASCII string B
Q = here
Q+1        u8
Q+11       u8            (integer stat)
Q+12       u8            (integer stat)
Q+13       fixed-point   (Biodiversity or Mineral Richness)
Q+21       fixed-point   (the other of the two)
Q+29       u8            (Gravity/Temperature/Atmosphere enum)
Q+30       u8            (enum)
Q+31       u8            (enum)
Q+35       u32 BE len N
Q+39       N UTF-16BE code units = Planet Name
```

## Header

| Offset | Size | Type | Field |
|---|---|---|---|
| 0x0 | 8 | | `VS3RDAEH` magic |
| 0xD | 4 | u32 BE | **Turn number** (verified against turn-115 save and the 180–184 autosave series) |

## Game settings (`VSRGMMAG` / GAMMGRSV)

Cursor at marker+13: skip 1; u16 BE; u16 BE; skip 6×4; skip 8; skip 1;
then two settings bytes (difficulty / victory-condition flags — needs a
confirming diff).

## Treasury (not yet located in current-format saves)

Facts, from Bhruic's FinanceWraparound patch and period cheat recipes:

- Signed 32-bit integer, big-endian, capped at 2,147,483,647.
- Pre-patch saves: at empire-name string offset + 27. Post-patch (our
  format): shifted; recipe no longer lands.
- Fallback search space: five consecutive undecoded 8-byte fixed-point
  slots at player-record offset +17…+56 (relative per-player in PLAYERSV),
  which Bhruic parsed past but never mapped.

## Ship records (unimplemented)

Self-describing tag stream: `tShipEmt ShipItem ShipWarp ShpMItem ShEngine
ShWepSys ShldTech FtrChass`, terminated by `KARSIMAT` ("TAMISRAK"
reversed). Fully mapped by Bhruic's Ship Editor.

## Non-save cheat routes (for the README/FAQ someday)

- `RandomEvents.txt` accepts the dev cheat modifiers verbatim
  (`mAddGold+=100000000`, `MdUnrest*=0`, `mPopGrow*=3`, …) as high-weight
  events — the never-loaded `Spreadsheets/CheatCodes.txt` documents them.
- `Moo3Settings.ini` dev keys: `disableFogOfWar`, `showAllFleets`,
  `UIcivID`, `TurnsPerTurn`, `disableMilitaryMaint`, `useEditMode`, …
