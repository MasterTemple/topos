Add this to `Cargo.toml`

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"

[dev-dependencies]
wasm-bindgen-test = "0.3"

[profile.release]
lto = true
opt-level = 's'
```

Install wasm-pack if you don't have it

```bash
cargo install wasm-pack
```

Build

```bash
wasm-pack build --target nodejs # or --target web for browser
```

Create TypeScript package

```bash
mkdir your-ts-package
cd your-ts-package
npm init -y
npm install typescript --save-dev
npx tsc --init
```

Install in TypeScript

```bash
npm install /home/dgmastertemple/Documents/GitHub/topos/crates/topos-ts/pkg/
```

Transpile

```bash
npx tsc
```

If you get an error like

```text
λ npx tsc
index.ts:1:10 - error TS1295: ECMAScript imports and exports cannot be written in a CommonJS file under 'verbatimModuleSyntax'. Adjust the 'type' field in the nearest 'package.json' to make this file an ECMAScript module, or adjust your 'verbatimModuleSyntax', 'module', and 'moduleResolution' settings in TypeScript.

1 import { greet } from 'topos-ts';
           ~~~~~
```

change `package.json`

```jsonc
"type": "commonjs",
// to
"type": "module",
```

Run

```bash
node index.js
```
