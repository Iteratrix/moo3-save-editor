# moo3-save

**Use it here: <https://iteratrix.github.io/moo3-save-editor/>**

Save editor for [Master of Orion 3](https://store.steampowered.com/app/1148100/Master_of_Orion_3/)
(Quicksilver, 2003). Scan your galaxy's species populations and replace any
species with any other — per planet, per scope, or galaxy-wide — directly in
your browser. Nothing is uploaded anywhere; all parsing runs locally as
WebAssembly.

The format was reverse-engineered from Bhruic's long-dead MOO3 Save Editor
and the game binary, and is verified against real saves by a roundtrip
battery.

## The Ithkul problem

Ithkul are MOO3's Harvester species: when they share a planet with anyone
else, they bioharvest (eat) the other populations. This is hardcoded — no
diplomacy, game setting, or targeted action removes them once they arrive
via FLU Generator specials, conquest, or migration.

The editor's defaults target exactly this: find Ithkul on planets shared
with other species and turn them into someone harmless.

## Using it

Open the web editor, drop your `.gam` file on it, pick target, replacement,
and scope, preview, apply. On Chromium browsers the editor writes straight
back to the file after a permission prompt; elsewhere it downloads the
edited file for you to move back into the save folder.

Save locations (autosaves in `AutoSaveHistory/` inside each):

| Platform | Path |
|---|---|
| Windows (Steam) | `C:\Program Files (x86)\Steam\steamapps\common\Master of Orion 3\SaveGameFiles\` |
| Linux (Steam) | `~/.steam/steam/steamapps/common/Master of Orion 3/SaveGameFiles/` |
| macOS (Steam) | `~/Library/Application Support/Steam/steamapps/common/Master of Orion 3/SaveGameFiles/` |

Close the game before editing and keep the backup the page offers.

## What it can edit

- **Any species into any other**, with the sub-race reset so the game
  rebuilds it from defaults. Population sizes, buildings, and everything
  else stay untouched.
- **Scopes**: shared planets only (the anti-Ithkul default, optionally only
  where a specific species needs protecting), named planets (partial match —
  `Alrisha` covers the whole system), or galaxy-wide.
- **Your systems only**: player ownership is auto-detected from the save.
- **Region fields** (planet inspector): population per region, plus owner,
  terrain, and ecosystem via the CLI. The layout was verified against both
  the parser and the serializer of Bhruic's 2003 editor
  (`docs/region-format.md`).
- **The turn counter.**

## CLI

The same core ships as a command-line tool:

```
cargo run -p moo3-save-cli --release -- scan                # report, newest save auto-detected
cargo run -p moo3-save-cli --release -- replace --dry-run   # preview the default Ithkul purge
cargo run -p moo3-save-cli --release -- replace --target klackon --replace-with human --planet "Psi Tauri I"
cargo run -p moo3-save-cli --release -- replace --mine --all --replace-with darlok
cargo run -p moo3-save-cli --release -- planet "Alrisha"    # inspect regions (owner, pop, terrain, eco)
cargo run -p moo3-save-cli --release -- edit --planet "Alrisha I" --region 0 --pop 5.5 --eco 2
cargo run -p moo3-save-cli --release -- turn --set 100      # rewind the turn counter
cargo run -p moo3-save-cli --release -- corpus <dir>        # verification battery over a save folder
```

Commands that write create a `.bak` backup first.

## Format notes (for the curious)

A `.gam` save is one big-endian binary blob: UTF-16BE strings, a custom
8-byte fixed-point number (6-byte signed integer + 1/65536 fraction), and
section markers stored as reversed ASCII — `VS3RDAEH` is "HEADR3SV",
`VSYXALAG` is "GALAXYSV". There are no checksums and almost no
self-describing lengths, so the parser walks every system, planet, and
population region structurally; a single mis-skip desyncs everything after
it, which is what the verify battery exists to catch.

Module docs in `moo3-save-core` carry the details:

- `galaxy`: systems, orbit slots, population regions, and the two-byte
  `race1`/`race2` edit surface
- `species`: the `race1` species table from the game data
- `empire`: the empire table and the ownership-list heuristic behind `--mine`
- `replace`: plan/apply split and the stale-plan guard
- `verify`: parse, edit, re-parse, assert-nothing-else-changed

## Development

```
cargo test                              # unit + fixture tests
MOO3_CORPUS_DIR=<your saves dir> cargo test    # full corpus verification
cargo run -p moo3-save-cli -- corpus <dir>     # same checks, readable report
wasm-pack build moo3-save-web --target web --out-dir ../web/pkg
python3 -m http.server -d web           # dev server; SW skipped on localhost
```

`test-data/` contains one gzipped fixture save. The verification battery
(`moo3-save-core/src/verify.rs`) parses, edits, and re-parses every save it
is given and asserts nothing else changed; `scripts/bridge-test.mjs`
exercises the WASM boundary in Node.

Not affiliated with Quicksilver Software, Infogrames, or Atari. Back up
your saves; use at your own risk.

## License

MIT
