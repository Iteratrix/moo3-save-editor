import init, { summarize, plan_replace, apply_replace } from "./pkg/moo3_save_web.js";

// Start loading the WASM immediately; loadFile awaits this so a file
// dropped before the module finishes initializing still works.
const ready = init();

const el = (id) => document.getElementById(id);
const dropZone = el("drop-zone");
const editor = el("editor");
const statusLine = el("status");

const hasFsAccess = "showOpenFilePicker" in window;
el("fs-hint").textContent = hasFsAccess
  ? "Your browser can save directly back to the file after you grant permission."
  : "Your browser will download the edited file; move it back into your save folder.";

let state = null;

function setStatus(message, kind = "") {
  statusLine.textContent = message;
  statusLine.className = kind;
}

function fillSpeciesSelect(select, names, selected) {
  select.replaceChildren();
  for (const name of names) {
    const option = document.createElement("option");
    option.value = name;
    option.textContent = name;
    option.selected = name === selected;
    select.append(option);
  }
}

function render() {
  const { summary } = state;
  el("file-name").textContent = state.fileName;
  const owned = summary.player_systems > 0
    ? ` · you own ${summary.player_systems} systems`
    : "";
  el("file-meta").textContent =
    `${summary.systems} systems · ${summary.regions} populated regions · ${summary.empires.length} empires${owned}`;

  const rows = el("species-table").querySelector("tbody");
  rows.replaceChildren();
  for (const { name, pop, regions, systems } of summary.species) {
    const row = document.createElement("tr");
    if (name === "Ithkul") row.className = "ithkul";
    for (const value of [name, pop.toFixed(1), String(regions), String(systems)]) {
      const cell = document.createElement("td");
      cell.textContent = value;
      row.append(cell);
    }
    row.querySelectorAll("td:not(:first-child)").forEach((cell) => cell.classList.add("num"));
    rows.append(row);
  }

  const present = summary.species.map((entry) => entry.name);
  const target = present.includes("Ithkul") ? "Ithkul" : present[0];
  fillSpeciesSelect(el("target"), present, target);
  fillSpeciesSelect(el("replacement"), summary.known_species, "Klackon");
  const protect = el("protect");
  protect.replaceChildren();
  const anyone = document.createElement("option");
  anyone.value = "";
  anyone.textContent = "anyone";
  protect.append(anyone);
  for (const name of present) {
    const option = document.createElement("option");
    option.value = name;
    option.textContent = name;
    protect.append(option);
  }

  el("mine-row").hidden = summary.player_systems === 0;
  el("mine").checked = false;
  el("plan-result").hidden = true;
  el("save-btn").textContent = state.handle ? "Apply & save to file" : "Apply & download";
  el("save-btn").disabled = false;
  editor.hidden = false;
}

function buildOptions() {
  const scope = document.querySelector("input[name=scope]:checked").value;
  return JSON.stringify({
    target: el("target").value,
    replacement: el("replacement").value,
    scope,
    planets: el("planets").value.split(",").map((name) => name.trim()).filter(Boolean),
    protect: el("protect").value || null,
    mine: el("mine").checked && !el("mine-row").hidden,
  });
}

function showPlan() {
  let plan;
  try {
    plan = JSON.parse(plan_replace(state.originalBytes, buildOptions()));
  } catch (error) {
    setStatus(`Preview failed: ${error}`, "error");
    return null;
  }
  setStatus("");
  const result = el("plan-result");
  result.hidden = false;
  el("plan-summary").textContent = plan.count === 0
    ? `No ${el("target").value} regions match this scope.`
    : `${plan.count} regions (${plan.pop.toFixed(2)} pop) would become ${el("replacement").value}:`;
  const list = el("plan-list");
  list.replaceChildren();
  for (const { planet, region, pop } of plan.regions) {
    const item = document.createElement("li");
    item.textContent = `${planet} R${region}: pop=${pop.toFixed(4)}`;
    list.append(item);
  }
  return plan;
}

