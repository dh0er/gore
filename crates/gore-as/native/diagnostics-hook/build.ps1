param([switch]$UpdateEmbeddedAsset)

$ErrorActionPreference = 'Stop'
$Here = Split-Path -Parent $MyInvocation.MyCommand.Path
$MinHook = Join-Path $Here 'vendor/minhook/src'

function Resolve-NativeTool {
    param(
        [string]$Override,
        [string]$ReproducibleDefault,
        [string]$CommandName
    )
    if ($Override) { return $Override }
    if (Test-Path -LiteralPath $ReproducibleDefault -PathType Leaf) {
        return $ReproducibleDefault
    }
    $Command = Get-Command $CommandName -CommandType Application -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($Command) { return $Command.Source }
    throw "missing 64-bit MinGW tool $CommandName; set CC/CXX or add it to PATH"
}

$Cc = Resolve-NativeTool $env:CC 'C:\Strawberry\c\bin\gcc.exe' 'gcc.exe'
$Cxx = Resolve-NativeTool $env:CXX 'C:\Strawberry\c\bin\g++.exe' 'g++.exe'
$Flags = @('-O2', '-m64', "-I$Here/vendor/minhook/include", '-DUNICODE', '-D_UNICODE')

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "native command failed with exit code ${LASTEXITCODE}: $FilePath $($Arguments -join ' ')"
    }
}

Push-Location $Here
try {
    foreach ($Output in @('buffer.o', 'hook.o', 'trampoline.o', 'hde64.o', 'ashook.o', 'ashook.dll')) {
        Remove-Item -LiteralPath $Output -Force -ErrorAction SilentlyContinue
    }
    Invoke-Native $Cc ($Flags + @('-c', "$MinHook/buffer.c", '-o', 'buffer.o'))
    Invoke-Native $Cc ($Flags + @('-c', "$MinHook/hook.c", '-o', 'hook.o'))
    Invoke-Native $Cc ($Flags + @('-c', "$MinHook/trampoline.c", '-o', 'trampoline.o'))
    Invoke-Native $Cc ($Flags + @('-c', "$MinHook/hde/hde64.c", '-o', 'hde64.o'))
    Invoke-Native $Cxx ($Flags + @('-std=c++17', '-c', 'ashook.cpp', '-o', 'ashook.o'))
    Invoke-Native $Cxx @(
        '-shared', '-static', '-Wl,--no-insert-timestamp', '-o', 'ashook.dll',
        'ashook.o', 'buffer.o', 'hook.o', 'trampoline.o', 'hde64.o', '-lkernel32', '-luser32'
    )
    if (-not (Test-Path -LiteralPath 'ashook.dll' -PathType Leaf)) {
        throw 'native linker reported success but did not create ashook.dll'
    }
    $Built = Get-Item ashook.dll
    $Hash = Get-FileHash $Built.FullName -Algorithm SHA256
    $Built | Select-Object FullName, Length, LastWriteTime
    $Hash | Select-Object Algorithm, Hash, Path
    if ($UpdateEmbeddedAsset) {
        $Asset = Join-Path $Here '../../assets/gore-as-diagnostics-hook.dll'
        Copy-Item -LiteralPath $Built.FullName -Destination $Asset -Force
        $AssetHash = Get-FileHash $Asset -Algorithm SHA256
        if ($AssetHash.Hash -ne $Hash.Hash) {
            throw "embedded asset hash mismatch after copy"
        }
        $AssetHash | Select-Object Algorithm, Hash, Path
    }
} finally {
    Pop-Location
}
