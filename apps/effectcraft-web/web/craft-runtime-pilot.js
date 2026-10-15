// Optional smoke: ?craftRuntime=1 connects SharedWorker at /craft-worker-host.js
import { connect } from "./craft-runtime-loader.js";

function banner(text, ok) {
  const el = document.getElementById("status") || document.getElementById("loading");
  if (!el) return;
  el.className = ok ? "ok" : "err";
  if (el.parentElement?.id === "loading" && ok) el.parentElement.querySelector("#status")?.classList.add("ok");
  el.textContent = text;
}

export async function runCraftRuntimePilot() {
  banner("craft-runtime: connecting SharedWorker…", true);
  const rt = connect();
  const ready = await rt.init({
    app_id: "effectcraft",
    capabilities: ["tier_store"],
  });
  await rt.storageFlush();
  banner(`craft-runtime ready (worker /craft-worker-host.js) batch=${ready.batch_id ?? "—"}`, true);
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
