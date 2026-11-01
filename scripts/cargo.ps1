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

# A dev build links the release Qt DLLs but cargo does not copy them next to the
# executable, so a direct launch of target\<profile>\hydrus_client.exe dies with
# "Qt6Core.dll was not found" system-error popups. Self-heal that once per build:
# if the executable exists and Qt is not beside it, deploy the runtime there.
if ($code -eq 0) {
    $root = Split-Path -Parent $PSScriptRoot
    $targets = foreach ($candidate in 'release', 'debug') {
        $exe = Join-Path $root "target\$candidate\hydrus_client.exe"
        if ((Test-Path $exe) -and -not (Test-Path (Join-Path (Split-Path -Parent $exe) 'Qt6Core.dll'))) {
            Get-Item $exe
        }
    }
    $needsQt = $targets | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($needsQt) {
        $profileDir = Split-Path -Parent $needsQt.FullName
        $windeployqt = Join-Path $qt 'windeployqt.exe'
        if (Test-Path $windeployqt) {
            Write-Host "Deploying Qt runtime into $profileDir..."
            $deployArgs = @(
                '--no-translations'
                '--no-opengl-sw'
                '--no-system-d3d-compiler'
                '--dir', $profileDir
                $needsQt.FullName
            )
            & $windeployqt @deployArgs | Out-Null
            # Do not let windeployqt's exit code mask a successful build.
            $LASTEXITCODE = $code
        }
    }
}
$ErrorActionPreference = $previousPreference
exit $code
