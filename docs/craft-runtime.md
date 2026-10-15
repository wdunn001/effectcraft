# craft-runtime pilot (effectcraft-web)

Incremental wiring for [craft-runtime](https://github.com/wdunn001/craft-libs/tree/expand-craft-gpu-ecosystem/crates/runtime) alongside the existing [`store`](../apps/effectcraft-web/src/store.rs) OPFS mirror. **Store keys are unchanged** (`config/*`, `files/…`); see craft-libs [pilot-effectcraft](https://github.com/wdunn001/craft-libs/blob/expand-craft-gpu-ecosystem/docs/pilot-effectcraft.md).

## Dependency

`apps/effectcraft-web/Cargo.toml` pins:

```toml
craft-runtime = { git = "https://github.com/wdunn001/craft-libs.git", branch = "expand-craft-gpu-ecosystem", package = "craft-runtime" }
```

## Protocol (craft-worker-host)

Host → worker JSON (`type` in snake_case):

| `type` | Purpose |
| --- | --- |
| `init` | `{ app_id, capabilities[] }` → `ready` + device caps |
| `storage_put` | `{ key, bytes[] }` → (no reply; mirror hot tier) |
| `storage_flush` | → `storage_flushed` (+ optional `pending[]` for OPFS glue) |
| `gpu_jobs` | `{ batch_id, jobs[] }` → `gpu_batch_done` |
| `cancel` | `{ batch_id }` |

Worker → host: `ready`, `storage_flushed`, `gpu_batch_done`, `error`.

Rust types: `craft_worker_host::HostMessage` / `WorkerMessage`. Native round-trip tests live in [`craft_runtime_bridge.rs`](../apps/effectcraft-web/src/craft_runtime_bridge.rs).

## Integration surfaces

1. **Rust bridge** — `CraftRuntimeBridge::handle_worker_json` (native tests) and `craftRuntimeHostMessage` (wasm export when the full web crate is built for `wasm32`).
2. **Dedicated worker** — [`web/worker.js`](../apps/effectcraft-web/web/worker.js) detects craft-runtime `init` (`app_id` present) and host messages (`storage_put`, `storage_flush`, `gpu_jobs`, `cancel`), delegating to `craftRuntimeHostMessage` after wasm init. EffectCraft job/frame `init` is unchanged (`module` / `base` / `frames` / `gpu`).
3. **SharedWorker (parallel stub)** — [`web/craft-worker-host.js`](../apps/effectcraft-web/web/craft-worker-host.js) implements the JS bootstrap; optional [`web/craft-runtime-loader.js`](../apps/effectcraft-web/web/craft-runtime-loader.js) opens the worker from the page. When wasm sets `globalThis.craftRuntimeHostMessage` inside the SharedWorker context, messages dispatch through the same JSON path as Rust.

## Not wired yet

- Replacing `persist::schedule_flush` / `store` with `craft-tier-store` (Phase B in craft-libs pilot).
- Routing frame/job GPU through SharedWorker `gpu_jobs` (Phase D).
- Serving brotli-compressed `craft-worker-host.js` from the static host (CollabCraft / packaging).

## Verify

On the lab GPU box (`.88`):

```bash
./scripts/test-craft-runtime-bridge.sh
```

Or: `cargo test -p effectcraft-web craft_runtime_bridge`
