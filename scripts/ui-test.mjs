// Headless browser test of the real web UI, driven over CDP (no deps —
// Node >= 22 for the built-in WebSocket). Serves web/, opens e2e.html
// (index.html + e2e.js, which drops the fixture save on the real app.js
// handlers), and polls document.title for the E2E verdict.
//
// Prereqs: chromium on PATH and a prior
//   wasm-pack build moo3-save-web --target web --out-dir ../web/pkg
import { spawn, execSync } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import url from "node:url";
import zlib from "node:zlib";

const webDir = path.join(path.dirname(url.fileURLToPath(import.meta.url)), "..", "web");
const PORT = 8123;
const CDP_PORT = 9222;

const fixture = zlib.gunzipSync(
  fs.readFileSync(path.join(webDir, "..", "test-data", "synthesis-turn115.gam.gz")),
);
const e2eHtml = fs
  .readFileSync(path.join(webDir, "index.html"), "utf-8")
  .replace(
    '<script type="module" src="app.js"></script>',
    '<script type="module" src="app.js"></script>\n<script type="module" src="e2e.js"></script>',
  );

const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm" };
const server = http.createServer((req, res) => {
  const name = req.url === "/" ? "/index.html" : req.url.split("?")[0];
  if (name === "/e2e.html") {
    res.writeHead(200, { "content-type": "text/html" });
    return res.end(e2eHtml);
  }
  if (name === "/e2e-fixture.gam") {
    res.writeHead(200, { "content-type": "application/octet-stream" });
    return res.end(fixture);
  }
  const file = path.join(webDir, path.normalize(name).replace(/^([.][.][/\\])+/, ""));
  if (!file.startsWith(webDir) || !fs.existsSync(file) || !fs.statSync(file).isFile()) {
    res.writeHead(404);
    return res.end();
  }
  res.writeHead(200, { "content-type": types[path.extname(file)] ?? "application/octet-stream" });
  res.end(fs.readFileSync(file));
});
await new Promise((resolve) => server.listen(PORT, resolve));

// CHROME_BIN first, then Chrome before the chromium-browser name: on
// GitHub runners `chromium-browser` is a broken snap shim while
// `google-chrome` is a real install.
const candidates = [
  process.env.CHROME_BIN,
  "google-chrome",
  "google-chrome-stable",
  "chromium",
  "chromium-browser",
].filter(Boolean);
const browserBin = candidates.find((bin) => {
  try {
    execSync(`command -v ${JSON.stringify(bin)}`, { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
});
if (!browserBin) {
  console.error("no chromium/chrome on PATH (set CHROME_BIN)");
  process.exit(1);
}
console.log(`browser: ${browserBin}`);
const profile = fs.mkdtempSync("/tmp/moo3-ui-test-");
const browser = spawn(
  browserBin,
  [
    "--headless=new",
    "--disable-gpu",
    "--no-sandbox",
    `--remote-debugging-port=${CDP_PORT}`,
    `--user-data-dir=${profile}`,
    "about:blank",
  ],
  { stdio: "ignore" },
);
browser.on("error", (error) => {
  console.error(`browser failed to start: ${error}`);
  server.close();
  process.exit(1);
});

function cleanup(code) {
  browser.kill();
  server.close();
  fs.rmSync(profile, { recursive: true, force: true });
  process.exit(code);
}

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function cdpTarget() {
  for (let i = 0; i < 50; i++) {
    try {
      const list = await fetch(`http://localhost:${CDP_PORT}/json`).then((r) => r.json());
      const page = list.find((t) => t.type === "page");
      if (page) return page.webSocketDebuggerUrl;
    } catch {
      // browser not up yet
    }
    await wait(200);
  }
  throw new Error("CDP endpoint never came up");
}

const ws = new WebSocket(await cdpTarget());
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = reject;
});
let nextId = 1;
const pending = new Map();
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.id && pending.has(msg.id)) {
    pending.get(msg.id)(msg);
    pending.delete(msg.id);
  }
};
function send(method, params = {}) {
  const id = nextId++;
  ws.send(JSON.stringify({ id, method, params }));
  return new Promise((resolve) => pending.set(id, resolve));
}

await send("Page.enable");
await send("Page.navigate", { url: `http://localhost:${PORT}/e2e.html` });

let title = "";
for (let i = 0; i < 120; i++) {
  await wait(500);
  const reply = await send("Runtime.evaluate", { expression: "document.title", returnByValue: true });
  title = reply.result?.result?.value ?? "";
  if (title.startsWith("E2E ")) break;
}

await send("Emulation.setDeviceMetricsOverride", {
  width: 900,
  height: 2600,
  deviceScaleFactor: 1,
  mobile: false,
});
const shot = await send("Page.captureScreenshot", { format: "png" });
if (shot.result?.data) {
  fs.writeFileSync("/tmp/moo3-ui.png", Buffer.from(shot.result.data, "base64"));
  console.log("screenshot: /tmp/moo3-ui.png");
}

console.log(title);
if (title.startsWith("E2E PASS")) {
  console.log("ui test passed");
  cleanup(0);
} else {
  console.log("ui test FAILED");
  cleanup(1);
}
