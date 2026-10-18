#!/usr/bin/env bash
# Builds the npm package (topos-bible) into dist/wasm/pkg, with its README and license.
set -euo pipefail
cd "$(dirname "$0")/.."
boltffi pack wasm --release
cp packaging/npm/README.md dist/wasm/pkg/README.md
cp ../../LICENSE dist/wasm/pkg/LICENSE
# web.js, bundler.js, and node.js export `initialized`, which BoltFFI's types leave out
types=dist/wasm/pkg/topos_bible.d.ts
if ! grep -q 'export declare const initialized' "$types"; then
  printf '%s\n' '' \
    '/** Resolves once the WebAssembly module is loaded: await it before the first call in a browser (in Node and Bun it already has) */' \
    'export declare const initialized: Promise<void>;' >> "$types"
fi
