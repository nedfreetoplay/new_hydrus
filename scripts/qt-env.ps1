# Shared Qt/MSVC discovery for the build, run and deploy scripts.
# Every lookup can be overridden from the environment:
#   HYDRUS_VS_VCVARS  full path to vcvars64.bat
#   HYDRUS_QT_BIN     Qt bin directory (contains Qt6Core.dll and windeployqt.exe)
#   QT_ROOT           Qt installation prefix (bin/qmake.exe)
#   QMAKE             full path to qmake.exe

function Find-VcVars {
    if ($env:HYDRUS_VS_VCVARS -and (Test-Path $env:HYDRUS_VS_VCVARS)) {
        return $env:HYDRUS_VS_VCVARS
    }

    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path $vswhere) {
        $install = & $vswhere -latest -products * `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
            -property installationPath 2>$null
        if ($install) {
            $candidate = Join-Path $install 'VC\Auxiliary\Build\vcvars64.bat'
            if (Test-Path $candidate) { return $candidate }
        }
    }

    $editions = 'Community', 'Professional', 'Enterprise', 'BuildTools'
    foreach ($edition in $editions) {
        $candidate = "C:\Program Files\Microsoft Visual Studio\2022\$edition\VC\Auxiliary\Build\vcvars64.bat"
        if (Test-Path $candidate) { return $candidate }
    }

    return $null
}

function Find-QtBin {
    if ($env:HYDRUS_QT_BIN -and (Test-Path (Join-Path $env:HYDRUS_QT_BIN 'Qt6Core.dll'))) {
        return $env:HYDRUS_QT_BIN
    }

    if ($env:QMAKE -and (Test-Path $env:QMAKE)) {
        return (Split-Path -Parent $env:QMAKE)
    }

    if ($env:QT_ROOT) {
        $bin = Join-Path $env:QT_ROOT 'bin'
        if (Test-Path (Join-Path $bin 'Qt6Core.dll')) { return $bin }
    }

    $kits = 'msvc2022_64', 'msvc2019_64', 'msvc2017_64'
    foreach ($kit in $kits) {
        foreach ($version in '6.12.0', '6.11.3', '6.10.3') {
            $bin = "C:\Qt\$version\$kit\bin"
            if (Test-Path (Join-Path $bin 'Qt6Core.dll')) { return $bin }
        }
    }

    # Any other Qt version installed under C:\Qt: pick the newest.
    $versions = Get-ChildItem 'C:\Qt' -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match '^\d+\.\d+\.\d+$' } |
        Sort-Object { [version]$_.Name } -Descending
    foreach ($version in $versions) {
        foreach ($kit in $kits) {
            $bin = Join-Path $version.FullName "$kit\bin"
            if (Test-Path (Join-Path $bin 'Qt6Core.dll')) { return $bin }
        }
    }

    return $null
}
