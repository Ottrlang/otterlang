# OtterLang 🦦

An experimental indentation-sensitive programming language with an LLVM backend. OtterLang compiles to native binaries with a focus on simplicity and performance.

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

#### Build from Source

```bash
# Clone the repository
git clone https://github.com/jonathanmagambo/otterlang.git
cd otterlang

# Build the compiler
cargo build --release

# Install globally (optional)
cargo install --path .
```

The compiled binary will be available at `target/release/otterlang` or `target/release/otter` (depending on your configuration).

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
- **`otter:time`** - Time utilities (now_ms, sleep, etc.)
- **`otter:task`** - Task-based concurrency (experimental)
- **`otter:rand`** - Random number generation
- **`otter:json`** - JSON parsing and serialization
- **`otter:net`** - Networking (HTTP, TCP)
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

Import Rust crates using the `rust:` namespace:

```otter
use rust:serde_json as json

fn main:
    # Use serde_json functions
    pass
```

## Examples

See the `examples/` directory for more complete examples:

- `hello.otter` - Basic "Hello, World!"
- `advanced_pipeline.otter` - Complex computation pipeline
- `task_benchmark.otter` - Task runtime demonstration (experimental)

## Project Structure

```
otterlang/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── cli.rs           # Command handling
│   ├── lexer/           # Tokenizer
│   ├── parser/          # Chumsky-based parser
│   ├── ast/             # Abstract syntax tree
│   ├── codegen/         # LLVM code generation
│   ├── runtime/         # Runtime and stdlib
│   └── utils/           # Diagnostics and helpers
├── stdlib/otter/        # Standard library modules
├── examples/            # Example programs
└── tests/               # Test suite
```

## CLI Commands

```bash
# Run a program
otter run program.otter

# Build an executable
otter build program.otter -o output

# Debug flags
otter run program.otter --dump-tokens    # Show token stream
otter run program.otter --dump-ast       # Show AST
otter run program.otter --dump-ir        # Show LLVM IR
otter run program.otter --time           # Show compilation timing
otter run program.otter --profile         # Show build profile

# Release mode (optimized)
otter build program.otter --release
```

## Current Limitations

⚠️ **Early Access Release** - OtterLang is experimental and subject to change.

### Known Limitations

1. **Module System**: Only Rust FFI imports (`use rust:crate`) are fully supported. General module imports from `.otter` files are not yet implemented.

2. **Type System**: Type inference is limited. Explicit type annotations are recommended for complex code.

3. **Async/Tasks**: The task runtime is experimental and behind the `task-runtime` feature flag. Not all task features are fully implemented.

4. **Standard Library**: Some stdlib modules may have incomplete implementations.

5. **Error Messages**: Error reporting is still being improved. Use `--dump-tokens` and `--dump-ast` for debugging.

6. **Platform Support**: Currently tested on macOS and Linux. Windows support is experimental.

7. **LLVM Dependency**: Requires LLVM 15 specifically. Other versions are not supported.

## Contributing

Contributions are welcome! This is an early-stage project, so expect breaking changes.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Status

**Early Access (v0.1.0)** - Not production-ready. Use at your own risk.
