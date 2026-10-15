// DedicatedWorker bootstrap for craft-runtime smoke / local testing.
// Accepts the same HostMessage types as the SharedWorker (craft-worker-host.js) but routes
// replies through self.postMessage instead of MessagePort.postMessage.
//
// Message shapes match craft-worker-host Rust serde (snake_case):
//   Host → worker: init, storage_put, storage_flush, gpu_jobs, cancel
//   Worker → host: ready, gpu_batch_done, storage_flushed, error
//
// WASM hooks (same as craft-worker-host.js):
//
//   globalThis.craftRuntimeHostMessage = (jsonStr) => jsonReplyArray;
//   globalThis.craftRuntimeHandleGpuBatch = async ({ batch_id, jobs }) => GpuJobOutcome[];
//   globalThis.craftRuntimeFlushPending   = (data) => { pending_count: number };

const STUB_CAPS = {
  backend_name: 'dedicated-worker-stub',
  device_name: 'pending-wasm',
  max_storage_buffer_binding_size: 134217728,
  max_buffer_size: 268435456,
  max_texture_dimension_2d: 8192,
};

function postError(message) {
  self.postMessage({ type: 'error', message: String(message) });
}

function postWorkerReplies(json) {
  let replies;
  try {
    replies = JSON.parse(json);
  } catch (err) {
    postError(err);
    return;
  }
  if (!Array.isArray(replies)) {
    postError('craftRuntimeHostMessage must return a JSON array');
    return;
  }
  for (const reply of replies) {
    self.postMessage(reply);
  }
}

function stubGpuOutcomes(jobs) {
  return jobs.map(() => ({ outputs: [], data: null }));
}

async function dispatchGpuJobs(data) {
  const batchId = data.batch_id;
  const jobs = Array.isArray(data.jobs) ? data.jobs : [];
  const wasmHook = globalThis.craftRuntimeHostMessage;

  try {
    if (typeof wasmHook === 'function') {
      postWorkerReplies(wasmHook(JSON.stringify(data)));
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
    self.postMessage({ type: 'gpu_batch_done', batch_id: batchId, outcomes });
  } catch (err) {
    postError(err && typeof err.message === 'string' ? err.message : String(err));
  }
}

self.onmessage = (ev) => {
  const data = ev.data;
  if (!data || typeof data.type !== 'string') {
    postError('missing message type');
    return;
  }

  const wasmHook = globalThis.craftRuntimeHostMessage;

  switch (data.type) {
    case 'init':
      if (typeof wasmHook === 'function') {
        postWorkerReplies(wasmHook(JSON.stringify(data)));
        return;
      }
      self.postMessage({ type: 'ready', caps: STUB_CAPS });
      return;

    case 'storage_put':
      if (typeof wasmHook === 'function') {
        postWorkerReplies(wasmHook(JSON.stringify(data)));
      }
      return;

    case 'storage_flush': {
      if (typeof wasmHook === 'function') {
        postWorkerReplies(wasmHook(JSON.stringify(data)));
        return;
      }
      const hook = globalThis.craftRuntimeFlushPending;
      let pendingCount = 0;
      if (typeof hook === 'function') {
        try {
          const result = hook(data);
          pendingCount =
            result && typeof result.pending_count === 'number' ? result.pending_count : 0;
        } catch (err) {
          postError(err);
          return;
        }
      }
      self.postMessage({ type: 'storage_flushed', pending_count: pendingCount });
      return;
    }

    case 'gpu_jobs':
      void dispatchGpuJobs(data);
      return;

    case 'cancel':
      if (typeof wasmHook === 'function') {
        postWorkerReplies(wasmHook(JSON.stringify(data)));
      }
      return;

    default:
      postError(`unknown host message type: ${data.type}`);
  }
};
