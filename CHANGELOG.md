# Changelog

All notable changes to OtterLang will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2024-XX-XX

### Added
- Initial early-access release
- Lexer with indentation-aware tokenization
- Parser using Chumsky parsing library
- LLVM code generation backend (LLVM 15)
- CLI with `run` and `build` commands
- Standard library modules: math, io, time, task, rand, json, net, fmt
- Rust FFI support via `use rust:crate` imports
- Compilation cache for faster rebuilds
- Basic error diagnostics with source location
- Support for functions, variables, control flow (if/else, for, while)
- String interpolation (f-strings)
- Lambda expressions
- Range expressions (`0..10`)
- `elif` branching support in if statements

### Limitations
- Module system only supports Rust FFI imports
- Type inference is limited
- Task runtime is experimental
- Windows support is experimental
- Requires LLVM 15 specifically

## Versioning Policy

**Early Access (v0.x.x)**: Breaking changes may occur without notice. The API and language syntax are subject to change.

**Pre-1.0 Releases**: 
- `0.MAJOR.MINOR` - Major features or breaking changes
- `0.MAJOR.PATCH` - Bug fixes and minor additions

**Post-1.0 Releases** (planned):
- `MAJOR.MINOR.PATCH` - Standard semantic versioning
- Breaking changes will increment MAJOR version

