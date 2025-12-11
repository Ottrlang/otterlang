if (-not (Test-Path 'C:\Program Files\Microsoft Visual Studio\2022')) {
    throw "Visual Studio 2022 not found."
}

# Launch Visual Studio Developer Shell with x64 architecture
& 'C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1' -Arch amd64

# Create and move to contrib directory
$cwd = Get-Location
$contribPath = "$cwd\contrib"
if (-not (Test-Path $contribPath)) {
    New-Item -ItemType Directory -Path $contribPath | Out-Null
}
Set-Location $contribPath

# Download LLVM
if (-not (Test-Path "llvm")) {
    Write-Host "Downloading LLVM..."
    $llvmVersion = "18.1.8"
    $clang = "clang+llvm-$llvmVersion-x86_64-pc-windows-msvc"
    $archive = "$clang.tar.xz"
    $downloadUrl = "https://github.com/llvm/llvm-project/releases/download/llvmorg-$llvmVersion/$archive"
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archive -UseBasicParsing
    tar -xf $archive
    Move-Item $clang llvm
    Remove-Item $archive
}

# Ensure Ninja is available (vcpkg requires >= 1.13.1)
$ninjaDir = Join-Path $contribPath "ninja"
if (-not (Test-Path $ninjaDir)) {
    Write-Host "Downloading ninja build tool..."
    New-Item -ItemType Directory -Path $ninjaDir | Out-Null
    $ninjaVersion = "1.13.1"
    $ninjaArchive = Join-Path $contribPath "ninja-win.zip"
    $ninjaUrl = "https://github.com/ninja-build/ninja/releases/download/v$ninjaVersion/ninja-win.zip"
    $maxAttempts = 5
    for ($attempt = 1; $attempt -le $maxAttempts; $attempt++) {
        try {
            Invoke-WebRequest -Uri $ninjaUrl -OutFile $ninjaArchive -UseBasicParsing
            break
        } catch {
            if ($attempt -eq $maxAttempts) {
                throw $_
            }
            $delay = [Math]::Min(30, 5 * $attempt)
            Write-Host "Download failed (attempt $attempt). Retrying in $delay seconds..."
            Start-Sleep -Seconds $delay
        }
    }
    Expand-Archive -Path $ninjaArchive -DestinationPath $ninjaDir -Force
    Remove-Item $ninjaArchive
}

$env:PATH = "$ninjaDir;$env:PATH"

# Clone vcpkg
$vcpkgRoot = "vcpkg"
if (-not (Test-Path "$cwd\contrib\vcpkg")) {
    Write-Host "Cloning vcpkg..."
    git clone https://github.com/microsoft/vcpkg $vcpkgRoot | Out-Null
    & "$vcpkgRoot\bootstrap-vcpkg.bat" -disableMetrics | Out-Null
}

# Install libxml2 via vcpkg
$libDir = Join-Path $vcpkgRoot "installed\x64-windows\lib"
$primaryLib = Join-Path $libDir "libxml2.lib"
if (-not (Test-Path $primaryLib)) {
    Write-Host "Installing libxml2 via vcpkg..."
    & "$vcpkgRoot\vcpkg.exe" install libxml2:x64-windows --clean-after-build | Out-Null
    $staticName = Join-Path $libDir "libxml2s.lib"
    if (-not (Test-Path $staticName)) {
        Copy-Item $primaryLib $staticName
    }
}

Set-Location ..

Write-Host "Setting up environment..."
$env:LLVM_SYS_181_PREFIX = "$contribPath\llvm"
$env:LIB = "$contribPath\vcpkg\installed\x64-windows\lib;$env:LIB"
$env:LIBXML2_LIB_PATH = "$contribPath\vcpkg\installed\x64-windows\lib"
$env:INCLUDE = "$contribPath\vcpkg\installed\x64-windows\include\libxml2;$env:INCLUDE"
$env:LIBXML2_INCLUDE_PATH = "$contribPath\vcpkg\installed\x64-windows\include\libxml2"
$env:PATH = "$contribPath\llvm\bin;$env:PATH"
$env:RUSTFLAGS = "-C link-arg=legacy_stdio_definitions.lib -C link-arg=oldnames.lib"
