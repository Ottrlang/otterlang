# OtterLang Project Structure

This document tracks the organization of the OtterLang codebase to help maintain consistency and navigation.

## Directory Structure

```
otterlang/
├── .github/
│   └── workflows/          # GitHub Actions CI/CD workflows
│       ├── ci.yml          # Continuous integration
│       └── release.yml     # Release automation
├── examples/               # Example OtterLang programs
│   ├── hello.otter
│   ├── advanced_pipeline.otter
│   ├── task_benchmark.otter
│   └── ...
├── ffi/                    # FFI bridge definitions
│   ├── chrono/             # Chrono crate bridge
│   ├── libm/               # Math library bridge
│   ├── nalgebra/          # Linear algebra bridge
│   ├── rand/               # Random number bridge
│   ├── rayon/              # Parallel processing bridge
│   ├── serde_json/         # JSON serialization bridge
│   └── otterlang_ffi_demo/ # Example FFI bridge
├── src/
│   ├── main.rs             # CLI entry point
│   ├── lib.rs              # Library root
│   ├── cli.rs              # Command-line interface
│   ├── version.rs          # Version constant
│   ├── ast/                # Abstract Syntax Tree
│   │   ├── mod.rs
│   │   └── nodes.rs        # AST node definitions
│   ├── lexer/              # Lexical analysis
│   │   ├── mod.rs
│   │   ├── token.rs        # Token definitions
│   │   └── tokenizer.rs     # Tokenizer implementation
│   ├── parser/             # Syntax analysis
│   │   ├── mod.rs
│   │   └── grammar.rs      # Chumsky grammar definitions
│   ├── codegen/            # LLVM code generation
│   │   ├── mod.rs
│   │   ├── llvm.rs         # LLVM IR generation
│   │   └── symbols.rs      # Symbol management
│   ├── runtime/            # Runtime and standard library
│   │   ├── mod.rs
│   │   ├── ffi.rs          # FFI runtime
│   │   ├── ffi_api.rs      # FFI API bindings
│   │   ├── strings.rs      # String runtime
│   │   ├── symbol_registry.rs # Symbol registry
│   │   ├── stdlib/         # Standard library implementations
│   │   │   ├── mod.rs
│   │   │   ├── builtins.rs # Built-in functions
│   │   │   ├── fmt.rs      # Formatting
│   │   │   ├── io.rs       # Input/output
│   │   │   ├── json.rs     # JSON handling
│   │   │   ├── math.rs     # Math functions
│   │   │   ├── net.rs      # Networking
│   │   │   ├── rand.rs     # Random numbers
│   │   │   ├── runtime.rs  # Runtime utilities
│   │   │   ├── sync.rs     # Synchronization
│   │   │   ├── sys.rs      # System calls
│   │   │   ├── task.rs     # Task concurrency
│   │   │   └── time.rs     # Time utilities
│   │   ├── task/           # Task runtime
│   │   │   ├── mod.rs
│   │   │   ├── channel.rs  # Task channels
│   │   │   ├── metrics.rs  # Task metrics
│   │   │   ├── scheduler.rs # Task scheduler
│   │   │   └── task.rs     # Task definitions
│   │   └── jit/            # JIT execution engine (experimental)
│   │       ├── mod.rs
│   │       ├── engine.rs
│   │       ├── executor.rs
│   │       ├── adaptive/   # Adaptive optimization
│   │       ├── cache/      # Function caching
│   │       ├── concurrency/ # Concurrency management
│   │       ├── layout/     # Layout optimization
│   │       ├── optimization/ # Code optimization
│   │       ├── profiler/   # Profiling
│   │       └── specialization/ # Code specialization
│   ├── cache/              # Compilation cache
│   │   ├── mod.rs
│   │   ├── manager.rs      # Cache manager
│   │   ├── metadata.rs     # Cache metadata
│   │   └── path.rs         # Cache path utilities
│   ├── ffi/                # FFI bridge infrastructure
│   │   ├── mod.rs
│   │   ├── cargo_bridge.rs # Cargo bridge builder
│   │   ├── dynamic_loader.rs # Dynamic library loader
│   │   ├── metadata.rs     # Bridge metadata
│   │   ├── rust_stubgen.rs # Rust stub generator
│   │   └── symbol_registry.rs # Bridge symbol registry
│   └── utils/              # Utility modules
│       ├── mod.rs
│       ├── bench.rs        # Benchmarking
│       ├── errors.rs       # Error diagnostics
│       ├── logger.rs       # Logging
│       ├── profiler.rs     # Profiling utilities
│       └── timer.rs        # Timing utilities
├── stdlib/otter/           # OtterLang standard library source
│   ├── builtins.otter
│   ├── io.otter
│   ├── json.otter
│   ├── math.otter
│   ├── net.otter
│   ├── rand.otter
│   ├── runtime.otter
│   ├── task.otter
│   └── time.otter
├── tests/                  # Integration tests
│   ├── cache_tests.rs
│   ├── ffi_tests.rs
│   ├── lexer_tests.rs
│   ├── parser_tests.rs
│   ├── performance_tests.rs
│   ├── stdlib_tests.rs
│   └── task_runtime.rs
├── Cargo.toml              # Rust project manifest
├── Cargo.lock              # Dependency lock file
├── LICENSE                 # MIT License
├── README.md               # Project documentation
├── CHANGELOG.md            # Version history
├── structure.md            # This file
└── plan.md                 # Development roadmap
```

## Key Components

### Compilation Pipeline

1. **Lexer** (`src/lexer/`): Tokenizes source code with indentation awareness
2. **Parser** (`src/parser/`): Builds AST from tokens using Chumsky
3. **Codegen** (`src/codegen/`): Generates LLVM IR from AST
4. **Cache** (`src/cache/`): Manages compilation caching for faster rebuilds

### Runtime Components

- **FFI System** (`src/runtime/ffi.rs`, `src/ffi/`): Handles Rust crate bridges
- **Standard Library** (`src/runtime/stdlib/`): Built-in functions and modules
- **Task Runtime** (`src/runtime/task/`): Concurrency primitives (experimental)
- **JIT Engine** (`src/runtime/jit/`): JIT execution engine (experimental)

### FFI Bridge System

FFI bridges allow importing Rust crates via `use rust:crate` syntax. Bridges are defined in `ffi/*/bridge.yaml` files and compiled into dynamic libraries.

## Module Organization Principles

1. **Separation of Concerns**: Each major phase (lexing, parsing, codegen) is isolated
2. **Runtime Separation**: Runtime code is separate from compiler code
3. **Test Organization**: Tests mirror source structure
4. **Example-Driven**: Examples demonstrate language features

## Future Structure Considerations

- **Module System**: When implemented, will add `src/module/` for module resolution
- **Package Manager**: May add `src/pkg/` for package management
- **Language Server**: Could add `src/lsp/` for IDE support

