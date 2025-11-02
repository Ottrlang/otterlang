# OtterLang Development Roadmap

This document tracks planned features and improvements for OtterLang.

## Current Status: Early Access (v0.1.0)

### ✅ Completed for EA Release

- [x] Core compiler pipeline (lexer, parser, codegen)
- [x] LLVM backend integration
- [x] CLI with run/build commands
- [x] Basic standard library modules
- [x] Rust FFI support
- [x] Compilation caching
- [x] Error diagnostics
- [x] `elif` statement support
- [x] Documentation and installation guides
- [x] CI/CD setup

### 🔄 In Progress / Experimental

- [ ] Task runtime (behind feature flag)
- [ ] JIT execution engine
- [ ] Windows platform support

## Short-Term Goals (v0.2.0)

### Module System

- [ ] Implement `.otter` file module imports
- [ ] Support for `use otter:module` syntax
- [ ] Module resolution and path handling
- [ ] Include module dependencies in cache fingerprint

### Language Features

- [ ] Match expressions (pattern matching)
- [ ] Struct/record types
- [ ] Array/list literals
- [ ] Dictionary/map literals
- [ ] Type aliases
- [ ] Generic type parameters

### Developer Experience

- [ ] REPL (read-eval-print loop)
- [ ] Formatter (`otter fmt`)
- [ ] Syntax highlighting (Tree-sitter or TextMate grammar)
- [ ] Better error messages with suggestions
- [ ] Debug mode with stack traces

## Medium-Term Goals (v0.3.0 - v0.5.0)

### Type System

- [ ] Type inference improvements
- [ ] Type checking phase
- [ ] Better type error messages
- [ ] Type annotations for generics

### Standard Library

- [ ] Complete all stdlib module implementations
- [ ] Async/await support
- [ ] File system operations
- [ ] HTTP client/server
- [ ] Database drivers (SQLite, Postgres)

### Performance

- [ ] Optimization passes
- [ ] Profile-guided optimization
- [ ] Link-time optimization (LTO)
- [ ] Function inlining

### Task Runtime

- [ ] Complete task runtime implementation (see TASK_RUNTIME_TODO.md)
- [ ] Work-stealing scheduler
- [ ] Typed channels
- [ ] Task cancellation
- [ ] Task-local storage

## Long-Term Goals (v1.0.0+)

### Package Management

- [ ] Package manager (`otterpkg` or similar)
- [ ] Dependency resolution
- [ ] Package registry
- [ ] Version management

### Tooling

- [ ] Language Server Protocol (LSP)
- [ ] Debugger integration
- [ ] Profiler tools
- [ ] Benchmarking framework

### Platform Support

- [ ] Full Windows support
- [ ] Cross-compilation
- [ ] WebAssembly target
- [ ] Embedded targets

### Documentation

- [ ] Language specification
- [ ] Standard library documentation
- [ ] Tutorial series
- [ ] API reference

## Experimental Features

### JIT Engine

- [ ] Hot function detection
- [ ] Adaptive optimization
- [ ] Code specialization
- [ ] Function caching

### Memory Management

- [ ] Garbage collection options
- [ ] Memory profiling
- [ ] Reference counting

## Known Issues & Technical Debt

1. **Module System**: Currently only Rust FFI imports work
2. **Type System**: Limited inference, needs type checking phase
3. **Error Messages**: Could be more helpful with suggestions
4. **Windows Support**: Experimental, needs testing
5. **Task Runtime**: Incomplete implementation

## Contributing

See the main README for contribution guidelines. Focus areas for contributors:

- Module system implementation
- Standard library completeness
- Error message improvements
- Documentation
- Testing coverage

## Versioning

See CHANGELOG.md for versioning policy details.

- **v0.x.x**: Early access, breaking changes allowed
- **v1.0.0+**: Stable API, semantic versioning

