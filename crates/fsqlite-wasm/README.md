# `fsqlite-wasm`

`fsqlite-wasm` is the Rust crate that produces FrankenSQLite's browser-facing
WebAssembly package.

The intended npm artifact is published as `@frankensqlite/core` and exposes the
generated `wasm-bindgen` glue plus the `FrankenDB` / `FrankenPreparedStatement`
APIs implemented in [`src/lib.rs`](./src/lib.rs).

## Package Build

Build a publishable package into `target/fsqlite-wasm-pkg/`:

```bash
./scripts/build_fsqlite_wasm_package.sh
```

Choose a different output directory or `wasm-pack` target:

```bash
FSQLITE_WASM_TARGET=web ./scripts/build_fsqlite_wasm_package.sh target/fsqlite-wasm-web
FSQLITE_WASM_TARGET=nodejs ./scripts/build_fsqlite_wasm_package.sh target/fsqlite-wasm-node
```

The helper script:

- runs `wasm-pack build`
- normalizes the generated `package.json` to the `@frankensqlite/core` package name
- copies README/license files into the output package
- validates the generated `.wasm`, `.js`, and `.d.ts` artifacts exist
- runs `npm pack` so the result is ready for registry or local install testing
- enforces a packed tarball size budget of 2 MiB by default (`FSQLITE_WASM_MAX_PACKED_BYTES=0` disables the guard)

## Expected Package Contents

- `frankensqlite_wasm_bg.wasm`
- `frankensqlite_wasm.js`
- `frankensqlite_wasm.d.ts`
- `snippets/`
- `README.md`
- `LICENSE`

## Import Example

```ts
import init, { FrankenDB } from "@frankensqlite/core";

await init();

const db = new FrankenDB(":memory:");
db.execute("CREATE TABLE users(id INTEGER PRIMARY KEY, name TEXT)");
db.execute("INSERT INTO users(name) VALUES('Ada')");

const result = db.query("SELECT id, name FROM users ORDER BY id");
console.log(result.rows);
```

## WASM Memory Management

FrankenSQLite's WASM package runs inside the browser's WebAssembly linear
memory, so the hard upper bound remains 4 GiB for the whole module. The
database-specific knobs exposed by `FrankenDB.openWithOptions()` and
`FrankenDB.importWithOptions()` let you budget FrankenSQLite's own heap usage
inside that ceiling:

```ts
const db = FrankenDB.openWithOptions(":memory:", {
  pageBufferMax: 256,
  memory: {
    initialPages: 4,
    growthChunkPages: 1,
    maxPages: 512,
    warnAtPercent: 75,
    onWarning(stats) {
      console.warn("FrankenSQLite memory pressure", stats);
    },
  },
});
```

- `pageBufferMax` caps the pager's page-buffer pool in pages.
- `memory.initialPages`, `memory.growthChunkPages`, and `memory.maxPages`
  express the same policy in WebAssembly pages (`64 KiB` each). The byte-level
  aliases `memory.initialReserveBytes`, `memory.growthChunkBytes`, and
  `memory.maxBytes` remain available when callers want exact byte counts.
- `memory.maxPages` / `memory.maxBytes` act as a hard cap for tracked
  `MemoryVfs` heap usage. When the engine crosses that cap, operations fail
  with a structured out-of-memory error instead of trapping through an
  `unreachable`.
- `memory.warnAtPercent` derives the warning threshold from the tracked max.
  `memory.warningThresholdBytes` remains available for exact byte thresholds.
- `memory.onWarning` fires once when usage crosses the configured threshold and
  the payload includes both byte-level fields and page-oriented diagnostics such
  as `initialReservePages`, `growthChunkPages`, `trackedMaxPages`, and
  `linearMemoryPages` when available.
- `db.memoryStats()` now also emits page-cache pressure advisory fields:
  `pageCachePressureLevel`, `pageCachePressureBudgetBytes`,
  `recommendedPageBufferMaxPages`, `recommendedPageBufferMaxBytes`, and
  `trackedHeadroomBytes`. These let the JS side decide when to ratchet
  `pageBufferMax` down before the tracked heap reaches its hard cap.

Call `db.memoryStats()` at any point to inspect tracked heap bytes, page-cache
resident bytes, page-cache capacity, configured warning thresholds, growth
events, current linear-memory size/pages (when running under `wasm32`), and the
derived page-cache pressure recommendation.
