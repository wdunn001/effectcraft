// Pilot: open the craft-runtime SharedWorker and speak craft-worker-host JSON.
// Does not replace effectcraft `store` / `persist` yet — parallel integration only.
//
// Usage (optional, from the page after static assets load):
//   const rt = effectcraftCraftRuntime.connect({ workerUrl: "craft-worker-host.js" });
//   await rt.init({ app_id: "effectcraft", capabilities: ["gpu", "tier_store"] });
//   await rt.storageFlush();

const CRAFT_HOST_TYPES = new Set([
  "init",
  "storage_put",
  "storage_flush",
  "gpu_jobs",
  "cancel",
]);

function isCraftHostMessage(data) {
  return data && typeof data.type === "string" && CRAFT_HOST_TYPES.has(data.type);
}

/**
 * @param {{ workerUrl?: string }} [opts]
 */
export function connect(opts = {}) {
  const workerUrl = opts.workerUrl ?? "craft-worker-host.js";
  const worker = new SharedWorker(workerUrl, { type: "module", name: "craft-runtime" });
  const port = worker.port;

  /** @type {Map<number, { resolve: Function, reject: Function, replies: object[] }>} */
  const gpuWaiters = new Map();
  let gpuSeq = 0;

  port.onmessage = (ev) => {
    const msg = ev.data;
    if (!msg || typeof msg.type !== "string") return;
    if (msg.type === "gpu_batch_done" && gpuWaiters.has(msg.batch_id)) {
      const w = gpuWaiters.get(msg.batch_id);
      gpuWaiters.delete(msg.batch_id);
      w.resolve(msg);
      return;
    }
    if (msg.type === "error" && msg.message) {
      for (const [, w] of gpuWaiters) {
        w.reject(new Error(msg.message));
      }
      gpuWaiters.clear();
    }
  };

  port.start();

  function postHost(data) {
    if (!isCraftHostMessage(data)) {
      return Promise.reject(new Error("not a craft-worker-host message"));
    }
    port.postMessage(data);
    return Promise.resolve();
  }

  return {
    port,
    postHost,
    init(payload) {
      return new Promise((resolve, reject) => {
        const onReply = (ev) => {
          const msg = ev.data;
          if (msg?.type === "ready") {
            port.removeEventListener("message", onReply);
            resolve(msg);
          } else if (msg?.type === "error") {
            port.removeEventListener("message", onReply);
            reject(new Error(msg.message));
          }
        };
        port.addEventListener("message", onReply);
        port.postMessage({ type: "init", ...payload });
      });
    },
    storagePut(key, bytes) {
      return postHost({ type: "storage_put", key, bytes });
    },
    storageFlush() {
      return new Promise((resolve, reject) => {
        const onReply = (ev) => {
          const msg = ev.data;
          if (msg?.type === "storage_flushed") {
            port.removeEventListener("message", onReply);
            resolve(msg);
          } else if (msg?.type === "error") {
            port.removeEventListener("message", onReply);
            reject(new Error(msg.message));
          }
        };
        port.addEventListener("message", onReply);
        port.postMessage({ type: "storage_flush" });
      });
    },
    gpuJobs(batchId, jobs) {
      return new Promise((resolve, reject) => {
        gpuWaiters.set(batchId, { resolve, reject, replies: [] });
        port.postMessage({ type: "gpu_jobs", batch_id: batchId, jobs });
      });
    },
  };
}
