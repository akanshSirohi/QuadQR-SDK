#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/packages/quadqr-js"
npm run setup:wasm
npm run check:source
npm run build:wasm
npm run verify:wasm
npm run check
npm run serve
