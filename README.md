# OtterLang

OtterLang is an experimental indentation-sensitive programming language. This repository contains a minimal compiler prototype capable of lexing, parsing, and JIT executing a "Hello from OtterLang" program using LLVM via [inkwell](https://github.com/TheDan64/inkwell).

## Usage

```bash
cargo run -- run examples/hello.otter
```

## Layout

The project is organised as follows:

- `src/main.rs` – entry point that wires the CLI.
- `src/cli.rs` – command handling and pipeline orchestration.
- `src/lexer` – whitespace-aware tokenizer.
- `src/parser` – Chumsky-based grammar producing the AST.
- `src/ast` – typed syntax tree definitions.
- `src/codegen` – LLVM code generation using inkwell.
- `src/utils` – diagnostics and shared helpers.
- `examples/hello.otter` – sample program executed by the pipeline.
