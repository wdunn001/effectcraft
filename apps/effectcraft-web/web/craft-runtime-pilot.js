// Optional smoke: craft-runtime-smoke.html or ?craftRuntime=1 (redirects to smoke page).
import { connect } from "./craft-runtime-loader.js";

function banner(text, ok) {
  const el = document.getElementById("status") || document.getElementById("loading");
  if (!el) return;
  el.className = ok ? "ok" : "err";
  el.textContent = text;
}

function withTimeout(promise, ms, label) {
  return Promise.race([
    promise,
    new Promise((_, reject) => {
      setTimeout(() => reject(new Error(`${label} timed out after ${ms}ms`)), ms);
    }),
  ]);
}

export async function runCraftRuntimePilot() {
  const params = new URLSearchParams(location.search);
  const dedicated = /craft-runtime-smoke\.html$/i.test(location.pathname);
  banner(
    dedicated
      ? "craft-runtime smoke: DedicatedWorker at /craft-dedicated-worker.js …"
      : "craft-runtime: SharedWorker at /craft-worker-host.js …",
    true,
  );
  const rt = connect({ dedicated });
  const ready = await withTimeout(
    rt.init({
      app_id: "effectcraft",
      capabilities: ["tier_store"],
    }),
    15000,
    "worker init",
  );

  const mode = dedicated ? "DedicatedWorker" : "SharedWorker";
  const parts = [`craft-runtime ready (${mode}, web-root worker)`];

  // OPFS flush test (?flush=1)
  if (params.has("flush")) {
    await withTimeout(rt.storageFlush(), 15000, "storage_flush");
    parts.push("storage_flush ok");
  } else {
    parts.push("Add ?flush=1 to test OPFS flush");
  }

  // GPU batch test via worker (?jobs=<n>, default n=1) — craft-libs#9 SharedWorker jobs.
  // Without wasm the worker stubs craft.noop outcomes; with wasm it routes through the bridge.
  if (params.has("jobs")) {
    const n = Math.max(1, parseInt(params.get("jobs"), 10) || 1);
    const jobs = Array.from({ length: n }, () => ({
      provider_id: "craft.noop",
      params: [],
      inputs: [],
      outputs: [],
    }));
    const done = await withTimeout(rt.gpuJobs(1, jobs), 15000, "gpu_jobs");
    parts.push(`gpu_batch_done (${done.outcomes?.length ?? 0}/${n} outcomes)`);
  } else {
    parts.push("Add ?jobs=1 to test GPU batch via worker");
  }

  // Cold folder hook — reads craftRuntimeColdFolders wasm export when the main app is running
  // (returns [] on the smoke page; craft-libs#9 cold folder UI hook).
  const cold = rt.coldFolders();
  if (cold.length > 0) {
    parts.push(`cold folders (need Reconnect): ${cold.join(", ")}`);
  }

  banner(parts.join(". "), true);
  return ready;
}

const smokePage = /craft-runtime-smoke\.html$/i.test(location.pathname);
const smokeQuery = new URLSearchParams(location.search).has("craftRuntime");
if (smokePage || smokeQuery) {
  runCraftRuntimePilot().catch((e) => {
    banner(`craft-runtime failed:\n${e?.stack || e}`, false);
    console.error(e);
  });
}
