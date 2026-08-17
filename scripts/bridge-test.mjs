// Exercises the WASM bridge in Node against the fixture save. Build first:
//   wasm-pack build moo3-save-web --target nodejs --out-dir ../target/wasm-node-test
import { createRequire } from "node:module";
import fs from "node:fs";
import zlib from "node:zlib";

const require = createRequire(import.meta.url);
const { summarize, plan_replace, apply_replace } =
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

const before = summary.species.find((s) => s.name === "Klackon")?.pop ?? 0;
const edited = apply_replace(save, options({}));
const after = JSON.parse(summarize(Buffer.from(edited)));
assert(!after.species.some((s) => s.name === "Ithkul"), "no Ithkul remain");
const klackon = after.species.find((s) => s.name === "Klackon");
assert(klackon.pop > before, "Klackon population grew");
assert(after.regions === summary.regions, "region count unchanged");

console.log("bridge tests passed");
