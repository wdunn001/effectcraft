//! Parallel path for SharedWorker `craft-runtime` messages (pilot; does not replace [`store`] yet).
//!
//! JSON shapes match [`craft_worker_host::HostMessage`] / [`WorkerMessage`] (see
//! `craft-libs/assets/craft-worker-host.js`).

use craft_runtime::{HostMessage, Runtime, WorkerKind, WorkerMessage};
use serde_json;

pub struct CraftRuntimeBridge {
    runtime: Runtime,
}

impl Default for CraftRuntimeBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl CraftRuntimeBridge {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::builder()
                .with_default_providers()
                .with_worker_kind(WorkerKind::SharedWorker)
                .null_context()
                .build(),
        }
    }

    /// Typed host message → worker replies (native / tests).
    pub fn handle_host_message(&mut self, msg: HostMessage) -> Vec<WorkerMessage> {
        self.runtime.handle_host_message(msg)
    }

    /// JSON host message in, JSON array of worker replies out (one round-trip).
    pub fn handle_worker_json(&mut self, msg: &str) -> String {
        let host = match serde_json::from_str::<HostMessage>(msg) {
            Ok(h) => h,
            Err(e) => {
                return json_replies(&[WorkerMessage::Error { message: e.to_string() }]);
            }
        };
        json_replies(&self.handle_host_message(host))
    }
}

fn json_replies(replies: &[WorkerMessage]) -> String {
    serde_json::to_string(replies).unwrap_or_else(|_| "[]".to_owned())
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::CraftRuntimeBridge;
    use std::cell::RefCell;
    use wasm_bindgen::prelude::*;

    thread_local! {
        static BRIDGE: RefCell<CraftRuntimeBridge> = RefCell::new(CraftRuntimeBridge::new());
    }

    /// Host → worker JSON (`HostMessage`); returns a JSON array of `WorkerMessage`.
    #[wasm_bindgen(js_name = craftRuntimeHostMessage)]
    pub fn craft_runtime_host_message(msg: &str) -> String {
        BRIDGE.with(|b| b.borrow_mut().handle_worker_json(msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_runtime::WorkerMessage;

    #[test]
    fn init_storage_flush_and_noop_gpu_roundtrip() {
        let mut bridge = CraftRuntimeBridge::new();
        let ready = bridge.handle_worker_json(r#"{"type":"init","app_id":"effectcraft","capabilities":["gpu"]}"#);
        assert!(ready.contains("ready"));
        bridge.handle_worker_json(r#"{"type":"storage_put","key":"config/session.json","bytes":[123,125]}"#);
        let flushed = bridge.handle_worker_json(r#"{"type":"storage_flush"}"#);
        assert!(flushed.contains("storage_flushed"));
        let gpu = bridge.handle_worker_json(
            r#"{"type":"gpu_jobs","batch_id":1,"jobs":[{"provider_id":"craft.noop","params":[],"inputs":[],"outputs":[]}]}"#,
        );
        assert!(gpu.contains("gpu_batch_done"));
    }

    #[test]
    fn typed_roundtrip_matches_json() {
        let mut bridge = CraftRuntimeBridge::new();
        let replies = bridge.handle_host_message(HostMessage::Init {
            app_id: "effectcraft".into(),
            capabilities: vec!["gpu".into(), "tier_store".into()],
        });
        assert!(matches!(replies.as_slice(), [WorkerMessage::Ready { .. }]));
        let json = bridge.handle_worker_json(r#"{"type":"storage_flush"}"#);
        assert!(json.contains("storage_flushed"));
    }
}