function download(bytes, name) {
  const blob = new Blob([bytes], { type: "application/octet-stream" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = name;
  anchor.click();
  URL.revokeObjectURL(url);
}

async function save() {
  const plan = showPlan();
  if (!plan || plan.count === 0) return;
  let edited;
  try {
    edited = apply_replace(state.originalBytes, buildOptions());
  } catch (error) {
    setStatus(`Edit failed: ${error}`, "error");
    return;
  }
  if (state.handle) {
    try {
      const writable = await state.handle.createWritable();
      await writable.write(edited);
      await writable.close();
      setStatus(`Converted ${plan.count} regions and saved back to the original file. Keep the backup!`, "ok");
    } catch (error) {
      setStatus(`Could not write file (${error}); downloading instead.`, "error");
      download(edited, state.fileName);
    }
  } else {
    download(edited, state.fileName);
    setStatus(`Converted ${plan.count} regions. Replace the original file in your save folder (keep a backup).`, "ok");
  }
  await loadFile(new File([edited], state.fileName), state.handle);
}

async function loadFile(file, handle) {
  await ready;
  const bytes = new Uint8Array(await file.arrayBuffer());
  let summary;
  try {
    summary = JSON.parse(summarize(bytes));
  } catch (error) {
    setStatus(`Could not read this file as a MOO3 save: ${error}`, "error");
    return;
  }
  state = {
    fileName: file.name,
    originalBytes: bytes,
    handle: handle ?? null,
    summary,
  };
  setStatus("");
  render();
}

async function openViaPicker() {
  if (hasFsAccess) {
    try {
      const [handle] = await window.showOpenFilePicker({
        types: [{ description: "MOO3 save", accept: { "application/octet-stream": [".gam"] } }],
      });
      await loadFile(await handle.getFile(), handle);
      return;
    } catch (error) {
      if (error?.name === "AbortError") return;
    }
  }
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".gam";
  input.addEventListener("change", () => {
    if (input.files?.[0]) void loadFile(input.files[0]);
  });
  input.click();
}

dropZone.addEventListener("click", () => void openViaPicker());
dropZone.addEventListener("keydown", (event) => {
  if (event.key === "Enter" || event.key === " ") void openViaPicker();
});
dropZone.addEventListener("dragover", (event) => {
  event.preventDefault();
  dropZone.classList.add("drag");
});
dropZone.addEventListener("dragleave", () => dropZone.classList.remove("drag"));
dropZone.addEventListener("drop", async (event) => {
  event.preventDefault();
  dropZone.classList.remove("drag");
  // Capture both synchronously: the drag data store is neutered once the
  // handler yields, so anything read after an await comes back empty.
  const item = event.dataTransfer?.items?.[0];
  const file = event.dataTransfer?.files?.[0];
  if (hasFsAccess && item?.getAsFileSystemHandle) {
    try {
      const handle = await item.getAsFileSystemHandle();
      if (handle?.kind === "file") {
        await loadFile(await handle.getFile(), handle);
        return;
      }
    } catch {
      // Synthetic drops have no backing handle; fall through to File.
    }
  }
  if (file) void loadFile(file);
});

el("backup-btn").addEventListener("click", () => {
  download(state.originalBytes, state.fileName.replace(/\.gam$/i, "") + ".backup.gam");
});
el("preview-btn").addEventListener("click", () => void showPlan());
el("save-btn").addEventListener("click", () => void save());
for (const radio of document.querySelectorAll("input[name=scope]")) {
  radio.addEventListener("change", () => {
    el("plan-result").hidden = true;
  });
}

// Offline support. Skipped on localhost so dev never fights a stale cache;
// persistent storage keeps the browser from evicting the cache under pressure.
function registerServiceWorker() {
  if (!("serviceWorker" in navigator)) return;
  if (location.hostname === "localhost" || location.hostname === "127.0.0.1") return;
  navigator.serviceWorker.register("./sw.js").catch((err) => {
    console.warn("Service worker registration failed:", err);
  });
  if (navigator.storage?.persist) {
    navigator.storage.persist().catch(() => {});
  }
}

await ready;
registerServiceWorker();
