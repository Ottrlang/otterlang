# OtterLang 🦦

<p>
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://github.com/jonathanmagambo/otterlang/blob/main/image.png?raw=true" width="400">
      <img src="https://github.com/jonathanmagambo/otterlang/blob/main/image.png?raw=true" width="400" alt="OtterLang Logo" />
    </picture>
    <br>
    <strong>Simple like Python, fast with Rust, and everything in between.</strong>
</p>

[![Build Status](https://github.com/jonathanmagambo/otterlang/workflows/CI/badge.svg)](https://github.com/jonathanmagambo/otterlang/actions)

[![Discord](https://img.shields.io/badge/Discord-Join%20Server-5865F2?style=flat&logo=discord&logoColor=white)](https://discord.gg/y3b4QuvyFk)

An experimental indentation-sensitive programming language with an LLVM backend. OtterLang compiles to native binaries with a focus on simplicity and performance.

## Performance Benchmarks

Benchmarked against C and Rust using the [Leibniz formula for π](https://en.wikipedia.org/wiki/Leibniz_formula_for_%CF%80) (100,000,000 iterations):

| Language | Compiler | Avg Time (5 runs) | Relative to C |
| ------------- | ------------- | ------------- | ------------- |
| C | `gcc -O3` | 0.070s | 1.00x (baseline) |
| Rust | `rustc -O` | 0.080s | 1.14x |
| OtterLang | `otter --release` | 0.090s | 1.28x |

*Each run includes a warm-up execution (not timed). Benchmarks run on macOS with LLVM 15. Run `examples/benchmark.sh` to test yourself.*

**Note:** These benchmarks are run with a warm-up execution and may not be 100% accurate. Results can vary based on system load, CPU throttling, and other factors.

## Quick Setup

```bash
# Clone the repository
git clone https://github.com/jonathanmagambo/otterlang.git
cd otterlang

# Run the setup script
./setup.sh
```

The setup script will build and install the `otter` command globally. After installation, you can use `otter` from anywhere.

## Installation

### Prerequisites

OtterLang requires **LLVM 15** installed on your system. The compiler uses [inkwell](https://github.com/TheDan64/inkwell) which requires LLVM development libraries.

#### macOS

```bash
# Using Homebrew
brew install llvm@15

# Set environment variables (add to ~/.zshrc or ~/.bash_profile)
export LLVM_SYS_150_PREFIX=$(brew --prefix llvm@15)
export PATH="$LLVM_SYS_150_PREFIX/bin:$PATH"
```

#### Ubuntu/Debian

```bash
# Install LLVM 15
sudo apt-get update
sudo apt-get install -y llvm-15 llvm-15-dev clang-15

# Set environment variable
export LLVM_SYS_150_PREFIX=/usr/lib/llvm-15
```

#### Manual Build

```bash
# Clone the repository
git clone https://github.com/jonathanmagambo/otterlang.git
cd otterlang

# Build the compiler
cargo build --release

# Install globally
cargo install --path . --bin otter
```

## Quick Start

Create a simple program:

```otter
fn main:
    print("Hello from OtterLang!")
```

Save it as `hello.otter` and run:

```bash
otter run hello.otter
```

Or build a standalone executable:

```bash
otter build hello.otter -o hello
./hello
```

## Syntax Overview

OtterLang uses **indentation-based syntax** (similar to Python) with whitespace-sensitive blocks.

### Functions

```otter
fn greet(name: string) -> string:
    return f"Hello, {name}!"

fn main:
    message = greet("World")
    print(message)
```

### Variables and Types

```otter
fn main:
    # Numbers (floats)
    x = 42.0
    y = 3.14
    
    # Strings
    name = "Otter"
    
    # Booleans
    is_active = true
    
    # Type annotations (optional)
    count: int = 10
```

### Control Flow

```otter
fn main:
    x = 10.0
    
    # If/else
    if x > 5.0:
        print("x is greater than 5")
    else:
        print("x is less than or equal to 5")
    
    # For loops
    for i in 0..10:
        print(i)
    
    # While loops
    counter = 0.0
    while counter < 10.0:
        print(counter)
        counter = counter + 1.0
```

### F-Strings (String Interpolation)

```otter
fn main:
    name = "Otter"
    age = 3.0
    message = f"My name is {name} and I'm {age} years old"
    print(message)
```

### Standard Library Modules

OtterLang provides several built-in modules:

- **`otter:math`** - Mathematical functions (sin, cos, sqrt, etc.)
- **`otter:io`** - File I/O operations
- **`otter:fs`** - File system operations
- **`otter:http`** - HTTP client/server
- **`otter:time`** - Time utilities (now_ms, sleep, etc.)
- **`otter:task`** - Task-based concurrency
- **`otter:rand`** - Random number generation
- **`otter:json`** - JSON parsing and serialization
- **`otter:net`** - Networking (TCP)
- **`otter:fmt`** - Formatting utilities

```otter
use otter:math
use otter:time

fn main:
    value = math.sin(3.14 / 2.0)
    print(f"sin(π/2) = {value}")
    
    start = time.now_ms()
    time.sleep(1000)  # Sleep for 1 second
    elapsed = time.now_ms() - start
    print(f"Elapsed: {elapsed} ms")
```

### FFI (Foreign Function Interface)

OtterLang supports importing Rust crates through a Foreign Function Interface system. This allows you to use existing Rust libraries in your OtterLang programs.

#### Using FFI

Import Rust crates using the `rust:` namespace:

```otter
use rust:serde_json as json

fn main:
    # Use serde_json functions
    pass
```

#### How FFI Works

1. **Bridge Configuration**: Each Rust crate requires a `bridge.yaml` file in `ffi/<crate-name>/` that defines how to call Rust functions
2. **Automatic Compilation**: When you `use rust:crate`, OtterLang automatically:
   - Downloads the Rust crate via Cargo (if not already present)
   - Generates FFI bindings
   - Compiles the bridge library
   - Makes functions available to your OtterLang code
3. **Function Calling**: FFI functions are called just like regular OtterLang functions

#### Available FFI Bridges

OtterLang includes bridges for several popular Rust crates:

- `serde_json` - JSON serialization/deserialization
- `rand` - Random number generation
- `chrono` - Date and time handling
- `reqwest` - HTTP client
- `rusqlite` - SQLite database driver
- `postgres` - PostgreSQL database driver
- `rayon` - Parallel data processing
- `nalgebra` - Linear algebra library
- `libm` - Mathematical functions

#### Creating Custom FFI Bridges

To add support for a new Rust crate:

1. Create `ffi/<crate-name>/bridge.yaml`
2. Define function signatures and call expressions
3. See existing bridges in `ffi/` for examples

Example bridge.yaml structure:
```yaml
dependency:
  name: "crate_name"
  version: "1.0"
functions:
  - name: "function_name"
    params: ["F64", "Str"]
    result: "F64"
    doc: "Function description"
    call:
      kind: expr
      expr: "crate_name::function({0}, {1})"
```

See [docs/DATABASE.md](docs/DATABASE.md) for examples of using database drivers via FFI.

## Examples

See the `examples/` directory for complete examples:

- `advanced_pipeline.otter` - Complex computation pipeline
- `task_benchmark.otter` - Task runtime demonstration

## CLI Commands

```bash
# Run a program
otter run program.otter

# Build an executable
otter build program.otter -o output

# Format code
otter fmt

# Profile memory usage
otter profile memory program.otter

# Start REPL
otter repl

# Debug flags
otter run program.otter --dump-tokens    # Show token stream
otter run program.otter --dump-ast       # Show AST
otter run program.otter --dump-ir        # Show LLVM IR
otter run program.otter --time           # Show compilation timing
otter run program.otter --profile        # Show build profile

# Release mode (optimized)
otter build program.otter --release

# Cross-compilation
otter build program.otter --target wasm32-unknown-unknown
otter build program.otter --target thumbv7m-none-eabi
```

## Project Structure

```
otterlang/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── cli.rs           # Command handling
│   ├── lexer/           # Tokenizer
│   ├── parser/          # Chumsky-based parser
│   ├── ast/             # Abstract syntax tree
│   ├── typecheck/       # Type checking
│   ├── codegen/         # LLVM code generation
│   ├── runtime/         # Runtime and stdlib
│   ├── tools/           # Developer tools (profiler)
│   └── utils/           # Diagnostics and helpers
├── stdlib/otter/        # Standard library modules
├── examples/            # Example programs
├── ffi/                 # FFI bridge configurations
└── docs/                # Documentation
```

## Features

- ✅ Indentation-based syntax
- ✅ Type inference and type checking
- ✅ LLVM backend with optimization
- ✅ Standard library modules
- ✅ FFI support for Rust crates
- ✅ Memory management (GC, profiling)
- ✅ Cross-compilation (WASM, embedded)
- ✅ Task-based concurrency
- ✅ REPL
- ✅ Code formatting
- ✅ Memory profiling

## Current Limitations

⚠️ **Early Access Release** - OtterLang is experimental and subject to change.

### Known Limitations

1. **Module System**: Module imports from `.otter` files are supported (both relative paths like `use ./math` and stdlib modules like `use otter:math`). However, complex module resolution features may have limitations.

2. **Type System**: Type inference is limited. Explicit type annotations are recommended for complex code, especially for generic types and function signatures.

3. **Async/Tasks**: The task runtime is functional but some advanced features may be incomplete. Task spawning and channels are supported, but some edge cases may need refinement.

4. **Standard Library**: Most stdlib modules are implemented, but some may have incomplete implementations or missing edge cases. Check individual module documentation for details.

5. **Error Messages**: Error reporting is still being improved. Use `--dump-tokens` and `--dump-ast` for debugging.

6. **Platform Support**: Currently tested on macOS and Linux. Windows support is experimental.

7. **LLVM Dependency**: Requires LLVM 15 specifically. Other versions are not supported.

## Contributing

Contributions are welcome! This is an early-stage project, so expect breaking changes.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Status

**Early Access (v0.1.0)** - Not production-ready. Use at your own risk.

