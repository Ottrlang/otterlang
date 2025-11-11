# Installation Guide

## Using Nix (Recommended)

```bash
nix develop
cargo +nightly build --release
```

The Nix flake automatically provides Rust nightly, LLVM 18, and all dependencies.

## Manual Setup

**Prerequisites:**
- Rust (via rustup) - nightly required for FFI features
- LLVM 18

### macOS

```bash
brew install llvm@18
export LLVM_SYS_181_PREFIX=$(brew --prefix llvm@18)
export LLVM_SYS_180_PREFIX=$LLVM_SYS_181_PREFIX
export PATH="$LLVM_SYS_181_PREFIX/bin:$PATH"
rustup toolchain install nightly
cargo +nightly build --release
```

### Ubuntu/Debian

```bash
sudo apt-get install -y llvm-18 llvm-18-dev clang-18
export LLVM_SYS_181_PREFIX=/usr/lib/llvm-18
export LLVM_SYS_180_PREFIX=$LLVM_SYS_181_PREFIX
rustup toolchain install nightly
cargo +nightly build --release
```

### Fedora 43

```bash
sudo dnf -y install llvm18 llvm18-devel clang18
export LLVM_SYS_181_PREFIX=/usr/lib64/llvm18
export LLVM_SYS_180_PREFIX=$LLVM_SYS_181_PREFIX
rustup toolchain install nightly
cargo +nightly build --release
```

### Windows

```powershell
# Install LLVM 18.1 using llvmenv (recommended)
cargo install llvmenv --locked
llvmenv install 18.1
llvmenv global 18.1

# Set environment variables
$llvmPath = llvmenv prefix
$env:LLVM_SYS_181_PREFIX = $llvmPath
$env:LLVM_SYS_180_PREFIX = $llvmPath
$env:Path = "$llvmPath\bin;$env:Path"

# Alternative: Install using winget or Chocolatey
# winget install --id LLVM.LLVM --silent --accept-package-agreements --accept-source-agreements
# choco install llvm -y
# $env:LLVM_SYS_181_PREFIX = "C:\Program Files\LLVM"
# $env:LLVM_SYS_180_PREFIX = $env:LLVM_SYS_181_PREFIX
# $env:Path = "$env:LLVM_SYS_181_PREFIX\bin;$env:Path"

# Install Rust nightly
rustup toolchain install nightly
rustup default nightly

# Build
cargo +nightly build --release
```

**Note for Windows:** If using winget/Chocolatey, LLVM may be installed in `C:\Program Files\LLVM` or `C:\Program Files (x86)\LLVM`.

**Important:** On Windows, you must use the **x64 Native Tools Command Prompt for VS 2022** to build. The MSVC linker requires environment variables that are automatically set in the Developer Command Prompt. Open it from the Start menu, then navigate to your project directory and run the build commands. Regular PowerShell/CMD will not have the MSVC environment configured.

## After Building

Once the build completes successfully, you can:

**Run a program:**
```bash
cargo +nightly run --release --bin otterlang -- run examples/basic/hello.ot
```

**Build an executable:**
```bash
cargo +nightly run --release --bin otterlang -- build examples/basic/hello.ot -o hello
```

**Run tests:**
```bash
cargo +nightly test --release
```

**Use the compiler directly:**
```bash
# The binary is located at:
# target/release/otterlang (or target/release/otterlang.exe on Windows)
./target/release/otterlang run program.ot
# Or on Windows:
# target\release\otterlang.exe run program.ot
```

**Note:** If you built using Nix and get a "libffi.so.8" error, run through Nix:
```bash
nix develop
./target/release/otterlang run program.ot
```

