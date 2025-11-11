# OtterLang

<p>
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://github.com/jonathanmagambo/otterlang/blob/main/image.png?raw=true" width="400">
      <img src="https://github.com/jonathanmagambo/otterlang/blob/main/image.png?raw=true" width="400" alt="OtterLang Logo" />
    </picture>
    <br>
    <strong>Simple syntax, native performance, transparent Rust FFI.</strong>
</p>

[![Build Status](https://github.com/jonathanmagambo/otterlang/workflows/CI/badge.svg)](https://github.com/jonathanmagambo/otterlang/actions)
[![Discord](https://img.shields.io/badge/Discord-Join%20Server-5865F2?style=flat&logo=discord&logoColor=white)](https://discord.gg/y3b4QuvyFk)

An indentation-sensitive programming language with an LLVM backend. OtterLang compiles to native binaries with a focus on simplicity and performance.

## Quick Start

```bash
git clone https://github.com/jonathanmagambo/otterlang.git
cd otterlang

# Using Nix (recommended)
nix develop
cargo +nightly build --release

# Create and run your first program
cat > hello.ot << 'EOF'
def main():
    print("Hello from OtterLang!")
EOF

otter run hello.ot
```

## Installation

See [Installation Guide](docs/INSTALLATION.md) for detailed setup instructions for all platforms.

**Quick install with Nix:**
```bash
nix develop
cargo +nightly build --release
```

## Language Features

OtterLang features a clean, indentation-based syntax with modern language features:

- **Pythonic syntax** - `def` for functions, `class` for structs, `print()` for output
- **Type system** - Static typing with type inference
- **Enums and pattern matching** - Tagged unions with `match` expressions
- **Exception handling** - `try/except/finally` blocks with zero-cost abstractions
- **Concurrency** - `spawn` and `await` for async operations
- **Transparent Rust FFI** - Use any Rust crate without manual bindings

For complete syntax and language details, see the [Language Specification](docs/LANGUAGE_SPEC.md).

### Transparent Rust FFI

Automatically use any Rust crate without manual configuration. No manual bindings needed - just `use rust:crate_name` and start using it. See [docs/FFI_TRANSPARENT.md](docs/FFI_TRANSPARENT.md) for details.

### Standard Library

Built-in modules include `core` (Option, Result), `math`, `io`, `time`, and more. See the [API Reference](docs/API_REFERENCE.md) for complete documentation.


## CLI Commands

See [CLI Reference](docs/CLI.md) for all available commands.

```bash
otterlang run program.ot          # Run program
otterlang build program.ot -o out # Build executable
otterlang fmt                      # Format code
otterlang repl                     # Start REPL
```

OtterLang supports WebAssembly compilation. See [WebAssembly Support](docs/WEBASSEMBLY.md) for details.

## Examples

See [Examples](docs/EXAMPLES.md) for a complete list of example programs.

## VSCode Extension

OtterLang includes a full-featured VSCode extension with syntax highlighting, LSP support, and IDE features. See [vscode-extension/README.md](vscode-extension/README.md) for installation and usage.

## Documentation

- **[Installation Guide](docs/INSTALLATION.md)** - Setup instructions for all platforms
- **[Language Specification](docs/LANGUAGE_SPEC.md)** - Complete language reference
- **[CLI Reference](docs/CLI.md)** - Command-line interface documentation
- **[WebAssembly Support](docs/WEBASSEMBLY.md)** - Compiling to WebAssembly
- **[Examples](docs/EXAMPLES.md)** - Example programs
- **[Tutorials](docs/TUTORIALS.md)** - Step-by-step guides
- **[API Reference](docs/API_REFERENCE.md)** - Standard library documentation
- **[FFI Guide](docs/FFI_TRANSPARENT.md)** - Using Rust crates from OtterLang

## Status

**Early Access (v0.1.0)** - Experimental, not production-ready.

### Known Limitations

- Type inference is limited (explicit types recommended)
- Module system has some limitations
- Requires LLVM 18 and Rust nightly (for FFI features)

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT License - see [LICENSE](LICENSE).
