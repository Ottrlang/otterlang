# WebAssembly Support

OtterLang can compile to WebAssembly! Use the `--target` flag:

```bash
# Compile to WebAssembly (wasm32-unknown-unknown)
otterlang build program.ot --target wasm32-unknown-unknown -o program.wasm

# Compile to WebAssembly System Interface (wasm32-wasi)
otterlang build program.ot --target wasm32-wasi -o program.wasm
```

## Requirements

- LLVM 18 with WebAssembly target support
- `clang` and `wasm-ld` in your PATH (usually included with LLVM)

## Target Differences

### wasm32-wasi

The generated binary can talk directly to WASI's stdio and wall-clock APIs.

### wasm32-unknown-unknown

For the more barebones `wasm32-unknown-unknown` target, we import a minimal host surface so you can decide how to surface output:

- `env.otter_write_stdout(ptr: i32, len: i32)` – write UTF-8 data to stdout
- `env.otter_write_stderr(ptr: i32, len: i32)` – write UTF-8 data to stderr
- `env.otter_time_now_ms() -> i64` – optional wall-clock timestamp in ms

## JavaScript Host Example

A tiny JavaScript host that wires these up under Node.js looks like:

```js
import fs from 'node:fs';

const memory = new WebAssembly.Memory({ initial: 8 });
const decoder = new TextDecoder();

const env = {
  memory,
  otter_write_stdout(ptr, len) {
    const bytes = new Uint8Array(memory.buffer, ptr, len);
    process.stdout.write(decoder.decode(bytes));
  },
  otter_write_stderr(ptr, len) {
    const bytes = new Uint8Array(memory.buffer, ptr, len);
    process.stderr.write(decoder.decode(bytes));
  },
  otter_time_now_ms() {
    return BigInt(Date.now());
  },
};

const { instance } = await WebAssembly.instantiate(fs.readFileSync('program.wasm'), { env });
instance.exports.main?.();
```

The generated `.wasm` file can be run in any WebAssembly runtime (Node.js, browsers, wasmtime, etc.).

