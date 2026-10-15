#!/usr/bin/env bash
set -euo pipefail
export PATH="${HOME}/.cargo/bin:/usr/local/cargo/bin:${PATH}"
cd apps/effectcraft-web
cargo test craft_runtime_bridge
