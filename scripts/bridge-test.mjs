// Exercises the WASM bridge in Node against the fixture save. Build first:
//   wasm-pack build moo3-save-web --target nodejs --out-dir ../target/wasm-node-test
import { createRequire } from "node:module";
import fs from "node:fs";
import zlib from "node:zlib";

const require = createRequire(import.meta.url);
const { summarize, plan_replace, apply_replace, planet_regions, apply_field_edits } =
  require("../target/wasm-node-test/moo3_save_web.js");

function assert(condition, message) {
  if (!condition) {
    console.error(`FAIL: ${message}`);
    process.exit(1);
  }
}

const save = zlib.gunzipSync(
  fs.readFileSync(new URL("../test-data/synthesis-turn115.gam.gz", import.meta.url)),
);

const summary = JSON.parse(summarize(save));
assert(summary.systems === 250, "250 systems");
assert(summary.regions === 2794, "2794 populated regions");
assert(summary.empires.length === 11, "11 empires");
assert(summary.player_systems === 20, "20 player systems");
assert(summary.species.some((s) => s.name === "Ithkul"), "Ithkul present");
assert(summary.known_species.length === 21, "21 known species");

const options = (extra) =>
  JSON.stringify({ target: "Ithkul", replacement: "Klackon", scope: "everywhere", ...extra });

const plan = JSON.parse(plan_replace(save, options({})));
assert(plan.count > 0, "galaxy-wide Ithkul plan is non-empty");
assert(plan.regions.length === plan.count, "plan lists every region");
assert(plan.regions[0].planet.includes(" "), "planet names include numerals");

const shared = JSON.parse(plan_replace(save, options({ scope: "shared" })));
assert(shared.count === 0, "fixture has no shared-planet Ithkul (already purged)");

let threw = false;
try {
  plan_replace(save, options({ target: "Xenomorph" }));
} catch {
  threw = true;
}
assert(threw, "unknown species rejected");

assert(summary.turn === 115, "turn 115 in fixture");
const player = summary.empires.find((e) => e.id === 1);
assert(player?.au === 64573, "player treasury 64573 at turn 115");
assert(summary.empires.every((e) => Number.isInteger(e.au)), "all treasuries readable");

const richEdited = apply_field_edits(save, JSON.stringify({ treasuries: [{ id: 1, au: 999999 }] }));
const richAfter = JSON.parse(summarize(Buffer.from(richEdited)));
assert(richAfter.empires.find((e) => e.id === 1)?.au === 999999, "treasury edit applied");
assert(
  richAfter.empires.filter((e) => e.id !== 1).every(
    (e) => e.au === summary.empires.find((o) => o.id === e.id)?.au,
  ),
  "other treasuries untouched",
);

const planets = JSON.parse(planet_regions(save, "alrisha i"));
assert(planets.length >= 4, "Alrisha I..IV match");
const region = planets[0].regions[0];
assert(region.pop > 0 && region.offset > 0, "region carries pop and offset");
assert(Number.isInteger(region.eco_base), "eco is an integer");

const fieldEdited = apply_field_edits(
  save,
  JSON.stringify({ turn: 200, pops: [{ offset: region.offset, pop: 9.9 }] }),
);
const fieldAfter = JSON.parse(summarize(Buffer.from(fieldEdited)));
assert(fieldAfter.turn === 200, "turn edit applied");
const reRegion = JSON.parse(planet_regions(Buffer.from(fieldEdited), "alrisha i"))[0].regions[0];
assert(Math.abs(reRegion.pop - 9.9) < 1e-3, "pop edit applied");
assert(fieldAfter.regions === summary.regions, "region count stable after field edits");

let fieldThrew = false;
try {
  apply_field_edits(save, JSON.stringify({ pops: [{ offset: 12345, pop: 1 }] }));
} catch {
  fieldThrew = true;
}
assert(fieldThrew, "unknown region offset rejected");

const before = summary.species.find((s) => s.name === "Klackon")?.pop ?? 0;
const edited = apply_replace(save, options({}));
const after = JSON.parse(summarize(Buffer.from(edited)));
assert(!after.species.some((s) => s.name === "Ithkul"), "no Ithkul remain");
const klackon = after.species.find((s) => s.name === "Klackon");
assert(klackon.pop > before, "Klackon population grew");
assert(after.regions === summary.regions, "region count unchanged");

console.log("bridge tests passed");
