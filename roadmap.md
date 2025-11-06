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
  - Manual JIT control for predictable AOT performance with optional dynamic compilation
    - Explicit `jit` annotation for functions requiring JIT compilation
    - Allows predictable ahead-of-time speed for most code with JIT available for specific dynamic tasks
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

### Milestone 2.6: Embedded Development Features
- Strong type aliases (newtype pattern)
  - Create distinct types from base types (e.g., `UserId` from `int`, `FileDescriptor` from `int`)
  - Compile-time type checking prevents mixing semantically different types
  - Zero-cost abstraction: no runtime overhead, purely compile-time safety
  - Syntax: `type UserId = int` creates a strong alias (distinct from `int`)
- Defer statement for resource management
  - `defer` keyword ensures cleanup code runs when function exits (normal return or error)
  - Simplifies resource management (file handles, database connections, locks)
  - Multiple defer statements execute in reverse order (LIFO)
  - Complements error handling by guaranteeing cleanup even on early returns
- Critical section blocks
  - Built-in atomic block syntax for thread/coroutine-safe code sections
  - Ensures code runs without interruption for shared data structure updates
  - Prevents common concurrency bugs with clear, explicit syntax
  - Compiler-enforced synchronization guarantees
- Memory section control
  - Syntax to explicitly control variable placement in memory sections
  - Place large constant lookup tables in read-only sections (`.rodata`) instead of RAM
  - Support for `.data`, `.bss`, `.rodata`, and custom linker sections
  - Critical for embedded systems with limited RAM and strict memory constraints
- Cleaner bit manipulation syntax
  - Built-in syntax for common bit operations (set, clear, toggle, test bits)
  - Replace error-prone patterns like `CONFIG_REGISTER |= (1 << 5)` with readable syntax
  - Type-safe bit field operations with compile-time validation
  - Improves readability and reduces bugs in low-level embedded code

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
- Embedded development features
  - Strong type aliases prevent type confusion at compile time (e.g., `UserId` ≠ `ProductId` even if both are `int`)
  - `defer` statements guarantee resource cleanup even on error paths
  - Critical section blocks provide thread-safe code execution
  - Memory section control allows explicit placement of data in read-only sections
  - Bit manipulation syntax is readable and type-safe (no more `|= (1 << 5)` patterns)
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


