# OtterLang

<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/jonathanmagambo/otterlang/blob/main/image.png?raw=true" width="400">
    <img src="https://github.com/jonathanmagambo/otterlang/blob/main/image.png?raw=true" width="400" alt="OtterLang Logo" />
  </picture>
  <br>
  <strong>Simple syntax, native performance, transparent Rust FFI.</strong>
  <br><br>
  
  [![Build Status](https://github.com/jonathanmagambo/otterlang/workflows/CI/badge.svg)](https://github.com/jonathanmagambo/otterlang/actions)
  [![Discord](https://img.shields.io/badge/Discord-Join%20Server-5865F2?style=flat&logo=discord&logoColor=white)](https://discord.gg/y3b4QuvyFk)
  
  <br><br>
  An indentation-sensitive programming language with an LLVM backend. OtterLang compiles to native binaries with a focus on simplicity and performance.
</div>

<h3 align="center">
  <a href="docs/LANGUAGE_SPEC.md"><b>Language Spec</b></a>
  &nbsp;&#183;&nbsp;
  <a href="docs/GETTING_STARTED.md"><b>Getting Started</b></a>
  &nbsp;&#183;&nbsp;
  <a href="docs/EXAMPLES.md"><b>Examples</b></a>
  &nbsp;&#183;&nbsp;
  <a href="https://discord.gg/y3b4QuvyFk" target="_blank">Discord</a>
  &nbsp;&#183;&nbsp;
  <a href="docs/FFI_GUIDE.md"><b>FFI Guide</b></a>
  &nbsp;&#183;&nbsp;
  <a href="CONTRIBUTING.md"><b>Contributing</b></a>
</h3>

## Quick Start

```bash
git clone https://github.com/jonathanmagambo/otterlang.git
cd otterlang

# Environment setup (requires Rust nightly + LLVM 18)
nix develop            # recommended shell with all dependencies
cargo +nightly build --release

# Create and run your first program
cat > hello.ot << 'EOF'
fn main():
    println("Hello from OtterLang!")
EOF

./target/release/otter run hello.ot
```

See the [Getting Started Guide](docs/GETTING_STARTED.md) for detailed installation and usage instructions.

## Getting Started

1. Install Rust nightly (`rustup toolchain install nightly`) and ensure LLVM 18 is available (or simply run `nix develop`).
2. Build the toolchain: `cargo +nightly build --release`.
3. Run code with `otter run file.ot` or emit binaries with `otter build file.ot -o my_app`.

The [Getting Started Guide](docs/GETTING_STARTED.md) expands on editor setup, cache usage, the REPL, and snapshot testing.

## Language Features

OtterLang pairs indentation-aware syntax with modern language constructs:

- **Whitespace-driven grammar** – no braces or semicolons, just `fn`, blocks, and meaningful indentation.
- **Static typing with inference** – optional annotations, tuples, enums, and generics.
- **Structured error handling** – `Result<T, E>` enum with pattern matching plus `panic`/`recover` utilities.
- **Async task runtime** – `spawn` and `await` built on the Otter task scheduler.
- **Generational GC** – explicit root APIs (`gc.alloc`, `gc.add_root`, `gc.remove_root`) keep FFI safe.
- **Transparent Rust FFI** – import any crate with `use rust:crate_name` and call it directly.

### Transparent Rust FFI

The compiler shells out to `cargo`/rustdoc, normalizes the crate’s public API, and generates a native bridge automatically. Import crates directly from OtterLang (`use rust:serde::{json}`) without touching build scripts. See the [FFI Guide](docs/FFI_GUIDE.md) for configuration, caching, and troubleshooting tips.

### Standard Library

Built-in modules cover IO, math, JSON, tasks, runtime helpers, strings, networking, testing, and more. The [API Reference](docs/API_REFERENCE.md) documents every exported function.

 Only the true language primitives (enums, `Option`/`Result`, `panic`, `print`, `len`, strings, lists, maps, and arithmetic) live in the implicit prelude. All other stdlib functionality now follows a Python-style import model—`use http`, `use yaml`, `use task`, etc.—and nothing outside the prelude is visible until you import the module you need.


## Command Line Interface

The `otter` binary drives every workflow:

```bash
otter run program.ot            # Compile + execute
otter build program.ot -o app   # Emit native binary
otter fmt                       # Format .ot files
otter repl                      # Interactive REPL
otter test path/to/tests        # Run snapshot-style tests
```

Cross-compilation targets (including WebAssembly) are described in [docs/WEBASSEMBLY.md](docs/WEBASSEMBLY.md).

## Examples

Browse [docs/EXAMPLES.md](docs/EXAMPLES.md) and the `examples/` tree for runnable snippets that stress the parser, runtime, and FFI bridge.

## VSCode Extension

We ship a VS Code extension with syntax highlighting, snippets, diagnostics, and an LSP server. Installation and release notes live in [vscode-extension/README.md](vscode-extension/README.md).

## Documentation

- **[Language Specification](docs/LANGUAGE_SPEC.md)** – grammar, semantics, runtime model.
- **[Getting Started](docs/GETTING_STARTED.md)** – installation, CLI walkthrough, first project.
- **[Examples](docs/EXAMPLES.md)** – curated sample programs.
- **[Tutorials](docs/TUTORIALS.md)** – guided walkthroughs for specific topics.
- **[API Reference](docs/API_REFERENCE.md)** – stdlib module documentation.
- **[FFI Guide](docs/FFI_GUIDE.md)** – transparent Rust crate import workflow.
- **[WebAssembly](docs/WEBASSEMBLY.md)** – compiling OtterLang programs to WASM targets.

## Status

**Early Access (v0.1.0)** – experimental tooling, expect sharp edges.

### Known Limitations

- `match` arms do not support guard clauses (`case value if ...`) yet; only direct patterns are accepted.
- Transparent Rust FFI exposes functions and methods, but macros/proc-macros are ignored and structs/enums cross the boundary as opaque handles (see [FFI Guide](docs/FFI_GUIDE.md#limitations)).
- WebAssembly builds run without filesystem access or full FFI support, so many stdlib modules (`io`, `net`, `task`) are unavailable in that target ([details](docs/WEBASSEMBLY.md#limitations)).
- Building the toolchain currently requires LLVM 18 and Rust nightly because the bridge generator depends on nightly-only rustdoc features.
