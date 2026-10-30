# Cargo target runner: puts Qt DLLs on PATH for `cargo run` and `cargo test`,
# then starts the requested program with the arguments cargo passed.
$ErrorActionPreference = 'Stop'

. "$PSScriptRoot\qt-env.ps1"

$qt = Find-QtBin
if (-not $qt) {
    Write-Error 'Qt not found. Install Qt 6 (MSVC x64) or set HYDRUS_QT_BIN / QT_ROOT / QMAKE.'
    exit 1
}

$env:PATH = "$qt;$env:PATH"

$program = $args[0]
$programArgs = @()
if ($args.Count -gt 1) { $programArgs = $args[1..($args.Count - 1)] }

& $program @programArgs
exit $LASTEXITCODE
