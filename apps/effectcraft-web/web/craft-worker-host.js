// SharedWorker bootstrap for craft-runtime (craft-libs#5).
// Message `type` tags match craft-worker-host Rust serde (snake_case):
//   Host → worker: init, storage_put, storage_flush, gpu_jobs, cancel
//   Worker → host: ready, gpu_batch_done, storage_flushed, error
//
// Pilot: copied from craft-libs `assets/craft-worker-host.js` (expand-craft-gpu-ecosystem).
// Wire wasm via globalThis.craftRuntimeHostMessage (effectcraft_web.js) when embedded.
const connections = new Set();

const STUB_CAPS = {
  backend_name: 'worker-host-stub',
  device_name: 'pending-wasm',
  max_storage_buffer_binding_size: 134217728,
  max_buffer_size: 268435456,
  max_texture_dimension_2d: 8192,
};

function postError(port, message) {
  port.postMessage({ type: 'error', message: String(message) });
}

function postWorkerReplies(port, json) {
  let replies;
  try {
    replies = JSON.parse(json);
  } catch (err) {
    postError(port, err);
    return;
  }
  if (!Array.isArray(replies)) {
    postError(port, 'craftRuntimeHostMessage must return a JSON array');
    return;
  }
  for (const reply of replies) {
    port.postMessage(reply);
  }
}

function stubGpuOutcomes(jobs) {
  return jobs.map(() => ({ outputs: [], data: null }));
}

async function dispatchGpuJobs(port, data) {
  const batchId = data.batch_id;
  const jobs = Array.isArray(data.jobs) ? data.jobs : [];
  const wasmHook = globalThis.craftRuntimeHostMessage;

  try {
    if (typeof wasmHook === 'function') {
      postWorkerReplies(port, wasmHook(JSON.stringify(data)));
      return;
    }
    const hook = globalThis.craftRuntimeHandleGpuBatch;
    let outcomes;
    if (typeof hook === 'function') {
      outcomes = await hook({ batch_id: batchId, jobs });
    } else {
      outcomes = stubGpuOutcomes(jobs);
    }
    if (!Array.isArray(outcomes)) {
      throw new Error('craftRuntimeHandleGpuBatch must return an outcomes array');
    }
    port.postMessage({
      type: 'gpu_batch_done',
      batch_id: batchId,
      outcomes,
    });
  } catch (err) {
    const message =
      err && typeof err.message === 'string' ? err.message : String(err);
    postError(port, message);
  }
}

function handleHostMessage(port, data) {
  if (!data || typeof data.type !== 'string') {
    postError(port, 'missing message type');
    return;
  }

  const wasmHook = globalThis.craftRuntimeHostMessage;

  switch (data.type) {
    case 'init':
      if (typeof wasmHook === 'function') {
        postWorkerReplies(port, wasmHook(JSON.stringify(data)));
        return;
      }
      port.postMessage({ type: 'ready', caps: STUB_CAPS });
      return;
    case 'storage_put':
      if (typeof wasmHook === 'function') {
        postWorkerReplies(port, wasmHook(JSON.stringify(data)));
        return;
      }
      return;
    case 'storage_flush': {
      if (typeof wasmHook === 'function') {
        postWorkerReplies(port, wasmHook(JSON.stringify(data)));
        return;
      }
      const hook = globalThis.craftRuntimeFlushPending;
      let pendingCount = 0;
      if (typeof hook === 'function') {
        try {
          const result = hook(data);
          pendingCount =
            result && typeof result.pending_count === 'number'
              ? result.pending_count
              : 0;
        } catch (err) {
          postError(port, err);
          return;
        }
      }
      port.postMessage({ type: 'storage_flushed', pending_count: pendingCount });
      return;
    }
    case 'gpu_jobs':
      void dispatchGpuJobs(port, data);
      return;
    case 'cancel':
      if (typeof wasmHook === 'function') {
        postWorkerReplies(port, wasmHook(JSON.stringify(data)));
        return;
      }
      return;
    default:
      postError(port, `unknown host message type: ${data.type}`);
  }
}

self.onconnect = (event) => {
  const port = event.ports[0];
  connections.add(port);
  port.onmessage = (ev) => handleHostMessage(port, ev.data);
  port.start();
};
