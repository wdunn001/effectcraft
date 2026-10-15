#!/usr/bin/env bash
# Sync this repo to a lab host without piping git archive (breaks on Windows → Linux tar).
# Usage: ./scripts/sync-tree-to-lab.sh [user@host] [remote-dir]
set -euo pipefail
DEST="${1:-vinez@192.168.1.88}"
DIR="${2:-~/tmp/effectcraft-pilot}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TAR="$(mktemp -t effectcraft-sync.XXXXXX.tar)"
trap 'rm -f "$TAR"' EXIT

cd "$ROOT"
git archive --format=tar HEAD -o "$TAR"
ssh "$DEST" "rm -rf '$DIR' && mkdir -p '$DIR'"
scp "$TAR" "$DEST:/tmp/effectcraft-sync.tar"
ssh "$DEST" "tar -xf /tmp/effectcraft-sync.tar -C '$DIR' && rm -f /tmp/effectcraft-sync.tar"
echo "Synced to $DEST:$DIR"
