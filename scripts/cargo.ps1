. "$PSScriptRoot\qt-env.ps1"

$vs = Find-VcVars
if (-not $vs) {
    Write-Error 'vcvars64.bat not found. Install the MSVC build tools or set HYDRUS_VS_VCVARS.'
    exit 1
}

$qt = Find-QtBin
if (-not $qt) {
    Write-Error 'Qt not found. Install Qt 6 (MSVC x64) or set HYDRUS_QT_BIN / QT_ROOT / QMAKE.'
    exit 1
}

$env:QMAKE = Join-Path $qt 'qmake.exe'
$env:CMAKE_PREFIX_PATH = Split-Path -Parent $qt

$command = 'call "{0}" >nul && set "PATH={1};%PATH%" && cargo {2}' -f $vs, $qt, ($args -join ' ')

# Native tools write progress and warnings to stderr; do not let a caller's
# $ErrorActionPreference = 'Stop' turn that into a terminating error.
$previousPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
cmd /c $command
$code = $LASTEXITCODE
$ErrorActionPreference = $previousPreference
exit $code
