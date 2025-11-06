## OtterLang Roadmap

This roadmap outlines major areas of investment and their intended scope. Items are prioritized by impact and feasibility and grouped by milestone tiers. Timelines are indicative and may evolve based on feedback.

### Guiding Principles
- Simplicity first: syntax and tooling should be easy to learn and hard to misuse
- Performance by default: native codegen, low overhead abstractions
- Safety with explicit control: predictable resource and error handling
- Batteries included: great stdlib, formatter, profiler, and REPL

---

### Milestone 1: Error Handling and Language Ergonomics
- Result/Option return-based errors
  - Add `Result<T, E>` and `Option<T>` as first-class types
  - Standard library helpers for mapping/combining results
  - CLI and formatter awareness
- Syntactic sugar for simplicity
  - `?` operator to early-return on error from `Result<T, E>`
  - `try` expression support for concise error flows
- Fatal internal errors
  - `panic`/`raise` for unrecoverable states with backtrace hooks
- Pattern matching integration
  - Exhaustive `match` on `Result`/`Option` and algebraic data types
  - Destructuring and guards in `match` arms

### Milestone 2: Runtime and JIT Capabilities
- Per-function JIT
  - Julia-style per-function JIT for hot paths
  - Tiered execution (interp/JIT/native) with profiling feedback
- Live module loading
  - Reloadable modules for REPL and long-lived processes
  - Stable ABI for safe hot-swapping within a process

### Milestone 2.5: Embeddable Runtime and Sandbox
- Bytecode interpreter and VM
  - Lightweight bytecode format for embedded use cases
  - Deterministic execution mode for reproducible behavior
  - Low memory footprint suitable for game engines
- Sandboxed execution environment
  - Capability-based security model (filesystem, network, time access)
  - Resource limits (memory, CPU, execution time)
  - Isolation between embedded OtterLang instances
- Rust host integration
  - C-compatible FFI for embedding in C/C++ projects
  - Safe Rust API for creating and managing VM instances
  - Callback system for host-to-guest and guest-to-host communication
- Game engine integration
  - Plugin system for embedding in game engines and applications
  - Native plugin compatibility via C API
- WASM target for safe embedding
  - Compile OtterLang to WebAssembly for browser and WASM runtimes
  - Sandboxed execution in WASM environments

### Milestone 3: Tooling and Developer Experience
- Testing framework
  - Built-in test runner (`otter test`) and assertion library
  - Snapshot testing hooks and timing output
- Package manager (otterpkg)
  - Dependency resolution and version management
  - Local and remote package registry support
  - Lockfile generation and reproducible builds
  - Integration with `otter build` and module system
  - Project manifest (`Otter.toml`) for package metadata, dependencies, scripts, and targets
- Documentation and website
  - Official documentation site with search and examples
  - Interactive API reference with live code samples
  - Tutorials and guides for common use cases
  - Package registry browser and search
- Diagnostics and observability
  - Rich error messages with spans and suggestions
  - Built-in tracing and structured logs
- Formatter and LSP
  - Incremental formatter improvements
  - First-party LSP features (hover, go-to-def, diagnostics)

### Milestone 4: Libraries and Ecosystem
- Standard library depth
  - Collections, iterators, async utilities
  - IO/FS/Net ergonomics and safety improvements
- FFI bridges
  - Wider crate support and improved metadata extraction
  - Sandboxed loading and permission controls

---

### Acceptance Criteria (Examples)
- Error handling
  - A function returning `Result<T, E>` can be composed with `?`
  - `match` on `Result` is exhaustive and type-checked
- JIT
  - A hot function receives JIT compilation with measurable speedup
  - Live module swapping retains state isolation guarantees
- Embeddable runtime
  - VM can be instantiated from Rust with configurable sandbox permissions
  - OtterLang code runs in isolated environment with resource limits
  - Plugin system successfully loads and executes OtterLang scripts in host applications
  - WASM-compiled OtterLang runs in browser with sandboxed I/O
- Testing
  - `otter test` discovers and runs tests, returning non-zero on failure
  - Assertions report clear diffs and spans
- Package manager
  - `otterpkg init` creates a new project with dependencies
  - `otterpkg add <package>` resolves and installs dependencies
  - `otterpkg build` uses lockfile for reproducible builds
  - `Otter.toml` is parsed and validated (name, version, deps, scripts, targets)
  - `otterpkg run <script>` executes scripts defined in `Otter.toml`
- Documentation
  - Website hosts searchable docs with interactive examples
  - Package registry is browsable and searchable

### Out of Scope (for now)
- Distributed runtime
- Advanced macro system or compile-time evaluation

### Feedback
Feedback is welcome via issues and discussions. Priorities will be adjusted based on real-world usage.


