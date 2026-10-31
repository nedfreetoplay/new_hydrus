<#
.SYNOPSIS
    Build a self-contained Windows folder (and optionally a zip) of the client.

.DESCRIPTION
    Copies hydrus_client.exe next to the Qt DLLs, Qt plugins and the MSVC
    runtime, so the result runs on a machine that has no Qt installed and no
    Qt on PATH. `qt.conf` makes the plugin lookup independent of the working
    directory.

.PARAMETER Profile
    debug or release (default: release). Debug builds link the debug CRT, which
    is not redistributable; use them only for a local layout check.

.PARAMETER OutDir
    Destination folder (default: <repo>\dist\hydrus_client).

.PARAMETER Zip
    Also write <OutDir>.zip for distribution.

.EXAMPLE
    .\scripts\deploy.ps1 -Zip
    .\scripts\deploy.ps1 -Profile debug -OutDir C:\temp\hydrus_portable
#>
[CmdletBinding()]
param(
    [ValidateSet('debug', 'release')]
    [string]$Profile = 'release',

    [string]$OutDir,

    [switch]$Zip
)

$ErrorActionPreference = 'Stop'

. "$PSScriptRoot\qt-env.ps1"

$root = Split-Path -Parent $PSScriptRoot
if (-not $OutDir) { $OutDir = Join-Path $root 'dist\hydrus_client' }
$OutDir = [IO.Path]::GetFullPath($OutDir)

$vs = Find-VcVars
if (-not $vs) {
    throw 'vcvars64.bat not found. Install the MSVC build tools or set HYDRUS_VS_VCVARS.'
}

$qtBin = Find-QtBin
if (-not $qtBin) {
    throw 'Qt not found. Install Qt 6 (MSVC x64) or set HYDRUS_QT_BIN / QT_ROOT / QMAKE.'
}

$windeployqt = Join-Path $qtBin 'windeployqt.exe'
if (-not (Test-Path $windeployqt)) {
    throw "windeployqt.exe not found in $qtBin"
}

Write-Host "Building hydrus_client ($Profile)..."
$buildArgs = @('build', '-p', 'hydrus_client')
if ($Profile -eq 'release') { $buildArgs += '--release' }
& "$PSScriptRoot\cargo.ps1" @buildArgs
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

$exe = Join-Path $root "target\$Profile\hydrus_client.exe"
if (-not (Test-Path $exe)) { throw "Built executable not found: $exe" }

if (Test-Path $OutDir) { Remove-Item -Recurse -Force $OutDir }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $OutDir 'plugins') | Out-Null
Copy-Item $exe (Join-Path $OutDir 'hydrus_client.exe')

Write-Host "Deploying Qt runtime from $qtBin..."
$deployArgs = @(
    "--$Profile"
    '--no-translations'
    '--no-opengl-sw'
    '--no-system-d3d-compiler'
    '--no-compiler-runtime'
    '--plugindir', (Join-Path $OutDir 'plugins')
    '--dir', $OutDir
    (Join-Path $OutDir 'hydrus_client.exe')
)
$command = 'call "{0}" >nul && set "PATH={1};%PATH%" && "{2}" {3}' -f `
    $vs, $qtBin, $windeployqt, ($deployArgs -join ' ')
$previousPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
cmd /c $command
$code = $LASTEXITCODE
$ErrorActionPreference = $previousPreference
if ($code -ne 0) { throw 'windeployqt failed' }

# windeployqt copies the vc_redist installer instead of the DLLs on current
# Qt versions; copy the runtime DLLs themselves so the folder is self-contained.
if ($Profile -eq 'release') {
    $crtScript = Join-Path $env:TEMP 'hydrus_deploy_crt.bat'
    $crtLines = @(
        '@echo off'
        ('call "{0}" >nul' -f $vs)
        ('for /d %%d in ("%VCToolsRedistDir%x64\Microsoft.VC*.CRT") do copy /y "%%d\*.dll" "{0}" >nul' -f $OutDir)
    )
    Set-Content -Path $crtScript -Encoding ASCII -Value $crtLines
    $ErrorActionPreference = 'Continue'
    cmd /c $crtScript
    $code = $LASTEXITCODE
    $ErrorActionPreference = $previousPreference
    Remove-Item -Force $crtScript -ErrorAction SilentlyContinue
    if ($code -ne 0) { throw 'Copying the MSVC runtime failed' }
    if (-not (Test-Path (Join-Path $OutDir 'vcruntime140.dll'))) {
        throw 'vcruntime140.dll was not copied; is the MSVC redistributable installed?'
    }
}

$qtConf = "[Paths]`r`nPrefix = .`r`nPlugins = plugins`r`n"
[IO.File]::WriteAllText(
    (Join-Path $OutDir 'qt.conf'),
    $qtConf,
    (New-Object System.Text.UTF8Encoding($false))
)

$platforms = Join-Path $OutDir 'plugins\platforms'
$platform = Get-ChildItem $platforms -Filter 'qwindows*.dll' -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $platform) { throw 'The Windows platform plugin was not deployed' }

# vendored Hydrus static assets (icons, stylesheets, ...) next to the exe
$staticSrc = Join-Path $root 'static'
$staticDst = Join-Path $OutDir 'static'
if (Test-Path $staticSrc) {
    if (Test-Path $staticDst) { Remove-Item -Recurse -Force $staticDst }
    Copy-Item -Recurse -Force $staticSrc $staticDst
    Write-Host 'Copied vendored static assets'
} else {
    Write-Warning 'static/ not found; icons fall back to painted approximations'
}

# SVG image format plugin (needed for *.svg icons such as star.svg / cog.svg)
$imageFormats = Join-Path $OutDir 'plugins\imageformats'
if (-not (Test-Path $imageFormats)) { New-Item -ItemType Directory -Force -Path $imageFormats | Out-Null }
if (-not (Test-Path (Join-Path $imageFormats 'qsvg.dll'))) {
    $qtPlugins = Join-Path (Split-Path -Parent $qtBin) 'plugins\imageformats'
    $sourceSvg = Join-Path $qtPlugins 'qsvg.dll'
    if (Test-Path $sourceSvg) {
        Copy-Item -Force $sourceSvg (Join-Path $imageFormats 'qsvg.dll')
        Write-Host 'Copied qsvg imageformat plugin'
    } else {
        Write-Warning "qsvg.dll not found in $qtPlugins"
    }
}

if ($Zip) {
    $zipPath = "$OutDir.zip"
    if (Test-Path $zipPath) { Remove-Item -Force $zipPath }
    Compress-Archive -Path (Join-Path $OutDir '*') -DestinationPath $zipPath
    Write-Host "Zip: $zipPath"
}

$size = (Get-ChildItem $OutDir -Recurse -File | Measure-Object Length -Sum).Sum / 1MB
Write-Host ('Portable build: {0} ({1:N1} MB)' -f $OutDir, $size)
