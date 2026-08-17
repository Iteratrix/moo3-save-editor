---
name: wasm-pages-app
description: Architecture and invariants for Rust-core WASM apps deployed to GitHub Pages with offline support. Apply when working on a project generated from rust-wasm-pages-template, when adding assets to its web build, when touching sw.template.js/build.mjs, or when creating a new client-side WASM tool.
---

# WASM Pages App

The pattern: a pure Rust core compiled to WASM, a framework-free web page,
deployed as static files to GitHub Pages, working offline via a service
worker. Users' files never leave their machine — privacy is a feature, not a
disclaimer.

These are design docs as much as machine instructions; everything below holds
whether a human or an agent is editing.

## Architecture: Pure Core, Thin Adapters

- **`<name>-core`** holds all domain logic. No I/O, no platform assumptions,
  wasm-safe dependencies only. If a dependency won't build for
  `wasm32-unknown-unknown`, it doesn't belong in core.
- **`<name>-web`** is a thin wasm-bindgen bridge: (de)serialization at the
  boundary, logic in core. Start with a JSON-string API (trivial JS side);
  upgrade to `tsify` typed bindings when the boundary grows rich enough that
  stringly typing hurts.
- Other adapters (CLI, Python bindings, a server) follow the same rule: thin,
  at the edge. The CLI is nearly free and doubles as a scriptable test
  harness — include it unless there's a reason not to.

## Web Layer Rules

- No framework, no bundler beyond `web/build.mjs` (zero npm dependencies).
  Adding a JS toolchain is a deliberate decision, not a default.
- **Vendor, don't CDN.** Third-party JS/CSS gets committed under
  `web/vendor/` and precached. A CDN reference breaks offline support and
  adds a tracking surface.
- Dev flow: `wasm-pack build <name>-web --target web --out-dir ../web/pkg`,
  then serve `web/` with any static server. The service worker is skipped on
  localhost by design — dev never fights a stale cache.

## The Cache Story (service worker invariants)

`build.mjs` content-hashes every asset into its filename, rewrites the
reference chain (`index.html -> app.js -> pkg js -> wasm`), and generates
`sw.js` from `sw.template.js`. Invariants — breaking any of these breaks
real users:

1. **Cache-first requires content-hashing.** The SW serves non-HTML assets
   cache-first, which is only correct because a given URL's content never
   changes. Never add an unhashed, mutable asset to the cache-first path.
2. **Navigations are network-first** with cache fallback: fresh index.html
   when online, offline still works.
3. **SW version = hash of the precache manifest.** An unchanged deploy
   produces a byte-identical sw.js and clients ignore it. Don't stamp in
   timestamps or build numbers — that forces pointless re-downloads.
4. **No `skipWaiting`/`clients.claim`.** A new SW takes over only when the
   user closes every tab. Never "fix" a stale-update complaint by adding
   skipWaiting — it reloads people mid-task.
5. **`build.mjs` must throw when an expected reference string is missing.**
   That error is the only thing standing between a refactor and a silently
   broken deploy.

**Adding an asset:** `emitHashed()` it (with replacements for anything it
references), rewrite its referrer, add it to the precache list. CI runs
`build.mjs`, so a missing reference fails the PR.

**Recovery (bricked clients):** deploy `web/sw-killswitch.js` as `sw.js`.
It deletes all caches, unregisters itself, and reloads controlled clients.
Once clients recover, deploy the real sw.js again.

## Release Flow

- CI on every push: clippy (pedantic, `-D warnings`), fmt, tests, wasm +
  web build.
- Deploy: push a version tag (`v0.1.0`) — pages.yml builds and publishes
  `web/out/`. One-time repo setup: Settings → Pages → Source: "GitHub
  Actions".
- After deploying a SW change, verify on the live origin: load once, go
  offline, reload. Localhost cannot exercise the SW path.
